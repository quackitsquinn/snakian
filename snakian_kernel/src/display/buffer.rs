use core::{cmp::min, mem};

use bootloader_api::info::{FrameBuffer, FrameBufferInfo, PixelFormat};
use conquer_once::spin::OnceCell;
use spin::Mutex;

use crate::prelude::*;

use super::{vector::Vector, ColorTuple};

// FIXME: soo i kinda didnt keep track of what stuff is x and what stuff is y, so half the stuff is flipped and in general its a mess.
// so fix it.
const MAX_BUFF_SIZE: Vector = Vector::new(64, 64);

pub struct Buffer<'a> {
    pub display: &'a mut [ColorTuple],
    pub(super) buf: FrameBuffer,
    pub(super) config: FrameBufferInfo,
}

impl<'a> Buffer<'a> {
    /// Creates a new buffer from the given framebuffer.
    pub(super) fn new(buf: FrameBuffer) -> Buffer<'a> {
        // Currently only supports 24-bit color. 256 color support will be added later.
        if buf.info().pixel_format == PixelFormat::U8 {
            panic!("U8 pixel format is not supported!");
        }
        // Make the buffer mutable
        let mut buf = buf;
        // Get the buffer info
        let config = buf.info();

        let flat = buf.buffer_mut();
        // SAFETY: This is safe because we checked the pixel format above, so we know that the buffer is 24-bit.
        // We do this to convert a u8 slice to a ColorTuple slice (u8, u8, u8).
        let display = unsafe {
            core::slice::from_raw_parts_mut(
                flat.as_ptr() as *mut ColorTuple,
                flat.len() / mem::size_of::<ColorTuple>(),
            )
        };
        // The size of the character buffer. Because we dont currently have an allocator, we use this rather than a vec.
        // TODO: when a alloc algorithm is implemented, this should be converted to a vec, and removed from the struct.
        let char_buf_size = Vector::new(
            min(config.width as usize / 8, MAX_BUFF_SIZE.x as usize) - 1,
            min(config.width as usize / 8, MAX_BUFF_SIZE.y as usize - 1) - 1,
        );

        info!("vgainfo: {{");
        info!("  width: {}", config.width);
        info!("  height: {}", config.height);
        info!("  bytes_per_pixel: {}", config.bytes_per_pixel);
        info!("  char_buff_size: {:?}", char_buf_size);
        info!("}}");

        Buffer {
            display,
            buf: buf,
            config: config,
        }
    }
    /// Clears the buffer.
    pub fn clear(&mut self) {
        for rgb in self.display.iter_mut() {
            *rgb = (0, 0, 0);
        }
    }
    /// Updates a pixel at the given x and y coordinates.
    #[inline(always)] // inlined because this is called a lot and is very small
    pub fn set_px(&mut self, x: usize, y: usize, color: ColorTuple) {
        let idx = y * self.config.width as usize + x;
        self.display[idx] = color;
    }
    /// Updates a pixel at the given x and y coordinates, scaled by the given scale.
    pub fn draw_px_scaled(&mut self, x: usize, y: usize, color: ColorTuple, scale: u8) {
        for i in 0..scale {
            for j in 0..scale {
                self.set_px(x + i as usize, y + j as usize, color);
            }
        }
    }
}

pub static BUFFER: OnceCell<Mutex<Buffer>> = OnceCell::uninit();

pub(super) fn init(buf: FrameBuffer) {
    BUFFER
        .try_init_once(|| Mutex::new(Buffer::new(buf)))
        .expect("Failed to initialize buffer");
}
