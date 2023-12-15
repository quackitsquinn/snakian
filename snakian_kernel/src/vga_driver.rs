use core::{fmt::{Write, self}, mem};

use bootloader_api::{info::{FrameBufferInfo, FrameBuffer}, config, BootInfo};
use conquer_once::spin::OnceCell;
use volatile::Volatile;
use lazy_static::lazy_static;
use spin::Mutex;

use crate::{interrupts, dbg, serial_print, serial_println};


#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Color {
    Black = 0,
    Blue = 1,
    Green = 2,
    Cyan = 3,
    Red = 4,
    Magenta = 5,
    Brown = 6,
    LightGray = 7,
    DarkGray = 8,
    LightBlue = 9,
    LightGreen = 10,
    LightCyan = 11,
    LightRed = 12,
    Pink = 13,
    Yellow = 14,
    White = 15,
}


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct ColorCode(u8);

impl ColorCode {
    pub fn new(foreground: Color, background: Color, blink: bool) -> ColorCode {
        // 4 bits for foreground color, 4 bits for background color
        ColorCode((background as u8) << 4 | (foreground as u8) | (blink as u8) << 7)
    }
}

impl Default for ColorCode {
    fn default() -> Self {
        ColorCode::new(Color::White, Color::Black, false)
    }

}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
struct ScreenChar {
    ascii_character: u8,
    color_code: ColorCode,
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

pub type RGB = (u8, u8, u8);
pub type CharSprite = [RGB; 8 * 8]; // will be added later, but this skeleton is here for now.

const MAX_BUFF_SIZE: usize = 256;
// also chars will be taken from https://github.com/dhepper/font8x8/tree/master


struct Buffer<'a> {
    display: &'a [RGB],
    config: FrameBufferInfo,
    char_scale: usize, // this will be used to scale the characters to the screen size. (variable font size)
    char_buff_size: (usize, usize),
    char_buffer: [[ScreenChar; MAX_BUFF_SIZE]; MAX_BUFF_SIZE],
}

impl<'a> Buffer<'a> {
    pub fn new(buf: &FrameBuffer) -> Buffer<'a> {
        let config = buf.info();

        let flat = buf.buffer();
        // SAFETY: the buffer is a slice of RGB tuples, which are the size of u8 * 3, so it is safe to transmute from a slice of u8s to a slice of RGBs
        let display = unsafe { mem::transmute::<&[u8], &[RGB]>(flat) };

        let char_buf_size = (config.width as usize / 8, config.width as usize / 8);

