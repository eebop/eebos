use core::{cell::{Cell, OnceCell, RefCell}, cmp, fmt::{Debug, Write}, iter, ops::{Index, Range}, ptr::NonNull, result};

use alloc::{alloc::{AllocError, Allocator, Global}, borrow::ToOwned, boxed::Box, collections::{btree_map::BTreeMap, btree_set::BTreeSet}, fmt::format, slice, string::{String, ToString}, vec::Vec};
use alloc::format;

use elf::{ElfBytes, ParseError, abi::STB_GLOBAL, dynamic::DynamicTable, endian::{AnyEndian, EndianParse}, file::FileHeader, segment::ProgramHeader, string_table::StringTable, symbol::{self, Symbol, SymbolTable}};
use dyshared::{Page, PageAligned, screen::Screen, CAllocator};
use paging::{MappedPageAllocator, TransToPhys};

use crate::data_repr::{DataLoc, SharedDataRepr};


#[derive(Debug)]
pub struct Module<LA: CAllocator, DA: MappedPageAllocator> {
    pub allocation: Box<[Page], DA::A>,
    // pub init_fns: Vec<u32, LA>,
    // pub fini_fns: Vec<u32, LA>,
    pub symbols: BTreeMap<String, (Symbol, Relocation), LA>,
    pub fhdr: FileHeader<AnyEndian>
}


pub enum AllocationStrategy {
    Exact,
    ArbitraryKernel
}

#[derive(Clone, Debug)]
enum OneOf<Item, A: Iterator<Item=Item> + Clone, B: Iterator<Item=Item> + Clone> {
    A(A),
    B(B)
}

impl<Item, A: Iterator<Item=Item> + Clone, B: Iterator<Item=Item> + Clone> Iterator for OneOf<Item, A, B> {
    type Item = Item;
    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::A(inner) => {
                inner.next()
            },
            Self::B(inner) => {
                inner.next()
            }
        }
    }
}

