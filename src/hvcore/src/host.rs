//! Virtualizes processor(s) and handles VM-exits.
//!
//! This is THE file we should read the most. This file implements many of the
//! essential pieces for understanding how to use Intel VT-x, such as entering
//! VMX operation, initializing VMCS, launching a guest, and handling VM-exits.
//!
//! The most important function to read is [`entry_point`].

use alloc::alloc::handle_alloc_error;
use core::{
    alloc::Layout,
    arch::global_asm,
    ffi::c_void,
    ops::Range,
    sync::atomic::{AtomicU64, Ordering},
};
use spin::Once;

use linked_list_allocator::LockedHeap;

use crate::{
    misc::{Page, Spa, cpu_dead_loop, zeroed_box},
    os_api,
    registers::Registers,
    serial_logger,
    x86_64::{
        control_registers::{CR4_VMXE, cr0, cr2, cr3, cr4, write_cr4},
        misc::Rflags,
        msr::{self, rdmsr, wrmsr},
        segment::{
            self, cs, ds, es, fs, gs, lar, ldtr, lsl, sgdt, sidt, ss, tr, vmx_access_rights,
        },
        vmx::{self, BasicExitReason, Vmcs, VmxonRegion, vmcs},
    },
};

/// The size of the hypervisor stack per processor, in page count.
const PER_PROCESSOR_HV_STACK_PAGE_COUNT: usize = 0x10;

/// The size of the hypervisor heap per processor, in bytes.
const PER_PROCESSOR_HV_HEAP_SIZE: usize = size_of::<VmxonRegion>()
    + size_of::<Vmcs>()
    + size_of::<[Page; PER_PROCESSOR_HV_STACK_PAGE_COUNT]>();

/// The range of the hypervisor heap in SPA.
static HV_HEAP_SPA_RANGE: Once<Range<usize>> = Once::new();

/// The number of VM-exit occurred across all processors.
static VMEXIT_COUNT: AtomicU64 = AtomicU64::new(0);

/// Returns the required hypervisor heap size in bytes.
#[must_use]
pub fn heap_size(processor_count: u32) -> usize {
    processor_count as usize * PER_PROCESSOR_HV_HEAP_SIZE
}

/// Sets up processor-global things like an allocator and logger.
///
/// This function must be called once before calling [`init`].
///
/// # Safety
/// This function is unsafe because:
/// - providing incorrect `heap_area` may cause memory corruption
/// - calling this function twice causes memory corruption
pub unsafe fn global_init(
    image_area: Range<usize>,
    heap_area: Range<usize>,
    to_spa: fn(va: *const c_void) -> Spa,
    to_va: fn(spa: Spa) -> *mut c_void,
) {
    // Initialize the global allocator first. The logger depends on it.
    let heap_bottom = heap_area.start as *mut u8;
    let heap_size = heap_area.end - heap_area.start;
    unsafe { ALLOCATOR.lock().init(heap_bottom, heap_size) };

    serial_logger::init(log::LevelFilter::Trace);
    log::info!("🔥 Initializing the hypervisor");
    log::debug!("Image range: {image_area:#x?}");
    log::debug!("Heap range : {heap_area:#x?}");

    os_api::init(to_spa, to_va);

    let heap_size = heap_area.end - heap_area.start;
    let heap_spa = Spa::from(heap_area.start as *const u8).as_u64() as usize;
    let heap_spa_range = heap_spa..heap_spa + heap_size;
    log::debug!("Heap range : {heap_spa_range:#x?} (SPA)");

    let _ = HV_HEAP_SPA_RANGE.call_once(|| heap_spa_range);
}

