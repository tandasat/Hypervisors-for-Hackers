//! Implements the PC UART driver.
//!
//! Reference: [OSDEV: Serial Ports](https://wiki.osdev.org/Serial_Ports)
use core::arch::asm;

#[allow(dead_code)]
#[derive(Copy, Clone)]
#[repr(u16)]
pub(crate) enum IoPort {
    Com1 = 0x3f8,
    Com2 = 0x2f8,
    Com3 = 0x3e8,
    Com4 = 0x2e8,
}

pub(crate) struct Uart {
    port: u16,
}

impl Uart {
    pub(crate) fn new(port: IoPort, baud_rate: u64) -> Self {
        let port = port as u16;

        // Initializes the COM port in case it was not.
        let divisor = u16::try_from(115_200 / baud_rate).unwrap();
        unsafe {
            outb(port + 1, 0); // Disable all interrupts
            outb(port + 3, 0b1000_0000); // Set the divisor latch access bit (DLAB)

            // Set the baud rate.
            outb(port, divisor as u8);
            outb(port + 1, (divisor >> 8) as u8);

            // 8 bits, no parity, one stop bit.
            outb(port + 3, 0b0000_0011);
        };
        Self { port }
    }

    fn send(&mut self, data: u8) {
        const UART_OFFSET_LINE_STATUS: u16 = 5;
        const UART_OFFSET_LINE_STATUS_THRE: u8 = 1 << 5;

        unsafe {
            while (inb(self.port + UART_OFFSET_LINE_STATUS) & UART_OFFSET_LINE_STATUS_THRE) == 0 {
                core::hint::spin_loop();
            }
            outb(self.port, data);
        }
    }
}

impl core::fmt::Write for Uart {
    fn write_str(&mut self, msg: &str) -> Result<(), core::fmt::Error> {
        for data in msg.bytes() {
            self.send(data);
        }
        Ok(())
    }
}

unsafe fn outb(port: u16, data: u8) {
    unsafe {
        asm!("out dx, al", in("al") data, in("dx") port, options(nomem, nostack, preserves_flags));
    }
}

unsafe fn inb(port: u16) -> u8 {
    unsafe {
        let ret: u8;
        asm!("in al, dx", in("dx") port, out("al") ret, options(nomem, nostack, preserves_flags));
        ret
    }
}
