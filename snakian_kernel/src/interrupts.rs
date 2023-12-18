use crate::{gdt::IST_FAULT_INDEX, hardware_interrupts::InterruptIndex, println};
use lazy_static::lazy_static;
use pic8259::ChainedPics;
use x86_64::{structures::idt::{InterruptDescriptorTable, InterruptStackFrame, PageFaultErrorCode}, instructions::hlt};

macro_rules! def_handler_isf {
    ($idt: expr, $name: ident) => {
        extern "x86-interrupt" fn $name(stack_frame: InterruptStackFrame) {
            crate::serial_println!("EXCEPTION: {}\n{:#?}", stringify!($name), stack_frame);
        }
        $idt.$name.set_handler_fn($name);
    };
}

macro_rules! def_handler_isf_code {
    ($idt: expr,$name: ident) => {
        extern "x86-interrupt" fn $name(stack_frame: InterruptStackFrame, error_code: u64) {
            crate::serial_println!(
                "EXCEPTION: {} ({})\n{:#?}",
                stringify!($name),
                error_code,
                stack_frame
            );
        }
        $idt.$name.set_handler_fn($name);
    };

    ($idt: expr,$name: ident, $_trap: expr) => {
        extern "x86-interrupt" fn $name(stack_frame: InterruptStackFrame, error_code: u64) -> ! {
            crate::serial_println!(
                "EXCEPTION: {} ({})\n{:#?}",
                stringify!($name),
                error_code,
                stack_frame
            );
            hlt_loop();
        }
        $idt.$name.set_handler_fn($name);
    };
}

lazy_static! {
    pub static ref IDT_LOADER: spin::Mutex<IdtLoader> = spin::Mutex::new(IdtLoader::new());
    static ref IDT: InterruptDescriptorTable = {
        // TODO: When a global allocator is added, use a leaking Box to allocate the IDT.
        // I really **REALLY** dislike this solution, but it's the only one that works for now (at my skill of rust magic)
        println!("Initializing IDT"); // we want to see when this happens to ensure that it's not happening too early
        // i really hope that lazy_static **WAITS** until somthing accesses the IDT before it initializes it
        let mut idt = InterruptDescriptorTable::new();

        x86_64::set_general_handler!(&mut idt,general_handler);

        def_handler_isf_code!(idt, general_protection_fault);

        def_handler_isf!(idt, breakpoint);

        def_handler_isf_code!(idt, double_fault, "no return");

        extern "x86-interrupt" fn page_fault_handler(
            stack_frame: InterruptStackFrame,
            error_code: PageFaultErrorCode,
        ) {
            use x86_64::registers::control::Cr2;

            println!("EXCEPTION: PAGE FAULT");
            println!("Accessed Address: {:?}", Cr2::read());
            println!("Error Code: {:?}", error_code);
            println!("{:#?}", stack_frame);
            hlt_loop();
        }
        idt.page_fault.set_handler_fn(page_fault_handler);

        let mut lock = IDT_LOADER.lock();
        lock.load(&mut idt);
        idt
    };
}

/// Initializes the IDT. This function should be called before any interrupts are enabled, and after all the handlers are added.
pub fn init_idt() {
    IDT_LOADER.lock();
    IDT.load();
    unsafe { PICS.lock().initialize() };
}

fn general_handler(stack_frame: InterruptStackFrame, index: u8, error_code: Option<u64>) {
    if let Some(code) = error_code {
        crate::serial_println!("EXCEPTION: {} ({})\n{:#?}", index, code, stack_frame);
    } else {
        crate::serial_println!("EXCEPTION: {}\n{:#?}", index, stack_frame);
    }
}

pub const PIC_1_OFFSET: u8 = 32;
pub const PIC_2_OFFSET: u8 = PIC_1_OFFSET + 8;

pub static PICS: spin::Mutex<ChainedPics> =
    spin::Mutex::new(unsafe { ChainedPics::new(PIC_1_OFFSET, PIC_2_OFFSET) });

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
        InterruptHandler { index, handler }
    }
}
/// Interrupt Descriptor Table Loader for loading the IDT
pub struct IdtLoader {
    handlers: [Option<InterruptHandler>; 256],
    loader_fns: [Option<fn(&mut InterruptDescriptorTable)>; 256],
    fndex: usize,
    is_loaded: bool,
}

impl IdtLoader {
    /// Creates a new and empty IDT Loader
    fn new() -> IdtLoader {
        IdtLoader {
            handlers: [None; 256],
            loader_fns: [None; 256],
            fndex: 0,
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

        for loader_fn in self.loader_fns.iter() {
            if let Some(loader_fn) = loader_fn {
                loader_fn(idt);
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
    /// Adds a new constructed handler to the IDT without checking if the index is valid
    pub fn add_raw_unchecked(&mut self, index: u8, handler: HandlerFn) {
        if self.is_loaded {
            panic!("Cannot add handler after IDT is loaded!");
        }
        self.handlers[index as usize] =
            Some(unsafe { InterruptHandler::new_unchecked(index, handler) });
    }
    /// Adds a new handler to the IDT constructed from the index and handler
    pub fn add_raw(&mut self, index: InterruptIndex, handler: HandlerFn) {
        if self.is_loaded {
            panic!("Cannot add handler after IDT is loaded!");
        }
        self.handlers[index.as_usize()] = Some(InterruptHandler::new(index, handler));
    }

    pub fn add_handler_fn(&mut self, fun: fn(&mut InterruptDescriptorTable)) {
        assert!(self.fndex < 256, "Cannot add more than 256 handlers!");
        if self.is_loaded {
            panic!("Cannot add handler after IDT is loaded!");
        }
        self.loader_fns[self.fndex] = Some(fun);
        self.fndex += 1;
    }
}

pub fn hlt_loop() -> ! {
    loop {
        x86_64::instructions::hlt();
    }
}
