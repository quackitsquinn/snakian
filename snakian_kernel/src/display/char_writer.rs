use core::cmp::min;

use crate::prelude::*;
use bootloader_api::info::{FrameBufferInfo, PixelFormat};
use conquer_once::spin::OnceCell;
use spin::Mutex;

use super::{chars, vector::Vector, CharSprite, ColorCode, ScreenChar};

/// The max size for the character buffer.
const MAX_BUFF_SIZE: Vector = Vector::new(128, 64);

/// Gets the buffer from the display module.
macro_rules! get_buffer {
    () => {
        crate::lock_once!(crate::display::buffer::BUFFER)
    };
}

/// A Character writer that handles writing sprites to the screen.
pub struct CharWriter {
    /// The scale of the characters. Used becuse 8x8 on a 720p screen is tiny, and newer computers arent going to have a tiny VESA screen.
    pub(super) char_scale: usize,
    /// The size of the character buffer.
    /// Because we dont currently have an allocator, we use this rather than a vec.
    pub char_buff_size: Vector,
    /// The framebuffer info.
    pub(super) config: FrameBufferInfo,
    // TODO: when a alloc algorithm is implemented, this should be converted to a vec
    // [x position][y position]
    /// The character buffer.
    /// This will be converted to a vec when a alloc algorithm is implemented, but for now it is a fixed size array.
    /// Most of the time, not all of the buffer will be used, so it is not a huge deal.
    pub char_buffer: [[ScreenChar; MAX_BUFF_SIZE.x]; MAX_BUFF_SIZE.y],
    /// The pixel format of the framebuffer.
    pub(super) color_fmt: PixelFormat,
}

