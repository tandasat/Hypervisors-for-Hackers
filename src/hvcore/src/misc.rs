use core::alloc::Layout;

use alloc::{alloc::handle_alloc_error, boxed::Box};

/// A system physical address.
#[derive(Copy, Clone, Debug, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct Spa(u64);

impl Spa {
    /// Constructs [`Spa`].
    #[must_use]
    pub const fn new(spa: u64) -> Self {
        Self(spa)
    }

    /// Returns the address as u64.
    #[must_use]
    pub const fn as_u64(self) -> u64 {
        self.0
    }
}

pub(crate) const _SIZE_1KB: usize = 0x0000_0400;
pub(crate) const _SIZE_2KB: usize = 0x0000_0800;
pub(crate) const SIZE_4KB: usize = 0x0000_1000;
pub(crate) const _SIZE_8KB: usize = 0x0000_2000;
pub(crate) const _SIZE_16KB: usize = 0x0000_4000;
pub(crate) const _SIZE_32KB: usize = 0x0000_8000;
pub(crate) const _SIZE_64KB: usize = 0x0001_0000;
pub(crate) const _SIZE_128KB: usize = 0x0002_0000;
pub(crate) const _SIZE_256KB: usize = 0x0004_0000;
pub(crate) const _SIZE_512KB: usize = 0x0008_0000;
pub(crate) const _SIZE_1MB: usize = 0x0010_0000;
pub(crate) const _SIZE_2MB: usize = 0x0020_0000;
pub(crate) const _SIZE_4MB: usize = 0x0040_0000;
pub(crate) const _SIZE_8MB: usize = 0x0080_0000;
pub(crate) const _SIZE_16MB: usize = 0x0100_0000;
pub(crate) const _SIZE_32MB: usize = 0x0200_0000;
pub(crate) const _SIZE_64MB: usize = 0x0400_0000;
pub(crate) const _SIZE_128MB: usize = 0x0800_0000;
pub(crate) const _SIZE_256MB: usize = 0x1000_0000;
pub(crate) const _SIZE_512MB: usize = 0x2000_0000;
pub(crate) const _SIZE_1GB: usize = 0x4000_0000;
pub(crate) const _SIZE_2GB: usize = 0x8000_0000;
pub(crate) const _SIZE_4GB: usize = 0x0000_0001_0000_0000;
pub(crate) const _SIZE_8GB: usize = 0x0000_0002_0000_0000;
pub(crate) const _SIZE_16GB: usize = 0x0000_0004_0000_0000;
pub(crate) const _SIZE_32GB: usize = 0x0000_0008_0000_0000;
pub(crate) const _SIZE_64GB: usize = 0x0000_0010_0000_0000;
pub(crate) const _SIZE_128GB: usize = 0x0000_0020_0000_0000;
pub(crate) const _SIZE_256GB: usize = 0x0000_0040_0000_0000;
pub(crate) const _SIZE_512GB: usize = 0x0000_0080_0000_0000;

/// The structure representing a single memory page (4KB).
//
// This does not _always_ have to be allocated at the page aligned address, but
// very often it is, so let us specify the alignment.
#[derive(Debug)]
#[repr(C, align(4096))]
pub(crate) struct Page([u8; SIZE_4KB]);

/// Indicates that the type is implementing this trait being zero-filled is sound.
///
/// # Safety
/// - The user must make sure the instance being zero is not undefined behaviour
///   and does not cause memory corruption.
pub(crate) unsafe trait ZeroIsSound {}

/// Returns zero-initialized Box of `T` without using stack during construction.
pub(crate) fn zeroed_box<T: ZeroIsSound>() -> Box<T> {
    let layout = Layout::new::<T>();
    assert!(layout.size() != 0);
    // Safety: `layout` ensured to be non-zero.
    let ptr = unsafe { alloc::alloc::alloc_zeroed(layout) };
    if ptr.is_null() {
        handle_alloc_error(layout);
    }
    unsafe { Box::from_raw(ptr.cast::<T>()) }
}

// Enters a dead loop that works as a breakpoint with a debugger.
#[inline(never)]
pub(crate) fn cpu_dead_loop() {
    let index = 0;
    while unsafe { core::ptr::read_volatile(&raw const index) } == 0 {
        core::hint::spin_loop();
    }
}

#[cfg(not(test))]
#[panic_handler]
fn handle_panic(info: &core::panic::PanicInfo<'_>) -> ! {
    log::error!("{info}");
    loop {
        unsafe { core::arch::asm!("cli; hlt", options(nomem, nostack)) };
    }
}
