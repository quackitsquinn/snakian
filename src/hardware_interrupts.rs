use x86_64::structures::idt::InterruptStackFrame;

use crate::{print, interrupts::{IDT_LOADER, InterruptIndex}};

use self::timer::TICKS;

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
            PICS.lock().notify_end_of_interrupt(InterruptIndex::Timer.as_u8());
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
pub fn init_hardware() {
    IDT_LOADER.lock().add_raw(InterruptIndex::Timer, timer::timer_interrupt_handler);
}