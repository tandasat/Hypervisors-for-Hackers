//! Implements Windows kernel-mode driver specifics, such as an entry point.
#![no_std]

mod eprintln;

use core::ffi::c_void;

use wdk_sys::{
    ALL_PROCESSOR_GROUPS, DRIVER_OBJECT, GROUP_AFFINITY, NT_SUCCESS, NTSTATUS, PAGED_CODE,
    PCUNICODE_STRING, PHYSICAL_ADDRESS, PROCESSOR_NUMBER, STATUS_INSUFFICIENT_RESOURCES,
    STATUS_SUCCESS,
    ntddk::{
        KdRefreshDebuggerNotPresent, KeGetProcessorNumberFromIndex, KeQueryActiveProcessorCountEx,
        KeRevertToUserGroupAffinityThread, KeSetSystemGroupAffinityThread,
        MmAllocateContiguousMemory, MmGetPhysicalAddress, MmGetVirtualForPhysical,
    },
};

/// The entry point.
#[unsafe(export_name = "DriverEntry")]
extern "system" fn driver_entry(
    driver: &DRIVER_OBJECT,
    _registry_path: PCUNICODE_STRING,
) -> NTSTATUS {
    // Break into a kernel debugger if present.
    unsafe {
        if KdRefreshDebuggerNotPresent() == 0 {
            core::arch::asm!("int3");
        }
    }

    eprintln!("Loading the hypervisor");

    // Allocate hypervisor heap.
    let highest = PHYSICAL_ADDRESS { QuadPart: -1 };
    let heap_size = hvcore::heap_size(processor_count());
    let heap = unsafe { MmAllocateContiguousMemory(heap_size as _, highest) };
    if heap.is_null() {
        eprintln!("memory allocation of {heap_size} bytes failed");
        return STATUS_INSUFFICIENT_RESOURCES;
    }
    let heap = heap as usize;
    let heap_area = heap..heap + heap_size;

    // Set up processor-global things like an allocator and logger.
    let image_base = driver.DriverStart as usize;
    let image_area = image_base..image_base + driver.DriverSize as usize;
    unsafe { hvcore::global_init(image_area, heap_area, to_spa, to_va) };

    // Virtualize all processors one by one.
    run_on_all_processors(|| {
        // Use `black_box` to avoid optimization by the compiler against this flag.
        let mut virtualized = core::hint::black_box(false);

        // Take a snapshot of current register values. This will be the initial
        // state of the guest including RIP. This means that the guest starts
        // execution right after this function call. Think of it as the setjmp()
        // function of C.
        let registers = hvcore::capture_registers();

        // Virtualize the processor if not yet. `hvcore::init` below switches
        // the processor to guest-mode and starts execution from here again,
        // based on the RIP captured above. This time, `virtualized` is already
        // true, so we bail out and avoid calling `hvcore::init` twice.
        if !virtualized {
            virtualized = true;
            let _ = core::hint::black_box(virtualized);

            eprintln!("Virtualizing the current processor");
            hvcore::init(&registers);
            // Unreachable. init() never returns and resumes execution at right
            // after capture_registers() above.
        }

        show_hypervisor_vendor();
        eprintln!("Virtualized the current processor");
    });

    // All done!
    eprintln!("Successfully loaded the hypervisor");
    STATUS_SUCCESS
}

/// Prints the hypervisor vendor name (CPUID 0x4000_0000).
fn show_hypervisor_vendor() {
    let regs = unsafe { core::arch::x86_64::__cpuid(0x4000_0000) };
    let mut result = [0u8; 12];
    result[..4].copy_from_slice(&regs.ebx.to_le_bytes());
    result[4..8].copy_from_slice(&regs.ecx.to_le_bytes());
    result[8..12].copy_from_slice(&regs.edx.to_le_bytes());
    let vendor = core::str::from_utf8(&result[..12]).unwrap();
    eprintln!("Hypervisor vendor (CPUID 0x4000_0000): {vendor}");
}

/// Returns the number of logical processors on the system.
fn processor_count() -> u32 {
    unsafe { KeQueryActiveProcessorCountEx(ALL_PROCESSOR_GROUPS as _) }
}

/// Runs `callback` on all logical processors one by one.
fn run_on_all_processors(callback: fn()) {
    PAGED_CODE!();

    for index in 0..processor_count() {
        let mut processor_number = PROCESSOR_NUMBER::default();
        let status = unsafe { KeGetProcessorNumberFromIndex(index, &raw mut processor_number) };
        assert!(
            NT_SUCCESS(status),
            "KeGetProcessorNumberFromIndex failed: {status:#x?}"
        );

        let mut old_affinity = GROUP_AFFINITY::default();
        let mut affinity = GROUP_AFFINITY {
            Group: processor_number.Group,
            Mask: 1 << processor_number.Number,
            Reserved: [0, 0, 0],
        };
        unsafe { KeSetSystemGroupAffinityThread(&raw mut affinity, &raw mut old_affinity) };
        callback();
        unsafe { KeRevertToUserGroupAffinityThread(&raw mut old_affinity) };
    }
}

/// Returns a system physical address of a given virtual address.
fn to_spa(va: *const c_void) -> hvcore::Spa {
    let pa = unsafe { MmGetPhysicalAddress(va.cast_mut()).QuadPart };
    assert!(pa != 0, "{va:#x?} cannot be translated into SPA");
    #[expect(clippy::cast_sign_loss)]
    hvcore::Spa::new(pa as _)
}

/// Returns a virtual address of the given system physical address in the current
/// address space.
fn to_va(spa: hvcore::Spa) -> *mut c_void {
    #[expect(clippy::cast_possible_wrap)]
    let pa = PHYSICAL_ADDRESS {
        QuadPart: spa.as_u64() as _,
    };
    let va = unsafe { MmGetVirtualForPhysical(pa) };
    assert!(!va.is_null(), "{spa:x?} cannot be translated into VA");
    va
}
