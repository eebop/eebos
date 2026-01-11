macro_rules! elf_data {
    ($data:ident) => {
        {
            unsafe extern "C" {
                static mut ${ concat(_binary_, $data, _start) }: u8;
                static mut ${ concat(_binary_, $data, _end  ) }: u8; 
            }
            unsafe {
                &raw mut ${ concat(_binary_, $data, _start) }..&raw mut ${ concat(_binary_, $data, _end ) }
            }
        }
    }
}

macro_rules! elf_map {
    ($( $i:ident ),*) => {
        [
            $( (stringify!($i), elf_data!($i)) ),*
        ]
    };
}

const _ELF_DATA: &'static [(&'static str, Range<*mut u8>)] = &elf_map!(pic);

static ELF_DATA: ManualOnceCell<BTreeMap<&str, &[u8]>> = ManualOnceCell::new();

fn init_elf_data() {
    let mut data = BTreeMap::<&str, &[u8]>::new();
    for entry in _ELF_DATA {
        // Unsafe here is fine because it's garenteed to be a valid slice at link time
        // We just can't make it a slice because code can't run at link time
        data.insert(entry.0, unsafe { slice::from_mut_ptr_range(entry.1.clone()) } );
    }
    unsafe { ELF_DATA.init(data) };
}

fn string_elf(name: &str) -> Process<Global, DummyAllocator> {
    let data = ELF_DATA.get()[name];
    load_elf(data)


fn load_elf<A: Allocator + Clone>(code: &[u8]) -> Process<Global, A> {
    let file = ElfBytes::<AnyEndian>::minimal_parse(code).expect("Can't parse!");

    let x = file.segments().expect("Can't get segments!");

    let got = file.section_header_by_name(".got").expect(".got currently required as is necessary for PIE").expect(".got currently required as is necessary for PIE");

    let mut loads: Vec<ProgramHeader> = Vec::new();

    let mut init_fns: Vec<u32> = Vec::new();
    let mut init_ptr: Option<usize> = None; // these really should be u64 but we are in 32 bit mode so there's not even a way to load a module > 2^31 bits
    let mut init_size: Option<usize> = None;

    let mut fini_fns: Vec<u32> = Vec::new();
    let mut fini_ptr: Option<usize> = None;
    let mut fini_size: Option<usize> = None;

    let mut earliest: Option<u32> = None;
    let mut latest: Option<u32> = None;


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
                            init_fns.push(symbol.d_ptr() as u32);
                        },
                        elf::abi::DT_FINI => {
                            fini_fns.push(symbol.d_ptr() as u32);
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
                            panic!("Unknown dynamic symbol: {:x}", symbol.d_tag);
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
                panic!("Unknown program header: {:x}", other);
            }
            
        }
    }

    assert_ne!(loads.len(), 0);

    let earliest = earliest.unwrap() as usize;
    let latest = latest.unwrap() as usize;

    // It'd be better to just allocate the sections we need instead of inclusively
    let num_pages = usize::div_ceil(latest - earliest, 0x1000);

    let mut owned_data = Page::uninit_many(num_pages as usize, Global);

    let new_earliest = owned_data.as_ptr() as usize; 

    let array = owned_data.as_contiguous();

    for header in loads {
        let start = header.p_vaddr as usize - earliest as usize;
        array[start..][..header.p_filesz as usize].copy_from_slice(&code[header.p_offset as usize..][..header.p_filesz as usize]);
        array[start..][header.p_filesz as usize ..header.p_memsz as usize].fill(0);
    }

    if let (Some(rinit_ptr), Some(rinit_size)) = (init_ptr, init_size) {
        let subslice = &array[rinit_ptr - earliest..][..rinit_size];

        let ptrbuf = reinterpret_slice::<u8, u32>(subslice).expect("Malformed INIT_ARRAY directive");

        init_fns.extend_from_slice(ptrbuf);

    } else {
        let (None, None) = (init_ptr, init_size) else {
            // Todo: possiblity that arrays may be null-terminated without SZ element
            panic!("Error INIT_ARRAY but not INIT_ARRAYSZ or visa versa");
        };
    }
    if let (Some(rfini_ptr), Some(rfini_size)) = (fini_ptr, fini_size) {
        let subslice = &array[rfini_ptr - earliest..][..rfini_size];

        let ptrbuf = reinterpret_slice::<u8, u32>(subslice).expect("Malformed INIT_ARRAY directive");

        fini_fns.extend_from_slice(ptrbuf);

    } else {
        let (None, None) = (fini_ptr, fini_size) else {
            // Todo: possiblity that arrays may be null-terminated without SZ element
            panic!("Error FINI_ARRAY but not FINI_ARRAYSZ or visa versa");
        };
    }

    let got_data = &mut array[got.sh_addr as usize - earliest..][..got.sh_size as usize];

    let got_data = reinterpret_slice_mut::<u8, u32>(got_data).expect(".got must contain 32 bit dwords");

    if let Some(dyn_header) =  file.section_header_by_name(".dynamic").unwrap() {
        // First element must point to dynamic header, if it exists
        got_data[0] = dyn_header.sh_addr as u32 - earliest as u32 + new_earliest as u32;
    }

    // writeln!(Screen::new(), "\n\n\n got is {got_data:?}");

    // for elem in got_data[3..].iter_mut() {
    //     *elem = *elem as u32 - earliest as u32 + new_earliest as u32;
    // }

    assert!(init_fns.len() == 0);
    assert!(fini_fns.len() == 0);

    let start: extern "C" fn() -> ! = unsafe { core::mem::transmute(file.ehdr.e_entry as u32 - earliest as u32 + new_earliest as u32) };

    let got_ptr = &raw mut *got_data;


    Process::new(got_ptr, start, Rc::new(RefCell::new([owned_data])))
}