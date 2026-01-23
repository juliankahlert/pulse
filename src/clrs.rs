//! clrs.cc inspired color palette for the terminal
//! A nicer color palette for the command line
//!
//! Based on https://clrs.cc/
//!
//! ## Colors
//!
//! | Name    | Hex     | RGB            |
//! |---------|---------|----------------|
//! | Navy    | #001f3f | (0, 31, 63)    |
//! | Blue    | #0074D9 | (0, 116, 217)  |
//! | Aqua    | #7FDBFF | (127, 219, 255)|
//! | Teal    | #39CCCC | (57, 204, 204) |
//! | Olive   | #3D9970 | (61, 153, 112) |
//! | Green   | #2ECC40 | (46, 204, 64)  |
//! | Lime    | #01FF70 | (1, 255, 112)  |
//! | Yellow  | #FFDC00 | (255, 220, 0)  |
//! | Orange  | #FF851B | (255, 133, 27) |
//! | Red     | #FF4136 | (255, 65, 54)  |
//! | Maroon  | #85144b | (133, 20, 75)  |
//! | Fuchsia | #F012BE | (240, 18, 190) |
//! | Purple  | #B10DC9 | (177, 13, 201) |
//! | Black   | #111111 | (17, 17, 17)   |
//! | Gray    | #AAAAAA | (170, 170, 170)|
//! | Silver  | #DDDDDD | (221, 221, 221)|
//! | White   | #FFFFFF | (255, 255, 255)|
//!
//! ## Usage
//!
//! ```rust
//! use l::clrs::{Clrs, Color};
//!
//! let navy = Clrs::navy();
//! println!("{}", "Hello Navy!".color(navy));
//!
//! let custom = Clrs::rgb(255, 0, 0);
//! println!("{}", "Red!".color(custom));
//! ```

use owo_colors::{DynColors, Rgb};
use std::fmt;
use std::os::unix::fs::FileTypeExt;
use std::str::FromStr;

/// Color palette for the application
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Clrs {
    Navy,
    Blue,
    Aqua,
    Teal,
    Olive,
    Green,
    Lime,
    Yellow,
    Orange,
    Red,
    Maroon,
    Fuchsia,
    Purple,
    Black,
    Gray,
    Silver,
    White,
    Magenta,
}

impl Clrs {
    /// Convert to owo-colors DynColors
    pub fn to_dyn(self) -> DynColors {
        self.into()
    }

    /// Get the RGB values
    pub fn rgb_values(self) -> Rgb {
        self.into()
    }

    /// Navy #001f3f
    pub fn navy() -> Self {
        Clrs::Navy
    }

    /// Blue #0074D9
    pub fn blue() -> Self {
        Clrs::Blue
    }

    /// Aqua #7FDBFF
    pub fn aqua() -> Self {
        Clrs::Aqua
    }

    /// Teal #39CCCC
    pub fn teal() -> Self {
        Clrs::Teal
    }

    /// Olive #3D9970
    pub fn olive() -> Self {
        Clrs::Olive
    }

    /// Green #2ECC40
    pub fn green() -> Self {
        Clrs::Green
    }

    /// Lime #01FF70
    pub fn lime() -> Self {
        Clrs::Lime
    }

    /// Yellow #FFDC00
    pub fn yellow() -> Self {
        Clrs::Yellow
    }

    /// Orange #FF851B
    pub fn orange() -> Self {
        Clrs::Orange
    }

    /// Red #FF4136
    pub fn red() -> Self {
        Clrs::Red
    }

    /// Maroon #85144b
    pub fn maroon() -> Self {
        Clrs::Maroon
    }

    /// Fuchsia #F012BE
    pub fn fuchsia() -> Self {
        Clrs::Fuchsia
    }

    /// Purple #B10DC9
    pub fn purple() -> Self {
        Clrs::Purple
    }

    /// Black #111111
    pub fn black() -> Self {
        Clrs::Black
    }

    /// Gray #AAAAAA
    pub fn gray() -> Self {
        Clrs::Gray
    }

