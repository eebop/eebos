#![no_std]
#![no_main]
#![feature(allocator_api)]
#![feature(ptr_mask)]
#![feature(alloc_error_handler)]
#![feature(ascii_char)]

#![allow(internal_features)]
#![feature(rustc_attrs)]
#![feature(ptr_as_ref_unchecked)]

// Todo: add checks for all the writeln!s
#![allow(unused_must_use)]

// This symbol is required for an allocator to work with --emit obj in no_std
// My understanding is that it "tells" the compiler that you know what you're doing
#[rustc_std_internal_symbol]
fn __rust_no_alloc_shim_is_unstable_v2() {}

// I have no idea why I need this and why #[alloc_error_handler] doesn't work

#[rustc_std_internal_symbol]
fn __rust_alloc_error_handler(_: core::alloc::Layout) -> ! {
    panic!("memory allocation failed");
}

extern crate alloc;

use core::*;
use core::{arch::asm, alloc::{GlobalAlloc, Layout}, fmt::{Write}, panic::PanicInfo};
use alloc::vec::Vec;
use elf::symbol;
use elf::{self, endian::AnyEndian, segment::ProgramHeader, ElfBytes};

mod syscall;
mod process;
use shared::screen::Screen;


#[panic_handler]
fn panic<'a, 'b>(info: &'a PanicInfo<'b>) -> ! {
    let mut s = Screen {line: 0, row: 0};
    writeln!(&mut s, "{}", info);
    loop {}
}

unsafe extern "C" {
    static _binary_pic_start: u8;
    static _binary_pic_size: u8;
    static _binary_pic_end: u8;

    fn call64(ptr: u32);

}

// The inits aren't actually called, so global = 0 does nothing
// Initialization must be done in rustmain()

// Passes data to the allocator from main()
static mut DATAPTR: *mut u8 = core::ptr::null_mut();

#[derive(Debug)]
struct RelocInfo {
    cmd: ProgramHeader,
    start: usize,
    length: usize
}

struct SimpleAllocator;

unsafe impl GlobalAlloc for SimpleAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let align_needed = layout.align() - ((layout.align() - 1) & unsafe { DATAPTR.addr()});

        unsafe {
            DATAPTR = DATAPTR.add(align_needed);
            let out = DATAPTR;
            DATAPTR = DATAPTR.add(layout.size());
            out
        }
    }
    
    unsafe fn dealloc(&self, _: *mut u8, _: Layout) {
        // pass
    }
}

#[global_allocator]
static ALLOCATOR: SimpleAllocator = SimpleAllocator;

fn reinterpret_slice<T, U>(i: &[T]) -> Option<&[U]> {
    let size = i.len() * size_of::<T>();
    if size % size_of::<U>() != 0 {
        return None;
    }
    let newsize = size / size_of::<U>();
    unsafe {
        let ptr = i.as_ptr() as *const U;
        Some(slice::from_raw_parts(ptr, newsize))
    }
}


fn reinterpret_slice_mut<T, U>(i: &mut [T]) -> Option<&mut [U]> {
    let size = i.len() * size_of::<T>();
    if size % size_of::<U>() != 0 {
        return None;
    }
    let newsize = size / size_of::<U>();
    unsafe {
        let ptr = i.as_ptr() as *mut U;
        Some(slice::from_raw_parts_mut(ptr, newsize))
    }
}

// fn aligned_slice<T: Copy + Default>(s: &mut Screen, size: usize, align: usize) -> &'static mut [T] {
//     assert!(align.is_power_of_two());
//     let layout = Layout::from_size_align(size * size_of::<T>(), cmp::max(align, align_of::<T>())).unwrap();
//     let out = unsafe {        
        
//         // unsafe {
//         //     DATAPTR = DATAPTR.add(add_on)
//         // }

//         // writeln!(s, "DATA is currently: {:x}", unsafe {DATAPTR as usize});


//         let aligned_ptr = alloc::alloc::alloc(layout) as *mut T;


//         // writeln!(s, "Alloc; ptr is {:x}, DATA is {:x}", aligned_ptr as usize, unsafe { DATAPTR as usize});
//         slice::from_raw_parts_mut(aligned_ptr, size)
//     };

//     for elem in out.iter_mut() {
//         *elem = T::default();
//     }
//     out
// }

fn relocate_symbol(symbol: u64, relocations: &Vec<RelocInfo>) -> (usize, u64) {
    for reloc in relocations.iter().enumerate() {
        if reloc.1.cmd.p_vaddr <= symbol && symbol < reloc.1.cmd.p_vaddr + reloc.1.cmd.p_memsz {
            // Match
            let index = symbol - reloc.1.cmd.p_vaddr;
            return (reloc.0, index);
        }
    }
    panic!("Unable to relocate symbol located at {}", symbol);
}