pub fn load_mod<'a, LA: CAllocator, DA: MappedPageAllocator>(name: &'a str, lookup: &impl Index<&'a str, Output=&'a [u8]>, alloc_strat: AllocationStrategy, local_alloc: LA, data_alloc: &mut DA) -> Module<LA, DA> {
    // Topological sort of so
    let mut all: BTreeMap<&str, SOChunk<LA>, _> = BTreeMap::new_in(local_alloc.clone());
    let mut leaves: BTreeSet<&str, _> = BTreeSet::new_in(local_alloc.clone());
    leaves.insert(name);
    

    // let x = lookup[name];

    while let Some(leaf) = leaves.pop_first() {

        writeln!(Screen, "leaf is: {:?}", leaf);
        let chunk = parse_elf(lookup[leaf], local_alloc.clone()).unwrap_or_else(|e| {writeln!(Screen, "ERROR: parse_elf: {e:?}"); panic!() });

        leaves.extend(&chunk.needed);

        all.insert(leaf, chunk);
    }

    let fhdr = all[name].phdr;

    let mut output: Vec<&str, _> = Vec::new_in(local_alloc.clone());
    let mut heads: BTreeSet<&str, _> = BTreeSet::new_in(local_alloc.clone());
    heads.insert(name);

    while let Some(curr) = heads.pop_first() {

        output.push(curr);
        for target in &all[curr].needed {
            if all.iter().filter(|(_, chunk)| chunk.needed.contains(target)).all(|(name, _)| output.contains(&name)) {
                heads.insert(*target);
            }
        }
    }


    let num_pages = all.iter()
        .map(|(_, chunk)| chunk.data.len().div_ceil(size_of::<Page>()))
        .sum::<usize>();

    let data = all.iter().map(|(name, chunk)| chunk);
    // let alloc = joint_alloc(&data, alloc_strat, local_alloc, data_alloc);

    // let mut arena: Box<[Page], <DA as MappedPageAllocator>::A> = match alloc_strat {
    //     AllocationStrategy::Exact => data_alloc.allocate_many(joint_alloc_exact(data, local_alloc), paging::PageType::Write),
    //     AllocationStrategy::ArbitraryKernel => data_alloc.allocate_many(joint_alloc_kernel(data, local_alloc), paging::PageType::Write),
    // }.unwrap();

    let data = match alloc_strat {
        AllocationStrategy::Exact => OneOf::A(joint_alloc_exact(data, local_alloc.clone())),
        AllocationStrategy::ArbitraryKernel => OneOf::B(joint_alloc_kernel(data, local_alloc.clone()))
    };

    // writeln("")

    let data = data.zip(all.iter());

    // let mut doc = Vec::new_in(local_alloc.clone());
    // data.clone().collect_into(&mut doc);
    // writeln!(Screen, "cloned data is: {doc:x?}");

    let new = data.clone()
        .map(|(page, (_, chunk))| (page as usize, chunk.data.len()))
        .map(|(ptr, size)| ptr..(ptr + size))
        .map(|range| range.step_by(size_of::<Page>()))
        .flatten()
        .map(|ptr| ptr as *mut Page);


    let mut doc = Vec::new_in(local_alloc.clone());
    new.clone().collect_into(&mut doc);
    writeln!(Screen, "now allocating for data_alloc, new is: {:?}", doc);
    let mut arena = data_alloc.allocate_many(new, num_pages, paging::PageType::Write).unwrap();

    let mut borrow = &mut *arena;

    let start = 0;
    let mut allocations = BTreeMap::new_in(local_alloc.clone());

    for (ptr, (name, obj)) in data {
        let size = obj.data.len().div_ceil(size_of::<Page>());
        let (this, tmp) = borrow.split_at_mut(size);
        borrow = tmp;
        let orig = obj.data.get_baseaddr();
        let reloc = Relocation {
            original_baseaddr: orig as u32,
            new_baseaddr: ptr as *mut u8
        };
        obj.data.write(this, reloc);
        allocations.insert(*name, (this, reloc));
    }



    writeln!(Screen, "order is: (last first) {output:?}");
    let mut symbols: BTreeMap<&str, (Symbol, Relocation), _> = BTreeMap::new_in(local_alloc.clone());
    // let mut init_fns = Vec::new_in(local_alloc.clone());
    // let mut fini_fns = Vec::new_in(local_alloc.clone());
    for val in output.into_iter().rev() {
        writeln!(Screen, "Now linking: {:?}", val);
        let mut object = all.remove(val).unwrap();
        let (data, reloc) = allocations.remove(val).unwrap();
        writeln!(Screen, "relocating: {:?}", val);
        relocate_mod(&mut object, reloc, data, &mut symbols).unwrap();
        // init_fns.append(&mut object.init_fns);
        // fini_fns.append(&mut object.fini_fns);
    }


    let mut map = BTreeMap::new_in(local_alloc.clone());
    symbols.into_iter().map(|(k, v)| (k.to_string(), v)).collect_into(&mut map);
    // todo!()
    Module { allocation: arena, /* init_fns, fini_fns, */ symbols: map, fhdr }

}

// fn string_elf(name: &str) -> SOChunk {
//     let data = *ELF_DATA.get().get(name).unwrap_or_else(|| panic!("Invalid elf: \"{}\". Valid are {:?}", name, ELF_DATA.get().keys()));
//     parse_elf(data).unwrap()
// }

fn reinterpret_slice<T, U>(i: &[T]) -> Result<&[U], InterpretError> {
    let size = i.len() * size_of::<T>();
    if size % size_of::<U>() != 0 {
        return Err(InterpretError::LayoutError("Array size was not a multiple of element size".to_string()));
    }
    let newsize = size / size_of::<U>();
    unsafe {
        let ptr = i.as_ptr() as *const U;
        Ok(slice::from_raw_parts(ptr, newsize))
    }
}

