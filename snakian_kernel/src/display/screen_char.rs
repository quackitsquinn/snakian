use super::ColorCode;


/// A struct to represent a single character on the screen.
/// Contains the ascii character and the color code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct ScreenChar {
    /// The ascii character to display.
    pub ascii_character: u8,
    /// The color code to display the character with.
    pub color_code: ColorCode,
}

impl ScreenChar {
    /// Creates a new ScreenChar with the given ascii character and default color code.
    pub fn from_u8(c: u8) -> ScreenChar {
        ScreenChar {
            ascii_character: c as u8,
            color_code: ColorCode::default(),
        }
    }
    /// Creates a new ScreenChar with the given ascii character and color code.
    pub fn new(c: u8, color_code: ColorCode) -> ScreenChar {
        ScreenChar {
            ascii_character: c as u8,
            color_code: color_code,
        }
    }
    
    /// Creates an empty ScreenChar with the default color code.
    pub fn none() -> ScreenChar {
        ScreenChar {
            ascii_character: 0,
            color_code: ColorCode::default(),
        }
    }
}
