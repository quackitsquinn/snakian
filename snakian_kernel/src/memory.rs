use x86_64::{structures::paging::{PageTable, Page}, registers::control::Cr3, VirtAddr};


/// Gets the active level 4 page table. 
/// 
#[warn(unsafe_op_in_unsafe_fn)]
pub unsafe fn get_l4_table(phys_mem_offset: VirtAddr) -> &'static mut PageTable {
    let (lvl4pgtbl, _) = Cr3::read();
    let phys = lvl4pgtbl.start_address();
    let virt = phys_mem_offset + phys.as_u64();
    let pg_tbl_ptr: *mut PageTable = virt.as_mut_ptr();

    unsafe {
        &mut *pg_tbl_ptr
    }
}