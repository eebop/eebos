use core::{cell::RefCell, cmp, fmt::{Debug, Write}, ops::Range, result};

use alloc::{alloc::{Allocator, Global}, borrow::ToOwned, boxed::Box, collections::{btree_map::BTreeMap, btree_set::BTreeSet}, fmt::format, rc::Rc, slice, string::{ParseError, String, ToString}, vec::Vec};

use elf::{ElfBytes, abi::STB_GLOBAL, dynamic::DynamicTable, endian::{AnyEndian, EndianParse}, segment::ProgramHeader, string_table::StringTable, symbol::{self, Symbol, SymbolTable}};
use shared::{Interface, SysCallData, process::{Page, PageAligned, Process}, screen::Screen, std::{DummyAllocator, ManualOnceCell}};


macro_rules! elf_data {
    ($data:ident) => {
        {
            unsafe extern "C" {
                static mut ${ concat(_binary_, $data, _start) }: u8;
                static mut ${ concat(_binary_, $data, _end  ) }: u8; 
            }
            &raw mut ${ concat(_binary_, $data, _start) }..&raw mut ${ concat(_binary_, $data, _end ) }
        }
    }
}

macro_rules! elf_map {
    ($( $i:ident ),*) => {
        [
            $( (concat!("lib", stringify!($i), ".so"), elf_data!($i)) ),*
        ]
    };
}

const _ELF_DATA: &'static [(&'static str, Range<*mut u8>)] = &elf_map!(test_mod, test_dep, paging, dyshared);

static ELF_DATA: ManualOnceCell<BTreeMap<&str, &[u8]>> = ManualOnceCell::new();

pub fn init_elf_data() {
    let mut data = BTreeMap::<&str, &[u8]>::new();
    for entry in _ELF_DATA {
        // Unsafe here is fine because it's garenteed to be a valid slice at link time
        // We just can't make it a slice because code can't run at link time
        data.insert(entry.0, unsafe { slice::from_mut_ptr_range(entry.1.clone()) } );
    }
    unsafe { ELF_DATA.init(data) };
}

// pub fn load_syscall(mut info: SysCallData) {
//     let out = string_elf(info.receive_abi(), DummyAllocator);
//     info.send_abi(out);
// }

pub struct Module {
    pub allocations: Vec<Box<[Page]>>,
    pub init_fns: Vec<u32>,
    pub fini_fns: Vec<u32>,
    pub symbols: BTreeMap<String, (Symbol, Relocation)>
}

pub fn load_mod(name: &str) -> Module {
    // Topological sort of so
    let mut all: BTreeMap<&str, SOChunk> = BTreeMap::new();
    let mut leaves: BTreeSet<&str> = BTreeSet::new();
    leaves.insert(name);

    while let Some(leaf) = leaves.pop_first() {
        let chunk = string_elf(leaf);
        for edge in &chunk.needed {
            leaves.insert(*edge);
        }
        all.insert(leaf, chunk);
    }

    let mut output: Vec<&str> = Vec::new();
    let mut heads: BTreeSet<&str> = BTreeSet::new();
    heads.insert(name);
    while let Some(curr) = heads.pop_first() {
        output.push(curr);
        for target in &all[curr].needed {
            if all.iter().filter(|(_, chunk)| chunk.needed.contains(target)).all(|(name, _)| output.contains(&name)) {
                heads.insert(*target);
            }
        }
    }
    writeln!(Screen::new(), "order is: (last first) {output:?}");
    let mut symbols: BTreeMap<&str, (Symbol, Relocation)> = BTreeMap::new();
    let mut init_fns = Vec::new();
    let mut fini_fns = Vec::new();
    let mut allocations = Vec::new();
    for val in output.iter().rev() {
        writeln!(Screen::new(), "Now linking: {:?}", val);
        let mut object = all.remove(val).unwrap();
        relocate_mod(&mut object, &mut symbols).unwrap();
        init_fns.append(&mut object.init_fns);
        fini_fns.append(&mut object.fini_fns);
        allocations.push(object.allocation);

    }

    let map = BTreeMap::from_iter(symbols.into_iter().map(|(k, v)| (k.to_string(), v)));
    Module { allocations, init_fns, fini_fns, symbols: map }

}

