/// Represents a single character cell on the terminal
/// 
/// Uses ANSI 256-color indices instead of RGB for 66% data reduction
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct CellData {
    pub char: char,
    pub fg: u8,  // ANSI 256-color index
    pub bg: u8,  // ANSI 256-color index
}

impl CellData {
    pub fn new(char: char, fg: u8, bg: u8) -> Self {
        Self { char, fg, bg }
    }
}
