#![no_std]
#![no_main]
#![feature(panic_info_message, custom_test_frameworks)]
#![test_runner(snakian_kernel::testing::test_runner)]
#![reexport_test_harness_main = "test_main"]

use core::panic::PanicInfo;

use bootloader_api::{entry_point, BootInfo};
use pc_keyboard::KeyCode;
use snakian_kernel::{
    dbg,
    display::{
        buffer::BUFFER,
        vga_driver::{self, WRITER},
    },
    eprintln, init,
    keyboard_driver::KEYBOARD_DRIVER,
    print, println,
};
use x86_64::instructions;

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
    let mut buf = BUFFER.get().unwrap().lock();
    buf.clear();
    buf.set_scale(2);
    drop(buf);
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
