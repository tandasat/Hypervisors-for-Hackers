use core::arch::global_asm;

/// Captures some of the current register values.
// This function must be inlined not to consume stack with the CALL instruction,
// so that, after VMLAUNCH, we can continue execution without using a return
// address on stack which is already destroyed at the time.
#[expect(clippy::inline_always)]
#[inline(always)]
#[must_use]
pub fn capture() -> Registers {
    let mut registers = Registers::default();
    unsafe { asm_capture_registers(&mut registers) };
    registers
}

unsafe extern "C" {
    /// Captures some of the current register values.
    fn asm_capture_registers(registers: &mut Registers);
}
global_asm!(include_str!("registers.S"));

#[derive(Clone, Debug, Default)]
#[repr(C)]
pub struct Registers {
    pub(crate) rax: u64,
    pub(crate) rbx: u64,
    pub(crate) rcx: u64,
    pub(crate) rdx: u64,
    pub(crate) rdi: u64,
    pub(crate) rsi: u64,
    pub(crate) rbp: u64,
    pub(crate) r8: u64,
    pub(crate) r9: u64,
    pub(crate) r10: u64,
    pub(crate) r11: u64,
    pub(crate) r12: u64,
    pub(crate) r13: u64,
    pub(crate) r14: u64,
    pub(crate) r15: u64,
    pub(crate) rflags: u64,
    pub(crate) rsp: u64,
    pub(crate) rip: u64,
    pub(crate) xmm0: Xmm,
    pub(crate) xmm1: Xmm,
    pub(crate) xmm2: Xmm,
    pub(crate) xmm3: Xmm,
    pub(crate) xmm4: Xmm,
    pub(crate) xmm5: Xmm,
}
const _: () = assert!(size_of::<Registers>() == 0xf0);

#[repr(C, align(16))]
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Xmm {
    pub(crate) low: u64,
    pub(crate) high: u64,
}
