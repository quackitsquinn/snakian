use core::cmp::min;

use crate::{dbg, display::vga_driver::ScreenChar};
use bootloader_api::info::{FrameBufferInfo, PixelFormat};
use conquer_once::spin::OnceCell;
use spin::Mutex;

use super::{chars, vga_driver::CharSprite, ColorCode, ColorTuple};

// The max size for the buffer.
const MAX_BUFF_SIZE: (usize, usize) = (128, 64);

macro_rules! get_buffer {
    () => {
        crate::lock_once!(crate::display::buffer::BUFFER)
    };
}
pub struct CharWriter {
    pub(super) char_scale: usize, // this will be used to scale the characters to the screen size. (variable font size)
    pub(super) char_buff_size: (usize, usize),
    pub(super) config: FrameBufferInfo,
    // TODO: when a alloc algorithm is implemented, this should be converted to a vec
    // [[x]y]
    pub(super) char_buffer: [[ScreenChar; MAX_BUFF_SIZE.0]; MAX_BUFF_SIZE.1],
    pub(super) color_fmt: PixelFormat,
}

impl CharWriter {
    pub fn new(config: FrameBufferInfo) -> CharWriter {
        CharWriter {
            char_scale: 1,
            char_buff_size: (
                min(config.width as usize / 8, MAX_BUFF_SIZE.0) - 1,
                min(config.height as usize / 8, MAX_BUFF_SIZE.1) - 1,
            ),
            config,
            char_buffer: [[ScreenChar::none(); MAX_BUFF_SIZE.0]; MAX_BUFF_SIZE.1],
            color_fmt: config.pixel_format,
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
                    get_buffer!().set_px(scrx, scry, color_code.to_format(self.color_fmt));
                }
            }
        }
    }
    /// Writes a 8x8 sprite to the screen with the given scale and color.
    pub fn write_8x8_buf_scaled(
        &mut self,
        sprite: CharSprite,
        y_position: usize,
        x_position: usize,
        color_code: ColorCode,
        scale: u8,
    ) {
        // Preconditions
        assert!(y_position < self.config.height - (8 * scale) as usize);
        assert!(x_position < self.config.width - (8 * scale) as usize);
        // Get the color code for the background if it exists, and the color code for the foreground
        let fill = color_code.has_bg;
        let bg_color = color_code.format_bg(self.color_fmt).unwrap_or((0, 0, 0));
        let fg_color = color_code.to_format(self.color_fmt);
        // Aquire the buffer so we dont have to lock it a bunch of times.
        let mut buf = get_buffer!();

        // Iterate over the bits in the sprite
        for sprite_y in 0..8 {
            for sprite_x in 0..8 {
                let c = sprite[sprite_y * 8 + sprite_x];
                // If the bit is set, draw a pixel at the corresponding position
                if c {
                    // Get the origin of the pixel
                    let scrx = x_position + sprite_x * scale as usize;
                    let scry = y_position + sprite_y * scale as usize;
                    // Draw the pixel scaled based off the origin
                    for x in 0..scale as usize {
                        for y in 0..scale as usize {
                            buf.set_px(scrx + x, scry + y as usize, fg_color);
                        }
                    }
                }
                // Else, if the bit is not set, and the color code has a background, draw a pixel with the background color
                else if fill {
                    // Get the origin of the pixel
                    let scrx = x_position + sprite_x * scale as usize;
                    let scry = y_position + sprite_y * scale as usize;
                    // Draw the pixel scaled based off the origin
                    for x in 0..scale as usize {
                        for y in 0..scale as usize {
                            buf.set_px(scrx + x, scry + y as usize, bg_color);
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
}

pub static CHAR_WRITER: OnceCell<Mutex<CharWriter>> = OnceCell::uninit();

pub fn init_char_writer(buf_info: FrameBufferInfo) {
    dbg!("Initializing char writer!");
    CHAR_WRITER
        .try_init_once(move || {
            dbg!("Initializing char writer container!");
            Mutex::new(CharWriter::new(buf_info))
        })
        .expect("Char writer already initialized!");
}