fn reinterpret_slice_mut<T, U>(i: &mut [T]) -> Result<&mut [U], InterpretError> {
    let size = i.len() * size_of::<T>();
    if size % size_of::<U>() != 0 {
        return Err(InterpretError::LayoutError("Array size was not a multiple of element size".to_string()));
    }
    let newsize = size / size_of::<U>();
    unsafe {
        let ptr = i.as_ptr() as *mut U;
        Ok(slice::from_raw_parts_mut(ptr, newsize))
    }
}

#[allow(unused)]
#[derive(Debug)]
enum InterpretError {
    Parse(elf::ParseError),
    InvalidElfState(String),
    MistargetedElf(String),
    LayoutError(String),
    SymbolError(String),
    AllocError(String)
}

impl From<elf::ParseError> for InterpretError {
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
    // Word8,
    // Word16,
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
    start: *mut u32,
    len: usize // in bytes
}

impl PtrSubslice {
    fn into_range(&self, elfzero: *const u32) -> Range<usize> {
        let diff = unsafe { self.start.offset_from(elfzero) } as usize;
        return (diff)..(diff + self.len)
    }

    fn into_ptr_slice(&self) -> *mut [u32] {
        unsafe { slice::from_raw_parts_mut(self.start, self.len.div_exact(size_of::<u32>()).unwrap()) }
    }

    fn maybe_from(start: Option<*mut u32>, len: Option<usize>) -> Result<Option<Self>, InterpretError> {
        if let (Some(start), Some(len)) = (start, len) {
            Ok(Some(PtrSubslice { start: start, len: len}))
        } else if let (None, None) = (start, len) {
            Ok(None)
        } else {
            Err(InterpretError::InvalidElfState("Unmatched array or arraysz".to_string()))
        }
    }
}

fn joint_alloc_exact<'b, 'a: 'b, LA: CAllocator + 'b>(data: impl Iterator<Item=&'b SOChunk<'a, LA>> + Clone, local_alloc: LA) -> impl Iterator<Item = *mut Page> + Clone {
    data.map(|val| val.data.get_baseaddr() as *mut Page)
}

fn joint_alloc_kernel<'b, 'a: 'b, LA: CAllocator + 'b>(data: impl Iterator<Item=&'b SOChunk<'a, LA>> + Clone, local_alloc: LA) -> impl Iterator<Item = *mut Page> + Clone {
    data.scan(0x80000000, |coord, chunk| {
        let size = chunk.data.len();
        let out = *coord;
        *coord += size.next_multiple_of(0x1000);
        Some(out as *mut Page)
    })
}

struct DynamicStruct<'data, A: CAllocator> {
    init_fn: Option<u32>,
    fini_fn: Option<u32>,
    init_array: Option<PtrSubslice>,
    fini_array: Option<PtrSubslice>,
    rel_array: &'data [Rel32],
    jmprel_array: &'data [Rel32], 
    needed: Vec<&'data str, A>
}