    /// Silver #DDDDDD
    pub fn silver() -> Self {
        Clrs::Silver
    }

    /// White #FFFFFF
    pub fn white() -> Self {
        Clrs::White
    }

    /// Magenta #FF00FF
    pub fn magenta() -> Self {
        Clrs::Magenta
    }

    /// Custom RGB color
    pub fn rgb(r: u8, g: u8, b: u8) -> DynColors {
        DynColors::Rgb(r, g, b)
    }

    /// Color for file types
    pub fn for_file_type(
        is_dir: bool,
        is_symlink: bool,
        is_executable: bool,
        path: &std::path::Path,
    ) -> Clrs {
        if is_symlink {
            Clrs::Aqua
        } else if is_dir {
            Clrs::Blue
        } else if Self::is_device_file(path) {
            Clrs::Magenta
        } else if is_executable {
            Clrs::Green
        } else {
            Clrs::Silver
        }
    }

    /// Color for permission warnings
    pub fn for_permission(is_write: bool, is_executable: bool) -> Clrs {
        if is_executable && is_write {
            Clrs::Red
        } else if is_write {
            Clrs::Orange
        } else {
            Clrs::Gray
        }
    }

    /// Color for file size
    pub fn for_size(size: u64) -> Clrs {
        if size > 10_000_000 {
            Clrs::Red
        } else if size > 1_000_000 {
            Clrs::Orange
        } else if size > 100_000 {
            Clrs::Yellow
        } else {
            Clrs::Green
        }
    }

    /// Color for device files (block/char devices)
    pub fn for_device_file(is_device: bool) -> Clrs {
        if is_device {
            Clrs::Magenta
        } else {
            Clrs::Silver
        }
    }

    /// Check if a path is a device file
    pub fn is_device_file(path: &std::path::Path) -> bool {
        if let Ok(metadata) = std::fs::metadata(path) {
            let file_type = metadata.file_type();
            file_type.is_block_device() || file_type.is_char_device()
        } else {
            false
        }
    }
}

impl From<Clrs> for DynColors {
    fn from(c: Clrs) -> Self {
        let rgb = c.rgb_values();
        DynColors::Rgb(rgb.0, rgb.1, rgb.2)
    }
}

impl FromStr for Clrs {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Navy" => Ok(Clrs::Navy),
            "Blue" => Ok(Clrs::Blue),
            "Aqua" => Ok(Clrs::Aqua),
            "Teal" => Ok(Clrs::Teal),
            "Olive" => Ok(Clrs::Olive),
            "Green" => Ok(Clrs::Green),
            "Lime" => Ok(Clrs::Lime),
            "Yellow" => Ok(Clrs::Yellow),
            "Orange" => Ok(Clrs::Orange),
            "Red" => Ok(Clrs::Red),
            "Maroon" => Ok(Clrs::Maroon),
            "Fuchsia" => Ok(Clrs::Fuchsia),
            "Purple" => Ok(Clrs::Purple),
            "Black" => Ok(Clrs::Black),
            "Gray" => Ok(Clrs::Gray),
            "Silver" => Ok(Clrs::Silver),
            "White" => Ok(Clrs::White),
            "Magenta" => Ok(Clrs::Magenta),
            _ => Err(format!("Unknown color: {}", s)),
        }
    }
}

impl From<Clrs> for Rgb {
    fn from(c: Clrs) -> Self {
        match c {
            Clrs::Navy => Rgb(0, 31, 63),
            Clrs::Blue => Rgb(0, 116, 217),
            Clrs::Aqua => Rgb(127, 219, 255),
            Clrs::Teal => Rgb(57, 204, 204),
            Clrs::Olive => Rgb(61, 153, 112),
            Clrs::Green => Rgb(46, 204, 64),
            Clrs::Lime => Rgb(1, 255, 112),
            Clrs::Yellow => Rgb(255, 220, 0),
            Clrs::Orange => Rgb(255, 133, 27),
            Clrs::Red => Rgb(255, 65, 54),
            Clrs::Maroon => Rgb(133, 20, 75),
            Clrs::Fuchsia => Rgb(240, 18, 190),
            Clrs::Purple => Rgb(177, 13, 201),
            Clrs::Black => Rgb(17, 17, 17),
            Clrs::Gray => Rgb(170, 170, 170),
            Clrs::Silver => Rgb(221, 221, 221),
            Clrs::White => Rgb(255, 255, 255),
            Clrs::Magenta => Rgb(255, 0, 255),
        }
    }
}

