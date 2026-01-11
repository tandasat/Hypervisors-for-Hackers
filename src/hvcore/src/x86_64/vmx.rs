use core::{arch::asm, fmt::Formatter};

use crate::{
    misc::{SIZE_4KB, Spa, ZeroIsSound},
    x86_64::misc::rflags,
};

use super::misc::{RFLAGS_CF, RFLAGS_ZF, Rflags};

use derive_more::{Debug, Display, Error, TryFrom};

pub(crate) unsafe fn vmxon(vmxon: Spa) -> Result<(), Error> {
    unsafe {
        asm!("vmxon [{}]", in(reg) &vmxon.as_u64(), options(nostack));
        result(rflags())
    }
}

pub(crate) unsafe fn vmclear(vmcs: Spa) -> Result<(), Error> {
    unsafe {
        asm!("vmclear [{}]", in(reg) &vmcs.as_u64(), options(nostack));
        result(rflags())
    }
}

pub(crate) unsafe fn vmptrld(vmcs: Spa) -> Result<(), Error> {
    unsafe {
        asm!("vmptrld [{}]", in(reg) &vmcs.as_u64(), options(nostack));
        result(rflags())
    }
}

pub(crate) unsafe fn vmread(encoding: u32) -> Result<u64, Error> {
    unsafe {
        let value;
        asm!("vmread {}, {}", out(reg) value, in(reg) u64::from(encoding), options(nomem, nostack));
        result(rflags()).and(Ok(value))
    }
}

pub(crate) unsafe fn vmwrite(encoding: u32, value: u64) -> Result<(), Error> {
    unsafe {
        asm!("vmwrite {}, {}", in(reg) u64::from(encoding), in(reg) value, options(nomem, nostack));
        result(rflags())
    }
}

pub(crate) fn result(flags: Rflags) -> Result<(), Error> {
    if flags.0 & RFLAGS_ZF != 0 {
        let error = unsafe { vmread(vmcs::ro::VM_INSTRUCTION_ERROR) }.unwrap();
        let error = ErrorNumber::try_from(error).unwrap();
        Err(Error::VmFailValid(error))
    } else if flags.0 & RFLAGS_CF != 0 {
        Err(Error::VmFailInvalid)
    } else {
        Ok(())
    }
}

/// See: 32.2 CONVENTIONS
#[derive(Clone, Copy, Debug, Display, Error)]
pub(crate) enum Error {
    #[display("Failed for: {_0}")]
    VmFailValid(ErrorNumber),

    #[display("Failed without an error number")]
    VmFailInvalid,
}

/// The region of memory that the logical processor uses to support VMX operation.
///
/// See: 26.11.5 VMXON Region
#[repr(C, align(4096))]
pub(crate) struct VmxonRegion {
    pub(crate) revision_id: u32,
    data: [u8; 4092],
}
const _: () = assert!(size_of::<VmxonRegion>() == SIZE_4KB);

unsafe impl ZeroIsSound for VmxonRegion {}

impl core::fmt::Debug for VmxonRegion {
    fn fmt(&self, format: &mut Formatter<'_>) -> core::fmt::Result {
        format
            .debug_struct("Vmxon")
            .field("Revision ID", &self.revision_id)
            .finish_non_exhaustive()
    }
}

#[repr(C, align(4096))]
pub(crate) struct Vmcs {
    pub(crate) revision_id: u32,
    pub(crate) abort_indicator: u32,
    data: [u8; 4088],
}
const _: () = assert!(size_of::<Vmcs>() == SIZE_4KB);

unsafe impl ZeroIsSound for Vmcs {}