fn get_dynamic_data<'data, E: EndianParse, LA: CAllocator>(code: &'data [u8], table: DynamicTable<E>, a: LA) -> Result<DynamicStruct<'data, LA>, InterpretError> {
    let mut init_fn: Option<u32> = None;
    let mut init_ptr: Option<*mut u32> = None;
    let mut init_size: Option<usize> = None;

    let mut fini_fn: Option<u32> = None;
    let mut fini_ptr: Option<*mut u32> = None;
    let mut fini_size: Option<usize> = None;

    let mut needed_offsets: Vec<usize, _> = Vec::new_in(a.clone());

    let mut needed_strs: Vec<&'data str, _> = Vec::new_in(a.clone());

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
                init_ptr = Some(symbol.d_ptr() as *mut u32);
            },
            elf::abi::DT_INIT_ARRAYSZ => {
                // size, in bytes, of DT_INIT_ARRAY section
                init_size = Some(symbol.d_val() as usize);
            },
            elf::abi::DT_FINI_ARRAY => {
                fini_ptr = Some(symbol.d_ptr() as *mut u32);
            },
            elf::abi::DT_FINI_ARRAYSZ => {
                fini_size = Some(symbol.d_val() as usize);
            },
            elf::abi::DT_INIT => {
                init_fn = Some(symbol.d_ptr() as u32);
            },
            elf::abi::DT_FINI => {
                fini_fn = Some(symbol.d_ptr() as u32);
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
            return Err(InterpretError::InvalidElfState("Need strtab and strsz to load a NEEDED so".to_string()));
        };
        let slice = &code[strtab..(strtab+strsz)];
        let dyntab = StringTable::new(slice);
        for offset in needed_offsets {
            needed_strs.push(dyntab.get(offset)?);
        }
    }


    let init_array = PtrSubslice::maybe_from(init_ptr, init_size)?;

    let fini_array = PtrSubslice::maybe_from(fini_ptr, fini_size)?;

    fn slice_option(ptr: Option<usize>, byte_len: Option<usize>) -> Option<Range<usize>> {
        if let (Some(ptr), Some(len)) = (ptr, byte_len) {
            return Some(ptr..(ptr+len))
        } else if let (None, None) = (ptr, byte_len) {
            return None
        } else {
            panic!("misssing ptr without size")
        }
    }

    let rel_array = slice_option(relptr, relsz)
        .map(|x| &code[x])
        .map(|x| reinterpret_slice::<u8, Rel32>(x))
        .transpose()?
        .unwrap_or_default();

    let jmprel_array = slice_option(jmprelptr, jmprelsz)
        .map(|x| &code[x])
        .map(|x| reinterpret_slice::<u8, Rel32>(x))
        .transpose()?
        .unwrap_or_default();

    return Ok(DynamicStruct { init_fn, fini_fn, init_array: init_array, fini_array: fini_array, rel_array: rel_array, jmprel_array: jmprel_array, needed: needed_strs });
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

#[derive(Debug)]
struct SOChunk<'data, LA: CAllocator> {
    // init_fns: Vec<u32, LA>,
    // fini_fns: Vec<u32, LA>,
    rel_array: &'data [Rel32],
    jmprel_array: &'data [Rel32],
    needed: Vec<&'data str, LA>,
    dynsymstr: Option<(SymbolTable<'data, AnyEndian>, StringTable<'data>)>,
    data: SharedDataRepr<'data, LA>,
    phdr: FileHeader<AnyEndian>
}

