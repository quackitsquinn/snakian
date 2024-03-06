use conquer_once::spin::OnceCell;
use x86_64::{
    registers::control::Cr3,
    structures::paging::{OffsetPageTable, PageTable,Translate},
    PhysAddr, VirtAddr,
};

use spin::Mutex;

use crate::lock_once;

pub static OFFSET_PAGE_TABLE: OnceCell<Mutex<OffsetPageTable>> = OnceCell::uninit();
/// Initializes the memory module. This function should be called before any other memory functions.
/// This function has no dependencies, so it can be called at the start of kernel initialization.
pub unsafe fn init(physical_memory_offset: u64){
    OFFSET_PAGE_TABLE.init_once(|| {
        let level_4_table = unsafe { get_l4_table(VirtAddr::new(physical_memory_offset)) };
        Mutex::new(unsafe {
            OffsetPageTable::new(level_4_table, VirtAddr::new(physical_memory_offset))
        })
    });
}

/// Gets the active level 4 page table.
unsafe fn get_l4_table(phys_mem_offset: VirtAddr) -> &'static mut PageTable {
    let (lvl4pgtbl, _) = Cr3::read();
    let phys = lvl4pgtbl.start_address();
    let virt = phys_mem_offset + phys.as_u64();
    let pg_tbl_ptr: *mut PageTable = virt.as_mut_ptr();

    unsafe { &mut *pg_tbl_ptr }
}

/// Translates a virtual address to a physical address. If the address is not mapped, this function returns None.
pub fn translate_addr(addr: VirtAddr) -> Option<PhysAddr> {
    let offset_page_table = lock_once!(OFFSET_PAGE_TABLE);
    offset_page_table.translate_addr(addr)
}