impl core::fmt::Debug for Vmcs {
    #[rustfmt::skip]
    #[expect(clippy::too_many_lines)]
    fn fmt(&self, format: &mut Formatter<'_>) -> core::fmt::Result {
        /// The wrapper of the VMREAD instruction. Returns zero on error.
        fn vmread_relaxed(encoding: u32) -> u64 {
            unsafe { vmread(encoding) }.unwrap_or(0)
        }

        // Dump the current VMCS.
        format.debug_struct("Vmcs")
        .field("Revision ID                                    ", &self.revision_id)

        // 16-Bit Guest-State Fields
        .field("Guest ES Selector                              ", &vmread_relaxed(vmcs::guest::ES_SELECTOR))
        .field("Guest CS Selector                              ", &vmread_relaxed(vmcs::guest::CS_SELECTOR))
        .field("Guest SS Selector                              ", &vmread_relaxed(vmcs::guest::SS_SELECTOR))
        .field("Guest DS Selector                              ", &vmread_relaxed(vmcs::guest::DS_SELECTOR))
        .field("Guest FS Selector                              ", &vmread_relaxed(vmcs::guest::FS_SELECTOR))
        .field("Guest GS Selector                              ", &vmread_relaxed(vmcs::guest::GS_SELECTOR))
        .field("Guest LDTR Selector                            ", &vmread_relaxed(vmcs::guest::LDTR_SELECTOR))
        .field("Guest TR Selector                              ", &vmread_relaxed(vmcs::guest::TR_SELECTOR))
        .field("Guest interrupt status                         ", &vmread_relaxed(vmcs::guest::INTERRUPT_STATUS))
        .field("PML index                                      ", &vmread_relaxed(vmcs::guest::PML_INDEX))
        .field("Guest UINV                                     ", &vmread_relaxed(vmcs::guest::UINV))

        // 64-Bit Guest-State Fields
        .field("VMCS link pointer                              ", &vmread_relaxed(vmcs::guest::LINK_PTR))
        .field("Guest IA32_DEBUGCTL                            ", &vmread_relaxed(vmcs::guest::IA32_DEBUGCTL))
        .field("Guest IA32_PAT                                 ", &vmread_relaxed(vmcs::guest::IA32_PAT))
        .field("Guest IA32_EFER                                ", &vmread_relaxed(vmcs::guest::IA32_EFER))
        .field("Guest IA32_PERF_GLOBAL_CTRL                    ", &vmread_relaxed(vmcs::guest::IA32_PERF_GLOBAL_CTRL))
        .field("Guest PDPTE0                                   ", &vmread_relaxed(vmcs::guest::PDPTE0))
        .field("Guest PDPTE1                                   ", &vmread_relaxed(vmcs::guest::PDPTE1))
        .field("Guest PDPTE2                                   ", &vmread_relaxed(vmcs::guest::PDPTE2))
        .field("Guest PDPTE3                                   ", &vmread_relaxed(vmcs::guest::PDPTE3))
        .field("Guest IA32_BNDCFGS                             ", &vmread_relaxed(vmcs::guest::IA32_BNDCFGS))
        .field("Guest IA32_RTIT_CTL                            ", &vmread_relaxed(vmcs::guest::IA32_RTIT_CTL))
        .field("Guest IA32_LBR_CTL                             ", &vmread_relaxed(vmcs::guest::IA32_LBR_CTL))
        .field("Guest IA32_PKRS                                ", &vmread_relaxed(vmcs::guest::IA32_PKRS))

        // 32-Bit Guest-State Fields
        .field("Guest ES Limit                                 ", &vmread_relaxed(vmcs::guest::ES_LIMIT))
        .field("Guest CS Limit                                 ", &vmread_relaxed(vmcs::guest::CS_LIMIT))
        .field("Guest SS Limit                                 ", &vmread_relaxed(vmcs::guest::SS_LIMIT))
        .field("Guest DS Limit                                 ", &vmread_relaxed(vmcs::guest::DS_LIMIT))
        .field("Guest FS Limit                                 ", &vmread_relaxed(vmcs::guest::FS_LIMIT))
        .field("Guest GS Limit                                 ", &vmread_relaxed(vmcs::guest::GS_LIMIT))
        .field("Guest LDTR Limit                               ", &vmread_relaxed(vmcs::guest::LDTR_LIMIT))
        .field("Guest TR Limit                                 ", &vmread_relaxed(vmcs::guest::TR_LIMIT))
        .field("Guest GDTR limit                               ", &vmread_relaxed(vmcs::guest::GDTR_LIMIT))
        .field("Guest IDTR limit                               ", &vmread_relaxed(vmcs::guest::IDTR_LIMIT))
        .field("Guest ES access rights                         ", &vmread_relaxed(vmcs::guest::ES_ACCESS_RIGHTS))
        .field("Guest CS access rights                         ", &vmread_relaxed(vmcs::guest::CS_ACCESS_RIGHTS))
        .field("Guest SS access rights                         ", &vmread_relaxed(vmcs::guest::SS_ACCESS_RIGHTS))
        .field("Guest DS access rights                         ", &vmread_relaxed(vmcs::guest::DS_ACCESS_RIGHTS))
        .field("Guest FS access rights                         ", &vmread_relaxed(vmcs::guest::FS_ACCESS_RIGHTS))
        .field("Guest GS access rights                         ", &vmread_relaxed(vmcs::guest::GS_ACCESS_RIGHTS))
        .field("Guest LDTR access rights                       ", &vmread_relaxed(vmcs::guest::LDTR_ACCESS_RIGHTS))
        .field("Guest TR access rights                         ", &vmread_relaxed(vmcs::guest::TR_ACCESS_RIGHTS))
        .field("Guest interruptibility state                   ", &vmread_relaxed(vmcs::guest::INTERRUPTIBILITY_STATE))
        .field("Guest activity state                           ", &vmread_relaxed(vmcs::guest::ACTIVITY_STATE))
        .field("Guest SMBASE                                   ", &vmread_relaxed(vmcs::guest::SMBASE))
        .field("Guest IA32_SYSENTER_CS                         ", &vmread_relaxed(vmcs::guest::IA32_SYSENTER_CS))
        .field("VMX-preemption timer value                     ", &vmread_relaxed(vmcs::guest::VMX_PREEMPTION_TIMER_VALUE))

        // Natural-Width Guest-State Fields
        .field("Guest CR0                                      ", &vmread_relaxed(vmcs::guest::CR0))
        .field("Guest CR3                                      ", &vmread_relaxed(vmcs::guest::CR3))
        .field("Guest CR4                                      ", &vmread_relaxed(vmcs::guest::CR4))
        .field("Guest ES Base                                  ", &vmread_relaxed(vmcs::guest::ES_BASE))
        .field("Guest CS Base                                  ", &vmread_relaxed(vmcs::guest::CS_BASE))
        .field("Guest SS Base                                  ", &vmread_relaxed(vmcs::guest::SS_BASE))
        .field("Guest DS Base                                  ", &vmread_relaxed(vmcs::guest::DS_BASE))
        .field("Guest FS Base                                  ", &vmread_relaxed(vmcs::guest::FS_BASE))
        .field("Guest GS Base                                  ", &vmread_relaxed(vmcs::guest::GS_BASE))
        .field("Guest LDTR base                                ", &vmread_relaxed(vmcs::guest::LDTR_BASE))
        .field("Guest TR base                                  ", &vmread_relaxed(vmcs::guest::TR_BASE))
        .field("Guest GDTR base                                ", &vmread_relaxed(vmcs::guest::GDTR_BASE))
        .field("Guest IDTR base                                ", &vmread_relaxed(vmcs::guest::IDTR_BASE))
        .field("Guest DR7                                      ", &vmread_relaxed(vmcs::guest::DR7))
        .field("Guest RSP                                      ", &vmread_relaxed(vmcs::guest::RSP))
        .field("Guest RIP                                      ", &vmread_relaxed(vmcs::guest::RIP))
        .field("Guest RFLAGS                                   ", &vmread_relaxed(vmcs::guest::RFLAGS))
        .field("Guest pending debug exceptions                 ", &vmread_relaxed(vmcs::guest::PENDING_DBG_EXCEPTIONS))
        .field("Guest IA32_SYSENTER_ESP                        ", &vmread_relaxed(vmcs::guest::IA32_SYSENTER_ESP))
        .field("Guest IA32_SYSENTER_EIP                        ", &vmread_relaxed(vmcs::guest::IA32_SYSENTER_EIP))
        .field("Guest IA32_S_CET                               ", &vmread_relaxed(vmcs::guest::IA32_S_CET))
        .field("Guest SSP                                      ", &vmread_relaxed(vmcs::guest::SSP))
        .field("Guest IA32_INTERRUPT_SSP_TABLE_ADDR            ", &vmread_relaxed(vmcs::guest::IA32_INTERRUPT_SSP_TABLE_ADDR))

        // 16-Bit Host-State Fields
        .field("Host ES Selector                               ", &vmread_relaxed(vmcs::host::ES_SELECTOR))
        .field("Host CS Selector                               ", &vmread_relaxed(vmcs::host::CS_SELECTOR))
        .field("Host SS Selector                               ", &vmread_relaxed(vmcs::host::SS_SELECTOR))
        .field("Host DS Selector                               ", &vmread_relaxed(vmcs::host::DS_SELECTOR))
        .field("Host FS Selector                               ", &vmread_relaxed(vmcs::host::FS_SELECTOR))
        .field("Host GS Selector                               ", &vmread_relaxed(vmcs::host::GS_SELECTOR))
        .field("Host TR Selector                               ", &vmread_relaxed(vmcs::host::TR_SELECTOR))

        // 64-Bit Host-State Fields
        .field("Host IA32_PAT                                  ", &vmread_relaxed(vmcs::host::IA32_PAT))
        .field("Host IA32_EFER                                 ", &vmread_relaxed(vmcs::host::IA32_EFER))
        .field("Host IA32_PERF_GLOBAL_CTRL                     ", &vmread_relaxed(vmcs::host::IA32_PERF_GLOBAL_CTRL))
        .field("Host IA32_PKRS                                 ", &vmread_relaxed(vmcs::host::IA32_PKRS))

        // 32-Bit Host-State Fields
        .field("Host IA32_SYSENTER_CS                          ", &vmread_relaxed(vmcs::host::IA32_SYSENTER_CS))

        // Natural-Width Host-State Fields
        .field("Host CR0                                       ", &vmread_relaxed(vmcs::host::CR0))
        .field("Host CR3                                       ", &vmread_relaxed(vmcs::host::CR3))
        .field("Host CR4                                       ", &vmread_relaxed(vmcs::host::CR4))
        .field("Host FS Base                                   ", &vmread_relaxed(vmcs::host::FS_BASE))
        .field("Host GS Base                                   ", &vmread_relaxed(vmcs::host::GS_BASE))
        .field("Host TR base                                   ", &vmread_relaxed(vmcs::host::TR_BASE))
        .field("Host GDTR base                                 ", &vmread_relaxed(vmcs::host::GDTR_BASE))
        .field("Host IDTR base                                 ", &vmread_relaxed(vmcs::host::IDTR_BASE))
        .field("Host IA32_SYSENTER_ESP                         ", &vmread_relaxed(vmcs::host::IA32_SYSENTER_ESP))
        .field("Host IA32_SYSENTER_EIP                         ", &vmread_relaxed(vmcs::host::IA32_SYSENTER_EIP))
        .field("Host RSP                                       ", &vmread_relaxed(vmcs::host::RSP))
        .field("Host RIP                                       ", &vmread_relaxed(vmcs::host::RIP))
        .field("Host IA32_S_CET                                ", &vmread_relaxed(vmcs::host::IA32_S_CET))
        .field("Host SSP                                       ", &vmread_relaxed(vmcs::host::SSP))
        .field("Host IA32_INTERRUPT_SSP_TABLE_ADDR             ", &vmread_relaxed(vmcs::host::IA32_INTERRUPT_SSP_TABLE_ADDR))

        // 16-Bit Control Fields
        .field("Virtual-processor identifier                   ", &vmread_relaxed(vmcs::control::VPID))
        .field("Posted-interrupt notification vector           ", &vmread_relaxed(vmcs::control::POSTED_INTERRUPT_NOTIFICATION_VECTOR))
        .field("EPTP index                                     ", &vmread_relaxed(vmcs::control::EPTP_INDEX))
        .field("HLAT prefix size                               ", &vmread_relaxed(vmcs::control::HLAT_PREFIX_SIZE))
        .field("Last PID-pointer index                         ", &vmread_relaxed(vmcs::control::LAST_PID_POINTER_INDEX))

        // 64-Bit Control Fields
        .field("Address of I/O bitmap A                        ", &vmread_relaxed(vmcs::control::IO_BITMAP_A_ADDR))
        .field("Address of I/O bitmap B                        ", &vmread_relaxed(vmcs::control::IO_BITMAP_B_ADDR))
        .field("Address of MSR bitmaps                         ", &vmread_relaxed(vmcs::control::MSR_BITMAPS_ADDR))
        .field("VM-exit MSR-store address                      ", &vmread_relaxed(vmcs::control::VMEXIT_MSR_STORE_ADDR))
        .field("VM-exit MSR-load address                       ", &vmread_relaxed(vmcs::control::VMEXIT_MSR_LOAD_ADDR))
        .field("VM-entry MSR-load address                      ", &vmread_relaxed(vmcs::control::VMENTRY_MSR_LOAD_ADDR))
        .field("Executive-VMCS pointer                         ", &vmread_relaxed(vmcs::control::EXECUTIVE_VMCS_PTR))
        .field("PML address                                    ", &vmread_relaxed(vmcs::control::PML_ADDR))
        .field("TSC offset                                     ", &vmread_relaxed(vmcs::control::TSC_OFFSET))
        .field("Virtual-APIC address                           ", &vmread_relaxed(vmcs::control::VIRT_APIC_ADDR))
        .field("APIC-access address                            ", &vmread_relaxed(vmcs::control::APIC_ACCESS_ADDR))
        .field("Posted-interrupt descriptor address            ", &vmread_relaxed(vmcs::control::POSTED_INTERRUPT_DESC_ADDR))
        .field("VM-function controls                           ", &vmread_relaxed(vmcs::control::VM_FUNCTION_CONTROLS))
        .field("EPT pointer                                    ", &vmread_relaxed(vmcs::control::EPTP))
        .field("EOI-exit bitmap 0                              ", &vmread_relaxed(vmcs::control::EOI_EXIT0))
        .field("EOI-exit bitmap 1                              ", &vmread_relaxed(vmcs::control::EOI_EXIT1))
        .field("EOI-exit bitmap 2                              ", &vmread_relaxed(vmcs::control::EOI_EXIT2))
        .field("EOI-exit bitmap 3                              ", &vmread_relaxed(vmcs::control::EOI_EXIT3))
        .field("EPTP-list address                              ", &vmread_relaxed(vmcs::control::EPTP_LIST_ADDR))
        .field("VMREAD-bitmap address                          ", &vmread_relaxed(vmcs::control::VMREAD_BITMAP_ADDR))
        .field("VMWRITE-bitmap address                         ", &vmread_relaxed(vmcs::control::VMWRITE_BITMAP_ADDR))
        .field("Virtualization-exception information address   ", &vmread_relaxed(vmcs::control::VIRT_EXCEPTION_INFO_ADDR))
        .field("XSS-exiting bitmap                             ", &vmread_relaxed(vmcs::control::XSS_EXITING_BITMAP))
        .field("ENCLS-exiting bitmap                           ", &vmread_relaxed(vmcs::control::ENCLS_EXITING_BITMAP))
        .field("Sub-page-permission-table pointer              ", &vmread_relaxed(vmcs::control::SUBPAGE_PERM_TABLE_PTR))
        .field("TSC multiplier                                 ", &vmread_relaxed(vmcs::control::TSC_MULTIPLIER))
        .field("Tertiary processor-based VM-execution controls ", &vmread_relaxed(vmcs::control::TERTIARY_PROCBASED_EXEC_CONTROLS))
        .field("Low PASID directory address                    ", &vmread_relaxed(vmcs::control::LOW_PASID_DIRECTORY_ADDRESS))
        .field("High PASID directory address                   ", &vmread_relaxed(vmcs::control::HIGH_PASID_DIRECTORY_ADDRESS))
        .field("Shared EPT pointer                             ", &vmread_relaxed(vmcs::control::SHARED_EPT_POINTER))
        .field("PCONFIG-exiting bitmap                         ", &vmread_relaxed(vmcs::control::PCONFIG_EXITING_BITMAP))
        .field("HLATP                                          ", &vmread_relaxed(vmcs::control::HLATP))
        .field("PID-pointer table address                      ", &vmread_relaxed(vmcs::control::PID_POINTER_TABLE_ADDRESS))
        .field("Secondary VM-exit controls                     ", &vmread_relaxed(vmcs::control::SECONDARY_VM_EXIT_CONTROLS))
        .field("IA32_SPEC_CTRL mask                            ", &vmread_relaxed(vmcs::control::IA32_SPEC_CTRL_MASK))
        .field("IA32_SPEC_CTRL shadow                          ", &vmread_relaxed(vmcs::control::IA32_SPEC_CTRL_SHADOW))

        // 32-Bit Control Fields
        .field("Pin-based VM-execution controls                ", &vmread_relaxed(vmcs::control::PINBASED_EXEC_CONTROLS))
        .field("Primary processor-based VM-execution controls  ", &vmread_relaxed(vmcs::control::PRIMARY_PROCBASED_EXEC_CONTROLS))
        .field("Exception bitmap                               ", &vmread_relaxed(vmcs::control::EXCEPTION_BITMAP))
        .field("Page-fault error-code mask                     ", &vmread_relaxed(vmcs::control::PAGE_FAULT_ERR_CODE_MASK))
        .field("Page-fault error-code match                    ", &vmread_relaxed(vmcs::control::PAGE_FAULT_ERR_CODE_MATCH))
        .field("CR3-target count                               ", &vmread_relaxed(vmcs::control::CR3_TARGET_COUNT))
        .field("Primary VM-exit controls                       ", &vmread_relaxed(vmcs::control::PRIMARY_VMEXIT_CONTROLS))
        .field("VM-exit MSR-store count                        ", &vmread_relaxed(vmcs::control::VMEXIT_MSR_STORE_COUNT))
        .field("VM-exit MSR-load count                         ", &vmread_relaxed(vmcs::control::VMEXIT_MSR_LOAD_COUNT))
        .field("VM-entry controls                              ", &vmread_relaxed(vmcs::control::VMENTRY_CONTROLS))
        .field("VM-entry MSR-load count                        ", &vmread_relaxed(vmcs::control::VMENTRY_MSR_LOAD_COUNT))
        .field("VM-entry interruption-information field        ", &vmread_relaxed(vmcs::control::VMENTRY_INTERRUPTION_INFO_FIELD))
        .field("VM-entry exception error code                  ", &vmread_relaxed(vmcs::control::VMENTRY_EXCEPTION_ERR_CODE))
        .field("VM-entry instruction length                    ", &vmread_relaxed(vmcs::control::VMENTRY_INSTRUCTION_LEN))
        .field("TPR threshold                                  ", &vmread_relaxed(vmcs::control::TPR_THRESHOLD))
        .field("Secondary processor-based VM-execution controls", &vmread_relaxed(vmcs::control::SECONDARY_PROCBASED_EXEC_CONTROLS))
        .field("PLE_Gap                                        ", &vmread_relaxed(vmcs::control::PLE_GAP))
        .field("PLE_Window                                     ", &vmread_relaxed(vmcs::control::PLE_WINDOW))
        .field("Instruction-timeout control                    ", &vmread_relaxed(vmcs::control::INSTRUCTION_TIMEOUT_CONTROL))

        // Natural-Width Control Fields
        .field("CR0 guest/host mask                            ", &vmread_relaxed(vmcs::control::CR0_GUEST_HOST_MASK))
        .field("CR4 guest/host mask                            ", &vmread_relaxed(vmcs::control::CR4_GUEST_HOST_MASK))
        .field("CR0 read shadow                                ", &vmread_relaxed(vmcs::control::CR0_READ_SHADOW))
        .field("CR4 read shadow                                ", &vmread_relaxed(vmcs::control::CR4_READ_SHADOW))
        .field("CR3-target value 0                             ", &vmread_relaxed(vmcs::control::CR3_TARGET_VALUE0))
        .field("CR3-target value 1                             ", &vmread_relaxed(vmcs::control::CR3_TARGET_VALUE1))
        .field("CR3-target value 2                             ", &vmread_relaxed(vmcs::control::CR3_TARGET_VALUE2))
        .field("CR3-target value 3                             ", &vmread_relaxed(vmcs::control::CR3_TARGET_VALUE3))

        // 16-Bit Read-Only Data Fields

        // 64-Bit Read-Only Data Fields
        .field("Guest-physical address                         ", &vmread_relaxed(vmcs::ro::GUEST_PHYSICAL_ADDR))

        // 32-Bit Read-Only Data Fields
        .field("VM-instruction error                           ", &vmread_relaxed(vmcs::ro::VM_INSTRUCTION_ERROR))
        .field("Exit reason                                    ", &vmread_relaxed(vmcs::ro::EXIT_REASON))
        .field("VM-exit interruption information               ", &vmread_relaxed(vmcs::ro::VMEXIT_INTERRUPTION_INFO))
        .field("VM-exit interruption error code                ", &vmread_relaxed(vmcs::ro::VMEXIT_INTERRUPTION_ERR_CODE))
        .field("IDT-vectoring information field                ", &vmread_relaxed(vmcs::ro::IDT_VECTORING_INFO))
        .field("IDT-vectoring error code                       ", &vmread_relaxed(vmcs::ro::IDT_VECTORING_ERR_CODE))
        .field("VM-exit instruction length                     ", &vmread_relaxed(vmcs::ro::VMEXIT_INSTRUCTION_LEN))
        .field("VM-exit instruction information                ", &vmread_relaxed(vmcs::ro::VMEXIT_INSTRUCTION_INFO))

        // Natural-Width Read-Only Data Fields
        .field("Exit qualification                             ", &vmread_relaxed(vmcs::ro::EXIT_QUALIFICATION))
        .field("I/O RCX                                        ", &vmread_relaxed(vmcs::ro::IO_RCX))
        .field("I/O RSI                                        ", &vmread_relaxed(vmcs::ro::IO_RSI))
        .field("I/O RDI                                        ", &vmread_relaxed(vmcs::ro::IO_RDI))
        .field("I/O RIP                                        ", &vmread_relaxed(vmcs::ro::IO_RIP))
        .field("Guest-linear address                           ", &vmread_relaxed(vmcs::ro::GUEST_LINEAR_ADDR))
        .finish_non_exhaustive()
    }
}

