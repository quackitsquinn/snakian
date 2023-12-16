use core::{
    cmp::min,
    fmt::{self, Write},
    mem,
};

use bootloader_api::{
    config,
    info::{self, FrameBuffer, FrameBufferInfo},
    BootInfo,
};
use conquer_once::spin::OnceCell;
use lazy_static::lazy_static;
use spin::Mutex;
use volatile::Volatile;

use crate::{chars, dbg, interrupts, serial_print, serial_println};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ColorCode(u8, u8, u8);

impl ColorCode {
    pub fn new(r: u8, g: u8, b: u8) -> ColorCode {
        // 4 bits for foreground color, 4 bits for background color
        ColorCode(r, g, b)
    }
}

impl Default for ColorCode {
    fn default() -> Self {
        ColorCode::new(255, 255, 255)
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
pub type CharSprite = [bool; 8 * 8]; // will be added later, but this skeleton is here for now.
                                     // FIXME: soo i kinda didnt keep track of what stuff is x and what stuff is y, so half the stuff is flipped and in general its a mess.
                                     // so fix it.
const MAX_BUFF_SIZE: (usize, usize) = (32, 32);
// also chars will be taken from https://github.com/dhepper/font8x8/tree/master

pub struct Buffer<'a> {
    display: &'a mut [RGB],
    buf: FrameBuffer,
    config: FrameBufferInfo,
    char_scale: usize, // this will be used to scale the characters to the screen size. (variable font size)
    char_buff_size: (usize, usize),
    char_buffer: [[ScreenChar; MAX_BUFF_SIZE.1]; MAX_BUFF_SIZE.0],
}

impl<'a> Buffer<'a> {
    pub fn new(buf: FrameBuffer) -> Buffer<'a> {
        let mut buf = buf;
        let config = buf.info();

        let flat = buf.buffer_mut();

        let display = unsafe {
            core::slice::from_raw_parts_mut(
                flat.as_ptr() as *mut RGB,
                flat.len() / mem::size_of::<RGB>(),
            )
        };

        let char_buf_size = (
            min(config.width as usize / 8, MAX_BUFF_SIZE.0) - 1,
            min(config.width as usize / 8, MAX_BUFF_SIZE.1 - 1) - 1,
        );

        Buffer {
            display,
            buf: buf,
            config,
            char_scale: 1,
            char_buff_size: char_buf_size,
            char_buffer: [[ScreenChar::none(); MAX_BUFF_SIZE.1]; MAX_BUFF_SIZE.0],
        }
    }
    pub fn clear(&mut self) {
        for rgb in self.display.iter_mut() {
            *rgb = (0, 0, 0);
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
                dbg!("Writing to ({}, {})", row, col);
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
                        (color_code.0, color_code.1, color_code.2);
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
        assert!(row < self.config.height - 8);
        assert!(col < self.config.width - 8);
        for y in 0..8 {
            for x in 0..8 {
                let c = buf[y * 8 + x];
                if c {
                    let scrx = col + x * scale as usize;
                    let scry = row + y * scale as usize;
                    for i in 0..scale {
                        for j in 0..scale {
                            self.display[self.xy_to_index(scrx + i as usize, scry + j as usize)] =
                                (color_code.0, color_code.1, color_code.2);
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

    pub fn set_scale(&mut self, scale: usize) {
        self.char_scale = scale;
        self.char_buff_size = (
            min(self.config.width as usize / (8 * scale), MAX_BUFF_SIZE.0),
            min(self.config.height as usize / (8 * scale), MAX_BUFF_SIZE.1),
        );
    }
}

fn clone_framebuf(buf: &FrameBuffer) -> FrameBuffer {
    let mut ptrptr = buf as *const FrameBuffer as *const u64;
    // SAFETY: this is safe because the FrameBuffer struct is repr(C)
    // and the first field is a u64, which is the address of the framebuffer
    let addr = unsafe { *ptrptr };
    // clones the framebuffer info
    let info = buf.info();
    unsafe { FrameBuffer::new(addr, info) }
}
pub struct Writer<'a> {
    col_pos: usize,
    row_pos: usize,
    pub color_code: ColorCode,
    pub buffer: Buffer<'a>,
}

impl<'a> Writer<'a> {
    pub fn new(config: FrameBuffer) -> Writer<'a> {
        Writer {
            col_pos: 0,
            row_pos: 5,
            color_code: ColorCode::default(),
            buffer: Buffer::new(config),
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
        self.buffer.flush_char_buf();
    }

    fn new_line(&mut self) {
        let buf_height = self.buffer.char_buff_size.1;
        self.col_pos = 0;
        self.row_pos += 1;
        if self.row_pos >= buf_height {
            self.shift_up();
            self.row_pos = buf_height - 1;
        }
        self.buffer.flush_char_buf();
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
        self.buffer.flush_char_buf();
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
        assert!(row < MAX_BUFF_SIZE.1);
        assert!(col < MAX_BUFF_SIZE.0);
        let color_code = self.color_code;
        self.buffer.char_buffer[row][col] = ScreenChar::new(byte, color_code);
    }

    pub fn write_string_at(&mut self, s: &str, row: usize, col: usize, wrap: bool) {
        assert!(row < MAX_BUFF_SIZE.1);
        assert!(col < MAX_BUFF_SIZE.0);
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
        self.buffer.flush_char_buf();
    }

    pub fn clear(&mut self) {
        self.buffer.clear();
        self.buffer.flush_char_buf();
    }

    pub fn fill(&mut self, c: u8) {
        self.buffer.fill(c, self.color_code);
        self.buffer.flush_char_buf();
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
