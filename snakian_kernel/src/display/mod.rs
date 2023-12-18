pub mod chars;
pub mod vga_driver;
pub mod color_code;

pub(super) type ColorTuple = (u8, u8, u8);


pub use crate::display::vga_driver::init_vga as init;
pub use crate::display::color_code::ColorCode;