impl CharWriter {
    /// Creates a new CharWriter with the given framebuffer info.
    pub fn new(config: FrameBufferInfo) -> Self {
        CharWriter {
            char_scale: 1,
            char_buff_size: Vector::new(
                min(config.width as usize / 8, MAX_BUFF_SIZE.x) - 1,
                min(config.height as usize / 8, MAX_BUFF_SIZE.y) - 1,
            ),
            config,
            char_buffer: [[ScreenChar::none(); MAX_BUFF_SIZE.x]; MAX_BUFF_SIZE.y],
            color_fmt: config.pixel_format,
        }
    }
    /// Writes a 8x8 sprite to the screen with the given color.
    /// Faster than `write_8x8_buf_scaled`, but does not support scaling.
    pub fn write_8x8_sprite(&mut self, sprite: CharSprite, y: usize, x: usize, color_code: ColorCode) {
        // Preconditions. The sprite must fit on the screen.
        // Granted this would still crash if it was out of bounds, but it would be a lot harder to debug.
        assert!(y < self.config.height - 8, "Character is too high! {} > ({} - 8)", y, self.config.height);
        assert!(x < self.config.width - 8, "Character is too far right! {} > ({} - 8)", x, self.config.width);
        // Get the color code for the background if it exists, and the color code for the foreground
        let fg = color_code.to_format(self.color_fmt);
        let mut bg = (0, 0, 0);
        let mut bg_is_some = false;
        // If the color code has a background, get the background color
        if color_code.bg_color.is_some() {
            bg = color_code.format_bg(self.color_fmt).unwrap();
            bg_is_some = true;
        }
        // Aquire the buffer so we dont have to lock it a bunch of times.
        let mut buf = get_buffer!();
        // Iterate over the bits in the sprite
        for sprite_y in 0..8 {
            for sprite_x in 0..8 {
                // Is the pixel set?
                let px = sprite[sprite_y * 8 + sprite_x];
                if px {
                    buf.set_px(x + sprite_x, y + sprite_y, fg);
                } else if bg_is_some {
                    buf.set_px(x + sprite_x, y + sprite_y, bg);
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
                // Get if the bit is set
                let c = sprite[sprite_y * 8 + sprite_x];
                // If the bit is set, draw a pixel at the corresponding position
                if c {
                    // Get the origin of the pixel.
                    // Becuase its scaled, we need to multiply the position by the scale.
                    let scrx = x_position + sprite_x * scale as usize;
                    let scry = y_position + sprite_y * scale as usize;
                    // Draw the pixel scaled based off the origin
                    buf.draw_px_scaled(scrx, scry, fg_color, scale);
                }
                // Else, if the bit is not set, and the color code has a background, draw a pixel with the background color
                else if fill {
                    // Get the origin of the pixel
                    let scrx = x_position + sprite_x * scale as usize;
                    let scry = y_position + sprite_y * scale as usize;
                    // Draw the pixel scaled based off the origin
                    buf.draw_px_scaled(scrx, scry, bg_color, scale);
                }
            }
        }
    }

    /// Flushes the character buffer to the screen.
    /// 
    /// This is a very slow operation, and should be avoided if possible.
    /// 
    /// When possible, always use flush_char_at or flush_row instead.
    pub fn flush_char_buf(&mut self) {
        // Go through the entire buffer and write each character to the screen.
        let buf_width = self.char_buff_size.x - 1;
        let buf_height = self.char_buff_size.y - 1;
        // Iterate over the buffer, and for each character, write it to the screen.
        for y in 0..buf_height {
            for x in 0..buf_width {
                let c = self.char_buffer[y][x];
                let char_sprite = chars::get_char_sprite(c.ascii_character as char);
                self.write_8x8_buf_scaled(
                    char_sprite,
                    y * 8 * self.char_scale,
                    x * 8 * self.char_scale,
                    c.color_code,
                    self.char_scale as u8,
                );
            }
        }
    }
    /// Flushes the character at the given position to the screen.
    /// This is significantly faster than flush_char_buf, and is the fastest way to write a single character to the screen.
    pub fn flush_char_at(&mut self, char_y: usize, char_x: usize) {
        // Preconditions
        assert!(char_y < self.char_buff_size.y, "char_y out of bounds! {} > {}", char_y, self.char_buff_size.y);
        assert!(char_x < self.char_buff_size.x, "char_x out of bounds! {} > {}", char_x, self.char_buff_size.x);

        let c = self.char_buffer[char_y][char_x];
        let char_sprite = chars::get_char_sprite(c.ascii_character as char);
        self.write_8x8_buf_scaled(
            char_sprite,
            char_y * 8 * self.char_scale,
            char_x * 8 * self.char_scale,
            c.color_code,
            self.char_scale as u8,
        );
    }
    /// Flushes the given row to the screen.
    /// This is slower than flush_char_at, but faster than flush_char_buf.
    pub fn flush_row(&mut self, row: usize) {
        // Preconditions
        assert!(row < self.char_buff_size.y, "row out of bounds! {} > {}", row, self.char_buff_size.y);

        let buf_width = self.char_buff_size.x - 1;
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
    /// Sets the scale of the characters.
    /// This will not update already written characters, so it is recommended to call clear() or flush_char_buf() after calling this.
    /// This will also update the size of the character buffer.
    pub fn set_scale(&mut self, scale: usize) {
        self.char_scale = scale;
        self.char_buff_size = Vector::new(
            min(self.config.width as usize / (8 * scale), MAX_BUFF_SIZE.x) - 1,
            min(self.config.height as usize / (8 * scale), MAX_BUFF_SIZE.y) - 1,
        );
    }

    /// Clears the row at the given index.
    pub fn clear_row(&mut self, row: usize) {
        self.fill_row(row, b' ', ColorCode::default());
    }
    /// Fills the entire character buffer with the given character and color.
    pub fn fill(&mut self, c: u8, color_code: ColorCode) {
        for row in 0..self.char_buff_size.x {
            for col in 0..self.char_buff_size.y {
                self.char_buffer[row][col] = ScreenChar::new(c, color_code);
            }
        }
    }
    /// Fills the given row with the given character and color.
    pub fn fill_row(&mut self, row: usize, c: u8, color_code: ColorCode) {
        for col in 0..self.char_buff_size.x {
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
