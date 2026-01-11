use core::arch::asm;

#[derive(Copy, Clone, Debug)]
#[repr(transparent)]
pub(crate) struct Rflags(pub(crate) u64);

/// Zero Flag (ZF)
pub(crate) const RFLAGS_ZF: u64 = 1 << 6;

/// Carry Flag (CF)
pub(crate) const RFLAGS_CF: u64 = 1 << 0;

// This function must be inlined not to change the RFLAGS register after execution
// of a VMX instruction.
#[expect(clippy::inline_always)]
#[inline(always)]
pub(crate) fn rflags() -> Rflags {
    let rflags;
    unsafe { asm!("pushf; pop {}", out(reg) rflags, options(nomem)) };
    Rflags(rflags)
}

/// See: 7.15 EXCEPTION AND INTERRUPT REFERENCE
#[expect(dead_code)]
#[derive(Clone, Copy, Debug)]
#[repr(u8)]
pub(crate) enum Exception {
    /// #DE - Error Code: No.
    DivideError = 0,

    /// #DB - Error Code: No.
    Debug,

    /// Non-maskable Interrupt -  Error Code: No.
    Nmi,

    /// #BP - Error Code: No.
    Breakpoint,

    /// #OF - Error Code: No.
    Overflow,

    /// #BR - Error Code: No.
    BoundRangeExceeded,

    /// #UD - Error Code: No.
    InvalidOpcode,

    /// #NM - Error Code: No.
    DeviceNotAvailable,

    /// #DF - Error Code: Yes (zero).
    DoubleFault,

    /// #TS - Error Code: Yes.
    InvalidTss = 0xa,

    /// #NP - Error Code: Yes.
    SegmentNotPresent,

    /// #SS - Error Code: Yes.
    StackSegmentFault,

    /// #GP - Error Code: Yes.
    GeneralProtection,

    /// #PF - Error Code: Yes.
    PageFault,

    /// #MF - Error Code: No.
    X87FloatingPointError = 0x10,

    /// #AC - Error Code: Yes.
    AlignmentCheck,

    /// #MC - Error Code: No.
    MachineCheck,

    /// #XM - Error Code: No.
    SimdFloatingPointError,

    /// #VE - Error Code: No.
    #[allow(clippy::enum_variant_names)]
    VirtualizationException,

    /// #CP - Error Code: Yes.
    ControlProtection,
}