impl fmt::Display for Clrs {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let rgb = self.rgb_values();
        write!(f, "#{:02x}{:02x}{:02x}", rgb.0, rgb.1, rgb.2)
    }
}

/// Generate ANSI 16-color equivalents for terminals that don't support truecolor
impl From<Clrs> for owo_colors::AnsiColors {
    fn from(c: Clrs) -> Self {
        match c {
            Clrs::Navy => owo_colors::AnsiColors::Black,
            Clrs::Blue => owo_colors::AnsiColors::Blue,
            Clrs::Aqua => owo_colors::AnsiColors::Cyan,
            Clrs::Teal => owo_colors::AnsiColors::Cyan,
            Clrs::Olive => owo_colors::AnsiColors::Green,
            Clrs::Green => owo_colors::AnsiColors::Green,
            Clrs::Lime => owo_colors::AnsiColors::Green,
            Clrs::Yellow => owo_colors::AnsiColors::Yellow,
            Clrs::Orange => owo_colors::AnsiColors::Yellow,
            Clrs::Red => owo_colors::AnsiColors::Red,
            Clrs::Maroon => owo_colors::AnsiColors::Red,
            Clrs::Fuchsia => owo_colors::AnsiColors::Magenta,
            Clrs::Purple => owo_colors::AnsiColors::Magenta,
            Clrs::Black => owo_colors::AnsiColors::Black,
            Clrs::Gray => owo_colors::AnsiColors::White,
            Clrs::Silver => owo_colors::AnsiColors::White,
            Clrs::White => owo_colors::AnsiColors::White,
            Clrs::Magenta => owo_colors::AnsiColors::Magenta,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_colors_display() {
        assert_eq!(format!("{}", Clrs::Navy), "#001f3f");
        assert_eq!(format!("{}", Clrs::Blue), "#0074d9");
        assert_eq!(format!("{}", Clrs::Red), "#ff4136");
    }

    #[test]
    fn test_colors_rgb() {
        assert_eq!(Clrs::Navy.rgb_values(), Rgb(0, 31, 63));
        assert_eq!(Clrs::Blue.rgb_values(), Rgb(0, 116, 217));
        assert_eq!(Clrs::Red.rgb_values(), Rgb(255, 65, 54));
    }

    #[test]
    fn test_for_file_type() {
        let path = std::path::Path::new("dummy");
        assert_eq!(Clrs::for_file_type(true, false, false, path), Clrs::Blue);
        assert_eq!(Clrs::for_file_type(false, true, false, path), Clrs::Aqua);
        assert_eq!(Clrs::for_file_type(false, false, true, path), Clrs::Green);
        assert_eq!(Clrs::for_file_type(false, false, false, path), Clrs::Silver);
    }

    #[test]
    fn test_for_permission() {
        assert_eq!(Clrs::for_permission(true, true), Clrs::Red);
        assert_eq!(Clrs::for_permission(true, false), Clrs::Orange);
        assert_eq!(Clrs::for_permission(false, false), Clrs::Gray);
    }

    #[test]
    fn test_for_size() {
        assert_eq!(Clrs::for_size(15_000_000), Clrs::Red);
        assert_eq!(Clrs::for_size(5_000_000), Clrs::Orange);
        assert_eq!(Clrs::for_size(500_000), Clrs::Yellow);
        assert_eq!(Clrs::for_size(50_000), Clrs::Green);
    }

    #[test]
    fn test_custom_rgb() {
        let color = Clrs::rgb(255, 0, 0);
        assert_eq!(color, DynColors::Rgb(255, 0, 0));
    }
}
