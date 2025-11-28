/// Represents a single character cell on the terminal
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct CellData {
    pub char: char,
    pub fg: (u8, u8, u8),
    pub bg: (u8, u8, u8),
}

impl CellData {
    pub fn new(char: char, fg: (u8, u8, u8), bg: (u8, u8, u8)) -> Self {
        Self { char, fg, bg }
    }
}
