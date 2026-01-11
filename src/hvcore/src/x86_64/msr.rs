#![allow(dead_code)]

use core::arch::asm;

pub(crate) unsafe fn rdmsr(msr: u32) -> u64 {
    let high: u64;
    let low: u64;
    unsafe {
        asm!("rdmsr", out("eax") low, out("edx") high, in("ecx") msr, options(nomem, nostack, preserves_flags));
    };
    (high << 32) | low
}

pub(crate) unsafe fn wrmsr(msr: u32, value: u64) {
    let low = value as u32;
    let high = (value >> 32) as u32;
    unsafe {
        asm!("wrmsr", in("ecx") msr, in("eax") low, in("edx") high, options(nomem, nostack, preserves_flags));
    };
}

/// MTRR Information See Section 11.11.1, MTRR Feature  Identification.
pub(crate) const IA32_MTRRCAP: u32 = 0xfe;

/// CS register target for CPL 0 code (R/W) See Table 35-2. See Section 5.8.7, Performing Fast Calls to  System Procedures with the SYSENTER and  SYSEXIT Instructions.
pub(crate) const IA32_SYSENTER_CS: u32 = 0x174;

/// Stack pointer for CPL 0 stack (R/W) See Table 35-2. See Section 5.8.7, Performing Fast Calls to  System Procedures with the SYSENTER and  SYSEXIT Instructions.
pub(crate) const IA32_SYSENTER_ESP: u32 = 0x175;

/// CPL 0 code entry point (R/W) See Table 35-2. See Section 5.8.7, Performing  Fast Calls to System Procedures with the SYSENTER and SYSEXIT Instructions.
pub(crate) const IA32_SYSENTER_EIP: u32 = 0x176;

/// Variable Range Base MTRR See Section 11.11.2.3, Variable Range MTRRs.
pub(crate) const IA32_MTRR_PHYSBASE0: u32 = 0x200;

/// Variable Range Mask MTRR See Section 11.11.2.3, Variable Range MTRRs.
pub(crate) const IA32_MTRR_PHYSMASK0: u32 = 0x201;

/// Variable Range Mask MTRR See Section 11.11.2.3, Variable Range MTRRs.
pub(crate) const IA32_MTRR_PHYSBASE1: u32 = 0x202;

/// Variable Range Mask MTRR See Section 11.11.2.3, Variable Range MTRRs.
pub(crate) const IA32_MTRR_PHYSMASK1: u32 = 0x203;

/// Variable Range Mask MTRR See Section 11.11.2.3, Variable Range MTRRs.
pub(crate) const IA32_MTRR_PHYSBASE2: u32 = 0x204;

/// Variable Range Mask MTRR See Section 11.11.2.3, Variable Range MTRRs .
pub(crate) const IA32_MTRR_PHYSMASK2: u32 = 0x205;

/// Variable Range Mask MTRR See Section 11.11.2.3, Variable Range MTRRs.
pub(crate) const IA32_MTRR_PHYSBASE3: u32 = 0x206;

/// Variable Range Mask MTRR See Section 11.11.2.3, Variable Range MTRRs.
pub(crate) const IA32_MTRR_PHYSMASK3: u32 = 0x207;

/// Variable Range Mask MTRR See Section 11.11.2.3, Variable Range MTRRs.
pub(crate) const IA32_MTRR_PHYSBASE4: u32 = 0x208;

/// Variable Range Mask MTRR See Section 11.11.2.3, Variable Range MTRRs.
pub(crate) const IA32_MTRR_PHYSMASK4: u32 = 0x209;

/// Variable Range Mask MTRR See Section 11.11.2.3, Variable Range MTRRs.
pub(crate) const IA32_MTRR_PHYSBASE5: u32 = 0x20a;

/// Variable Range Mask MTRR See Section 11.11.2.3, Variable Range MTRRs.
pub(crate) const IA32_MTRR_PHYSMASK5: u32 = 0x20b;

/// Variable Range Mask MTRR See Section 11.11.2.3, Variable Range MTRRs.
pub(crate) const IA32_MTRR_PHYSBASE6: u32 = 0x20c;

/// Variable Range Mask MTRR See Section 11.11.2.3, Variable Range MTRRs.
pub(crate) const IA32_MTRR_PHYSMASK6: u32 = 0x20d;

/// Variable Range Mask MTRR See Section 11.11.2.3, Variable Range MTRRs.
pub(crate) const IA32_MTRR_PHYSBASE7: u32 = 0x20e;

/// Variable Range Mask MTRR See Section 11.11.2.3, Variable Range MTRRs.
pub(crate) const IA32_MTRR_PHYSMASK7: u32 = 0x20f;

/// if IA32_MTRR_CAP\[7:0\] >  8
pub(crate) const IA32_MTRR_PHYSBASE8: u32 = 0x210;

/// if IA32_MTRR_CAP\[7:0\] >  8
pub(crate) const IA32_MTRR_PHYSMASK8: u32 = 0x211;

/// if IA32_MTRR_CAP\[7:0\] >  9
pub(crate) const IA32_MTRR_PHYSBASE9: u32 = 0x212;

/// if IA32_MTRR_CAP\[7:0\] >  9
pub(crate) const IA32_MTRR_PHYSMASK9: u32 = 0x213;