bitfield::bitfield! {
    /// See: Table 29-3. Exit Qualification for Control-Register Accesses
    #[derive(Clone, Copy, Debug)]
    pub(crate) struct VmExitQualificationMovCr(u64);

    pub control_register, _: 3, 0;
    pub access_type, _: 5, 4;
    pub lmsw_operand_type, _: 6;
    pub general_purpose_register, _: 11, 8;
    pub lmsw_source_data, _: 31, 16;
}

/// See: Table 29-3. Exit Qualification for Control-Register Accesses
#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq, TryFrom)]
#[try_from(repr)]
#[repr(u8)]
pub(crate) enum MovCrAccessType {
    MovToCr,
    MovFromCr,
    Clts,
    Lmsw,
}

bitfield::bitfield! {
    /// See: Table 26-17. Format of the VM-Entry Interruption-Information Field
    #[derive(Clone, Copy, Debug)]
    pub(crate) struct VmEntryInterruptionInfo(u32);

    pub vector, set_vector: 7, 0;
    pub interruption_type, set_interruption_type: 10, 8;
    pub deliver_error_code, set_deliver_error_code: 11;
    pub valid, set_valid: 31;
}

/// See: Table 26-17. Format of the VM-Entry Interruption-Information Field
#[allow(dead_code)]
#[derive(Clone, Copy, Debug, TryFrom)]
#[try_from(repr)]
#[repr(u8)]
pub(crate) enum InterruptType {
    ExternalInterrupt,
    NonMaskableInterrupt = 2,
    HardwareException,
    SoftwareInterrupt,
    PrivilegedSoftwareException,
    SoftwareException,
    OtherEvent,
}

