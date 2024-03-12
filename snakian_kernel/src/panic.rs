use core::panic::PanicInfo;
use core::fmt::Write;

use x86_64::instructions::hlt;
use x86_64::instructions::interrupts::without_interrupts;

use crate::display::ColorCode;
use crate::hardware_interrupts::timer::TICKS_UNSAFE;
use crate::HAS_INIT;
use crate::prelude::*;
use crate::display;

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
    unsafe { display::WRITER.get().unwrap().force_unlock() }
    let mut writer = lock_once!(display::WRITER);
    info!("Panic writer initialized!");
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
                ); // we dont care if this fails, its a panic so if we panic while panicing, we infinitly recurse until the stack overflows
                   // forces the write position to the beginning of the buffer.
                writer.set_pos(0, 0);
            });
            hlt(); // hault the cpu until the next interrupt
        }
    }
}