fn parse_elf<'data, LA: CAllocator>(code: &'data [u8], local_alloc: LA) -> Result<SOChunk<'data, LA>, InterpretError> {

    // assert!((&raw const *code)());

    let file = ElfBytes::<AnyEndian>::minimal_parse(code)?;

    let x = file.segments().expect("Can't get segments!");

    let mut loads: Vec<ProgramHeader, _> = Vec::new_in(local_alloc.clone());

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
                // let dynam = file.dynamic().expect("parsing error").expect("found dynamic section");
                let (start, len) = (header.p_offset as usize, header.p_filesz as usize);
                let buf = &code[start..(start + len)];
                let dynam = DynamicTable::new(
                file.ehdr.endianness,
                    file.ehdr.class,
                    buf,
                );

                dyn_data = Some(get_dynamic_data(code, dynam, local_alloc.clone())?);

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


    assert_ne!(loads.len(), 0);

    let earliest = earliest.unwrap() as usize;
    assert!(earliest % 0x1000 == 0);
    let latest = latest.unwrap() as usize;

    // It'd be better to just allocate the sections we need instead of inclusively
    let num_pages = usize::div_ceil((latest - earliest) as usize, 0x1000);

    // let mut owned_data = data_alloc.allocate_many(earliest as *mut Page, paging::PageType::Write, num_pages as usize)
        // .map_err(|e| InterpretError::AllocError("Error putting data into pages".to_string()))?;


    // let new_earliest = owned_data.as_ptr() as usize; 


    // let array = owned_data.as_contiguous();

    let mut repr = SharedDataRepr::new_in(local_alloc.clone());

    for header in loads {
        let slice = &code[header.p_offset as usize..][..header.p_filesz as usize];
        let data = DataLoc::new(slice, header.p_vaddr as *mut u8, header.p_filesz as usize, header.p_memsz as usize);
        repr.insert(data);
        // let start = header.p_vaddr as usize - earliest as usize;
        // array[start..][..header.p_filesz as usize].copy_from_slice(&code[header.p_offset as usize..][..header.p_filesz as usize]);
        // array[start..][header.p_filesz as usize ..header.p_memsz as usize].fill(0);
    }

    // let mut init_fns = Vec::new_in(local_alloc.clone());
    // let mut fini_fns = Vec::new_in(local_alloc.clone());
    let mut rel_array: &'data [Rel32] = &[];
    let mut jmprel_array: &'data [Rel32] = &[];
    let mut needed = Vec::new_in(local_alloc.clone());
    if let Some(dyn_data) = dyn_data {
        // if let Some(init_fn) = dyn_data.init_fn {
        //     init_fns.push(init_fn);
        // }
        // if let Some(init_array) = dyn_data.init_array {
        //     let range = init_array.into_ptr_slice();

        //     // let len = range.len().div_exact(size_of::<u32>()).unwrap();

        //     init_fns.extend(iter::repeat(0).take(range.len()));
            
        //     let start = init_fns.len() - range.len();
        //     let slice = reinterpret_slice_mut::<u32, u8>(&mut init_fns[start..])?;
        //     repr.get_slice(slice, range);
        // }

        // if let Some(fini_array) = dyn_data.fini_array {
        //     let range = fini_array.into_ptr_slice();

        //     // let len = range.len().div_exact(size_of::<u32>()).unwrap();

        //     fini_fns.extend(iter::repeat(0).take(range.len()));

        //     let start = fini_fns.len() - range.len();
        //     let slice = reinterpret_slice_mut::<u32, u8>(&mut fini_fns[start..])?;
        //     repr.get_slice(slice, range);
        // }
        // if let Some(fini_fn) = dyn_data.init_fn {
        //     init_fns.push(fini_fn);
        // }
        
        rel_array = dyn_data.rel_array;
        jmprel_array = dyn_data.jmprel_array;
        needed = dyn_data.needed;
    }

    // let got = file.section_header_by_name(".got")?.expect(".got currently required as is necessary for PIE");

    // let got_data = &mut array[got.sh_addr as usize - earliest as usize..][..got.sh_size as usize];

    // let got_data = reinterpret_slice_mut::<u8, u32>(got_data).expect(".got must contain 32 bit dwords");

    // if let Some(dyn_header) = file.section_header_by_name(".dynamic").unwrap() {
    //     // First element must point to dynamic header, if it exists
    //     got_data[0] = dyn_header.sh_addr as u32 - earliest as u32 + new_earliest as u32;
    // }

    let dynsymstr = file.dynamic_symbol_table()?;

    // let relocation = Relocation {original_baseaddr: earliest as u32, new_baseaddr: new_earliest as *mut u8};

    Ok(SOChunk {
        // init_fns,
        // fini_fns,
        rel_array,
        jmprel_array,
        needed,
        dynsymstr,
        data: repr,
        phdr: file.ehdr,
    })
}

fn get_bytes_at_symbol<const N: usize>(slice: &[u8], ptr: u32) -> [u8; N] {
    slice[(ptr as usize)..][..N].try_into().unwrap()
}

fn set_bytes_at_symbol<T>(slice: &mut [u8], ptr: u32, data: T) -> Result<(), InterpretError> {
    reinterpret_slice_mut(&mut slice[(ptr as usize)..][..core::mem::size_of::<T>()])?[0] = data;
    Ok(())
}