fn string_elf(name: &str) -> SOChunk {
    let data = *ELF_DATA.get().get(name).unwrap_or_else(|| panic!("Invalid elf: \"{}\". Valid are {:?}", name, ELF_DATA.get().keys()));
    parse_elf(data).unwrap()
}

fn reinterpret_slice<T, U>(i: &[T]) -> Result<&[U], IntepretError> {
    let size = i.len() * size_of::<T>();
    if size % size_of::<U>() != 0 {
        return Err(IntepretError::LayoutError("Array size was not a multiple of element size".to_string()));
    }
    let newsize = size / size_of::<U>();
    unsafe {
        let ptr = i.as_ptr() as *const U;
        Ok(slice::from_raw_parts(ptr, newsize))
    }
}

fn reinterpret_slice_mut<T, U>(i: &mut [T]) -> Result<&mut [U], IntepretError> {
    let size = i.len() * size_of::<T>();
    if size % size_of::<U>() != 0 {
        return Err(IntepretError::LayoutError("Array size was not a multiple of element size".to_string()));
    }
    let newsize = size / size_of::<U>();
    unsafe {
        let ptr = i.as_ptr() as *mut U;
        Ok(slice::from_raw_parts_mut(ptr, newsize))
    }
}

#[derive(Debug)]
enum IntepretError {
    Parse(elf::ParseError),
    InvalidElfState(String),
    MistargetedElf(String),
    LayoutError(String),
    SymbolError(String)
}

impl From<elf::ParseError> for IntepretError {
    fn from(value: elf::ParseError) -> Self {
        Self::Parse(value)
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
struct Rel32 {
    offset: u32,
    info: u32 // Least 8 bits are type
              // Greatest 24 bytes are symbol index
}

enum RelocSize {
    Word8,
    Word16,
    Word32
}

impl Rel32 {
    fn get_type(&self) -> u8 {
        return (self.info & 0xFF) as u8;
    }
    fn get_size(&self) -> RelocSize {
        RelocSize::Word32
    }
    fn get_symbol(&self) -> u32 {
        return self.info >> 8;
    }
}

impl Debug for Rel32 {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Rel32")
            .field("type", &self.get_type())
            .field("symbol", &self.get_symbol())
            .field("offset", &self.offset)
            .finish()
    }
}

// range object which contains a pointer instead of a index,
// so it has to be offset by the location in memory
#[derive(Debug, Clone, Copy)]
struct PtrSubslice {
    start: usize,
    len: usize
}

impl PtrSubslice {
    fn into_range(&self, elfzero: usize) -> Range<usize> {
        return (self.start - elfzero)..(self.start - elfzero + self.len)
    }
    fn maybe_from(start: Option<usize>, len: Option<usize>) -> Result<Option<Self>, IntepretError> {
        if let (Some(start), Some(len)) = (start, len) {
            Ok(Some(PtrSubslice { start: start, len: len}))
        } else if let (None, None) = (start, len) {
            Ok(None)
        } else {
            Err(IntepretError::InvalidElfState("Unmatched array or arraysz".to_string()))
        }
    }
}
struct DynamicStruct<'data> {
    init_fns: Vec<u32>,
    fini_fns: Vec<u32>,
    init_array: Option<PtrSubslice>,
    fini_array: Option<PtrSubslice>,
    rel_array: &'data [Rel32],
    jmprel_array: &'data [Rel32], 
    needed: Vec<&'data str>
}

