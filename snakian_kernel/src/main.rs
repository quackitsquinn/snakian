#![no_std]
#![no_main]
#![feature(panic_info_message, custom_test_frameworks)]
#![test_runner(snakian_kernel::testing::test_runner)]
#![reexport_test_harness_main = "test_main"]
#![deny(unsafe_op_in_unsafe_fn)]

use core::{mem, panic::PanicInfo};

use bootloader_api::{entry_point, BootInfo};
use pc_keyboard::KeyCode;
#[allow(unused_imports)]
use snakian_kernel::prelude::*;

use snakian_kernel::{
    dbg,
    display::{self, terminal::WRITER, CHAR_WRITER},
    init,
    keyboard_driver::KEYBOARD_DRIVER,
    memory,
};
use x86_64::{instructions, structures::paging::FrameAllocator, PhysAddr, VirtAddr};

#[panic_handler]
pub fn panic_handle(panic: &PanicInfo) -> ! {
    snakian_kernel::panic::panic_handler(panic)
}

#[cfg(test)]
#[panic_handler]
pub fn panic_handle(panic: &PanicInfo) -> ! {
    snakian_kernel::testing::panic_handler(panic)
}

static mut random_state: u64 = 0xdeadbeef;

pub fn rand() -> u64 {
    // this is a purely a random number, we dont care if the number gets corrupted
    // hell, if the number gets corrupted, it might be even more random
    unsafe {
        random_state ^= random_state.wrapping_shl(13);
        random_state ^= random_state.wrapping_shr(7);
        random_state ^= random_state.wrapping_shl(17);
    }
    unsafe { random_state }
}

pub fn rand_range(min: u64, max: u64) -> u64 {
    rand() % (max - min) + min
}

pub fn rand_byte() -> u8 {
    rand() as u8
}


//TODO: add basic interpreter for commands (poke, peek, )
fn os_entry_point(boot_info: &'static mut BootInfo) -> ! {
    init(boot_info);
    dbg!("Initialized hardware!");
    dbg!("Entering main loop!");
    let mut buf = lock_once!(CHAR_WRITER);
    buf.set_scale(2);
    drop(buf);

    eprintln!("wewewowe");

    //hlt_loop();

    //panic_runner("test_panic", "test_panic");
    // let mut buf = lock_once!(display::buffer::BUFFER);

    // loop {
    //     let ind = rand_range(0, buf.display.len() as u64) as usize;
    //     let c = (rand_byte(), rand_byte(), rand_byte());
    //     buf.display[ind] = c;
    //     if KEYBOARD_DRIVER.lock().current_char.is_some() {
    //         break;
    //     }
    // }

    // drop(buf);

    let mut key: Option<char> = None;
    let mut keys = [0 as u8; 128];
    let mut i = 0;
    loop {
        let lock = KEYBOARD_DRIVER.lock();
        if let Some(curchar) = lock.current_char {
            if key != Some(curchar) {
                key = Some(curchar);
                if lock.current_char_as_key == Some(KeyCode::Backspace) {
                    WRITER.get().unwrap().lock().backspace();
                    keys[i] = 0;
                    i = i.saturating_sub(1);
                } else if lock.current_char_as_key == Some(KeyCode::Return) {
                    // parse a command here. This is intended to be super quick and dirty
                    if keys.starts_with(b"shup") {
                        lock_once!(WRITER).shift_up();
                    }
                    keys.iter_mut().for_each(|x| *x = 0);
                    i = 0;
                    print!("\n")
                } else {
                    print!("{}", key.unwrap());
                    keys[i] = key.unwrap() as u8;
                    i += 1;
                }
            }
        } else {
            key = None;
        }
        drop(lock);
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
