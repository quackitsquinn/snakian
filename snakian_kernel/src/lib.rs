

#![no_std]
#![no_main]
#![feature(panic_info_message, custom_test_frameworks, abi_x86_interrupt)]
#![test_runner(crate::testing::test_runner)]
#![reexport_test_harness_main = "test_main"]

use core::{panic::PanicInfo, mem};

use hardware_interrupts::init_hardware;

use crate::vga_driver::{ColorCode, Color};

pub mod serial;
pub mod vga_driver;
pub mod testing;
pub mod interrupts;
pub mod gdt;
pub mod hardware_interrupts;
pub mod keyboard_driver;

pub fn panic_handler(panic: &PanicInfo) -> ! {
    // forces the write position to the beginning of the buffer (will be changed this is just for quick and dirty testing)
    //vga_driver::WRITER.lock().reset();
    // set panic format to be red on white
    //vga_driver::WRITER.lock().color_code = ColorCode::new(Color::Red, Color::White, true);
    // write the panic message
    println!("Kernal Panic in file {} at line {}", panic.location().unwrap().file(), panic.location().unwrap().line());
    println!("Reason:{}", panic.message().unwrap());
    serial_println!("Kernal Panic in file {} at line {}", panic.location().unwrap().file(), panic.location().unwrap().line());
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
//TODO: determine if init stages should exist (aka multiple init functions like init_stage0 init_stage1 etc)
pub fn init(boot_info: &'static mut bootloader_api::BootInfo) {
    let vgabuf = boot_info.framebuffer.as_mut().unwrap();
    let info = vgabuf.info();
    // so i want to be able to init the screen manually, so we need to read a u64 at vgabuf (it has C representation of a struct)
    let ptr = unsafe { *(vgabuf as *mut _ as *mut u64) };
    // make the actual pointer
    let ptr = ptr as *mut u8;
    // SAFETY: we know that the pointer is valid because we just created it
    // we want to write a very basic clear to the screen to ensure that the screen is working
    let width = info.stride; // stride includes padding, which we want
    let height = info.height;

    for y in 0..=(height) {
        for x in 0..=(width * info.bytes_per_pixel) /* RGB */ {
            let offset = (y * width + x) as isize;
            unsafe {

                let ptr = ptr.offset(offset);
                // write a black pixel to the screen
                // SAFETY: we know that the pointer is valid because we just created it
                *ptr = 255;
            }
        }
    }
    serial_println!("Screen cleared");
    init_hardware();
    serial_println!("Hardware initialized");
    interrupts::init_idt();
    serial_println!("IDT initialized");
    gdt::init_gdt();
    serial_println!("GDT initialized");
    x86_64::instructions::interrupts::enable();
    serial_println!("Interrupts enabled");
}