fn get_dynamic_data<'data, E: EndianParse>(code: &'data [u8], table: DynamicTable<E>) -> Result<DynamicStruct<'data>, IntepretError> {
    let mut init_fns: Vec<u32> = Vec::new();
    let mut init_ptr: Option<usize> = None; // these really should be u64 but we are in 32 bit mode so there's not even a way to load a module > 2^31 bits
    let mut init_size: Option<usize> = None;

    let mut fini_fns: Vec<u32> = Vec::new();
    let mut fini_ptr: Option<usize> = None;
    let mut fini_size: Option<usize> = None;

    let mut needed_offsets: Vec<usize> = Vec::new();

    let mut needed_strs: Vec<&'data str> = Vec::new();

    let mut strtab: Option<usize> = None;
    let mut strsz: Option<usize> = None;

    let mut relptr: Option<usize> = None;
    let mut relsz: Option<usize> = None;

    let mut jmprelptr: Option<usize> = None;
    let mut jmprelsz: Option<usize> = None;


    for symbol in table {
        match symbol.d_tag {
            elf::abi::DT_NEEDED => {
                let index = symbol.d_val() as usize;
                needed_offsets.push(index);
                // let dynstr = file.section_data_as_strtab(&file.section_header_by_name(".dynstr")?.unwrap())?;
                // let target = dynstr.get(index).unwrap();
                // panic!("NEED: TARGET: {target}");
            },
            elf::abi::DT_SONAME => {
                // Name of the SO. Irrelevent
            }
            elf::abi::DT_FLAGS => {
                // None of the settings are meaningful yet
            },
            elf::abi::DT_FLAGS_1 => {

            }
            elf::abi::DT_REL => {
                relptr = Some(symbol.d_ptr() as usize);
            }
            elf::abi::DT_RELSZ => {
                relsz = Some(symbol.d_val() as usize);
            },
            elf::abi::DT_RELENT => {
                assert!(symbol.d_val() == 8);
            },
            elf::abi::DT_RELCOUNT => {
                // Something to do with optimizations. Ignore!
            }
            elf::abi::DT_STRTAB => {
                strtab = Some(symbol.d_ptr() as usize);
            },
            elf::abi::DT_STRSZ => {
                strsz = Some(symbol.d_val() as usize);
            }
            elf::abi::DT_SYMTAB => {
                // Instead, we use .dynsym. This is not quite correct, but doing it with SYMTAB is harder due to hashing
            },
            elf::abi::DT_SYMENT => {
                // Symbol size
            },
            elf::abi::DT_INIT_ARRAY => {
                init_ptr = Some(symbol.d_ptr() as usize);
            },
            elf::abi::DT_INIT_ARRAYSZ => {
                init_size = Some(symbol.d_val() as usize);
            },
            elf::abi::DT_FINI_ARRAY => {
                fini_ptr = Some(symbol.d_ptr() as usize);
            },
            elf::abi::DT_FINI_ARRAYSZ => {
                fini_size = Some(symbol.d_val() as usize);
            },
            elf::abi::DT_INIT => {
                init_fns.push(symbol.d_ptr() as u32);
            },
            elf::abi::DT_FINI => {
                fini_fns.push(symbol.d_ptr() as u32);
            },
            elf::abi::DT_JMPREL => {
                // DT_REL that are interobject
                jmprelptr = Some(symbol.d_ptr() as usize);
            },
            elf::abi::DT_PLTRELSZ => {
                jmprelsz = Some(symbol.d_val() as usize);
            }
            elf::abi::DT_GNU_HASH => {

            }
            elf::abi::DT_DEBUG => {
                // Debug not used
            },
            elf::abi::DT_PLTREL => {
                // Whether to use REL or RELA relocations
                assert!(symbol.d_val() == elf::abi::DT_REL as u64);
            },
            elf::abi::DT_PLTGOT => {
                // Pointer to the start of GOT
            }
            elf::abi::DT_NULL => {
                // Ignored, internal record-keeping
            }
            _ => {
                panic!("Unknown dynamic symbol: {:?}", symbol.d_tag);
            }
            
        }
    };
    if needed_offsets.len() != 0 {
        let (Some(strtab), Some(strsz)) = (strtab, strsz) else {
            return Err(IntepretError::InvalidElfState("Need strtab and strsz to load a NEEDED so".to_string()));
        };
        let slice = &code[strtab..(strtab+strsz)];
        let dyntab = StringTable::new(slice);
        for offset in needed_offsets {
            needed_strs.push(dyntab.get(offset)?);
        }
    }


    let init_array = PtrSubslice::maybe_from(init_ptr, init_size)?;

    let fini_array = PtrSubslice::maybe_from(fini_ptr, fini_size)?;

    let rel_array = PtrSubslice::maybe_from(relptr, relsz)?
        .map(|x| &code[x.into_range(0)])
        .map(|x| reinterpret_slice::<u8, Rel32>(x))
        .transpose()?
        .unwrap_or(&[]);

    let jmprel_array = PtrSubslice::maybe_from(jmprelptr, jmprelsz)?
        .map(|x| &code[x.into_range(0)])
        .map(|x| reinterpret_slice::<u8, Rel32>(x))
        .transpose()?
        .unwrap_or(&[]);

    return Ok(DynamicStruct { init_fns, fini_fns, init_array: init_array, fini_array: fini_array, rel_array: rel_array, jmprel_array: jmprel_array, needed: needed_strs });
}

