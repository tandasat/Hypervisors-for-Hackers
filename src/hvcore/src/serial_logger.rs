use core::fmt::Write;
use spin::{Mutex, Once};

use crate::drivers::uart::{IoPort, Uart};

pub(crate) fn init(level: log::LevelFilter) {
    let logger = LOGGER.call_once(SerialLogger::new);
    log::set_logger(logger).unwrap();
    log::set_max_level(level);
}

pub(crate) fn set_log_level(level: log::LevelFilter) {
    log::set_max_level(level);
}

struct SerialLogger {
    port: Mutex<Uart>,
}

impl SerialLogger {
    fn new() -> Self {
        Self {
            port: Mutex::new(Uart::new(IoPort::Com1, 115_200)),
        }
    }
}

impl log::Log for SerialLogger {
    fn enabled(&self, metadata: &log::Metadata<'_>) -> bool {
        metadata.level() <= log::Level::Trace
    }

    fn log(&self, record: &log::Record<'_>) {
        if self.enabled(record.metadata()) {
            let mut uart = self.port.lock();
            let _ = uart.write_fmt(format_args!(
                "#{}:{:5}: {}\n",
                apic_id(),
                record.level(),
                record.args()
            ));
        }
    }

    fn flush(&self) {}
}

static LOGGER: Once<SerialLogger> = Once::new();

/// Returns the initial APIC ID of this processor.
///
/// See: Table 1-17. Information Returned by CPUID Instruction
fn apic_id() -> u8 {
    (unsafe { core::arch::x86_64::__cpuid_count(0x1, 0) }.ebx >> 24) as _
}