        Buffer {
            display: display,
            config,
            char_scale: 1,
            char_buff_size: char_buf_size,
            char_buffer: [[ScreenChar::none(); MAX_BUFF_SIZE]; MAX_BUFF_SIZE],
        }
    }
    pub fn clear(&mut self) {
        self.fill(b' ', ColorCode::default());
    }

    pub fn clear_row(&mut self, row: usize) {
        self.fill_row(row, b' ', ColorCode::default());
    }

    pub fn fill(&mut self, c: u8, color_code: ColorCode) {
        for row in 0..self.char_buff_size.1 {
            for col in 0..self.char_buff_size.0 {
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

pub struct Writer<'a> {
    col_pos: usize,
    row_pos: usize,
    pub color_code: ColorCode,
    buffer: Buffer<'a>
}

impl<'a> Writer<'a> {
    pub fn new(config: &mut FrameBuffer) -> Writer<'a> {
        let buf = Buffer::new(&config);
        Writer {
            col_pos: 0,
            row_pos: 0,
            color_code: ColorCode::new(Color::White, Color::Black, false),
            buffer: buf,
        }
    }

    fn shift_up(&mut self) {
        let buf_height = self.buffer.char_buff_size.1;
        let buf_width = self.buffer.char_buff_size.0;
        for row in 1..buf_height {
            for col in 0..buf_width {
                let c = self.buffer.char_buffer[row][col];
                self.buffer.char_buffer[row - 1][col] = c;
            }
        }
        self.buffer.clear_row(buf_height - 1);
    }

    fn new_line(&mut self) {
        let buf_height = self.buffer.char_buff_size.1;
        self.col_pos = 0;
        self.row_pos += 1;
        if self.row_pos >= buf_height {
            self.shift_up();
            self.row_pos = buf_height - 1;
        }
    }

    pub fn write_byte(&mut self, byte: u8) {
        let buf_width = self.buffer.char_buff_size.0;
        match byte {
            b'\n' => self.new_line(),
            byte => {
                if self.col_pos >= buf_width {
                    self.new_line();
                }
                let row = self.row_pos;
                let col = self.col_pos;
                let color_code = self.color_code;
                self.buffer.char_buffer[row][col] = ScreenChar::new(byte, color_code);
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
        assert!(row < MAX_BUFF_SIZE);
        assert!(col < MAX_BUFF_SIZE);
        let color_code = self.color_code;
        self.buffer.char_buffer[row][col] = ScreenChar::new(byte, color_code);
    }

    pub fn write_string_at(&mut self, s: &str, row: usize, col: usize, wrap: bool) {
        assert!(row < MAX_BUFF_SIZE);
        assert!(col < MAX_BUFF_SIZE);
        let buf_width = self.buffer.char_buff_size.0;
        if wrap && s.len() > buf_width - col {
            let (first, second) = s.split_at(buf_width - col);
            self.write_string_at(first, row, col, false);
            self.write_string_at(second, row + 1, 0, true);
        } else {
            for (i, byte) in s.bytes().enumerate() {
                self.write_byte_at(byte, row, col + i);
            }
        }
    }

    pub fn clear(&mut self) {
        self.buffer.clear();
    }

    pub fn fill(&mut self, c: u8) {
        self.buffer.fill(c, self.color_code);
    }

    pub fn reset(&mut self) {
        self.col_pos = 0;
        self.row_pos = 0;
        self.color_code = ColorCode::default();
        self.buffer.clear();
    }

    pub fn backspace(&mut self) {
        let buf_width = self.buffer.char_buff_size.0;
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

impl<'a> Write for Writer<'a> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.write_string(s);
        Ok(())
    }
}


pub static WRITER: OnceCell<Mutex<Writer>> = OnceCell::uninit();

pub fn init(config: &mut FrameBuffer) {
    serial_println!("Initializing VGA driver!");
    let writer = Writer::new(config);
    dbg!("Made writer, initializing writer container!");
    WRITER.try_init_once(|| Mutex::new(writer)).expect("WRITER already initialized");
}

#[doc(hidden)]
pub fn _print(args: fmt::Arguments) {
    use x86_64::instructions::interrupts;
    interrupts::without_interrupts(|| {
        crate::serial::_print(args);
        // TODO: reimpliment this with the pixel based frame buffer.
        //WRITER.lock().write_fmt(args).unwrap();
    });
}

#[doc(hidden)]
pub fn _eprint(args: fmt::Arguments) {
    use x86_64::instructions::interrupts;
    interrupts::without_interrupts(|| {
        crate::serial::_print(format_args!("ERROR: {} ", args));
        /* 
        let mut writer = WRITER.lock();
        let orig = writer.color_code;
        writer.color_code = ColorCode::new(Color::Yellow, Color::Black, false);
        writer.write_fmt(args).unwrap();
        writer.color_code = orig;
        */ // TODO: reimpliment this with the pixel based frame buffer. while front-facing api is still the same, the backend has to be completely re-written.
});
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::vga_driver::_print(format_args!($($arg)*)));
}

#[macro_export]
macro_rules! println {
    () => ($crate::vga_driver::print!("\n"));
    ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
}

#[macro_export]
macro_rules! eprint {
    ($($arg:tt)*) => ($crate::vga_driver::_eprint(format_args!($($arg)*)));
}

#[macro_export]
macro_rules! eprintln {
    () => ($crate::vga_driver::eprint!("\n"));
    ($($arg:tt)*) => ($crate::eprint!("{}\n", format_args!($($arg)*)));
}
