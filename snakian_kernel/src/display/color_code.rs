use bootloader_api::info::PixelFormat;

use super::ColorTuple;

/// Converts a RGB Color Tuple to the specified PixelFormat.
/// FIXME: U8 is currently not supported because the screen buffer is assumed to be 24-bit.
fn conv_rgb_tuple(rgb: ColorTuple, format: PixelFormat) -> ColorTuple {
    match format {
        PixelFormat::Rgb => rgb,
        PixelFormat::Bgr => (rgb.2,rgb.1, rgb.0),
        PixelFormat::U8 => panic!("U8 pixel format is not supported!"),
        PixelFormat::Unknown { red_position, green_position, blue_position } => {
            let mut buf = [0u8; 3];
            buf[red_position as usize] = rgb.0;
            buf[green_position as usize] = rgb.1;
            buf[blue_position as usize] = rgb.2;
            (buf[0], buf[1], buf[2])
        }
        _ => unreachable!()
    }
}

/// A color code for the display driver. Intended to be used with the VGA writer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ColorCode {
    /// The color of the character
    pub char_color : ColorTuple,
    /// The background color of the character
    pub bg_color : Option<ColorTuple>,
    /// Whether or not the character has a background color
    /// TODO: remove this field and use bg_color.is_some() instead
    pub has_bg: bool,

}

impl ColorCode {
    /// Creates a new ColorCode with the input RGB values as the character color.
    pub fn new(r: u8, g: u8, b: u8) -> ColorCode {
        ColorCode {
            char_color: (r, g, b),
            has_bg: false,
            bg_color: None,
        }
    }
    /// Creates a new ColorCode with the input RGB values as the character color and the input RGB values as the background color.
    pub fn new_with_bg(for_char: ColorTuple, bg: ColorTuple) -> ColorCode {
        ColorCode {
            char_color: for_char,
            has_bg: true,
            bg_color: Some(bg),
        }
    }
    /// Returns the foreground ColorTuple in the given format.
    pub fn to_format(&self, format: PixelFormat) -> ColorTuple {
        conv_rgb_tuple(self.char_color, format)
    }
    /// Returns the background ColorTuple in the given format. If there is no background color, returns None.
    pub fn format_bg(&self, format: PixelFormat) -> Option<ColorTuple> {
        if self.bg_color.is_some() {
            Some(conv_rgb_tuple(self.bg_color.unwrap(), format))
        } else {
            None
        }
    }
}

impl Default for ColorCode {
    fn default() -> Self {
        ColorCode::new_with_bg((255, 255, 255), (0, 0, 0))
    }
}