fn relocate_as_ptr(symbol: usize, relocations: &Vec<RelocInfo>) -> usize {
    let (index, address) = relocate_symbol(symbol as u64, relocations);
    relocations[index].start + address as usize
}

fn debug_section(s: &mut Screen, name: &str, file: &ElfBytes<AnyEndian>) {
    let got = file.section_header_by_name(name).unwrap().unwrap();

    let (data, com) = file.section_data(&got).unwrap();


    writeln!(s, "here\n");

    let None = com else {
        panic!("Compression isn't implemented!")
    };

    writeln!(s, "data for section '{}': {:#010x?}", name, reinterpret_slice::<u8, u32>(data).unwrap());

}

fn as_fn_ptr<T>(ptr: usize) -> fn() -> T {
    unsafe {
        core::mem::transmute(ptr)
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn rustmain(mem: *mut u8) {
    // Due to C not calling our initializers, this code must be preformed as soon as we get control
    unsafe {
        DATAPTR = mem;
    }

    let mut s = Screen { line: 0, row: 0};

    s.clear_screen();

    // let mut global_state = syscall::STATE.inner.take().unwrap();
    // global_state.interrupts[0x30] = Some(syscall::submit_syscall_syscall);
    // global_state.interrupts[0xff] = Some(syscall::debug_print_syscall);
    // syscall::STATE.inner.set(Some(global_state));
    
    let code = unsafe {
        core::slice::from_raw_parts(&_binary_pic_start as *const u8, &_binary_pic_size as *const u8 as usize)
    };
    
    writeln!(&mut s, "{:x?}", &code[0..5]);
    
    load_elf(s, code);
    // loop {}
}

fn load_elf(mut s: Screen, code: &[u8]) -> Process {
    let file = ElfBytes::<AnyEndian>::minimal_parse(code).expect("Can't parse!");

    let x = file.segments().expect("Can't get segments!");

    let got = file.section_header_by_name(".got").expect(".got currently required as is necessary for PIE").expect(".got currently required as is necessary for PIE");

    let mut slices: Vec<&'static mut [u8]> = Vec::new();

    let mut init_fns: Vec<u64> = Vec::<u64>::new();
    let mut init_ptr: Option<usize> = None; // these really should be u64 but we are in 32 bit mode so there's not even a way to load a module > 2^31 bits
    let mut init_size: Option<usize> = None;

    let mut fini_fns = Vec::<u64>::new();
    let mut fini_ptr: Option<usize> = None;
    let mut fini_size: Option<usize> = None;


    for a in x {
        match a.p_type {
            elf::abi::PT_PHDR => {
                // elf table-size record-keeping; ignore
            },
            elf::abi::PT_LOAD => {
                // Allocate a.p_memsz bytes into data. data & !(a.p_align - 1) must equal a.p_vaddr & !(a.p_align - 1)
                // Then, copy code[a.p_offset..a.p_offset + a.p_filesz] into data
                let start_fptr = a.p_offset as usize;

                let size_fptr = a.p_filesz as usize;
                let size_mptr = a.p_memsz as usize;

                let slice = &code[start_fptr..start_fptr + size_fptr];

                let offset = a.p_vaddr as usize & (a.p_align as usize - 1); // aligned will give us a block starting at 2^n, so we need to offset our data

                writeln!(&mut s, "offset is {:x}", offset);

                let data = aligned_slice::<u8>(&mut s, offset + size_mptr, a.p_align as usize);
                
                writeln!(&mut s, "ptr: {:x}", data.as_ptr() as usize);

                let data = &mut data[offset..];

                writeln!(&mut s, "ptr: {:x}", data.as_ptr() as usize);

                data[..size_fptr].copy_from_slice(slice);

                
                relocations.push(RelocInfo { cmd: a, start: data.as_ptr() as usize, length: data.len()});
                slices.push(data);
            },
            elf::abi::PT_DYNAMIC => {
                let dynam = file.dynamic().unwrap().unwrap();
            
                for symbol in dynam {
                    match symbol.d_tag {
                        elf::abi::DT_FLAGS => {
                            // None of the settings are meaningful yet
                        },
                        elf::abi::DT_FLAGS_1 => {

                        }
                        elf::abi::DT_RELA => {
                            // RELA* are for now ignored (all programs must be compiled as PIE)
                        },
                        elf::abi::DT_RELASZ => {

                        },
                        elf::abi::DT_RELAENT => {

                        },
                        elf::abi::DT_RELACOUNT => {

                        }
                        elf::abi::DT_STRTAB => {
                            // No runtime-linking implemented yet
                        },
                        elf::abi::DT_STRSZ => {
                            // Linking name lookup table size
                        }
                        elf::abi::DT_SYMTAB => {
                            // Symbols not yet used
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
                            init_fns.push(symbol.d_ptr());
                        },
                        elf::abi::DT_FINI => {
                            fini_fns.push(symbol.d_ptr());
                        },
                        elf::abi::DT_GNU_HASH => {

                        }
                        elf::abi::DT_DEBUG => {
                            // Debug not used
                        }
                        elf::abi::DT_NULL => {
                            // Ignored, internal record-keeping
                        }
                        _ => {
                            writeln!(&mut s, "Unknown dynamic symbol: {:x}", symbol.d_tag);
                        }
                        
                    }
                }
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
                writeln!(&mut s, "Unknown program header: {:x}", other);
            }
            
        }
    }

    if let (Some(rinit_ptr), Some(rinit_size)) = (init_ptr, init_size) {
        writeln!(&mut s, "rinits: {:x}, {:x}, {:x}", rinit_ptr, rinit_size, code.len());

        let (table, index) = relocate_symbol(rinit_ptr as u64, &relocations);

        let buf = &mut slices[table][index as usize..(index as usize + rinit_size)];

        let ptrbuf = reinterpret_slice::<u8, u64>(buf).expect("Malformed INIT_ARRAY directive");

        init_fns.extend_from_slice(ptrbuf);
    } else {
        let (None, None) = (init_ptr, init_size) else {
            // Todo: possiblity that arrays may be null-terminated without SZ element
            panic!("Error INIT_ARRAY but not INIT_ARRAYSZ or visa versa");
        };
    }
    if let (Some(rfini_ptr), Some(rfini_size)) = (fini_ptr, fini_size) {
        writeln!(&mut s, "rfini_ptr: {:x}", rfini_ptr);
        let (table, index) = relocate_symbol(rfini_ptr as u64, &mut relocations);

        let buf = &mut slices[table][index as usize..(index as usize + rfini_size)];

        let ptrbuf = reinterpret_slice::<u8, u64>(buf).expect("Malformed INIT_ARRAY directive");

        fini_fns.extend_from_slice(ptrbuf);
    } else {
        let (None, None) = (fini_ptr, fini_size) else {
            // Todo: possiblity that arrays may be null-terminated without SZ element
            panic!("Error FINI_ARRAY but not FINI_ARRAYSZ or visa versa");
        };
    }
    // writeln!(&mut s, "inits: {:x?}\nfinis: {:x?}", init_fns, fini_fns);


    // debug_section(&mut s, ".got", &file);
    // debug_section(&mut s, ".got.plt", &file);

    
    // unsafe { call64(0); }

    let (table, index) = relocate_symbol(got.sh_addr, &relocations);

    let index: usize = index.try_into().unwrap();

    let data = &mut slices[table][index..index+(got.sh_size as usize)];


    let data = reinterpret_slice_mut::<u8, u32>(data).expect(".got must contain 32 bit dwords");

    writeln!(&mut s, "here3!");


    match file.section_header_by_name(".dynamic").unwrap() {
        Some(dyn_header) => {
            // First element must point to dynamic header, if it exists
            let (t, i) = relocate_symbol(dyn_header.sh_addr, &relocations);
            data[0] = relocations[t].start as u32 + i as u32;
        },
        None => {}
    }

    writeln!(&mut s, "data is: {:x?}, {:p}", data, data);

    for elem in data[3..].iter_mut() {
        // if *elem != 0 {
        let old = *elem;
        *elem = relocate_as_ptr(*elem as usize, &relocations) as u32;
        // let val =  unsafe { str::from_utf8_unchecked(slice::from_raw_parts((* elem) as *const u8, 8)) };
        writeln!(&mut s, "relocating {:x?} to {:x?}", old, *elem);
        // }
    }

    // for mut ptr in init_fns {
    //     if ptr != 0 {
    //         ptr = relocate_as_ptr(ptr as usize, &relocations) as u64;
    //         let fn_ptr = as_fn_ptr::<()>(ptr as usize);
    //         fn_ptr();
    //     }
    // }


    let mut relocs_dbg = Vec::<(usize, usize, usize)>::new();

    for reloc in &relocations {
        relocs_dbg.push((reloc.cmd.p_vaddr as usize, reloc.start, reloc.length));   
    }

    writeln!(&mut s, "INFO: {:x?}", relocs_dbg);

    let ptr = as_fn_ptr::<u32>(relocate_as_ptr(file.ehdr.e_entry as usize, &relocations));

    unsafe { asm!("xchg bx, bx") };

    let out = ptr();

    writeln!(&mut s, "Got out: {out}");

    for mut ptr in fini_fns {
        if ptr != 0 {
            ptr = relocate_as_ptr(ptr as usize, &relocations) as u64;
            let fn_ptr = as_fn_ptr::<()>(ptr as usize);
            fn_ptr();
        }
    }
}