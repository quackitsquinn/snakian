//! This module contains the display logic for the kernel.
//! 
//! This includes the VGA driver, the buffer, and the character writer.
//! - The buffer is a 24-bit color buffer that is used to draw to the screen.
//! - The character writer is a simple writer that writes to the VGA buffer.
//! - The VGA driver is a higher level driver that is essentially a terminal writer.


pub mod buffer;
mod char_writer;
mod vector;
pub mod screen_char;
pub mod chars;
pub mod color_code;
pub mod vga_driver; // low level char writer

pub(super) type ColorTuple = (u8, u8, u8);
pub type CharSprite = [bool; 8 * 8];

use crate::lock_once;
use bootloader_api::info::FrameBuffer;

// Re-export various modules for ease of use (and shorter imports)
pub use crate::display::{
    buffer::Buffer, char_writer::CHAR_WRITER, color_code::ColorCode, vga_driver::WRITER, screen_char::ScreenChar
};

/// Clones the framebuffer and returns a new FrameBuffer struct.
// HACK: This is gross. The framebuffer struct does not implement clone, so we have to do this.
// I need to make a issue on the bootloader repo to add a clone method.
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
    char_writer::init_char_writer(buf.info());
    vga_driver::init_vga();
    lock_once!(buffer::BUFFER).clear();
}
