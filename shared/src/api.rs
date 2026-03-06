// use crate::SysCallInternal;
// use crate::make_syscall;
// use alloc::boxed::Box;

// #[macro_export]
// macro_rules! syscall {
//     ($mangled:ident = $name:ident, $inputTy:ty, $outputTy:ty, $syscallName:expr) => {
//         extern "C" fn $mangled (input: *mut ::core::ffi::c_void, out: *mut SysCallInternal) {
//             let input = input as *mut $inputTy;
//             let result = $name(unsafe { (*input).clone() });
//             unsafe { *out }.send_abi::<$outputTy>(result);
//             make_syscall::<([u8; 8], *mut SysCallInternal), !, 0x21>(($syscallName, out));
//         }
//     };
// }

// #[macro_export]
// macro_rules! syscall_manual_return {
//     ($mangled:ident = $name:ident, $inputTy:ty, $outputTy:ty, $syscallName:expr) => {
//         extern "C" fn $mangled (input: *mut ::core::ffi::c_void, out: *mut SysCallInternal) {
//             let input = input as *mut $inputTy;
//             let result = $name(unsafe { (*input).clone() }, SysCallData::new(&mut unsafe {*out}));
//             make_syscall::<([u8; 8], *mut SysCallInternal), !, 0x21>(($syscallName, out));
//         }
//     };
// }