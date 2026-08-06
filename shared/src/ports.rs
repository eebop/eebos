use core::arch::asm;

pub fn in8(port: u16) -> u8 {
    let out: u8;
    unsafe {
        asm!(
            "in al, dx",
            in("dx") port,
            lateout("al") out,
            options(nomem, nostack, preserves_flags)
        )
    }
    out
}

pub fn in16(port: u16) -> u16 {
    let out: u16;
    unsafe {
        asm!(
            "in ax, dx",
            in("dx") port,
            lateout("ax") out,
            options(nomem, nostack, preserves_flags)

        )
    }
    out
}

pub fn in32(port: u16) -> u32 {
    let out: u32;
    unsafe {
        asm!(
            "in eax, dx",
            in("dx") port,
            lateout("eax") out,
            options(nomem, nostack, preserves_flags)

        )
    }
    out
}

pub fn out8(port: u16, input: u8) {
    unsafe {
        asm!(
            "out dx, al",
            in("dx") port,
            in("al") input,
            options(nomem, nostack, preserves_flags)

        )
    }
}

pub fn out16(port: u16, input: u16) {
    unsafe {
        asm!(
            "out dx, ax",
            in("dx") port,
            in("ax") input,
            options(nomem, nostack, preserves_flags)

        )
    }
}

pub fn out32(port: u16, input: u32) {
    unsafe {
        asm!(
            "out dx, eax",
            in("dx") port,
            in("eax") input,
            options(nomem, nostack, preserves_flags)
        )
    }
}

pub fn io_wait() {
    out8(80, 0);
}