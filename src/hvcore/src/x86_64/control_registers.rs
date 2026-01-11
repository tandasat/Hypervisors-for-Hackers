use core::arch::asm;

pub(crate) fn cr0() -> u64 {
    let value;
    unsafe { asm!("mov {}, cr0", out(reg) value, options(nomem, nostack, preserves_flags)) };
    value
}

pub(crate) fn cr2() -> u64 {
    let value;
    unsafe { asm!("mov {}, cr2", out(reg) value, options(nomem, nostack, preserves_flags)) };
    value
}

pub(crate) fn cr3() -> u64 {
    let value;
    unsafe { asm!("mov {}, cr3", out(reg) value, options(nomem, nostack, preserves_flags)) };
    value
}

pub(crate) fn cr4() -> u64 {
    let value;
    unsafe { asm!("mov {}, cr4", out(reg) value, options(nomem, nostack, preserves_flags)) };
    value
}

pub(crate) unsafe fn write_cr4(value: u64) {
    unsafe { asm!("mov cr4, {}", in(reg) value, options(nomem, nostack, preserves_flags)) };
}

pub(crate) const CR4_VMXE: u64 = 1 << 13;
#[allow(dead_code)]
pub(crate) const CR4_SMEP: u64 = 1 << 20;