/// See: Table 32-1. VM-Instruction Error Numbers
#[derive(Clone, Copy, Debug, Display, TryFrom, Error)]
#[try_from(repr)]
#[repr(u64)]
pub(crate) enum ErrorNumber {
    #[display("VMCALL executed in VMX root operation")]
    VmcallInVmxRoot = 1,

    #[display("VMCLEAR with invalid physical address")]
    VmclearWithInvalidPa,

    #[display("VMCLEAR with VMXON pointer")]
    VmclearWithVmxonPtr,

    #[display("VMLAUNCH with non-clear VMCS")]
    VmlaunchWithNonClearVmcs,

    #[display("VMRESUME with non-launched VMCS")]
    VmresumeWithNonLaunchedVmcs,

    #[display("VMRESUME after VMXOFF (VMXOFF and VMXON between VMLAUNCH and VMRESUME)")]
    VmresumeAfterVmxoff,

    #[display("VM entry with invalid control field(s)")]
    VmEntryWithInvalidControlField,

    #[display("VM entry with invalid host-state field(s)")]
    VmEntryWithInvalidHostField,

    #[display("VMPTRLD with invalid physical address")]
    VmptrldWithInvalidPa,

    #[display("VMPTRLD with VMXON pointer")]
    VmptrldWithVmxonPtr,

    #[display("VMPTRLD with incorrect VMCS revision identifier")]
    VmptrldWithIncorrectVmcsRevId,

    #[display("VMREAD/VMWRITE from/to unsupported VMCS component")]
    VmreadVmwriteUnsupportedComponent,

    #[display("VMWRITE to read-only VMCS component")]
    VmwriteToReadOnlyComponent,

    #[display("VMXON executed in VMX root operation")]
    VmxonInVmxRoot = 15,

    #[display("VM entry with invalid executive-VMCS pointer")]
    VmEntryWithInvalidExecutiveVmcs,

    #[display("VM entry with non-launched executive VMCS")]
    VmEntryWithNonLaunchedExecutiveVmcs,

    #[display(
        "VM entry with executive-VMCS pointer not VMXON pointer (when attempting to deactivate the dual-monitor treatment of SMIs and SMM)"
    )]
    VmEntryWithExecutiveVmcs,

    #[display(
        "VMCALL with non-clear VMCS (when attempting to activate the dual-monitor treatment of SMIs and SMM)"
    )]
    VmcallWithNonClearVmcs,

    #[display("VMCALL with invalid VM-exit control fields")]
    VmcallWithInvalidVmExitControlField,

    #[display(
        "VMCALL with incorrect MSEG revision identifier (when attempting to activate the dual-monitor treatment of SMIs and SMM)"
    )]
    VmcallWithIncorrectMsegRevId = 22,

    #[display("VMXOFF under dual-monitor treatment of SMIs and SMM")]
    VmxoffUnderDualMonitorTreatment,

    #[display(
        "VMCALL with invalid SMM-monitor features (when attempting to activate the dual-monitor treatment of SMIs and SMM)"
    )]
    VmcallWithInvalidSmmMonitorFeatures,

    #[display(
        "VM entry with invalid VM-execution control fields in executive VMCS (when attempting to return from SMM)"
    )]
    VmEntryWithInvalidVmExecutionControlField,

    #[display("VM entry with events blocked by MOV SS")]
    VmEntryWithEventsBlockedByMovSs,

    #[display("Invalid operand to INVEPT/INVVPID")]
    InvalidOperandToInveptInvvpid = 27,
}

