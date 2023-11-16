

#![no_std]
#![no_main]
#![feature(panic_info_message, custom_test_frameworks, abi_x86_interrupt)]
#![test_runner(crate::testing::test_runner)]
#![reexport_test_harness_main = "test_main"]

use core::panic::PanicInfo;

use crate::vga_driver::{ColorCode, Color};

pub mod serial;
pub mod vga_driver;
pub mod testing;
pub mod interrupts;

pub fn panic_handler(panic: &PanicInfo) -> ! {
    // forces the write position to the beginning of the buffer (will be changed this is just for quick and dirty testing)
    vga_driver::WRITER.lock().reset();
    // set panic format to be red on white
    vga_driver::WRITER.lock().color_code = ColorCode::new(Color::Red, Color::White, true);
    // write the panic message
    println!("Kernal Panic in file {} at line {}", panic.location().unwrap().file(), panic.location().unwrap().line());
    println!("Reason:{}", panic.message().unwrap());
    serial_println!("Kernal Panic in file {} at line {}", panic.location().unwrap().file(), panic.location().unwrap().line());
    serial_println!("Reason:{}", panic.message().unwrap());
    loop {}
}

#[cfg(test)]
#[no_mangle]
pub extern "C" fn _start() {
    test_main();
    loop {}
}


#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    testing::panic_handler(info);
}
//TODO: determine if init stages should exist (aka multiple init functions like init_stage0 init_stage1 etc)
pub fn init() {
    interrupts::init_idt();
}