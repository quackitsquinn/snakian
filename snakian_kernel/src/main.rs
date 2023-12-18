#![no_std]
#![no_main]
#![feature(panic_info_message, custom_test_frameworks)]
#![test_runner(snakian_kernel::testing::test_runner)]
#![reexport_test_harness_main = "test_main"]

use core::fmt::Write;
use core::mem::transmute;
use core::panic::PanicInfo;

use bootloader_api::{entry_point, BootInfo, BootloaderConfig};
use pc_keyboard::KeyCode;
use snakian_kernel::interrupts::{self, init_idt};
use snakian_kernel::keyboard_driver::KEYBOARD_DRIVER;
use snakian_kernel::vga_driver::{self, ColorCode, WRITER};
use snakian_kernel::{
    chars, dbg, eprintln, hardware_interrupts, init, print, println, serial_println, sleep,
};
use spin::Mutex;
use x86_64::instructions;
use x86_64::registers::control::Cr3;
use x86_64::structures::idt::InterruptDescriptorTable;

//#[cfg(not(test))]
#[panic_handler]
pub fn panic_handle(panic: &PanicInfo) -> ! {
    use snakian_kernel::panic_handler;

    panic_handler(panic)
}

#[cfg(test)]
#[panic_handler]
pub fn panic_handle(panic: &PanicInfo) -> ! {
    snakian_kernel::panic_handler(panic);
}
//TODO: add basic interpreter for commands (poke, peek, )
fn os_entry_point(boot_info: &'static mut BootInfo) -> ! {
    init(boot_info);
    dbg!("Initialized hardware!");
    dbg!("Entering main loop!");
    let mut vga = vga_driver::WRITER.get().unwrap().lock();
    vga.buffer.clear();
    vga.buffer.set_scale(2);
    drop(vga);
    println!("Welcome to SnakianOS!");
    println!("test test test!");
    println!("test test test_");
    eprintln!("error!");
    eprintln!("hi shanananananabanananana");
    let mut key: Option<char> = None;
    loop {
        let lock = KEYBOARD_DRIVER.lock();
        if let Some(curchar) = lock.current_char {
            if key != Some(curchar) {
                key = Some(curchar);
                if lock.current_char_as_key == Some(KeyCode::Backspace) {
                    WRITER.get().unwrap().lock().backspace();
                } else {
                    print!("{}", key.unwrap());
                }
            }
        } else {
            key = None;
        }
        // This hlt is necessary because the keyboard driver needs to be able to unlock the keyboard
        instructions::hlt(); // or asm!("hlt", options(nomem, nostack));
    }
}

entry_point!(kmain, config = &snakian_kernel::BOOT_CONFIG);

#[no_mangle]
fn kmain(boot_info: &'static mut BootInfo) -> ! {
    #[cfg(test)]
    test_main();

    os_entry_point(boot_info);
}