/// See: Table C-1. Basic Exit Reasons
#[derive(Clone, Copy, Debug, Display, TryFrom)]
#[try_from(repr)]
#[repr(u16)]
pub(crate) enum BasicExitReason {
    #[display(
        "Guest software caused an exception and the bit in the exception bitmap associated with exception's vector was 1. Or, an NMI was delivered to the logical processor and the \"NMI exiting\" VM-execution control was 1"
    )]
    ExceptionOrNmi,

    #[display(
        "An external interrupt arrived and the \"external-interrupt exiting\" VM-execution control was 1"
    )]
    ExternalInterrupt,

    #[display(
        "The logical processor encountered an exception while attempting to call the double-fault handler and that exception did not itself cause a VM exit due to the exception bitmap"
    )]
    TripleTault,

    #[display("An INIT signal arrived")]
    InitSignal,

    #[display("A SIPI arrived while the logical processor was in the \"wait-for-SIPI\" state")]
    SipiSignal,

    #[display(
        "An SMI arrived immediately after retirement of an I/O instruction and caused an SMM VM exit"
    )]
    IoSmi,

    #[display(
        "An SMI arrived and caused an SMM VM exit but not immediately after retirement of an I/O instruction"
    )]
    OtherSmi,

    #[display(
        "At the beginning of an instruction, RFLAGS.IF was 1; events were not blocked by STI or by MOV SS; and the \"interrupt-window exiting\" VM-execution control was 1"
    )]
    InterruptWindow,

    #[display(
        "At the beginning of an instruction, there was no virtual-NMI blocking; events were not blocked by MOV SS; and the \"NMI-window exiting\" VM-execution control was 1"
    )]
    NmiWindow,

    #[display("Guest software attempted a task switch")]
    TaskSwitch,

    #[display("Guest software attempted to execute CPUID")]
    Cpuid,

    #[display("Guest software attempted to execute GETSEC")]
    Getsec,

    #[display(
        "Guest software attempted to execute HLT and the \"HLT exiting\" VM-execution control was 1"
    )]
    Hlt,

    #[display("Guest software attempted to execute INVD")]
    Indv,

    #[display(
        "Guest software attempted to execute INVLPG and the \"INVLPG exiting\" VM-execution control was 1"
    )]
    Invlpg,

    #[display(
        "Guest software attempted to execute RDPMC and the \"RDPMC exiting\" VM-execution control was 1"
    )]
    Rdpmc,

    #[display(
        "Guest software attempted to execute RDTSC and the \"RDTSC exiting\" VM-execution control was 1"
    )]
    Rdtsc,

    #[display("Guest software attempted to execute RSM in SMM")]
    Rsm,

    #[display(
        "VMCALL was executed either by guest software (causing an ordinary VM exit) or by the executive monitor (causing an SMM VM exit)"
    )]
    Vmcall,

    #[display("Guest software attempted to execute VMCLEAR")]
    Vmclear,

    #[display("Guest software attempted to execute VMLAUNCH")]
    Vmlaunch,

    #[display("Guest software attempted to execute VMPTRLD")]
    Vmptrld,

    #[display("Guest software attempted to execute VMPTRST")]
    Vmptrst,

    #[display("Guest software attempted to execute VMREAD")]
    Vmread,

    #[display("Guest software attempted to execute VMRESUME")]
    Vmresume,

    #[display("Guest software attempted to execute VMWRITE")]
    Vmwrite,

    #[display("Guest software attempted to execute VMXOFF")]
    Vmxoff,

    #[display("Guest software attempted to execute VMXON")]
    Vmxon,

    #[display(
        "Guest software attempted to access CR0, CR3, CR4, or CR8 using CLTS, LMSW, or MOV CR and the VM-execution control fields indicate that a VM exit should occur"
    )]
    MovCr,

    #[display(
        "Guest software attempted a MOV to or from a debug register and the \"MOV-DR exiting\" VM-execution control was 1"
    )]
    MovDr,

    #[display("Guest software attempted to execute an I/O instruction")]
    IoInstruction,

    #[display("Guest software attempted to execute RDMSR")]
    Rdmsr,

    #[display("Guest software attempted to execute WRMSR")]
    Wrmsr,

    #[display("A VM entry failed one of the checks")]
    VmEntryFailureDueToInvalidGuestState,

    #[display("A VM entry failed in an attempt to load MSRs")]
    VmEntryFailureDueToMsrLoading,

    #[display(
        "Guest software attempted to execute MWAIT and the \"MWAIT exiting\" VM-execution control was 1"
    )]
    Mwait = 36,

    #[display(
        "A VM exit occurred due to the 1-setting of the \"monitor trap flag\" VM-execution control or VM entry injected a pending MTF VM exit as part of VM entry"
    )]
    MonitorTrapFlag,

    #[display(
        "Guest software attempted to execute MONITOR and the \"MONITOR exiting\" VM-execution control was 1"
    )]
    Monitor = 39,

    #[display(
        "Either guest software attempted to execute PAUSE and the \"PAUSE exiting\" VM-execution control was 1 or the \"PAUSE-loop exiting\" VM-execution control was 1 and guest software executed a PAUSE loop with execution time exceeding PLE_Window"
    )]
    Pause,

    #[display("A machine-check event occurred during VM entry")]
    VmEntryFailureDueToMachineCheckEvent,

    #[display(
        "The logical processor determined that the value of bits 7:4 of the byte at offset 080H on the virtual-APIC page was below that of the TPR threshold VM-execution control field while the \"use TPR shadow\" VMexecution control was 1 either as part of TPR virtualization or VM entry"
    )]
    TprBelowThreshold = 43,

    #[display(
        "Guest software attempted to access memory at a physical address on the APIC-access page and the \"virtualize APIC accesses\" VM-execution control was 1"
    )]
    ApicAccess,

    #[display(
        "EOI virtualization was performed for a virtual interrupt whose vector indexed a bit set in the EOIexit bitmap."
    )]
    VirtualizedEoi,

    #[display(
        "Guest software attempted to execute LGDT, LIDT, SGDT, or SIDT and the \"descriptor-table exiting\" VM-execution control was 1"
    )]
    AccessToGdtrOrIdtr,

    #[display(
        "Guest software attempted to execute LLDT, LTR, SLDT, or STR and the \"descriptor-table exiting\" VM-execution control was 1."
    )]
    AccessToLdtrOrTr,

    #[display(
        "An attempt to access memory with a guest-physical address was disallowed by the configuration of the EPT paging structures"
    )]
    EptViolation,

    #[display(
        "An attempt to access memory with a guest-physical address encountered a misconfigured EPT paging-structure entry"
    )]
    EptMisconfiguration,

    #[display("Guest software attempted to execute INVEPT")]
    Invept,

    #[display(
        "Guest software attempted to execute RDTSCP and the \"enable RDTSCP\" and \"RDTSC exiting\" VM-execution controls were both 1"
    )]
    Rdtscp,

    #[display("The preemption timer counted down to zero")]
    VmxPreemptionTimerExpired,

    #[display("Guest software attempted to execute INVVPID")]
    Invvpid,

    #[display(
        "Guest software attempted to execute WBINVD or WBNOINVD and the \"WBINVD exiting\" VM-execution control was 1"
    )]
    WbinvdOrWbnoinvd,

    #[display("Guest software attempted to execute XSETBV")]
    Xsetbv,

    #[display(
        "Guest software completed a write to the virtual-APIC page that must be virtualized by VMM software"
    )]
    ApicWrite,

    #[display(
        "Guest software attempted to execute RDRAND and the \"RDRAND exiting\" VM-execution control was 1"
    )]
    Rdrand,

    #[display(
        "Guest software attempted to execute INVPCID and the \"enable INVPCID\" and \"INVLPG exiting\" VM-execution controls were both 1"
    )]
    Invpcid,

    #[display(
        "Guest software invoked a VM function with the VMFUNC instruction and the VM function either was not enabled or generated a function-specific condition causing a VM exit"
    )]
    Vmfunc,

    #[display(
        "Guest software attempted to execute ENCLS, \"enable ENCLS exiting\" VM-execution control was 1, and either (1) EAX < 63 and the corresponding bit in the ENCLS-exiting bitmap is 1; or (2) EAX > 63 and bit 63 in the ENCLS-exiting bitmap is 1"
    )]
    Encls,

    #[display(
        "Guest software attempted to execute RDSEED and the \"RDSEED exiting\" VM-execution control was 1"
    )]
    Rdseed,

    #[display(
        "The processor attempted to create a page-modification log entry and the value of the PML index was not in the range 0-511"
    )]
    PageModificationLogFull,

    #[display(
        "Guest software attempted to execute XSAVES, the \"enable XSAVES/XRSTORS\" was 1, and a bit was set in the logical-AND of the following three values: EDX:EAX, the IA32_XSS MSR, and the XSS-exiting bitmap"
    )]
    Xsaves,

    #[display(
        "Guest software attempted to execute XRSTORS, the \"enable XSAVES/XRSTORS\" was 1, and a bit was set in the logical-AND of the following three values: EDX:EAX, the IA32_XSS MSR, and the XSS-exiting bitmap"
    )]
    Xrstors,

    #[display(
        "Guest software attempted to execute PCONFIG, \"enable PCONFIG\" VM-execution control was 1, and either (1) EAX < 63 and the corresponding bit in the PCONFIG-exiting bitmap is 1; or (2) EAX > 63 and bit 63 in the PCONFIG-exiting bitmap is 1"
    )]
    Pconfig,

    #[display(
        "The processor attempted to determine an access's sub-page write permission and encountered an SPP miss or an SPP misconfiguration."
    )]
    SppRelatedevent,

    #[display(
        "Guest software attempted to execute UMWAIT and the \"enable user wait and pause\" and \"RDTSC exiting\" VM-execution controls were both 1"
    )]
    Umwait,

    #[display(
        "Guest software attempted to execute TPAUSE and the \"enable user wait and pause\" and \"RDTSC exiting\" VM-execution controls were both 1."
    )]
    Tpause,

    #[display(
        "Guest software attempted to execute LOADIWKEY and the \"LOADIWKEY exiting\" VM-execution control was 1."
    )]
    Loadiwkey,

    #[display(
        "A VM exit occurred during PASID translation because the present bit was clear in a PASID-directory entry, the valid bit was clear in a PASID-table entry, or one of the entries set a reserved bit"
    )]
    EnqcmdPasidTranslationFailure = 72,

    #[display(
        "A VM exit occurred during PASID translation because the present bit was clear in a PASID-directory entry, the valid bit was clear in a PASID-table entry, or one of the entries set a reserved bit."
    )]
    EnqcmdsPasidTranslationFailure,

    #[display(
        "The processor asserted a bus lock while the \"bus-lock detection\" VM-execution control was 1"
    )]
    BusLock,

    #[display(
        "The \"instruction timeout\" VM-execution control was 1 and certain operations prevented the processor from reaching an instruction boundary within the amount of time specified by the instruction-timeout control."
    )]
    InstructionTimeout,

    #[display("Guest software attempted to execute SEAMCALL")]
    Seamcall,

    #[display("Guest software attempted to execute TDCALL")]
    Tdcall,
}

pub(crate) mod vmcs {
    /// VM-execution, VM-exit, and VM-entry control fields.
    /// See: APPENDIX B FIELD ENCODING IN VMCS
    pub(crate) mod control {

        // See: B.1.1 16-Bit Control Fields
        /// Virtual-processor identifier (VPID).
        pub(crate) const VPID: u32 = 0x0;
        /// Posted-interrupt notification vector.
        pub(crate) const POSTED_INTERRUPT_NOTIFICATION_VECTOR: u32 = 0x2;
        /// EPTP index.
        pub(crate) const EPTP_INDEX: u32 = 0x4;
        /// HLAT prefix size
        pub(crate) const HLAT_PREFIX_SIZE: u32 = 0x6;
        /// Last PID-pointer index
        pub(crate) const LAST_PID_POINTER_INDEX: u32 = 0x8;

