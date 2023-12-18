pub mod buffer;
pub mod chars;
pub mod color_code;
pub mod vga_driver;

pub(super) type ColorTuple = (u8, u8, u8);

use bootloader_api::info::FrameBuffer;

// Re-export various modules for ease of use (and shorter imports)
pub use crate::display::{buffer::Buffer, color_code::ColorCode};

/// Clones the framebuffer and returns a new FrameBuffer struct.
/// HACK: This is gross. The framebuffer struct does not implement clone, so we have to do this.
/// I need to make a issue on the bootloader repo to add a clone method.
fn clone_framebuf(buf: &FrameBuffer) -> FrameBuffer {
    let ptrptr = buf as *const FrameBuffer as *const u64;
    // SAFETY: this is safe because the FrameBuffer struct is repr(C)
    // and the first field is a u64, which is the address of the framebuffer
    let addr = unsafe { *ptrptr };
    // clones the framebuffer info
    let info = buf.info();
    unsafe { FrameBuffer::new(addr, info) }
}

pub fn init(buf: &mut FrameBuffer) {
    let mut buf = clone_framebuf(&buf);
    buffer::init(clone_framebuf(&buf));
    vga_driver::init_vga(&mut buf);
}
