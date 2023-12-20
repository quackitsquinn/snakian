use core::{cmp::min, mem};

use bootloader_api::info::{FrameBuffer, FrameBufferInfo, PixelFormat};
use conquer_once::spin::OnceCell;
use spin::Mutex;

use crate::{dbg, display::chars};

use super::{
    vga_driver::{CharSprite, ScreenChar},
    ColorCode, ColorTuple,
};

// FIXME: soo i kinda didnt keep track of what stuff is x and what stuff is y, so half the stuff is flipped and in general its a mess.
// so fix it.
const MAX_BUFF_SIZE: (usize, usize) = (64, 64);
// TODO: move high level buffer stuff to a writer module
pub struct Buffer<'a> {
    pub display: &'a mut [ColorTuple],
    pub(super) buf: FrameBuffer,
    pub(super) config: FrameBufferInfo,
}

impl<'a> Buffer<'a> {
    pub(super) fn new(buf: FrameBuffer) -> Buffer<'a> {
        if buf.info().pixel_format == PixelFormat::U8 {
            panic!("U8 pixel format is not supported!");
        }
        let mut buf = buf;
        let config = buf.info();

        let flat = buf.buffer_mut();

        let display = unsafe {
            core::slice::from_raw_parts_mut(
                flat.as_ptr() as *mut ColorTuple,
                flat.len() / mem::size_of::<ColorTuple>(),
            )
        };

        let char_buf_size = (
            min(config.width as usize / 8, MAX_BUFF_SIZE.0) - 1,
            min(config.width as usize / 8, MAX_BUFF_SIZE.1 - 1) - 1,
        );

        dbg!("vgainfo: {{");
        dbg!("  width: {}", config.width);
        dbg!("  height: {}", config.height);
        dbg!("  bytes_per_pixel: {}", config.bytes_per_pixel);
        dbg!("  char_buff_size: {:?}", char_buf_size);
        dbg!("}}");

        Buffer {
            display,
            buf: buf,
            config: config,
        }
    }
    pub fn clear(&mut self) {
        for rgb in self.display.iter_mut() {
            *rgb = (0, 0, 0);
        }
    }

    pub fn set_px(&mut self, x: usize, y: usize, color: ColorTuple) {
        let idx = y * self.config.width as usize + x;
        self.display[idx] = color;
    }
}

pub static BUFFER: OnceCell<Mutex<Buffer>> = OnceCell::uninit();

pub(super) fn init(buf: FrameBuffer) {
    BUFFER
        .try_init_once(|| Mutex::new(Buffer::new(buf)))
        .expect("Failed to initialize buffer");
}
