use pic8259::ChainedPics;
use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame};
use lazy_static::lazy_static;
use crate::{println, gdt::IST_FAULT_INDEX};


lazy_static! {
    static ref IDT: InterruptDescriptorTable = {
        let mut idt = InterruptDescriptorTable::new();

        // breakpoint interrupt
        idt.breakpoint.set_handler_fn(breakpoint_handler);
        idt.double_fault.set_handler_fn(double_fault_handler);

        unsafe {
            idt.double_fault.set_handler_fn(double_fault_handler).set_stack_index(IST_FAULT_INDEX);
        }

        idt
    };
}

pub fn init_idt() {
    IDT.load();
}

extern "x86-interrupt" fn breakpoint_handler(stack_frame: InterruptStackFrame) {
        println!("EXCEPTION: BREAKPOINT\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn double_fault_handler(stack_frame: InterruptStackFrame, error_code: u64) -> ! {
        panic!("EXCEPTION: DOUBLE FAULT ({}) \n{:#?}",error_code, stack_frame);
}


pub const PIC_1_OFFSET: u8 = 32;
pub const PIC_2_OFFSET: u8 = PIC_1_OFFSET + 8;

pub static PICS: spin::Mutex<ChainedPics> = spin::Mutex::new(unsafe { ChainedPics::new(PIC_1_OFFSET, PIC_2_OFFSET) });

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum InterruptIndex {
    Timer = PIC_1_OFFSET,
}

impl InterruptIndex {
    pub fn as_u8(self) -> u8 {
        self as u8
    }

    pub fn as_usize(self) -> usize {
        usize::from(self.as_u8())
    }
}


type HandlerFn = extern "x86-interrupt" fn(InterruptStackFrame);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct InterruptHandler {
    pub index: u8,
    pub handler: HandlerFn,
}
/// An IDT Interrupt Handler
impl InterruptHandler {
    /// Creates a new InterruptHandler
    pub fn new(index: InterruptIndex, handler: HandlerFn) -> InterruptHandler {
        InterruptHandler {
            index: index.as_u8(),
            handler,
        }
    }
    /// Creates a new InterruptHandler without checking if the index is valid
    /// # Safety
    /// This function is unsafe because it doesn't check if the index is valid. When using this function, make sure that the index is valid and what you want.
    pub unsafe fn new_unchecked(index: u8, handler: HandlerFn) -> InterruptHandler {
        InterruptHandler {
            index,
            handler,
        }
    }
}
/// Interrupt Descriptor Table Loader for loading the IDT
pub struct IdtLoader {
    handlers: [Option<InterruptHandler>; 256],
    is_loaded: bool,
}

impl IdtLoader {
    /// Creates a new and empty IDT Loader
    fn new() -> IdtLoader {
        IdtLoader {
            handlers: [None; 256],
            is_loaded: false,
        }
    }
    /// Loads the contained IDT handlers into the IDT.
    fn load(&mut self, idt: &mut InterruptDescriptorTable) {
        if self.is_loaded {
            return;
        }

        for handler in self.handlers.iter() {
            if let Some(handler) = handler {
                idt[handler.index as usize].set_handler_fn(handler.handler);
            }
        }

        self.is_loaded = true;
    }
    /// Adds a new handler to the IDT
    /// # Panics
    /// This function panics if the IDT is already loaded.
    pub fn add_handler(&mut self, handler: InterruptHandler) {
        if self.is_loaded {
            panic!("Cannot add handler after IDT is loaded!");
        }
        self.handlers[handler.index as usize] = Some(handler);
    }
}