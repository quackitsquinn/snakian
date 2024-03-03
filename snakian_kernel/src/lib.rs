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
use spin::Mutex;
use x86_64::{
    instructions::{hlt, interrupts::without_interrupts},
    VirtAddr,
};

use crate::{display::ColorCode, hardware_interrupts::timer::TICKS_UNSAFE};

pub mod display;
pub mod gdt;
pub mod hardware_interrupts;
pub mod interrupts;
pub mod keyboard_driver;
pub mod memory;
pub mod serial;
pub mod testing;

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

pub fn panic_handler(panic: &PanicInfo) -> ! {
    serial_println!(
        "Kernal Panic in file {} at line {}",
        panic.location().unwrap().file(),
        panic.location().unwrap().line()
    );
    serial_println!("Panic Reason:{}", panic.message().unwrap());
    let message = panic.message().unwrap().as_str();
    panic_runner(
        panic.location().unwrap().file(),
        message.unwrap_or("No panic message"),
    )
}
/// This is the function that runs the animation when the kernel panics.
// TODO: make this more robust. Add error handling, so it can fall back to a simpler panic animation if it fails. Make it so that it theoretically can't panic.
pub fn panic_runner(location: &str, message: &str) -> ! {
    if !*HAS_INIT.lock() {
        serial_println!("Panic before init, cannot initialize panic writer!");
        // we can't panic if we haven't initialized the hardware
        loop {
            x86_64::instructions::hlt();
        }
    }
    let mut writer = lock_once!(display::WRITER);
    dbg!("Panic writer initialized!");
    writer.reset();
    // set panic format to be red on white
    writer.color_code = ColorCode::new_with_bg((255, 0, 0), (255, 255, 255));
    drop(writer);

    let mut ticks = 0;
    let mut color_timer: u64 = 0;
    loop {
        // we want to rely on the littlest amount of code as possible, keep it simple
        if unsafe { TICKS_UNSAFE } % 10 == 0 {
            let tick_compare = unsafe { TICKS_UNSAFE };
            without_interrupts(|| {
                if ticks != tick_compare {
                    ticks = tick_compare;
                } else {
                    return; // we don't want to do anything if the ticks haven't changed
                }
                color_timer += 1;
                let mut writer = lock_once!(display::WRITER);
                if color_timer % 2 == 0 {
                    writer.color_code = ColorCode::new_with_bg((255, 0, 0), (255, 255, 255));
                } else {
                    writer.color_code = ColorCode::new_with_bg((255, 255, 255), (255, 0, 0));
                }
                let _ = writeln!(
                    writer,
                    "Kernal Panic at location {} \nPanic Reason:{}",
                    location, message
                ); // we dont care if this fails, its a panic so if we panic while panicing, idek what happens
                   // forces the write position to the beginning of the buffer.
                writer.set_pos(0, 0);
            });
            hlt(); // hault the cpu until the next interrupt
        }
    }
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
    dbg!("Initializing hardware");
    dbg!("Initializing memory");
    unsafe { memory::init(boot_info.physical_memory_offset.into_option().unwrap()) };
    dbg!("Initializing VGA driver");
    let framebuf = boot_info.framebuffer.as_mut().unwrap();
    dbg!("Framebuffer address: {:p}", framebuf);
    display::init(framebuf);
    init_hardware();
    interrupts::init_idt();
    dbg!("Initialized IDT");
    gdt::init_gdt();
    dbg!("Initialized GDT");
    dbg!("Enabling interrupts");
    x86_64::instructions::interrupts::enable();
    dbg!("Enabled interrupts");
    dbg!("Initialized hardware");
    *HAS_INIT.lock() = true;
}
/// Contains several useful functions to be included in the prelude
// TODO: when alloc is implemented, add stuff like Vec, Box, etc (like pub use alloc::vec::Vec; etc)
pub mod prelude {
    pub use crate::{
        dbg, eprint, eprintln, lock_once, print, println, serial_print, serial_println,
    };
}