/// Virtualizes the current processor.
///
/// This function eventually makes the current processor transition to the VMX
/// non-root operation and resume execution based on based on the `register` and
/// the other current system register values, and handles VM-exits. Those are
/// implemented in `entry_point`.
///
/// This function itself switches the current stack to newly allocated hypervisor
/// stack and jumps to `entry_point`. Switching the stack is necessary to prevent
/// a guest from overwriting the hypervisor stack.
#[expect(clippy::missing_panics_doc)]
pub fn init(registers: &Registers) -> ! {
    let layout = Layout::array::<Page>(PER_PROCESSOR_HV_STACK_PAGE_COUNT).unwrap();
    let stack = unsafe { alloc::alloc::alloc_zeroed(layout) };
    if stack.is_null() {
        handle_alloc_error(layout);
    }
    let stack = stack as usize;
    let stack_area = stack..stack + layout.size();
    log::debug!("Stack range: {stack_area:#x?}");

    let stack_base = stack_area.end as u64 - 0x8;
    unsafe { asm_switch_stack(registers, entry_point, stack_base) };
}

unsafe extern "C" {
    /// Switches the current stack to `stack_base` and jumps to `entry_point`.
    unsafe fn asm_switch_stack(
        registers: &Registers,
        entry_point: extern "C" fn(&Registers) -> !,
        stack_base: u64,
    ) -> !;
}
global_asm!(
    r#"
    .align 16
    .global asm_switch_stack
    asm_switch_stack:
        mov     rsp, r8
        jmp     rdx
    "#
);

/// The function that puts the current processor into the VMX operation, allocates
/// and initializes a VMCS with `registers`, then, transitions to the VMX non-root
/// operation and handles VM-exits indefinitely.
extern "C" fn entry_point(registers: &Registers) -> ! {
    log::trace!("{registers:#x?}");
    log::info!("Initializing the guest");

    let registers = &mut registers.clone();

    // CR4.VMXE = 1 is required to enter VMX operation.
    // See: 25.7 ENABLING AND ENTERING VMX OPERATION
    unsafe { write_cr4(cr4() | CR4_VMXE) };

    // Allocate the VMXON region and enter VMX operation. The revision identifier
    // must be set before executing the VMXON instruction.
    // See: 26.11.5 VMXON Region
    let mut vmxon_region = zeroed_box::<VmxonRegion>();
    vmxon_region.revision_id = unsafe { rdmsr(msr::IA32_VMX_BASIC) } as _;
    unsafe { vmxon(&mut vmxon_region) };
    log::trace!("Entered VMX root operation");

    // Allocate and make the VMCS clear and current. The revision identifier
    // must be set before executing the VMPTRLD instruction.
    // See: 26.11.3 Initializing a VMCS
    // See: Figure 26-1. States of VMCS X
    let mut vmcs = zeroed_box::<Vmcs>();
    vmcs.revision_id = vmxon_region.revision_id;
    unsafe { vmclear(&mut vmcs) }; // Undefined -> Clear & Not Current
    unsafe { vmptrld(&mut vmcs) }; // Clear & Not Current -> Clear & Current
    log::trace!("Set current VMCS");

    initialize_guest_fields(registers);
    initialize_host_fields();
    initialize_control_fields();

    log::info!("Starting the guest");
    loop {
        log::trace!("Entering the guest");

        // Execute the guest until VM-exit occurs.
        unsafe { run_guest(registers) };

        let exit_reason_raw = unsafe { vmread(vmcs::ro::EXIT_REASON) } as u16;
        log::trace!("Exited the guest (reason: {exit_reason_raw})");

        match BasicExitReason::try_from(exit_reason_raw).unwrap() {
            BasicExitReason::Cpuid => handle_cpuid(registers),
            BasicExitReason::Rdmsr => handle_rdmsr(registers),
            BasicExitReason::Wrmsr => handle_wrmsr(registers),
            exit_reason => {
                log::error!("{vmcs:#x?} {registers:#x?} cr2: {:#x?},", cr2());
                panic!("Unhandled VM-exit {exit_reason_raw}💥: {exit_reason}");

                // If you want to inspect guest state better, you may comment out
                // the above panic, get out from this dead loop, then single step
                // instructions until VMRUN.
                #[allow(unreachable_code)]
                cpu_dead_loop();
            }
        }
        // Raise the log level to suppress noisy output at this point.
        serial_logger::set_log_level(log::LevelFilter::Info);

        let count = VMEXIT_COUNT.fetch_add(1, Ordering::Relaxed) + 1;
        if count.is_multiple_of(10000) {
            log::info!("VM-exit occurred {count} times");
        }
    }
}

