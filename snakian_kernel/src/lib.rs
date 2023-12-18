#![no_std]
#![no_main]
#![feature(panic_info_message, custom_test_frameworks, abi_x86_interrupt)]
#![test_runner(crate::testing::test_runner)]
#![reexport_test_harness_main = "test_main"]
// make it a compiler err becuase bad practice
#![deny(unsafe_op_in_unsafe_fn)]

use core::{mem, panic::PanicInfo};

use bootloader_api::{info::FrameBuffer, BootloaderConfig, config::Mapping};
use hardware_interrupts::init_hardware;
use x86_64::VirtAddr;

use display::ColorCode;

pub mod display;
pub mod gdt;
pub mod hardware_interrupts;
pub mod interrupts;
pub mod keyboard_driver;
pub mod serial;
pub mod testing;
pub mod memory;

#[macro_export]

/// Prints out to the serial port
macro_rules! dbg {
    () => ($crate::serial_print!("\n"));
    ($fmt:expr) => ($crate::serial_print!(concat!($fmt, "\n")));
    ($fmt:expr, $($arg:tt)*) => ($crate::serial_print!(concat!($fmt, "\n"), $($arg)*));
}

pub fn panic_handler(panic: &PanicInfo) -> ! {
    // forces the write position to the beginning of the buffer (will be changed this is just for quick and dirty testing)
    //vga_driver::WRITER.lock().reset();
    // set panic format to be red on white
    //vga_driver::WRITER.lock().color_code = ColorCode::new(Color::Red, Color::White, true);
    // write the panic message
    //println!("Kernal Panic in file {} at line {}", panic.location().unwrap().file(), panic.location().unwrap().line());
    //println!("Reason:{}", panic.message().unwrap());
    serial_println!(
        "Kernal Panic in file {} at line {}",
        panic.location().unwrap().file(),
        panic.location().unwrap().line()
    );
    serial_println!("Reason:{}", panic.message().unwrap());
    interrupts::hlt_loop();
}

#[cfg(test)]
#[no_mangle]
pub extern "C" fn _start() {
    test_main();
    interrupts::hlt_loop();
}

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

//TODO: determine if init stages should exist (aka multiple init functions like init_stage0 init_stage1 etc)
pub fn init(boot_info: &'static mut bootloader_api::BootInfo) {
    dbg!("Getting level 4 page table");
    let phys_offset = boot_info.physical_memory_offset.into_option().unwrap();
    let l4_table = unsafe { memory::get_l4_table(VirtAddr::new(phys_offset)) };
    for (i, entry) in l4_table.iter().enumerate() {
        if !entry.is_unused() {
            dbg!("L4 Entry {} is used", i);
        }
    }
    dbg!("Initializing hardware! {{");
    dbg!("   Initializing VGA driver!");
    let framebuf = boot_info.framebuffer.as_mut().unwrap();
    dbg!("      Framebuffer address: {:p}", framebuf);
    display::init(framebuf);
    init_hardware();
    interrupts::init_idt();
    serial_println!("   IDT initialized");
    gdt::init_gdt();
    serial_println!("   GDT initialized");
    x86_64::instructions::interrupts::enable();
    serial_println!("   Interrupts enabled");
    serial_println!("}} Hardware initialized");
}
