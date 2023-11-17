use core::fmt::{Write, self};

use volatile::Volatile;
use lazy_static::lazy_static;
use spin::Mutex;

use crate::interrupts;


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
}

const BUFFER_HEIGHT: usize = 25;
const BUFFER_WIDTH: usize = 80;

#[repr(transparent)]
struct Buffer {
    /// The array of characters that make up the buffer. height x width array (annoying)
    chars: [[Volatile<ScreenChar>; BUFFER_WIDTH]; BUFFER_HEIGHT],
}

impl Buffer {
    pub fn clear(&mut self) {
        self.fill(b' ', ColorCode::default());
    }

    pub fn clear_row(&mut self, row: usize) {
        assert!(row < BUFFER_HEIGHT);
        self.fill_row(row, b' ', ColorCode::default());
    }

    pub fn fill(&mut self, c: u8, color_code: ColorCode) {
        for row in 0..BUFFER_HEIGHT {
            for col in 0..BUFFER_WIDTH {
                self.chars[row][col] = Volatile::new(ScreenChar::new(c, color_code));
            }
        }
    }

    pub fn fill_row(&mut self, row: usize, c: u8, color_code: ColorCode) {
        assert!(row < BUFFER_HEIGHT);
        for col in 0..BUFFER_WIDTH {
            self.chars[row][col] = Volatile::new(ScreenChar::new(c, color_code));
        }
    }
}

pub struct Writer {
    col_pos: usize,
    row_pos: usize,
    pub color_code: ColorCode,
    buffer: &'static mut Buffer,
}

impl Writer {
    pub fn new() -> Writer {
        Writer {
            col_pos: 0,
            row_pos: 0,
            color_code: ColorCode::new(Color::White, Color::Black, false),
            buffer: unsafe { &mut *(0xb8000 as *mut Buffer) },
        }
    }
    fn shift_up(&mut self) {
        for row in 1..BUFFER_HEIGHT {
            for col in 0..BUFFER_WIDTH {
                let c = self.buffer.chars[row][col].read();
                self.buffer.chars[row - 1][col].write(c);
            }
        }
        self.buffer.clear_row(BUFFER_HEIGHT - 1);
    }
    fn new_line(&mut self) {
        self.col_pos = 0;
        self.row_pos += 1;
        if self.row_pos >= BUFFER_HEIGHT {
            self.shift_up();
            self.row_pos = BUFFER_HEIGHT - 1;
        }
    }

    pub fn write_byte(&mut self, byte: u8) {
        match byte {
            b'\n' => self.new_line(),
            byte => {
                if self.col_pos >= BUFFER_WIDTH {
                    self.new_line();
                }
                let row = self.row_pos;
                let col = self.col_pos;
                let color_code = self.color_code;
                self.buffer.chars[row][col].write(ScreenChar::new(byte, color_code));
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
        assert!(row < BUFFER_HEIGHT);
        assert!(col < BUFFER_WIDTH);
        let color_code = self.color_code;
        self.buffer.chars[row][col].write(ScreenChar::new(byte, color_code));
    }

    pub fn write_string_at(&mut self, s: &str, row: usize, col: usize, wrap: bool) {
        assert!(row < BUFFER_HEIGHT);
        assert!(col < BUFFER_WIDTH);
        if wrap && s.len() > BUFFER_WIDTH - col {
            let (first, second) = s.split_at(BUFFER_WIDTH - col);
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
}

impl Write for Writer {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.write_string(s);
        Ok(())
    }
}


lazy_static! {
    pub static ref WRITER: Mutex<Writer> = Mutex::new(Writer::new());
}

#[doc(hidden)]
pub fn _print(args: fmt::Arguments) {
    use x86_64::instructions::interrupts;
    interrupts::without_interrupts(|| {
        WRITER.lock().write_fmt(args).unwrap();
    });
}

#[doc(hidden)]
pub fn _eprint(args: fmt::Arguments) {
    use x86_64::instructions::interrupts;
    interrupts::without_interrupts(|| {
        let mut writer = WRITER.lock();
        let orig = writer.color_code;
        writer.color_code = ColorCode::new(Color::Yellow, Color::Black, false);
        writer.write_fmt(args).unwrap();
        writer.color_code = orig;
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


mod tests {
    use crate::vga_driver::{WRITER, BUFFER_HEIGHT};

    #[test_case]
    fn test_print() {
        print!("test");
        assert_eq!(WRITER.lock().col_pos, 4);
        assert_eq!(WRITER.lock().row_pos, 0);
        assert_eq!(WRITER.lock().buffer.chars[0][0].read().ascii_character, b't');
    }
}