use x86_64::structures::idt::InterruptStackFrame;

use crate::{
    interrupts::{IDT_LOADER, PIC_1_OFFSET},
    print,
};

use self::timer::TICKS;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum InterruptIndex {
    Timer = PIC_1_OFFSET,
    Keyboard,
}

impl InterruptIndex {
    pub fn as_u8(self) -> u8 {
        self as u8
    }

    pub fn as_usize(self) -> usize {
        usize::from(self.as_u8())
    }
}

pub mod timer {
    use spin::Mutex;

    use crate::{interrupts::PICS, println};

    use super::*;

    pub static TICKS: Mutex<u64> = Mutex::new(0);

    pub extern "x86-interrupt" fn timer_interrupt_handler(_stack_frame: InterruptStackFrame) {
        unsafe {
            // make SURE that we don't get interrupted while we're incrementing the tick count
            // also, nothing else should be writing to the tick count, so we don't need to worry about that
            TICKS.force_unlock();
            *TICKS.lock() += 1;
            PICS.lock()
                .notify_end_of_interrupt(InterruptIndex::Timer.as_u8());
        }
    }
    /// Sleeps for a given amount of ticks. This is a busy wait. (TODO: determine how long timer ticks are)
    #[macro_export]
    macro_rules! sleep {
        ($duration: expr) => {
            let start = $crate::hardware_interrupts::timer::TICKS.lock().clone();
            while $crate::hardware_interrupts::timer::TICKS.lock().clone() < start + $duration {}
        };
    }
}

pub mod keyboard {
    use pc_keyboard::{layouts, Keyboard, ScancodeSet1};
    use spin::Mutex;
    use x86_64::instructions::port::{Port, ReadOnlyAccess};

    use crate::{interrupts::PICS, keyboard_driver::KEYBOARD_DRIVER};
    use lazy_static::lazy_static;

    use super::*;

    lazy_static! {
        pub static ref KEYBOARD: Mutex<Keyboard<layouts::Us104Key, ScancodeSet1>> =
            Mutex::new(Keyboard::new(
                ScancodeSet1::new(),
                layouts::Us104Key,
                pc_keyboard::HandleControl::Ignore
            ));
    }

    pub extern "x86-interrupt" fn keyboard_interrupt_handler(_stack_frame: InterruptStackFrame) {
        let mut port = Port::new(0x60);
        let scancode: u8 = unsafe { port.read() };
        unsafe {
            KEYBOARD_DRIVER.force_unlock();
        }

        KEYBOARD_DRIVER.lock().handle_byte(scancode);
        unsafe {
            PICS.lock()
                .notify_end_of_interrupt(InterruptIndex::Keyboard.as_u8());
        }
    }
}
pub fn init_hardware() {
    IDT_LOADER
        .lock()
        .add_raw(InterruptIndex::Timer, timer::timer_interrupt_handler);
    IDT_LOADER.lock().add_raw(
        InterruptIndex::Keyboard,
        keyboard::keyboard_interrupt_handler,
    );
}
