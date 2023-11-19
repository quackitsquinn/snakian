

#![no_std]
#![no_main]
#![feature(panic_info_message, custom_test_frameworks)]
#![test_runner(snakian::testing::test_runner)]
#![reexport_test_harness_main = "test_main"]

use core::mem::transmute;
use core::panic::PanicInfo;
use core::fmt::Write;

use pc_keyboard::KeyCode;
use snakian::interrupts::{init_idt, self};
use snakian::keyboard_driver::KEYBOARD_DRIVER;
use snakian::vga_driver::{ColorCode, Color, WRITER};
use snakian::{serial_println, println, init, eprintln, sleep, print, hardware_interrupts};
use x86_64::instructions;
use x86_64::structures::idt::InterruptDescriptorTable;

#[cfg(not(test))]
#[panic_handler]
pub fn panic_handle(panic: &PanicInfo) -> ! {
    use snakian::panic_handler;

    panic_handler(panic)
}

#[cfg(test)]
#[panic_handler]
pub fn panic_handle(panic: &PanicInfo) -> ! {
    snakian::panic_handler(panic);
}

fn entry_point() -> ! {
    init();

    println!("Init complete!");
    println!("Entering loop");
    let mut key: Option<char> = None;
    loop {
        let lock = KEYBOARD_DRIVER.lock();
        if let Some(curchar) = lock.current_char {
            if key != Some(curchar) {
                key = Some(curchar);
                if lock.current_char_as_key == Some(KeyCode::Backspace) {
                    WRITER.lock().backspace();
                } else {
                    print!("{}", key.unwrap());
                }
            }
        } else {
            key = None;
        }
        // This hlt is necessary because the keyboard driver needs to be able to unlock the keyboard
        instructions::hlt();// or asm!("hlt", options(nomem, nostack));
    }
}

#[no_mangle]
pub extern "C" fn _start() -> ! {

    #[cfg(test)]
    test_main();

    entry_point();
}


