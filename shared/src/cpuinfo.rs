use core::arch::asm;
pub fn get_cr0() -> u32 {
    let result: u32;
    unsafe { asm!(
            "mov {result}, cr0",
            result = out(reg) result
        ) };
    result
}