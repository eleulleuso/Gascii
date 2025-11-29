/// Represents a single character cell on the terminal
/// 
/// Uses TrueColor (RGB) for maximum quality
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct CellData {
    pub char: char,
    pub fg: (u8, u8, u8),  // RGB
    pub bg: (u8, u8, u8),  // RGB
}

impl CellData {
}
