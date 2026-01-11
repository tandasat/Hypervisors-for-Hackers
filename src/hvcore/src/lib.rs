//! The OS agnostic parts of the hypervisor, ie, the core.
#![cfg_attr(not(test), no_std)]
extern crate alloc;

mod drivers;
mod host;
mod misc;
mod os_api;
mod registers;
mod serial_logger;
mod x86_64;

pub use host::{global_init, heap_size, init};
pub use misc::Spa;
pub use registers::capture as capture_registers;
