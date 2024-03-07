use core::fmt::{self, Write};

use conquer_once::spin::OnceCell;
use spin::Mutex;

use crate::{dbg, lock_once, serial_println, prelude::*};

use super::{buffer, char_writer::CHAR_WRITER, color_code::ColorCode};
use super::screen_char::ScreenChar;


/// A simple terminal writer that writes to the VESA framebuffer.
pub struct TerminalWriter {
    col_pos: usize,
    row_pos: usize,
    /// The color code to write with.
    pub color_code: ColorCode,
}

impl TerminalWriter {
    /// Creates a new terminal writer.
    pub(super) fn new() -> TerminalWriter {
        TerminalWriter {
            col_pos: 0,
            row_pos: 1,
            color_code: ColorCode::default(),
        }
    }
    /// Shifts the buffer up by one row.
    pub fn shift_up(&mut self) {
        let mut buf = lock_once!(CHAR_WRITER);
        let buf_width = buf.char_buff_size.x;
        let buf_height = buf.char_buff_size.y;
        for row in 1..buf_height {
            for col in 0..buf_width {
                let c = buf.char_buffer[row][col];
                buf.char_buffer[row - 1][col] = c;
            }
            buf.flush_row(row - 1);
        }
        buf.clear_row(buf_height - 1);
        buf.flush_row(buf_height - 1);
    }
    /// Moves the cursor to the next line.
    fn new_line(&mut self) {
        let buf_height = lock_once!(CHAR_WRITER).char_buff_size.y;
        self.col_pos = 0;
        self.row_pos += 1;
        if self.row_pos >= buf_height {
            self.shift_up();
            self.row_pos = buf_height - 1;
        }
    }

    pub fn write_byte(&mut self, byte: u8) {
        let mut buf = lock_once!(CHAR_WRITER);
        let buf_width = buf.char_buff_size.x;
        match byte {
            b'\n' => {
                drop(buf); // drop the lock so we can call new_line. otherwise we get a deadlock
                self.new_line()
            }
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
        lock_once!(CHAR_WRITER).char_buffer[row][col] = ScreenChar::new(byte, color_code);
    }

    pub fn write_string_at(&mut self, s: &str, row: usize, col: usize, wrap: bool) {
        let buf = lock_once!(CHAR_WRITER);
        let buf_width = buf.char_buff_size.y;
        if wrap && s.len() > buf_width - col {
            let (first, second) = s.split_at(buf_width - col);
            self.write_string_at(first, row, col, false);
            self.write_string_at(second, row + 1, 0, true);
        } else {
            for (i, byte) in s.bytes().enumerate() {
                self.write_byte_at(byte, row, col + i);
            }
        }
        lock_once!(CHAR_WRITER).flush_char_buf();
    }

    pub fn clear(&mut self) {
        let mut buf = lock_once!(buffer::BUFFER);
        buf.clear();
        lock_once!(CHAR_WRITER).flush_char_buf();
    }

    pub fn fill(&mut self, c: u8) {
        let mut buf = lock_once!(CHAR_WRITER);
        buf.fill(c, self.color_code);
        buf.flush_char_buf();
    }

    pub fn reset(&mut self) {
        self.col_pos = 0;
        self.row_pos = 0;
        self.color_code = ColorCode::default();
        lock_once!(buffer::BUFFER).clear();
    }

    pub fn backspace(&mut self) {
        let buf_width = lock_once!(CHAR_WRITER).char_buff_size.x;
        if self.col_pos > 0 {
            self.col_pos -= 1;
            self.write_byte(0);
            self.col_pos -= 1;
        } else if self.row_pos > 0 {
            self.row_pos -= 1;
            let buf = lock_once!(CHAR_WRITER);
            for col in (0..buf_width).rev() {
                trace!("at col {}", col);
                // Go to the last non-empty character in the row.
                if buf.char_buffer[col][self.row_pos].ascii_character != 0 {
                    trace!("found non-empty char at col {}", col);
                    self.col_pos = col;
                    break;
                }
            }
        }
    }

    pub fn set_pos(&mut self, row: usize, col: usize) {
        self.row_pos = row;
        self.col_pos = col;
    }
}

impl Write for TerminalWriter {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.write_string(s);
        Ok(())
    }
}

pub static WRITER: OnceCell<Mutex<TerminalWriter>> = OnceCell::uninit();

pub fn init_vga() {
    serial_println!("Initializing VGA driver!");
    dbg!("Initializing writer container!");
    WRITER
        .try_init_once(move || {
            let writer = TerminalWriter::new();
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
        lock_once!(WRITER).write_fmt(args).unwrap();
    });
}

#[doc(hidden)]
pub fn _eprint(args: fmt::Arguments) {
    use x86_64::instructions::interrupts;
    interrupts::without_interrupts(|| {
        crate::serial::_print(format_args!("ERROR: {} ", args));
        let mut writer = lock_once!(WRITER);
        let prev = writer.color_code;
        writer.color_code = ColorCode::new_with_bg((255, 0, 0), (255, 255, 255));
        writer.write_fmt(args).unwrap();
        writer.color_code = prev;
    });
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::display::terminal::_print(format_args!($($arg)*)));
}

#[macro_export]
macro_rules! println {
    () => ($crate::vga_driver::print!("\n"));
    ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
}

#[macro_export]
macro_rules! eprint {
    ($($arg:tt)*) => ($crate::display::terminal::_eprint(format_args!($($arg)*)));
}

#[macro_export]
macro_rules! eprintln {
    () => ($crate::vga_driver::eprint!("\n"));
    ($($arg:tt)*) => ($crate::eprint!("{}\n", format_args!($($arg)*)));
}
