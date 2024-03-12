#![no_std]
#![no_main]
#![feature(panic_info_message, custom_test_frameworks, abi_x86_interrupt)]
#![test_runner(crate::testing::test_runner)]
#![reexport_test_harness_main = "test_main"]
// make it a compiler err becuase bad practice
#![deny(unsafe_op_in_unsafe_fn)]

use core::{fmt::Write, mem, panic::PanicInfo};

use bootloader_api::{config::Mapping, entry_point, info::FrameBuffer, BootInfo, BootloaderConfig};
use hardware_interrupts::init_hardware;
use ::log::Level;
use spin::Mutex;
use x86_64::{
    instructions::{hlt, interrupts::without_interrupts},
    VirtAddr,
};

use crate::{display::ColorCode, hardware_interrupts::timer::TICKS_UNSAFE, prelude::*};

pub mod display;
pub mod gdt;
pub mod hardware_interrupts;
pub mod interrupts;
pub mod keyboard_driver;
pub mod memory;
pub mod serial;
pub mod testing;
pub mod log;
pub mod panic;

#[macro_export]
/// Prints out to the serial port with the file and line number
macro_rules! dbg {
    () => {
        $crate::serial_println!(
            "[{}:{}]",
            file!()
            line!()
        );
    };
    ($($arg:tt)*) => {
        $crate::serial_println!(
            "[{}:{}]: {}",
            file!(),
            line!(),
            format_args!($($arg)*)
        );
    };
}

/// Locks a OnceCell<Mutex<T>> and returns the lock
#[macro_export]
macro_rules! lock_once {
    ($oncelock:expr) => {{
        $oncelock
            .get()
            .expect(concat!(
                "OnceCell ",
                stringify!($oncelock),
                " not initialized!"
            ))
            .lock()
    }};
}

#[cfg(test)]
pub fn test_main_init(_: &'static mut BootInfo) -> ! {
    test_main();
    interrupts::hlt_loop()
}

#[cfg(test)]
entry_point!(test_main_init);

#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    testing::panic_handler(info);
}

pub static BOOT_CONFIG: BootloaderConfig = {
    let mut config = BootloaderConfig::new_default();
    config.mappings.physical_memory = Some(Mapping::Dynamic);
    config.kernel_stack_size = 512 * 1024; // we need a lot of space for the vga buffer
    config
};

pub static HAS_INIT: Mutex<bool> = Mutex::new(false);
//TODO: determine if init stages should exist (aka multiple init functions like init_stage0 init_stage1 etc)
pub fn init(boot_info: &'static mut bootloader_api::BootInfo) {
    #[cfg(debug_assertions)]
    log::init_logger(Level::Trace, Level::Warn);
    #[cfg(not(debug_assertions))]
    log::init_logger(Level::Trace, Level::Warn);
    info!("Initializing hardware");
    info!("Initializing memory");
    unsafe { memory::init(boot_info.physical_memory_offset.into_option().unwrap()) };
    info!("Initializing VGA driver");
    let framebuf = boot_info.framebuffer.as_mut().unwrap();
    info!("Framebuffer address: {:p}", framebuf);
    display::init(framebuf);
    info!("Initialized VGA driver");
    info!("Starting display logging");
    log::init_display_logger();
    info!("Initialized display logging");
    init_hardware();
    interrupts::init_idt();
    info!("Initialized IDT");
    gdt::init_gdt();
    info!("Initialized GDT");
    info!("Enabling interrupts");
    x86_64::instructions::interrupts::enable();
    info!("Enabled interrupts");
    info!("Initialized hardware");
    *HAS_INIT.lock() = true;
}
/// Contains several useful functions to be included in the prelude
// TODO: when alloc is implemented, add stuff like Vec, Box, etc (like pub use alloc::vec::Vec; etc)
pub mod prelude {
    pub use crate::{
        dbg, eprint, eprintln, lock_once, print, println, serial_print, serial_println,
    };
    pub use log::{debug, error, info, trace, warn};
}