fn initialize_guest_fields(registers: &Registers) {
    let idtr = unsafe { sidt() };
    let gdtr = unsafe { sgdt() };
    let tss = segment::Descriptor::try_new(&gdtr, tr()).unwrap();

    unsafe {
        vmwrite(vmcs::guest::ES_SELECTOR, es().0);
        vmwrite(vmcs::guest::CS_SELECTOR, cs().0);
        vmwrite(vmcs::guest::SS_SELECTOR, ss().0);
        vmwrite(vmcs::guest::DS_SELECTOR, ds().0);
        vmwrite(vmcs::guest::FS_SELECTOR, fs().0);
        vmwrite(vmcs::guest::GS_SELECTOR, gs().0);
        vmwrite(vmcs::guest::TR_SELECTOR, tr().0);
        vmwrite(vmcs::guest::LDTR_SELECTOR, ldtr().0);

        vmwrite(vmcs::guest::ES_LIMIT, lsl(es()));
        vmwrite(vmcs::guest::CS_LIMIT, lsl(cs()));
        vmwrite(vmcs::guest::SS_LIMIT, lsl(ss()));
        vmwrite(vmcs::guest::DS_LIMIT, lsl(ds()));
        vmwrite(vmcs::guest::FS_LIMIT, lsl(fs()));
        vmwrite(vmcs::guest::GS_LIMIT, lsl(gs()));
        vmwrite(vmcs::guest::TR_LIMIT, lsl(tr()));

        vmwrite(vmcs::guest::ES_ACCESS_RIGHTS, vmx_access_rights(lar(es())));
        vmwrite(vmcs::guest::CS_ACCESS_RIGHTS, vmx_access_rights(lar(cs())));
        vmwrite(vmcs::guest::SS_ACCESS_RIGHTS, vmx_access_rights(lar(ss())));
        vmwrite(vmcs::guest::DS_ACCESS_RIGHTS, vmx_access_rights(lar(ds())));
        vmwrite(vmcs::guest::FS_ACCESS_RIGHTS, vmx_access_rights(lar(fs())));
        vmwrite(vmcs::guest::GS_ACCESS_RIGHTS, vmx_access_rights(lar(gs())));
        vmwrite(vmcs::guest::TR_ACCESS_RIGHTS, vmx_access_rights(lar(tr())));
        vmwrite(vmcs::guest::LDTR_ACCESS_RIGHTS, vmx_access_rights(0));

        vmwrite(vmcs::guest::FS_BASE, rdmsr(msr::IA32_FS_BASE));
        vmwrite(vmcs::guest::GS_BASE, rdmsr(msr::IA32_GS_BASE));
        vmwrite(vmcs::guest::TR_BASE, tss.base());

        vmwrite(vmcs::guest::GDTR_BASE, gdtr.base);
        vmwrite(vmcs::guest::GDTR_LIMIT, gdtr.limit);
        vmwrite(vmcs::guest::IDTR_BASE, idtr.base);
        vmwrite(vmcs::guest::IDTR_LIMIT, idtr.limit);

        vmwrite(vmcs::guest::IA32_SYSENTER_CS, rdmsr(msr::IA32_SYSENTER_CS));
        vmwrite(
            vmcs::guest::IA32_SYSENTER_EIP,
            rdmsr(msr::IA32_SYSENTER_EIP),
        );
        vmwrite(
            vmcs::guest::IA32_SYSENTER_ESP,
            rdmsr(msr::IA32_SYSENTER_ESP),
        );

        // "If the "VMCS shadowing" VM-execution control is 1, (...). Otherwise,
        //  software should set this field to FFFFFFFF_FFFFFFFFH to avoid VM-entry
        //  failures."
        // See: 26.4.2 Guest Non-Register State
        vmwrite(vmcs::guest::LINK_PTR, u64::MAX);

        vmwrite(vmcs::guest::CR0, cr0());
        vmwrite(vmcs::guest::CR3, cr3());
        vmwrite(vmcs::guest::CR4, cr4());
        vmwrite(vmcs::guest::RSP, registers.rsp);
        vmwrite(vmcs::guest::RIP, registers.rip);
        vmwrite(vmcs::guest::RFLAGS, registers.rflags);
    };
}