/// Fixed Range MTRR See Section 11.11.2.2, Fixed Range MTRRs.
pub(crate) const IA32_MTRR_FIX64K_00000: u32 = 0x250;

/// Fixed Range MTRR See Section 11.11.2.2, Fixed Range MTRRs.
pub(crate) const IA32_MTRR_FIX16K_80000: u32 = 0x258;

/// Fixed Range MTRR See Section 11.11.2.2, Fixed Range MTRRs.
pub(crate) const IA32_MTRR_FIX16K_A0000: u32 = 0x259;

/// Fixed Range MTRR See Section 11.11.2.2, Fixed Range MTRRs.
pub(crate) const IA32_MTRR_FIX4K_C0000: u32 = 0x268;

/// Fixed Range MTRR See Section 11.11.2.2, Fixed Range MTRRs .
pub(crate) const IA32_MTRR_FIX4K_C8000: u32 = 0x269;

/// Fixed Range MTRR See Section 11.11.2.2, Fixed Range MTRRs .
pub(crate) const IA32_MTRR_FIX4K_D0000: u32 = 0x26a;

/// Fixed Range MTRR See Section 11.11.2.2, Fixed Range MTRRs.
pub(crate) const IA32_MTRR_FIX4K_D8000: u32 = 0x26b;

/// Fixed Range MTRR See Section 11.11.2.2, Fixed Range MTRRs.
pub(crate) const IA32_MTRR_FIX4K_E0000: u32 = 0x26c;

/// Fixed Range MTRR See Section 11.11.2.2, Fixed Range MTRRs.
pub(crate) const IA32_MTRR_FIX4K_E8000: u32 = 0x26d;

/// Fixed Range MTRR See Section 11.11.2.2, Fixed Range MTRRs.
pub(crate) const IA32_MTRR_FIX4K_F0000: u32 = 0x26e;

/// Fixed Range MTRR See Section 11.11.2.2, Fixed Range MTRRs.
pub(crate) const IA32_MTRR_FIX4K_F8000: u32 = 0x26f;

/// Default Memory Types (R/W)  Sets the memory type for the regions of physical memory that are not  mapped by the MTRRs.  See Section 11.11.2.1, IA32_MTRR_DEF_TYPE MSR.
pub(crate) const IA32_MTRR_DEF_TYPE: u32 = 0x2ff;

/// Reporting Register of Basic VMX Capabilities (R/O) See Table 35-2. See Appendix A.1, Basic VMX Information (If CPUID.01H:ECX.\[bit 9\])
pub(crate) const IA32_VMX_BASIC: u32 = 0x480;

/// Capability Reporting Register of Pin-based VM-execution  Controls (R/O) See Appendix A.3, VM-Execution Controls (If CPUID.01H:ECX.\[bit 9\])
pub(crate) const IA32_VMX_PINBASED_CTLS: u32 = 0x481;

/// Capability Reporting Register of Primary Processor-based  VM-execution Controls (R/O) See Appendix A.3, VM-Execution Controls (If CPUID.01H:ECX.\[bit 9\])
pub(crate) const IA32_VMX_PROCBASED_CTLS: u32 = 0x482;

/// Capability Reporting Register of VM-exit Controls (R/O) See Appendix A.4, VM-Exit Controls (If CPUID.01H:ECX.\[bit 9\])
pub(crate) const IA32_VMX_EXIT_CTLS: u32 = 0x483;

/// Capability Reporting Register of VM-entry Controls (R/O) See Appendix A.5, VM-Entry Controls (If CPUID.01H:ECX.\[bit 9\])
pub(crate) const IA32_VMX_ENTRY_CTLS: u32 = 0x484;

/// Capability Reporting Register of Secondary Processor-based  VM-execution Controls (R/O) See Appendix A.3, VM-Execution Controls (If CPUID.01H:ECX.\[bit 9\] and  IA32_VMX_PROCBASED_CTLS\[bit 63\])
pub(crate) const IA32_VMX_PROCBASED_CTLS2: u32 = 0x48b;

/// Capability Reporting Register of Pin-based VM-execution Flex  Controls (R/O) See Table 35-2
pub(crate) const IA32_VMX_TRUE_PINBASED_CTLS: u32 = 0x48d;

/// Capability Reporting Register of Primary Processor-based  VM-execution Flex Controls (R/O) See Table 35-2
pub(crate) const IA32_VMX_TRUE_PROCBASED_CTLS: u32 = 0x48e;

/// Capability Reporting Register of VM-exit Flex Controls (R/O) See Table 35-2
pub(crate) const IA32_VMX_TRUE_EXIT_CTLS: u32 = 0x48f;

/// Capability Reporting Register of VM-entry Flex Controls (R/O) See Table 35-2
pub(crate) const IA32_VMX_TRUE_ENTRY_CTLS: u32 = 0x490;

/// Capability Reporting Register of Tertiary Processor-based  VM-execution Controls (R/O)
pub(crate) const IA32_VMX_PROCBASED_CTLS3: u32 = 0x492;

/// Map of BASE Address of FS (R/W)  See Table 35-2.
pub(crate) const IA32_FS_BASE: u32 = 0xc000_0100;

/// Map of BASE Address of GS (R/W)  See Table 35-2.
pub(crate) const IA32_GS_BASE: u32 = 0xc000_0101;
