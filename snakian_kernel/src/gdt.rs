use lazy_static::lazy_static;
use x86_64::{structures::{idt::InterruptDescriptorTable, tss::TaskStateSegment, gdt::{GlobalDescriptorTable, Descriptor, SegmentSelector}}, VirtAddr, registers::segmentation::{CS, Segment}, instructions::tables::load_tss};

pub const IST_FAULT_INDEX: u16 = 0;

lazy_static! {
    static ref TSS: TaskStateSegment = {
        let mut tss = TaskStateSegment::new();
        tss.interrupt_stack_table[IST_FAULT_INDEX as usize] = {
            const STACK_SIZE: usize = 4096 * 5;
            static mut STACK: [u8; STACK_SIZE] = [0; STACK_SIZE];

            let stack_start = VirtAddr::from_ptr(unsafe { &STACK });
            let stack_end = stack_start + STACK_SIZE;
            stack_end
        };
        tss
    };

    static ref GDT: (GlobalDescriptorTable, Selectors) = {
        let mut gdt = GlobalDescriptorTable::new();
        let kcode = gdt.add_entry(Descriptor::kernel_code_segment());
        let tss = gdt.add_entry(Descriptor::tss_segment(&TSS));
        (gdt, Selectors {
            code_selector: kcode,
            tss_selector: tss,
        })
    };
}

pub fn init_gdt() {
    GDT.0.load();

    unsafe {
        CS::set_reg(GDT.1.code_selector);
        load_tss(GDT.1.tss_selector);
    }
    
}

struct Selectors {
    code_selector: SegmentSelector,
    tss_selector: SegmentSelector,
}