fn initialize_control_fields() {
    unsafe {
        vmwrite_control(
            vmcs::control::PRIMARY_VMEXIT_CONTROLS,
            vmcs::control::PrimaryExitControls::HOST_ADDRESS_SPACE_SIZE.bits(),
        );
        vmwrite_control(
            vmcs::control::VMENTRY_CONTROLS,
            vmcs::control::EntryControls::IA32E_MODE_GUEST.bits(),
        );
        vmwrite_control(vmcs::control::PINBASED_EXEC_CONTROLS, 0);
        vmwrite_control(
            vmcs::control::PRIMARY_PROCBASED_EXEC_CONTROLS,
            vmcs::control::PrimaryControls::SECONDARY_CONTROLS.bits(),
        );
        vmwrite_control(
            vmcs::control::SECONDARY_PROCBASED_EXEC_CONTROLS,
            (vmcs::control::SecondaryControls::ENABLE_RDTSCP
                | vmcs::control::SecondaryControls::ENABLE_XSAVES_XRSTORS)
                .bits(),
        );
    };
}

unsafe fn vmwrite_control(encoding: u32, value: u32) {
    unsafe { vmwrite(encoding, adjusted_vmx_control(encoding, value.into())) };
}

/// Returns the VM control value that is adjusted in consideration with the
/// VMX capability MSR.
fn adjusted_vmx_control(encoding: u32, requested_value: u64) -> u64 {
    const IA32_VMX_BASIC_VMX_CONTROLS_FLAG: u64 = 1 << 55;

    // This determines the right VMX capability MSR based on the value of
    // IA32_VMX_BASIC. This is required to fulfil the following requirements:
    //
    // "It is necessary for software to consult only one of the capability MSRs
    //  to determine the allowed settings of the pin based VM-execution controls:"
    // See: A.3.1 Pin-Based VM-Execution Controls
    let vmx_basic = unsafe { rdmsr(msr::IA32_VMX_BASIC) };
    let true_cap_msr_supported = (vmx_basic & IA32_VMX_BASIC_VMX_CONTROLS_FLAG) != 0;

    let cap_msr = match (encoding, true_cap_msr_supported) {
        (vmcs::control::PINBASED_EXEC_CONTROLS, true) => msr::IA32_VMX_TRUE_PINBASED_CTLS,
        (vmcs::control::PINBASED_EXEC_CONTROLS, false) => msr::IA32_VMX_PINBASED_CTLS,
        (vmcs::control::PRIMARY_PROCBASED_EXEC_CONTROLS, true) => msr::IA32_VMX_TRUE_PROCBASED_CTLS,
        (vmcs::control::PRIMARY_PROCBASED_EXEC_CONTROLS, false) => msr::IA32_VMX_PROCBASED_CTLS,
        (vmcs::control::PRIMARY_VMEXIT_CONTROLS, true) => msr::IA32_VMX_TRUE_EXIT_CTLS,
        (vmcs::control::PRIMARY_VMEXIT_CONTROLS, false) => msr::IA32_VMX_EXIT_CTLS,
        (vmcs::control::VMENTRY_CONTROLS, true) => msr::IA32_VMX_TRUE_ENTRY_CTLS,
        (vmcs::control::VMENTRY_CONTROLS, false) => msr::IA32_VMX_ENTRY_CTLS,
        // There is no TRUE MSR for IA32_VMX_PROCBASED_CTLS2. Just use
        // IA32_VMX_PROCBASED_CTLS2 unconditionally.
        (vmcs::control::SECONDARY_PROCBASED_EXEC_CONTROLS, _) => msr::IA32_VMX_PROCBASED_CTLS2,
        (vmcs::control::TERTIARY_PROCBASED_EXEC_CONTROLS, _) => {
            let allowed1 = unsafe { rdmsr(msr::IA32_VMX_PROCBASED_CTLS3) };
            let effective_value = requested_value & allowed1;
            assert!(
                effective_value | requested_value == effective_value,
                "One or more requested features are not supported: {effective_value:#x?} : {requested_value:#x?} "
            );
            return effective_value;
        }
        _ => panic!("Invalid encoding {encoding:#x?}"),
    };

    // Each bit of the following VMCS values might have to be set or cleared
    // according to the value indicated by the VMX capability MSRs.
    //  - pin-based VM-execution controls,
    //  - primary processor-based VM-execution controls,
    //  - secondary processor-based VM-execution controls.
    //
    // The VMX capability MSR is composed of two 32bit values, the lower 32bits
    // indicate bits can be 0, and the higher 32bits indicates bits can be 1.
    // In other words, if those bits are "cleared", corresponding bits MUST BE 1
    // and MUST BE 0 respectively. The below summarizes the interpretation:
    //
    //        Lower bits (allowed 0) Higher bits (allowed 1) Meaning
    // Bit X  1                      1                       The bit X is flexible
    // Bit X  1                      0                       The bit X is fixed to 0
    // Bit X  0                      1                       The bit X is fixed to 1
    //
    // The following code enforces this logic by setting bits that must be 1,
    // and clearing bits that must be 0.
    //
    // See: A.3.1 Pin-Based VM-Execution Controls
    // See: A.3.2 Primary Processor-Based VM-Execution Controls
    // See: A.3.3 Secondary Processor-Based VM-Execution Controls
    let capabilities = unsafe { rdmsr(cap_msr) };
    let allowed0 = capabilities as u32;
    let allowed1 = (capabilities >> 32) as u32;
    let requested_value = u32::try_from(requested_value).unwrap();
    let mut effective_value = requested_value;
    effective_value |= allowed0;
    effective_value &= allowed1;
    assert!(
        effective_value | requested_value == effective_value,
        "One or more requested features are not supported for {encoding:#x?}: {effective_value:#x?} vs {requested_value:#x?}"
    );
    u64::from(effective_value)
}