        // See: B.2.1 64-Bit Control Fields
        /// Address of I/O bitmap A.
        pub(crate) const IO_BITMAP_A_ADDR: u32 = 0x2000;
        /// Address of I/O bitmap B.
        pub(crate) const IO_BITMAP_B_ADDR: u32 = 0x2002;
        /// Address of MSR bitmaps.
        pub(crate) const MSR_BITMAPS_ADDR: u32 = 0x2004;
        /// VM-exit MSR-store address.
        pub(crate) const VMEXIT_MSR_STORE_ADDR: u32 = 0x2006;
        /// VM-exit MSR-load address.
        pub(crate) const VMEXIT_MSR_LOAD_ADDR: u32 = 0x2008;
        /// VM-entry MSR-load address.
        pub(crate) const VMENTRY_MSR_LOAD_ADDR: u32 = 0x200A;
        /// Executive-VMCS pointer.
        pub(crate) const EXECUTIVE_VMCS_PTR: u32 = 0x200C;
        /// PML address.
        pub(crate) const PML_ADDR: u32 = 0x200E;
        /// TSC offset.
        pub(crate) const TSC_OFFSET: u32 = 0x2010;
        /// Virtual-APIC address.
        pub(crate) const VIRT_APIC_ADDR: u32 = 0x2012;
        /// APIC-access address.
        pub(crate) const APIC_ACCESS_ADDR: u32 = 0x2014;
        /// Posted-interrupt descriptor address.
        pub(crate) const POSTED_INTERRUPT_DESC_ADDR: u32 = 0x2016;
        /// VM-function controls.
        pub(crate) const VM_FUNCTION_CONTROLS: u32 = 0x2018;
        /// EPT pointer.
        pub(crate) const EPTP: u32 = 0x201A;
        /// EOI-exit bitmap 0.
        pub(crate) const EOI_EXIT0: u32 = 0x201C;
        /// EOI-exit bitmap 1.
        pub(crate) const EOI_EXIT1: u32 = 0x201E;
        /// EOI-exit bitmap 2.
        pub(crate) const EOI_EXIT2: u32 = 0x2020;
        /// EOI-exit bitmap 3.
        pub(crate) const EOI_EXIT3: u32 = 0x2022;
        /// EPTP-list address.
        pub(crate) const EPTP_LIST_ADDR: u32 = 0x2024;
        /// VMREAD-bitmap address.
        pub(crate) const VMREAD_BITMAP_ADDR: u32 = 0x2026;
        /// VMWRITE-bitmap address.
        pub(crate) const VMWRITE_BITMAP_ADDR: u32 = 0x2028;
        /// Virtualization-exception information address.
        pub(crate) const VIRT_EXCEPTION_INFO_ADDR: u32 = 0x202A;
        /// XSS-exiting bitmap.
        pub(crate) const XSS_EXITING_BITMAP: u32 = 0x202C;
        /// ENCLS-exiting bitmap.
        pub(crate) const ENCLS_EXITING_BITMAP: u32 = 0x202E;
        /// Sub-page-permission-table pointer.
        pub(crate) const SUBPAGE_PERM_TABLE_PTR: u32 = 0x2030;
        /// TSC multiplier.
        pub(crate) const TSC_MULTIPLIER: u32 = 0x2032;
        /// Tertiary Processor-Based VM-Execution Controls.
        pub(crate) const TERTIARY_PROCBASED_EXEC_CONTROLS: u32 = 0x2034;
        /// Low PASID directory address.
        pub(crate) const LOW_PASID_DIRECTORY_ADDRESS: u32 = 0x2038;
        /// High PASID directory address.
        pub(crate) const HIGH_PASID_DIRECTORY_ADDRESS: u32 = 0x203A;
        /// Shared EPT pointer.
        pub(crate) const SHARED_EPT_POINTER: u32 = 0x203C;
        /// PCONFIG-exiting bitmap.
        pub(crate) const PCONFIG_EXITING_BITMAP: u32 = 0x203E;
        /// Hypervisor-managed linear-address translation pointer.
        pub(crate) const HLATP: u32 = 0x2040;
        /// PID-pointer table address.
        pub(crate) const PID_POINTER_TABLE_ADDRESS: u32 = 0x2042;
        /// Secondary VM-exit controls.
        pub(crate) const SECONDARY_VM_EXIT_CONTROLS: u32 = 0x2044;
        /// IA32_SPEC_CTRL mask.
        pub(crate) const IA32_SPEC_CTRL_MASK: u32 = 0x204A;
        /// IA32_SPEC_CTRL shadow.
        pub(crate) const IA32_SPEC_CTRL_SHADOW: u32 = 0x204C;

        // B.3.1 32-Bit Control Fields
        /// Pin-based VM-execution controls.
        pub(crate) const PINBASED_EXEC_CONTROLS: u32 = 0x4000;
        /// Primary processor-based VM-execution controls.
        pub(crate) const PRIMARY_PROCBASED_EXEC_CONTROLS: u32 = 0x4002;
        /// Exception bitmap.
        pub(crate) const EXCEPTION_BITMAP: u32 = 0x4004;
        /// Page-fault error-code mask.
        pub(crate) const PAGE_FAULT_ERR_CODE_MASK: u32 = 0x4006;
        /// Page-fault error-code match.
        pub(crate) const PAGE_FAULT_ERR_CODE_MATCH: u32 = 0x4008;
        /// CR3-target count.
        pub(crate) const CR3_TARGET_COUNT: u32 = 0x400A;
        /// VM-exit controls.
        pub(crate) const PRIMARY_VMEXIT_CONTROLS: u32 = 0x400C;
        /// VM-exit MSR-store count.
        pub(crate) const VMEXIT_MSR_STORE_COUNT: u32 = 0x400E;
        /// VM-exit MSR-load count.
        pub(crate) const VMEXIT_MSR_LOAD_COUNT: u32 = 0x4010;
        /// VM-entry controls.
        pub(crate) const VMENTRY_CONTROLS: u32 = 0x4012;
        /// VM-entry MSR-load count.
        pub(crate) const VMENTRY_MSR_LOAD_COUNT: u32 = 0x4014;
        /// VM-entry interruption-information field.
        pub(crate) const VMENTRY_INTERRUPTION_INFO_FIELD: u32 = 0x4016;
        /// VM-entry exception error code.
        pub(crate) const VMENTRY_EXCEPTION_ERR_CODE: u32 = 0x4018;
        /// VM-entry instruction length.
        pub(crate) const VMENTRY_INSTRUCTION_LEN: u32 = 0x401A;
        /// TPR threshold.
        pub(crate) const TPR_THRESHOLD: u32 = 0x401C;
        /// Secondary processor-based VM-execution controls.
        pub(crate) const SECONDARY_PROCBASED_EXEC_CONTROLS: u32 = 0x401E;
        /// PLE_Gap.
        pub(crate) const PLE_GAP: u32 = 0x4020;
        /// PLE_Window.
        pub(crate) const PLE_WINDOW: u32 = 0x4022;
        /// Instruction-timeout control.
        pub(crate) const INSTRUCTION_TIMEOUT_CONTROL: u32 = 0x4024;

        // B.4.1 Natural-Width Control Fields
        /// CR0 guest/host mask.
        pub(crate) const CR0_GUEST_HOST_MASK: u32 = 0x6000;
        /// CR4 guest/host mask.
        pub(crate) const CR4_GUEST_HOST_MASK: u32 = 0x6002;
        /// CR0 read shadow.
        pub(crate) const CR0_READ_SHADOW: u32 = 0x6004;
        /// CR4 read shadow.
        pub(crate) const CR4_READ_SHADOW: u32 = 0x6006;
        /// CR3-target value 0.
        pub(crate) const CR3_TARGET_VALUE0: u32 = 0x6008;
        /// CR3-target value 1.
        pub(crate) const CR3_TARGET_VALUE1: u32 = 0x600A;
        /// CR3-target value 2.
        pub(crate) const CR3_TARGET_VALUE2: u32 = 0x600C;
        /// CR3-target value 3.
        pub(crate) const CR3_TARGET_VALUE3: u32 = 0x600E;

        bitflags::bitflags! {
            /// See: Table 26-5. Definitions of Pin-Based VM-Execution Controls
            pub(crate) struct PinbasedControls: u32 {
                /// External-interrupt exiting.
                const EXTERNAL_INTERRUPT_EXITING = 1 << 0;
                /// NMI exiting.
                const NMI_EXITING = 1 << 3;
                /// Virtual NMIs.
                const VIRTUAL_NMIS = 1 << 5;
                /// Activate VMX-preemption timer.
                const VMX_PREEMPTION_TIMER = 1 << 6;
                /// Process posted interrupts.
                const POSTED_INTERRUPTS = 1 << 7;
            }
        }

        bitflags::bitflags! {
            /// See: Table 26-6. Definitions of Primary Processor-Based VM-Execution Controls
            pub(crate) struct PrimaryControls: u32 {
                /// Interrupt-window exiting.
                const INTERRUPT_WINDOW_EXITING = 1 << 2;
                /// Use TSC offsetting.
                const USE_TSC_OFFSETTING = 1 << 3;
                /// HLT exiting.
                const HLT_EXITING = 1 << 7;
                /// INVLPG exiting.
                const INVLPG_EXITING = 1 << 9;
                /// MWAIT exiting.
                const MWAIT_EXITING = 1 << 10;
                /// RDPMC exiting.
                const RDPMC_EXITING = 1 << 11;
                /// RDTSC exiting.
                const RDTSC_EXITING = 1 << 12;
                /// CR3-load exiting.
                const CR3_LOAD_EXITING = 1 << 15;
                /// CR3-store exiting.
                const CR3_STORE_EXITING = 1 << 16;
                /// CR8-load exiting.
                const CR8_LOAD_EXITING = 1 << 19;
                /// CR8-store exiting.
                const CR8_STORE_EXITING = 1 << 20;
                /// Use TPR shadow.
                const USE_TPR_SHADOW = 1 << 21;
                /// NMI-window exiting.
                const NMI_WINDOW_EXITING = 1 << 22;
                /// MOV-DR exiting
                const MOV_DR_EXITING = 1 << 23;
                /// Unconditional I/O exiting.
                const UNCOND_IO_EXITING = 1 << 24;
                /// Use I/O bitmaps.
                const USE_IO_BITMAPS = 1 << 25;
                /// Monitor trap flag.
                const MONITOR_TRAP_FLAG = 1 << 27;
                /// Use MSR bitmaps.
                const USE_MSR_BITMAPS = 1 << 28;
                /// MONITOR exiting.
                const MONITOR_EXITING = 1 << 29;
                /// PAUSE exiting.
                const PAUSE_EXITING = 1 << 30;
                /// Activate secondary controls.
                const SECONDARY_CONTROLS = 1 << 31;
            }
        }

