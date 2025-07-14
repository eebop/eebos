#![no_std]
#![no_main]

use core::panic::PanicInfo;

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}

enum PS2Token {
    MAIN,
    AUX
}

enum PS2PORT {
    CMD,
    DATA
}

#[no_mangle]
pub extern "C" fn ps2_init() {

}