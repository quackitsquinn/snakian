use x86_64::{
    registers::control::Cr3,
    structures::paging::{Page, PageTable, page_table::FrameError},
    VirtAddr, PhysAddr,
};

/// Gets the active level 4 page table.
pub unsafe fn get_l4_table(phys_mem_offset: VirtAddr) -> &'static mut PageTable {
    let (lvl4pgtbl, _) = Cr3::read();
    let phys = lvl4pgtbl.start_address();
    let virt = phys_mem_offset + phys.as_u64();
    let pg_tbl_ptr: *mut PageTable = virt.as_mut_ptr();

    unsafe { &mut *pg_tbl_ptr }
}


pub unsafe fn translate_addr(addr: VirtAddr, phys_mem_offset: VirtAddr) -> Option<PhysAddr> {
    let (page_tbl,_) = Cr3::read();
    let tables = [
        addr.p4_index(),
        addr.p3_index(),
        addr.p2_index(),
        addr.p1_index(),
    ];
    
    let mut frame = page_tbl;

    for &index in &tables {
        let virt = phys_mem_offset + frame.start_address().as_u64();
        let table_ptr: *const PageTable = virt.as_ptr();
        let table = unsafe { &*table_ptr };

        let entry = &table[index];

        frame = match entry.frame() {
            Ok(frame) => frame,
            Err(FrameError::FrameNotPresent) => {return None;},
            Err(FrameError::HugeFrame) => panic!("Huge pages not supported!"),

        };
    }

    Some(frame.start_address() + u64::from(addr.page_offset()));

    None
}