        bitflags::bitflags! {
            /// See: Table 26-7. Definitions of Secondary Processor-Based VM-Execution Controls
            pub(crate) struct SecondaryControls: u32 {
                /// Virtualize APIC accesses.
                const VIRTUALIZE_APIC = 1 << 0;
                /// Enable EPT.
                const ENABLE_EPT = 1 << 1;
                /// Descriptor-table exiting.
                const DTABLE_EXITING = 1 << 2;
                /// Enable RDTSCP.
                const ENABLE_RDTSCP = 1 << 3;
                /// Virtualize x2APIC mode.
                const VIRTUALIZE_X2APIC = 1 << 4;
                /// Enable VPID.
                const ENABLE_VPID = 1 << 5;
                /// WBINVD exiting.
                const WBINVD_EXITING = 1 << 6;
                /// Unrestricted guest.
                const UNRESTRICTED_GUEST = 1 << 7;
                /// APIC-register virtualization.
                const VIRTUALIZE_APIC_REGISTER = 1 << 8;
                /// Virtual-interrupt delivery.
                const VIRTUAL_INTERRUPT_DELIVERY = 1 << 9;
                /// PAUSE-loop exiting.
                const PAUSE_LOOP_EXITING = 1 << 10;
                /// RDRAND exiting.
                const RDRAND_EXITING = 1 << 11;
                /// Enable INVPCID.
                const ENABLE_INVPCID = 1 << 12;
                /// Enable VM functions.
                const ENABLE_VM_FUNCTIONS = 1 << 13;
                /// VMCS shadowing.
                const VMCS_SHADOWING = 1 << 14;
                /// Enable ENCLS exiting.
                const ENCLS_EXITING = 1 << 15;
                /// RDSEED exiting.
                const RDSEED_EXITING = 1 << 16;
                /// Enable PML.
                const ENABLE_PML = 1 << 17;
                /// EPT-violation #VE.
                const EPT_VIOLATION_VE = 1 << 18;
                /// Conceal VMX from PT.
                const CONCEAL_VMX_FROM_PT = 1 << 19;
                /// Enable XSAVES/XRSTORS.
                const ENABLE_XSAVES_XRSTORS = 1 << 20;
                /// Mode-based execute control for EPT.
                const MODE_BASED_EPT = 1 << 22;
                /// Sub-page write permissions for EPT.
                const SUB_PAGE_EPT = 1 << 23;
                /// Intel PT uses guest physical addresses.
                const INTEL_PT_GUEST_PHYSICAL = 1 << 24;
                /// Use TSC scaling.
                const USE_TSC_SCALING = 1 << 25;
                /// Enable user wait and pause.
                const ENABLE_USER_WAIT_PAUSE = 1 << 26;
            }
        }

        bitflags::bitflags! {
            /// See: Table 26-13. Definitions of Primary VM-Exit Controls
            pub(crate) struct PrimaryExitControls: u32 {
                /// Save debug controls.
                const SAVE_DEBUG_CONTROLS = 1 << 2;
                /// Host address-space size.
                const HOST_ADDRESS_SPACE_SIZE = 1 << 9;
                /// Load IA32_PERF_GLOBAL_CTRL.
                const LOAD_IA32_PERF_GLOBAL_CTRL = 1 << 12;
                /// Acknowledge interrupt on exit.
                const ACK_INTERRUPT_ON_EXIT = 1 << 15;
                /// Save IA32_PAT.
                const SAVE_IA32_PAT = 1 << 18;
                /// Load IA32_PAT.
                const LOAD_IA32_PAT = 1 << 19;
                /// Save IA32_EFER.
                const SAVE_IA32_EFER = 1 << 20;
                /// Load IA32_EFER.
                const LOAD_IA32_EFER = 1 << 21;
                /// Save VMX-preemption timer.
                const SAVE_VMX_PREEMPTION_TIMER = 1 << 22;
                /// Clear IA32_BNDCFGS.
                const CLEAR_IA32_BNDCFGS = 1 << 23;
                /// Conceal VMX from PT.
                const CONCEAL_VMX_FROM_PT = 1 << 24;
                /// Clear IA32_RTIT_CTL.
                const CLEAR_IA32_RTIT_CTL = 1 << 25;
            }
        }

        bitflags::bitflags! {
            /// See: Table 26-16. Definitions of VM-Entry Controls
            pub(crate) struct EntryControls: u32 {
                /// Load debug controls.
                const LOAD_DEBUG_CONTROLS = 1 << 2;
                /// IA-32e mode guest.
                const IA32E_MODE_GUEST = 1 << 9;
                /// Entry to SMM.
                const ENTRY_TO_SMM = 1 << 10;
                /// Deactivate dual-monitor treatment.
                const DEACTIVATE_DUAL_MONITOR = 1 << 11;
                /// Load IA32_PERF_GLOBAL_CTRL.
                const LOAD_IA32_PERF_GLOBAL_CTRL = 1 << 13;
                /// Load IA32_PAT.
                const LOAD_IA32_PAT = 1 << 14;
                /// Load IA32_EFER.
                const LOAD_IA32_EFER = 1 << 15;
                /// Load IA32_BNDCFGS.
                const LOAD_IA32_BNDCFGS = 1 << 16;
                /// Conceal VMX from PT.
                const CONCEAL_VMX_FROM_PT = 1 << 17;
                /// Load IA32_RTIT_CTL.
                const LOAD_IA32_RTIT_CTL = 1 << 18;
            }
        }
    }

    /// VM-exit information fields.
    /// See: APPENDIX B FIELD ENCODING IN VMCS
    pub(crate) mod ro {
        // See: B.2.2 64-Bit Read-Only Data Fields
        /// Guest-physical address.
        pub(crate) const GUEST_PHYSICAL_ADDR: u32 = 0x2400;

        // See: B.3.2 32-Bit Read-Only Data Fields
        /// VM-instruction error.
        pub(crate) const VM_INSTRUCTION_ERROR: u32 = 0x4400;
        /// Exit reason.
        pub(crate) const EXIT_REASON: u32 = 0x4402;
        /// VM-exit interruption information.
        pub(crate) const VMEXIT_INTERRUPTION_INFO: u32 = 0x4404;
        /// VM-exit interruption error code.
        pub(crate) const VMEXIT_INTERRUPTION_ERR_CODE: u32 = 0x4406;
        /// IDT-vectoring information field.
        pub(crate) const IDT_VECTORING_INFO: u32 = 0x4408;
        /// IDT-vectoring error code.
        pub(crate) const IDT_VECTORING_ERR_CODE: u32 = 0x440A;
        /// VM-exit instruction length.
        pub(crate) const VMEXIT_INSTRUCTION_LEN: u32 = 0x440C;
        /// VM-exit instruction information.
        pub(crate) const VMEXIT_INSTRUCTION_INFO: u32 = 0x440E;

        // See: B.4.2 Natural-Width Read-Only Data Fields
        /// Exit qualification.
        pub(crate) const EXIT_QUALIFICATION: u32 = 0x6400;
        /// I/O RCX.
        pub(crate) const IO_RCX: u32 = 0x6402;
        /// I/O RSI.
        pub(crate) const IO_RSI: u32 = 0x6404;
        /// I/O RDI.
        pub(crate) const IO_RDI: u32 = 0x6406;
        /// I/O RIP.
        pub(crate) const IO_RIP: u32 = 0x6408;
        /// Guest-linear address.
        pub(crate) const GUEST_LINEAR_ADDR: u32 = 0x640A;
    }

    /// Fields used to access guest-state area.
    /// See: APPENDIX B FIELD ENCODING IN VMCS
    pub(crate) mod guest {
        // See: B.1.2 16-Bit Guest-State Fields
        /// Guest ES selector.
        pub(crate) const ES_SELECTOR: u32 = 0x800;
        /// Guest CS selector.
        pub(crate) const CS_SELECTOR: u32 = 0x802;
        /// Guest SS selector.
        pub(crate) const SS_SELECTOR: u32 = 0x804;
        /// Guest DS selector.
        pub(crate) const DS_SELECTOR: u32 = 0x806;
        /// Guest FS selector.
        pub(crate) const FS_SELECTOR: u32 = 0x808;
        /// Guest GS selector.
        pub(crate) const GS_SELECTOR: u32 = 0x80A;
        /// Guest LDTR selector.
        pub(crate) const LDTR_SELECTOR: u32 = 0x80C;
        /// Guest TR selector.
        pub(crate) const TR_SELECTOR: u32 = 0x80E;
        /// Guest interrupt status.
        pub(crate) const INTERRUPT_STATUS: u32 = 0x810;
        /// PML index.
        pub(crate) const PML_INDEX: u32 = 0x812;
        /// Guest UINV.
        pub(crate) const UINV: u32 = 0x814;

