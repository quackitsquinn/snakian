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
    pub(super) display: &'a mut [ColorTuple],
    pub(super) buf: FrameBuffer,
    pub(super) config: FrameBufferInfo,
    pub(super) char_scale: usize, // this will be used to scale the characters to the screen size. (variable font size)
    pub(super) char_buff_size: (usize, usize),
    pub(super) char_buffer: [[ScreenChar; MAX_BUFF_SIZE.1]; MAX_BUFF_SIZE.0],
    pub(super) color_fmt: PixelFormat,
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
            config,
            char_scale: 1,
            char_buff_size: char_buf_size,
            char_buffer: [[ScreenChar::none(); MAX_BUFF_SIZE.1]; MAX_BUFF_SIZE.0],
            color_fmt: config.pixel_format,
        }
    }
    pub fn clear(&mut self) {
        for rgb in self.display.iter_mut() {
            *rgb = (0, 0, 0);
        }
        for row in 0..self.char_buff_size.0 {
            for col in 0..self.char_buff_size.1 {
                self.char_buffer[row][col] = ScreenChar::none();
            }
        }
        self.fill(b' ', ColorCode::default());
    }

    pub fn clear_row(&mut self, row: usize) {
        self.fill_row(row, b' ', ColorCode::default());
    }

    pub fn fill(&mut self, c: u8, color_code: ColorCode) {
        dbg!("buf size: {:?}", self.char_buff_size);
        for row in 0..self.char_buff_size.0 {
            for col in 0..self.char_buff_size.1 {
                self.char_buffer[row][col] = ScreenChar::new(c, color_code);
            }
        }
    }

    pub fn fill_row(&mut self, row: usize, c: u8, color_code: ColorCode) {
        for col in 0..self.char_buff_size.0 {
            self.char_buffer[row][col] = ScreenChar::new(c, color_code);
        }
    }

    pub fn xy_to_index(&self, x: usize, y: usize) -> usize {
        y * self.config.width as usize + x
    }

    pub fn write_8x8_buf(
        &mut self,
        buf: CharSprite,
        row: usize,
        col: usize,
        color_code: ColorCode,
    ) {
        assert!(row < self.config.height - 8);
        assert!(col < self.config.width - 8);
        for y in 0..8 {
            for x in 0..8 {
                let c = buf[y * 8 + x];
                if c {
                    let scrx = col + x;
                    let scry = row + y;
                    self.display[self.xy_to_index(scrx, scry)] =
                        color_code.to_format(self.color_fmt);
                }
            }
        }
    }

    pub fn write_8x8_buf_scaled(
        &mut self,
        buf: CharSprite,
        row: usize,
        col: usize,
        color_code: ColorCode,
        scale: u8,
    ) {
        assert!(row < self.config.height - (8 * scale) as usize);
        assert!(col < self.config.width - (8 * scale) as usize);
        let fill = color_code.has_bg;
        let color = color_code.format_bg(self.color_fmt).unwrap_or((0, 0, 0));
        for y in 0..8 {
            for x in 0..8 {
                let c = buf[y * 8 + x];
                if c {
                    let scrx = col + x * scale as usize;
                    let scry = row + y * scale as usize;
                    for i in 0..scale {
                        for j in 0..scale {
                            self.display[self.xy_to_index(scrx + i as usize, scry + j as usize)] =
                                color_code.to_format(self.color_fmt);
                        }
                    }
                } else if fill {
                    let scrx = col + x * scale as usize;
                    let scry = row + y * scale as usize;
                    for i in 0..scale {
                        for j in 0..scale {
                            self.display[self.xy_to_index(scrx + i as usize, scry + j as usize)] =
                                color;
                        }
                    }
                }
            }
        }
    }

    pub(crate) fn flush_char_buf(&mut self) {
        let buf_width = self.char_buff_size.1 - 1;
        let buf_height = self.char_buff_size.0 - 1;
        for row in 0..buf_height {
            for col in 0..buf_width {
                let c = self.char_buffer[row][col];
                let char_sprite = chars::get_char_sprite(c.ascii_character as char);
                self.write_8x8_buf_scaled(
                    char_sprite,
                    row * 8 * self.char_scale,
                    col * 8 * self.char_scale,
                    c.color_code,
                    self.char_scale as u8,
                );
            }
        }
    }

    pub(crate) fn flush_char_at(&mut self, row: usize, col: usize) {
        let c = self.char_buffer[row][col];
        let char_sprite = chars::get_char_sprite(c.ascii_character as char);
        self.write_8x8_buf_scaled(
            char_sprite,
            row * 8 * self.char_scale,
            col * 8 * self.char_scale,
            c.color_code,
            self.char_scale as u8,
        );
    }

    pub(crate) fn flush_row(&mut self, row: usize) {
        let buf_width = self.char_buff_size.1 - 1;
        let buf_height = self.char_buff_size.0 - 1;
        for col in 0..buf_width {
            let c = self.char_buffer[row][col];
            let char_sprite = chars::get_char_sprite(c.ascii_character as char);
            self.write_8x8_buf_scaled(
                char_sprite,
                row * 8 * self.char_scale,
                col * 8 * self.char_scale,
                c.color_code,
                self.char_scale as u8,
            );
        }
    }

    pub fn set_scale(&mut self, scale: usize) {
        self.char_scale = scale;
        self.char_buff_size = (
            min(self.config.width as usize / (8 * scale), MAX_BUFF_SIZE.0) - 1,
            min(self.config.height as usize / (8 * scale), MAX_BUFF_SIZE.1) - 1,
        );
        dbg!("set scale to {}", scale);
        dbg!("new char_buff_size: {:?}", self.char_buff_size);
    }
}

pub static BUFFER: OnceCell<Mutex<Buffer>> = OnceCell::uninit();

pub(super) fn init(buf: FrameBuffer) {
    BUFFER
        .try_init_once(|| Mutex::new(Buffer::new(buf)))
        .expect("Failed to initialize buffer");
}