fn initialize_host_fields() {
    let idtr = unsafe { sidt() };
    let gdtr = unsafe { sgdt() };
    let tss = segment::Descriptor::try_new(&gdtr, tr()).unwrap();

    unsafe {
        // The lower 3 bits of the selectors must be zero.
        // "In the selector field for each of CS, SS, DS, ES, FS, GS, and TR,
        //  the RPL (bits 1:0) and the TI flag (bit 2) must be 0."
        // See: 28.2.3 Checks on Host Segment and Descriptor-Table Registers
        vmwrite(vmcs::host::ES_SELECTOR, es().0 & !0b111);
        vmwrite(vmcs::host::CS_SELECTOR, cs().0 & !0b111);
        vmwrite(vmcs::host::SS_SELECTOR, ss().0 & !0b111);
        vmwrite(vmcs::host::DS_SELECTOR, ds().0 & !0b111);
        vmwrite(vmcs::host::FS_SELECTOR, fs().0 & !0b111);
        vmwrite(vmcs::host::GS_SELECTOR, gs().0 & !0b111);
        vmwrite(vmcs::host::TR_SELECTOR, tr().0 & !0b111);

        vmwrite(vmcs::host::CR0, cr0());
        vmwrite(vmcs::host::CR3, cr3());
        vmwrite(vmcs::host::CR4, cr4());

        vmwrite(vmcs::host::FS_BASE, rdmsr(msr::IA32_FS_BASE));
        vmwrite(vmcs::host::GS_BASE, rdmsr(msr::IA32_GS_BASE));
        vmwrite(vmcs::host::TR_BASE, tss.base());
        vmwrite(vmcs::host::GDTR_BASE, gdtr.base);
        vmwrite(vmcs::host::IDTR_BASE, idtr.base);
    };
}

