use core::{
    cmp::min,
    fmt::{self, Write},
    mem,
};

use bootloader_api::{
    config,
    info::{self, FrameBuffer, FrameBufferInfo, PixelFormat},
    BootInfo,
};
use conquer_once::spin::OnceCell;
use lazy_static::lazy_static;
use spin::Mutex;
use volatile::Volatile;

use crate::{display::chars, dbg, interrupts, serial_print, serial_println};

use super::{color_code::ColorCode, ColorTuple, buffer::{Buffer, self}, clone_framebuf};


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub(super) struct ScreenChar {
    pub ascii_character: u8,
    pub color_code: ColorCode,
}

impl ScreenChar {
    pub fn from_u8(c: u8) -> ScreenChar {
        ScreenChar {
            ascii_character: c as u8,
            color_code: ColorCode::default(),
        }
    }

    pub fn new(c: u8, color_code: ColorCode) -> ScreenChar {
        ScreenChar {
            ascii_character: c as u8,
            color_code: color_code,
        }
    }

    pub fn none() -> ScreenChar {
        ScreenChar {
            ascii_character: 0,
            color_code: ColorCode::default(),
        }
    }
}

pub type CharSprite = [bool; 8 * 8];

// also chars will be taken from https://github.com/dhepper/font8x8/tree/master

pub struct Writer{
    col_pos: usize,
    row_pos: usize,
    pub color_code: ColorCode,
}

impl Writer {
    pub fn new(config: FrameBuffer) -> Writer {
        Writer {
            col_pos: 0,
            row_pos: 5,
            color_code: ColorCode::default(),
        }
    }

    fn shift_up(&mut self) {
        let mut buf = buffer::BUFFER.get().unwrap().lock();
        let buf_height = buf.char_buff_size.1;
        let buf_width = buf.char_buff_size.0;
        for row in 1..buf_height {
            for col in 0..buf_width {
                let c = buf.char_buffer[row][col];
                buf.char_buffer[row - 1][col] = c;
            }
        }
        buf.clear_row(buf_height - 1);
        buf.flush_char_buf();
    }

    fn new_line(&mut self) {
        let buf_height = buffer::BUFFER.get().unwrap().lock().char_buff_size.1;
        self.col_pos = 0;
        self.row_pos += 1;
        if self.row_pos >= buf_height {
            self.shift_up();
            self.row_pos = buf_height - 1;
        }
    }

    pub fn write_byte(&mut self, byte: u8) {
        let mut buf = buffer::BUFFER.get().unwrap().lock();
        let buf_width = buf.char_buff_size.0;
        match byte {
            b'\n' => self.new_line(),
            byte => {
                if self.col_pos >= buf_width {
                    self.new_line();
                }
                let row = self.row_pos;
                let col = self.col_pos;
                let color_code = self.color_code;
                buf.char_buffer[row][col] = ScreenChar::new(byte, color_code);
                buf.flush_char_at(self.row_pos, self.col_pos);
                self.col_pos += 1;
            }
        }
    }

    pub fn write_string(&mut self, s: &str) {
        for byte in s.bytes() {
            match byte {
                // printable ASCII byte or newline
                0x20..=0x7e | b'\n' => self.write_byte(byte),
                // not part of printable ASCII range
                _ => self.write_byte(0xfe),
            }
        }
    }

    pub fn write_byte_at(&mut self, byte: u8, row: usize, col: usize) {
        let color_code = self.color_code;
        buffer::BUFFER.get().unwrap().lock().char_buffer[row][col] = ScreenChar::new(byte, color_code);
    }

    pub fn write_string_at(&mut self, s: &str, row: usize, col: usize, wrap: bool) {
        let mut buf = buffer::BUFFER.get().unwrap().lock();
        let buf_width = buf.char_buff_size.0;
        if wrap && s.len() > buf_width - col {
            let (first, second) = s.split_at(buf_width - col);
            self.write_string_at(first, row, col, false);
            self.write_string_at(second, row + 1, 0, true);
        } else {
            for (i, byte) in s.bytes().enumerate() {
                self.write_byte_at(byte, row, col + i);
            }
        }
        buf.flush_char_buf();
    }

    pub fn clear(&mut self) {
        let mut buf = buffer::BUFFER.get().unwrap().lock();
        buf.clear();
        buf.flush_char_buf();
    }

    pub fn fill(&mut self, c: u8) {
        let mut buf = buffer::BUFFER.get().unwrap().lock();
        buf.fill(c, self.color_code);
        buf.flush_char_buf();
    }

    pub fn reset(&mut self) {
        self.col_pos = 0;
        self.row_pos = 0;
        self.color_code = ColorCode::default();
        buffer::BUFFER.get().unwrap().lock().clear();
    }

    pub fn backspace(&mut self) {
        let buf_width = buffer::BUFFER.get().unwrap().lock().char_buff_size.0;
        if self.col_pos > 0 {
            self.col_pos -= 1;
            self.write_byte(b' ');
            self.col_pos -= 1;
        } else if self.row_pos > 0 {
            self.col_pos = buf_width - 1;
            self.row_pos -= 1;
            self.write_byte(b' ');
            self.col_pos = buf_width - 1;
            self.row_pos -= 1;
        }
    }
}

impl Write for Writer {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.write_string(s);
        Ok(())
    }
}

pub static WRITER: OnceCell<Mutex<Writer>> = OnceCell::uninit();

pub fn init_vga(config: &mut info::FrameBuffer) {
    serial_println!("Initializing VGA driver!");
    dbg!("Initializing writer container!");
    WRITER
        .try_init_once(move || {
            let writer = Writer::new(clone_framebuf(config));
            dbg!("Initialized writer! Moving to Mutex!");
            Mutex::new(writer)
        })
        .expect("WRITER already initialized");
    dbg!("Initialized writer container!");
}

#[doc(hidden)]
pub fn _print(args: fmt::Arguments) {
    use x86_64::instructions::interrupts;
    interrupts::without_interrupts(|| {
        crate::serial::_print(args);
        WRITER.get().unwrap().lock().write_fmt(args).unwrap();
    });
}

#[doc(hidden)]
pub fn _eprint(args: fmt::Arguments) {
    use x86_64::instructions::interrupts;
    interrupts::without_interrupts(|| {
        crate::serial::_print(format_args!("ERROR: {} ", args));
        let mut writer = WRITER.get().unwrap().lock();
        let prev = writer.color_code;
        writer.color_code = ColorCode::new_with_bg((255, 0, 0), (255,255,255));
        writer.write_fmt(args).unwrap();
        writer.color_code = prev;
    });
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::display::vga_driver::_print(format_args!($($arg)*)));
}

#[macro_export]
macro_rules! println {
    () => ($crate::vga_driver::print!("\n"));
    ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
}

#[macro_export]
macro_rules! eprint {
    ($($arg:tt)*) => ($crate::display::vga_driver::_eprint(format_args!($($arg)*)));
}

#[macro_export]
macro_rules! eprintln {
    () => ($crate::vga_driver::eprint!("\n"));
    ($($arg:tt)*) => ($crate::eprint!("{}\n", format_args!($($arg)*)));
}