#[derive(Clone, Copy, Debug)]
pub struct Relocation {
    pub original_baseaddr: u32,
    pub new_baseaddr: *mut u8
}

impl Relocation {
    pub fn relocate_ptr(&self, addr: u32) -> *mut u8 {
        unsafe { self.new_baseaddr.add((addr - self.original_baseaddr) as usize) }
    }
    pub fn relocate_slice(&self, addr: u32) -> u32 {
        addr - self.original_baseaddr
    }
}

struct SOChunk<'data> {
    init_fns: Vec<u32>,
    fini_fns: Vec<u32>,
    rel_array: &'data [Rel32],
    jmprel_array: &'data [Rel32],
    needed: Vec<&'data str>,
    dynsymtab: SymbolTable<'data, AnyEndian>,
    dynstrtab: StringTable<'data>,
    allocation: Box<[Page]>,
    baseaddr: Relocation
}

fn parse_elf(code: &[u8]) -> Result<SOChunk, IntepretError> {
    let file = ElfBytes::<AnyEndian>::minimal_parse(code)?;

    let x = file.segments().expect("Can't get segments!");

    let got = file.section_header_by_name(".got")?.expect(".got currently required as is necessary for PIE");

    let mut loads: Vec<ProgramHeader> = Vec::new();

    let mut earliest: Option<u32> = None;
    let mut latest: Option<u32> = None;

    let mut dyn_data= None;

    for header in x {
        match header.p_type {
            elf::abi::PT_PHDR => {
                // elf table-size record-keeping; ignore
            },
            elf::abi::PT_LOAD => {
                match earliest {
                    Some(e) => {
                        earliest = Some(cmp::min(e, header.p_vaddr as u32))
                    },
                    None => {
                        earliest = Some(header.p_vaddr as u32)
                    }
                }
                match latest {
                    Some(l) => {
                        latest = Some(cmp::max(l, header.p_vaddr as u32 + header.p_memsz as u32))
                    },
                    None => {
                        latest = Some(header.p_vaddr as u32 + header.p_memsz as u32)
                    }
                }
                loads.push(header);
            },
            elf::abi::PT_DYNAMIC => {
                let dynam = file.dynamic().unwrap().unwrap();
                dyn_data = Some(get_dynamic_data(code, dynam)?);

            },
            elf::abi::PT_NOTE => {
                // pass
            },
            elf::abi::PT_GNU_STACK => {
                // TODO: set the RWX flags of sections
                // (stack)
            },
            elf::abi::PT_GNU_RELRO => {
                // TODO: set the RWX flags of sections
                // (GOT)
            },
            elf::abi::PT_GNU_EH_FRAME => {
                // Something to do with stack unwinding
                // Unwinding is not yet supported!
            }
            other => {
                panic!("Unknown program header: {:x}", other);
            }
            
        }
    }

    let mut dyn_data = dyn_data.ok_or(IntepretError::MistargetedElf("No dynamic data".to_string()))?;

    assert_ne!(loads.len(), 0);

    let earliest = earliest.unwrap() as usize;
    let latest = latest.unwrap() as usize;

    // It'd be better to just allocate the sections we need instead of inclusively
    let num_pages = usize::div_ceil((latest - earliest) as usize, 0x1000);

    let mut owned_data = Page::uninit_many(num_pages as usize, Global);

    let new_earliest = owned_data.as_ptr() as usize; 


    let array = owned_data.as_contiguous();

    for header in loads {
        let start = header.p_vaddr as usize - earliest as usize;
        array[start..][..header.p_filesz as usize].copy_from_slice(&code[header.p_offset as usize..][..header.p_filesz as usize]);
        array[start..][header.p_filesz as usize ..header.p_memsz as usize].fill(0);
    }

    // if let (Some(rinit_ptr), Some(rinit_size)) = (init_ptr, init_size) {
    //     let subslice = &array[rinit_ptr - earliest..][..rinit_size];


    
        // let ptrbuf = reinterpret_slice::<u8, u32>(subslice).expect("Malformed INIT_ARRAY directive");
    if let Some(init_array) = dyn_data.init_array {
        let subslice = &array[init_array.into_range(earliest)];

        dyn_data.init_fns.extend_from_slice(reinterpret_slice::<u8, u32>(subslice)?);
    }

    if let Some(fini_array) = dyn_data.fini_array {
        let subslice = &array[fini_array.into_range(earliest)];
        let mut tmp = reinterpret_slice::<u8, u32>(subslice)?.to_vec();
        tmp.append(&mut dyn_data.fini_fns);
        dyn_data.fini_fns = tmp;
    }

    let got_data = &mut array[got.sh_addr as usize - earliest as usize..][..got.sh_size as usize];

    let got_data = reinterpret_slice_mut::<u8, u32>(got_data).expect(".got must contain 32 bit dwords");

    if let Some(dyn_header) = file.section_header_by_name(".dynamic").unwrap() {
        // First element must point to dynamic header, if it exists
        got_data[0] = dyn_header.sh_addr as u32 - earliest as u32 + new_earliest as u32;
    }

    let (dynsymtab, dynstrtab) = file.dynamic_symbol_table()?.ok_or(IntepretError::MistargetedElf("No dyn symbol table".to_string()))?;

    let relocation = Relocation {original_baseaddr: earliest as u32, new_baseaddr: new_earliest as *mut u8};

    Ok(SOChunk {
        init_fns: dyn_data.init_fns,
        fini_fns: dyn_data.fini_fns,
        rel_array: dyn_data.rel_array,
        jmprel_array: dyn_data.jmprel_array,
        needed: dyn_data.needed,
        dynsymtab,
        dynstrtab,
        allocation: owned_data,
        baseaddr: relocation
    })
}