        // See: B.2.3 64-Bit Guest-State Fields
        /// VMCS link pointer.
        pub(crate) const LINK_PTR: u32 = 0x2800;
        /// Guest IA32_DEBUGCTL.
        pub(crate) const IA32_DEBUGCTL: u32 = 0x2802;
        /// Guest IA32_PAT.
        pub(crate) const IA32_PAT: u32 = 0x2804;
        /// Guest IA32_EFER.
        pub(crate) const IA32_EFER: u32 = 0x2806;
        /// Guest IA32_PERF_GLOBAL_CTRL.
        pub(crate) const IA32_PERF_GLOBAL_CTRL: u32 = 0x2808;
        /// Guest PDPTE0.
        pub(crate) const PDPTE0: u32 = 0x280A;
        /// Guest PDPTE1.
        pub(crate) const PDPTE1: u32 = 0x280C;
        /// Guest PDPTE2.
        pub(crate) const PDPTE2: u32 = 0x280E;
        /// Guest PDPTE3.
        pub(crate) const PDPTE3: u32 = 0x2810;
        /// Guest IA32_BNDCFGS.
        pub(crate) const IA32_BNDCFGS: u32 = 0x2812;
        /// Guest IA32_RTIT_CTL.
        pub(crate) const IA32_RTIT_CTL: u32 = 0x2814;
        /// Guest IA32_LBR_CTL.
        pub(crate) const IA32_LBR_CTL: u32 = 0x2816;
        /// Guest IA32_PKRS.
        pub(crate) const IA32_PKRS: u32 = 0x2818;

        // See: B.3.3 32-Bit Guest-State Fields
        /// Guest ES limit.
        pub(crate) const ES_LIMIT: u32 = 0x4800;
        /// Guest CS limit.
        pub(crate) const CS_LIMIT: u32 = 0x4802;
        /// Guest SS limit.
        pub(crate) const SS_LIMIT: u32 = 0x4804;
        /// Guest DS limit.
        pub(crate) const DS_LIMIT: u32 = 0x4806;
        /// Guest FS limit.
        pub(crate) const FS_LIMIT: u32 = 0x4808;
        /// Guest GS limit.
        pub(crate) const GS_LIMIT: u32 = 0x480A;
        /// Guest LDTR limit.
        pub(crate) const LDTR_LIMIT: u32 = 0x480C;
        /// Guest TR limit.
        pub(crate) const TR_LIMIT: u32 = 0x480E;
        /// Guest GDTR limit.
        pub(crate) const GDTR_LIMIT: u32 = 0x4810;
        /// Guest IDTR limit.
        pub(crate) const IDTR_LIMIT: u32 = 0x4812;
        /// Guest ES access rights.
        pub(crate) const ES_ACCESS_RIGHTS: u32 = 0x4814;
        /// Guest CS access rights.
        pub(crate) const CS_ACCESS_RIGHTS: u32 = 0x4816;
        /// Guest SS access rights.
        pub(crate) const SS_ACCESS_RIGHTS: u32 = 0x4818;
        /// Guest DS access rights.
        pub(crate) const DS_ACCESS_RIGHTS: u32 = 0x481A;
        /// Guest FS access rights.
        pub(crate) const FS_ACCESS_RIGHTS: u32 = 0x481C;
        /// Guest GS access rights.
        pub(crate) const GS_ACCESS_RIGHTS: u32 = 0x481E;
        /// Guest LDTR access rights.
        pub(crate) const LDTR_ACCESS_RIGHTS: u32 = 0x4820;
        /// Guest TR access rights.
        pub(crate) const TR_ACCESS_RIGHTS: u32 = 0x4822;
        /// Guest interruptibility state.
        pub(crate) const INTERRUPTIBILITY_STATE: u32 = 0x4824;
        /// Guest activity state.
        pub(crate) const ACTIVITY_STATE: u32 = 0x4826;
        /// Guest SMBASE.
        pub(crate) const SMBASE: u32 = 0x4828;
        /// Guest IA32_SYSENTER_CS.
        pub(crate) const IA32_SYSENTER_CS: u32 = 0x482A;
        /// VMX-preemption timer value.
        pub(crate) const VMX_PREEMPTION_TIMER_VALUE: u32 = 0x482E;

        // See: B.4.3 Natural-Width Guest-State Fields
        /// Guest CR0.
        pub(crate) const CR0: u32 = 0x6800;
        /// Guest CR3.
        pub(crate) const CR3: u32 = 0x6802;
        /// Guest CR4.
        pub(crate) const CR4: u32 = 0x6804;
        /// Guest ES base.
        pub(crate) const ES_BASE: u32 = 0x6806;
        /// Guest CS base.
        pub(crate) const CS_BASE: u32 = 0x6808;
        /// Guest SS base.
        pub(crate) const SS_BASE: u32 = 0x680A;
        /// Guest DS base.
        pub(crate) const DS_BASE: u32 = 0x680C;
        /// Guest FS base.
        pub(crate) const FS_BASE: u32 = 0x680E;
        /// Guest GS base.
        pub(crate) const GS_BASE: u32 = 0x6810;
        /// Guest LDTR base.
        pub(crate) const LDTR_BASE: u32 = 0x6812;
        /// Guest TR base.
        pub(crate) const TR_BASE: u32 = 0x6814;
        /// Guest GDTR base.
        pub(crate) const GDTR_BASE: u32 = 0x6816;
        /// Guest IDTR base.
        pub(crate) const IDTR_BASE: u32 = 0x6818;
        /// Guest DR7.
        pub(crate) const DR7: u32 = 0x681A;
        /// Guest RSP.
        pub(crate) const RSP: u32 = 0x681C;
        /// Guest RIP.
        pub(crate) const RIP: u32 = 0x681E;
        /// Guest RFLAGS.
        pub(crate) const RFLAGS: u32 = 0x6820;
        /// Guest pending debug exceptions.
        pub(crate) const PENDING_DBG_EXCEPTIONS: u32 = 0x6822;
        /// Guest IA32_SYSENTER_ESP.
        pub(crate) const IA32_SYSENTER_ESP: u32 = 0x6824;
        /// Guest IA32_SYSENTER_EIP.
        pub(crate) const IA32_SYSENTER_EIP: u32 = 0x6826;
        /// Guest IA32_S_CET.
        pub(crate) const IA32_S_CET: u32 = 0x6828;
        /// Guest SSP.
        pub(crate) const SSP: u32 = 0x682A;
        /// Guest IA32_INTERRUPT_SSP_TABLE_ADDR.
        pub(crate) const IA32_INTERRUPT_SSP_TABLE_ADDR: u32 = 0x682C;
    }

    /// Fields used to access host-state area.
    /// See: APPENDIX B FIELD ENCODING IN VMCS
    pub(crate) mod host {
        // See: B.1.3 16-Bit Host-State Fields
        /// Host ES selector.
        pub(crate) const ES_SELECTOR: u32 = 0xC00;
        /// Host CS selector.
        pub(crate) const CS_SELECTOR: u32 = 0xC02;
        /// Host SS selector.
        pub(crate) const SS_SELECTOR: u32 = 0xC04;
        /// Host DS selector.
        pub(crate) const DS_SELECTOR: u32 = 0xC06;
        /// Host FS selector.
        pub(crate) const FS_SELECTOR: u32 = 0xC08;
        /// Host GS selector.
        pub(crate) const GS_SELECTOR: u32 = 0xC0A;
        /// Host TR selector.
        pub(crate) const TR_SELECTOR: u32 = 0xC0C;

        // See: B.2.4 64-Bit Host-State Fields
        /// Host IA32_PAT.
        pub(crate) const IA32_PAT: u32 = 0x2C00;
        /// Host IA32_EFER.
        pub(crate) const IA32_EFER: u32 = 0x2C02;
        /// Host IA32_PERF_GLOBAL_CTRL.
        pub(crate) const IA32_PERF_GLOBAL_CTRL: u32 = 0x2C04;
        /// Host IA32_PKRS.
        pub(crate) const IA32_PKRS: u32 = 0x2C06;

        // See: B.4.4 Natural-Width Host-State Fields
        /// Host IA32_SYSENTER_CS.
        pub(crate) const IA32_SYSENTER_CS: u32 = 0x4C00;

        // B.4.4.: natural-width host-state fields
        /// Host CR0.
        pub(crate) const CR0: u32 = 0x6C00;
        /// Host CR3.
        pub(crate) const CR3: u32 = 0x6C02;
        /// Host CR4.
        pub(crate) const CR4: u32 = 0x6C04;
        /// Host FS base.
        pub(crate) const FS_BASE: u32 = 0x6C06;
        /// Host GS base.
        pub(crate) const GS_BASE: u32 = 0x6C08;
        /// Host TR base.
        pub(crate) const TR_BASE: u32 = 0x6C0A;
        /// Host GDTR base.
        pub(crate) const GDTR_BASE: u32 = 0x6C0C;
        /// Host IDTR base.
        pub(crate) const IDTR_BASE: u32 = 0x6C0E;
        /// Host IA32_SYSENTER_ESP.
        pub(crate) const IA32_SYSENTER_ESP: u32 = 0x6C10;
        /// Host IA32_SYSENTER_EIP.
        pub(crate) const IA32_SYSENTER_EIP: u32 = 0x6C12;
        /// Host RSP.
        pub(crate) const RSP: u32 = 0x6C14;
        /// Host RIP.
        pub(crate) const RIP: u32 = 0x6C16;
        /// Host IA32_S_CET.
        pub(crate) const IA32_S_CET: u32 = 0x6C18;
        /// Host SSP.
        pub(crate) const SSP: u32 = 0x6C1A;
        /// Host IA32_INTERRUPT_SSP_TABLE_ADDR.
        pub(crate) const IA32_INTERRUPT_SSP_TABLE_ADDR: u32 = 0x6C1C;
    }
}