fn relocate_mod<'data, LA: CAllocator>(chunk: &mut SOChunk<'data, LA>, baseaddr: Relocation, allocation: &mut [Page], symbols: &mut BTreeMap<&'data str, (Symbol, Relocation), LA>) -> Result<(), InterpretError> {
    if let Some((ref dynsymtab, ref dynstrtab)) = chunk.dynsymstr {
        for symbol in dynsymtab.clone() {
            let name = dynstrtab.get(symbol.st_name as usize)?;
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
                            return Err(InterpretError::SymbolError(format!("Invalid symbol overload: {}", name)));
                        }
                    }
                    symbols.insert(name, (symbol, baseaddr));
                },
                elf::abi::STB_WEAK => {
                    if let None = curr {
                        symbols.insert(name, (symbol, baseaddr));
                    }
                }
                _ => {
                    return Err(InterpretError::InvalidElfState("Unknown symbol visibility".to_string()));
                }
            }
        }
    }

    let data = allocation.as_contiguous();

    for reloc in chunk.rel_array.iter().chain(chunk.jmprel_array.iter()) {
        let ptr = baseaddr.relocate_slice(reloc.offset);
        match reloc.get_size() {
            RelocSize::Word32 => {
                let addend: [u8; 4]  = get_bytes_at_symbol(data, ptr);
                let addend: u32 = u32::from_le_bytes(addend);

                let get_name =
                    || if let Some((ref dynsymtab, ref dynstrtab)) = chunk.dynsymstr {
                        dynsymtab.get(reloc.get_symbol() as usize)
                        .map(|symbol| dynstrtab.get(symbol.st_name as usize))
                        .flatten()
                    } else {
                        panic!("Error: no name for rel {reloc:?}")
                    };

                let get_symbol = 
                    || if let Some((ref dynsymtab, ref dynstrtab)) = chunk.dynsymstr {
                        dynsymtab.get(reloc.get_symbol() as usize)
                        .map(|symbol| dynstrtab.get(symbol.st_name as usize)
                            // If there is a known one, use that, otherwise use our UND
                            .map(|name| symbols.get(name).cloned().unwrap_or((symbol, baseaddr))))
                        .flatten()
                    } else {
                        panic!("Error: no symbol for rel {reloc:?}")
                    };

                let result = match reloc.get_type() {
                    // R_386_32
                    1 => {
                        let val = get_symbol()?;
                        // writeln!(Screen, "R_386_32 {}", get_name()?);
                        if val.0.is_undefined() && val.0.st_bind() != elf::abi::STB_WEAK {
                            return Err(InterpretError::InvalidElfState(format!("Symbol missing: {}", get_name()?)));
                        }
                        unsafe { val.1.relocate_ptr(val.0.st_value as u32).add(addend as usize) }
                    }
                    // R_386_GLOB_DAT
                    6 => {
                        let val = get_symbol()?;
                        // writeln!(Screen, "R_386_GLOB_DAT {}", get_name()?);
                        if val.0.is_undefined() && val.0.st_bind() != elf::abi::STB_WEAK {
                            return Err(InterpretError::InvalidElfState(format!("Symbol missing: {}", get_name()?)));
                        }
                        val.1.relocate_ptr(val.0.st_value as u32)
                    },
                    // R_386_JUMP_SLOT
                    7 => {
                        // Same as GLOB_DAT but we can lazy link
                        // We don't because that's harder
                        let val = get_symbol()?;
                        // writeln!(Screen, "R_386_JUMP_SLOT {}", get_name()?);
                        if val.0.is_undefined() && val.0.st_bind() != elf::abi::STB_WEAK {
                            return Err(InterpretError::InvalidElfState(format!("Symbol missing: {}", get_name()?)));
                        }
                        val.1.relocate_ptr(val.0.st_value as u32)
                    }
                    // R_386_RELATIVE
                    8 => {
                        // writeln!(Screen, "R_386_RELATIVE");
                        unsafe { baseaddr.new_baseaddr.sub(baseaddr.original_baseaddr as usize).add(addend as usize) }
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