fn get_bytes_at_symbol<const N: usize>(slice: &[u8], ptr: u32) -> [u8; N] {
    slice[(ptr as usize)..][..N].try_into().unwrap()
}

fn set_bytes_at_symbol<T>(slice: &mut [u8], ptr: u32, data: T) -> Result<(), IntepretError> {
    reinterpret_slice_mut(&mut slice[(ptr as usize)..][..core::mem::size_of::<T>()])?[0] = data;
    Ok(())
}

fn relocate_mod<'data>(chunk: &mut SOChunk<'data>, symbols: &mut BTreeMap<&'data str, (Symbol, Relocation)>) -> Result<(), IntepretError> {
    for symbol in chunk.dynsymtab.clone() {
        let name = chunk.dynstrtab.get(symbol.st_name as usize)?;
        let curr = symbols.get(name);
        if symbol.is_undefined() {
            continue;
        }
        match symbol.st_bind() {
            elf::abi::STB_LOCAL => {
                continue;
            },
            elf::abi::STB_GLOBAL => {
                if let Some(prev) = curr {
                    // Replace weak symbols
                    if prev.0.st_vis() != elf::abi::STB_WEAK {
                        return Err(IntepretError::SymbolError(format!("Invalid symbol overload: {}", name)));
                    }
                }
                symbols.insert(name, (symbol, chunk.baseaddr));
            },
            elf::abi::STB_WEAK => {
                if let None = curr {
                    symbols.insert(name, (symbol, chunk.baseaddr));
                }
            }
            _ => {
                return Err(IntepretError::InvalidElfState("Unknown symbol visibility".to_string()));
            }
        }
    }

    let data = chunk.allocation.as_contiguous();

    for reloc in chunk.rel_array.iter().chain(chunk.jmprel_array.iter()) {
        let ptr = chunk.baseaddr.relocate_slice(reloc.offset);
        match reloc.get_size() {
            RelocSize::Word32 => {
                let addend: [u8; 4]  = get_bytes_at_symbol(data, ptr);
                let addend: u32 = u32::from_le_bytes(addend);

                let get_name =
                    || chunk.dynsymtab.get(reloc.get_symbol() as usize)
                    .map(|symbol| chunk.dynstrtab.get(symbol.st_name as usize))
                    .flatten();

                let get_symbol = 
                    || chunk.dynsymtab.get(reloc.get_symbol() as usize)
                    .map(|symbol| chunk.dynstrtab.get(symbol.st_name as usize)
                        // If there is a known one, use that, otherwise use our UND
                        .map(|name| symbols.get(name).cloned().unwrap_or((symbol, chunk.baseaddr))))
                    .flatten();
                // writeln!(Screen::new(), "now relocating... {:?}", reloc);

                let result = match reloc.get_type() {
                    // R_386_32
                    1 => {
                        let val = get_symbol()?;
                        writeln!(Screen::new(), "R_386_32 {}", get_name()?);
                        if val.0.is_undefined() && val.0.st_bind() != elf::abi::STB_WEAK {
                            return Err(IntepretError::InvalidElfState(format!("Symbol missing: {}", get_name()?)));
                        }
                        unsafe { val.1.relocate_ptr(val.0.st_value as u32).add(addend as usize) }
                    }
                    // R_386_GLOB_DAT
                    6 => {
                        let val = get_symbol()?;
                        writeln!(Screen::new(), "R_386_GLOB_DAT {}", get_name()?);
                        if val.0.is_undefined() && val.0.st_bind() != elf::abi::STB_WEAK {
                            return Err(IntepretError::InvalidElfState(format!("Symbol missing: {}", get_name()?)));
                        }
                        val.1.relocate_ptr(val.0.st_value as u32)
                    },
                    // R_386_JUMP_SLOT
                    7 => {
                        // Same as GLOB_DAT but we can lazy link
                        // We don't because that's harder
                        let val = get_symbol()?;
                        writeln!(Screen::new(), "R_386_JUMP_SLOT {}", get_name()?);
                        if val.0.is_undefined() && val.0.st_bind() != elf::abi::STB_WEAK {
                            return Err(IntepretError::InvalidElfState(format!("Symbol missing: {}", get_name()?)));
                        }
                        val.1.relocate_ptr(val.0.st_value as u32)
                    }
                    // R_386_RELATIVE
                    8 => {
                        writeln!(Screen::new(), "R_386_RELATIVE");
                        unsafe { chunk.baseaddr.new_baseaddr.sub(chunk.baseaddr.original_baseaddr as usize).add(addend as usize) }
                    }
                    
                    _ => {panic!("unkown symbol: {:?}", reloc); }
                };
                set_bytes_at_symbol(data, ptr, result)?
            }
            _ => panic!()
        }
    }
    
    Ok(())
}