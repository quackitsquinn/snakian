
#![no_std]
#![no_main]
#![feature(panic_info_message, custom_test_frameworks)]
#![test_runner(crate::test_util::test_runner)]
#![reexport_test_harness_main = "test_main"]

use core::panic::PanicInfo;
use core::fmt::Write;

use vga_driver::{Writer, ColorCode, Color};

mod vga_driver;
mod test_util;
mod serial;

#[cfg(not(test))]
#[panic_handler]
pub fn panic_handle(panic: &PanicInfo) -> ! {
    // forces the write position to the beginning of the buffer (will be changed this is just for quick and dirty testing)
    vga_driver::WRITER.lock().reset();
    // set panic format to be red on white
    vga_driver::WRITER.lock().color_code = ColorCode::new(Color::Red, Color::White, false);
    // write the panic message
    println!("Kernal Panic in file {} at line {}", panic.location().unwrap().file(), panic.location().unwrap().line());
    println!("Reason:{}", panic.message().unwrap());
    serial_println!("Kernal Panic in file {} at line {}", panic.location().unwrap().file(), panic.location().unwrap().line());
    serial_println!("Reason:{}", panic.message().unwrap());
    loop {}
}

#[cfg(test)]
#[panic_handler]
pub fn panic_handle(panic: &PanicInfo) -> ! {
    use crate::test_util::exit_qemu;

    serial_println!("Kernal Panic in file {} at line {}", panic.location().unwrap().file(), panic.location().unwrap().line());
    serial_println!("Reason:{}", panic.message().unwrap());
    exit_qemu(test_util::QemuExitCode::Failed);
    loop {} // if qemu doesn't exit
}
fn handle_tests(){
    #[cfg(test)]
    test_main();
    loop {}
}


fn entry_point() -> ! {
    let mut cyc = 0u8;
    serial_println!("Hello World!");
    println!("test");
    loop {}
}

#[no_mangle]
pub extern "C" fn _start() -> ! {

    #[cfg(test)]
    handle_tests();

    entry_point();
}


