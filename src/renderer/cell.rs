/// Represents a 24-bit RGB color
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct RgbColor(pub u8, pub u8, pub u8);

/// Represents a single character cell on the terminal
/// 
/// Uses TrueColor (RGB) for maximum quality
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct CellData {
    pub char: char,
    pub fg: RgbColor,
    pub bg: RgbColor,
}

impl Default for CellData {
    fn default() -> Self {
        Self {
            char: ' ',
            fg: RgbColor(0, 0, 0),
            bg: RgbColor(0, 0, 0),
        }
    }
}
