use lazy_static::lazy_static;
use spin::Mutex;
use uart_16550::SerialPort;

use crate::interrupts;


const SERIAL_PORT_ADDR: u16 = 0x3F8;

lazy_static! {
    static ref SERIAL_PORT: Mutex<SerialPort> = {
        let mut serial_port = unsafe { SerialPort::new(SERIAL_PORT_ADDR) };
        serial_port.init();
        Mutex::new(serial_port)
    };
}
#[doc(hidden)]
pub fn _print(args: ::core::fmt::Arguments) {
    use core::fmt::Write;
    x86_64::instructions::interrupts::without_interrupts(|| {
        unsafe { SERIAL_PORT.force_unlock() }
        SERIAL_PORT.lock().write_fmt(args).expect("Printing to serial failed");
    });
}

#[macro_export]
macro_rules! serial_print {
    ($($arg:tt)*) => ($crate::serial::_print(format_args!($($arg)*)));
}

#[macro_export]
macro_rules! serial_println {
    () => ($crate::serial_print!("\n"));
    ($fmt:expr) => ($crate::serial_print!(concat!($fmt, "\n")));
    ($fmt:expr, $($arg:tt)*) => ($crate::serial_print!(concat!($fmt, "\n"), $($arg)*));
}