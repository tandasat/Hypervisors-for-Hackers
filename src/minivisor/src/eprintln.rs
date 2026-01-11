//! Implements Windows debug printing macros.
//!
//! Those should not be used within the host-mode, as it violates the boundary
//! of the host and guest (ie, "call-out") as well as because underneath Windows
//! API `DbgPrintEx` is not guaranteed to work in the host-mode where interrupts
//! are disabled.

use core::fmt::Write;

use spin::Mutex;
use wdk_sys::{_DPFLTR_TYPE::DPFLTR_IHVDRIVER_ID, DPFLTR_ERROR_LEVEL, ntddk::DbgPrintEx};

/// Debug prints a message to a kernel debugger with a newline.
#[macro_export]
macro_rules! eprintln {
    () => {
        ($crate::print!("\n"));
    };

    ($($arg:tt)*) => {
        ($crate::print!("{}\n", format_args!($($arg)*)))
    };
}

/// Debug prints a message to a kernel debugger without a newline.
#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => {
        ($crate::eprintln::print(format_args!("#{}: {}", $crate::eprintln::apic_id(), format_args!($($arg)*))))
    };
}

#[doc(hidden)]
pub(crate) fn apic_id() -> u8 {
    (unsafe { core::arch::x86_64::__cpuid_count(0x1, 0) }.ebx >> 24) as _
}

#[doc(hidden)]
pub(crate) fn print(args: core::fmt::Arguments<'_>) {
    Write::write_fmt(&mut *DEBUG_PRINTER.lock(), args).unwrap();
}

struct DbgPrinter;

impl Write for DbgPrinter {
    fn write_str(&mut self, msg: &str) -> core::fmt::Result {
        if !msg.is_ascii() {
            return Err(core::fmt::Error);
        }

        // Avoid heap allocation so the eprint(ln) macros are usable before
        // initializing the allocator.
        let mut buffer = [0u8; 256];
        let length = core::cmp::min(buffer.len() - 1, msg.len());
        buffer[..length].copy_from_slice(msg.as_bytes());
        let msg_ptr = buffer.as_mut_ptr().cast::<i8>();
        let _ = unsafe {
            DbgPrintEx(
                DPFLTR_IHVDRIVER_ID as _,
                DPFLTR_ERROR_LEVEL,
                c"%s".as_ptr(),
                msg_ptr,
            )
        };
        Ok(())
    }
}

static DEBUG_PRINTER: Mutex<DbgPrinter> = Mutex::new(DbgPrinter);