/// Handles the `CPUID` instruction.
fn handle_cpuid(guest: &mut Registers) {
    let leaf = guest.rax as u32;
    let sub_leaf = guest.rcx as u32;
    log::trace!("CPUID {leaf:#x?} {sub_leaf:#x?}");

    // We somehow need to emulate CPUID instruction by returning EAX, EBX, ECX,
    // and EDX according to RAX and RCX (leaf and sub leaf).
    unimplemented!("it is your first exercise :)");

    //guest.rax = result_eax;
    //guest.rbx = result_ebx;
    //guest.rcx = result_ecx;
    //guest.rdx = result_edx;
    //advance_guest_rip();
}

/// Handles the `RDMSR` instruction.
fn handle_rdmsr(guest: &mut Registers) {
    // Passthrough any MSR access request. Meaning we execute `RDMSR` with the
    // same input value (ECX) the guest intended, and updating guest's EDX:EAX
    // with the results of the instruction. Finally, update the guest RIP as well.
    // From the guest perspective, it looks as if the RDMSR instruction were
    // completed without being intercepted.
    let msr = guest.rcx as u32;
    log::trace!("RDMSR {msr:#x?}");
    let value = unsafe { rdmsr(msr) };
    guest.rax = value & 0xffff_ffff;
    guest.rdx = value >> 32;
    advance_guest_rip();
}

/// Handles the `WRMSR` instruction.
fn handle_wrmsr(guest: &mut Registers) {
    let msr = guest.rcx as u32;
    let value = (guest.rax & 0xffff_ffff) | ((guest.rdx & 0xffff_ffff) << 32);
    log::trace!("WRMSR {msr:#x?} {value:#x?}");
    unsafe { wrmsr(msr, value) };
    advance_guest_rip();
}

fn advance_guest_rip() {
    unsafe {
        vmwrite(
            vmcs::guest::RIP,
            vmread(vmcs::guest::RIP) + vmread(vmcs::ro::VMEXIT_INSTRUCTION_LEN),
        );
    };
}

unsafe fn run_guest(registers: &mut Registers) {
    let flags = unsafe { asm_run_guest(registers) };
    vmx::result(flags).unwrap_or_else(|err| panic!("asm_run_guest: {err}"));
}

unsafe fn vmxon(vmxon_region: &mut VmxonRegion) {
    let vmxon_region_pa = Spa::from(vmxon_region);
    unsafe { vmx::vmxon(vmxon_region_pa) }.unwrap_or_else(|err| panic!("VMXON: {err}"));
}

unsafe fn vmclear(vmcs: &mut Vmcs) {
    let vmcs_pa = Spa::from(vmcs);
    unsafe { vmx::vmclear(vmcs_pa) }.unwrap_or_else(|err| panic!("VMCLEAR: {err}"));
}

unsafe fn vmptrld(vmcs: &mut Vmcs) {
    let vmcs_pa = Spa::from(vmcs);
    unsafe { vmx::vmptrld(vmcs_pa) }.unwrap_or_else(|err| panic!("VMPTRLD: {err}"));
}

unsafe fn vmread(encoding: u32) -> u64 {
    unsafe { vmx::vmread(encoding).unwrap_or_else(|err| panic!("VMREAD: {err}")) }
}

unsafe fn vmwrite<T: Into<u64>>(encoding: u32, value: T) {
    unsafe {
        let value: u64 = value.into();
        vmx::vmwrite(encoding, value).unwrap_or_else(|err| panic!("VMWRITE: {err}"));
    }
}

global_asm!(include_str!("run_guest.S"));
unsafe extern "C" {
    /// Runs the guest until VM-exit occurs.
    unsafe fn asm_run_guest(registers: &mut Registers) -> Rflags;
}

#[cfg(not(test))]
#[global_allocator]
static ALLOCATOR: LockedHeap = LockedHeap::empty();

#[cfg(test)]
static ALLOCATOR: LockedHeap = LockedHeap::empty();
