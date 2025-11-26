use anyhow::Result;
use crossterm::{
    cursor,
    style::{self, Color, Print, SetBackgroundColor, SetForegroundColor},
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen},
    QueueableCommand,
    ExecutableCommand,
};
use std::io::{Stdout, Write};

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, clap::ValueEnum)]
pub enum DisplayMode {
    Ascii,
    Rgb,
}

pub struct DisplayManager {
    stdout: Stdout,
    mode: DisplayMode,
    last_cells: Option<Vec<crate::core::processor::CellData>>,
}

impl DisplayManager {
    pub fn new(mode: DisplayMode) -> Result<Self> {
        let mut stdout = std::io::stdout();
        terminal::enable_raw_mode()?;
        stdout.execute(EnterAlternateScreen)?;
        stdout.execute(cursor::Hide)?;
        // Disable line wrapping (DECRAWM) to prevent scrolling at edges
        stdout.execute(Print("\x1b[?7l"))?;
        
        Ok(Self {
            stdout,
            mode,
            last_cells: None,
        })
    }


    pub fn render_frame(&mut self, _frame_data: &[u8]) -> Result<()> {
        // Legacy method, no longer used.
        Ok(())
    }

    // Optimized Diffing Renderer
    // Takes a grid of CellData (calculated by Processor) and updates the terminal.
    pub fn render_diff(&mut self, cells: &[crate::core::processor::CellData], width: usize) -> Result<()> {
        // VSync Begin
        self.stdout.queue(Print("\x1b[?2026h"))?;

        let mut force_redraw = false;
        if self.last_cells.is_none() || self.last_cells.as_ref().unwrap().len() != cells.len() {
            self.stdout.queue(crossterm::terminal::Clear(crossterm::terminal::ClearType::All))?;
            self.last_cells = Some(vec![crate::core::processor::CellData { char: ' ', fg: (0,0,0), bg: (0,0,0) }; cells.len()]);
            force_redraw = true;
        }

        let last_cells = self.last_cells.as_mut().unwrap();
        let mut buffer = Vec::with_capacity(cells.len() * 20); // Heuristic size
        
        let mut last_fg: Option<(u8, u8, u8)> = None;
        let mut last_bg: Option<(u8, u8, u8)> = None;
        
        // Calculate centering offsets dynamically
        // We check terminal size every frame to handle resizing gracefully.
        let (term_cols, term_rows) = terminal::size().unwrap_or((80, 24));
        let content_width = width as u16;
        let content_height = (cells.len() / width) as u16;
        
        // Calculate offsets to center the content
        // If content is larger than terminal, offset is 0 (top-left aligned, effectively cropping)
        let offset_x = if term_cols > content_width { (term_cols - content_width) / 2 } else { 0 };
        let offset_y = if term_rows > content_height { (term_rows - content_height) / 2 } else { 0 };

        // Track virtual cursor position to minimize MoveTo commands
        let mut cursor_x: i32 = -1;
        let mut cursor_y: i32 = -1;

        if force_redraw {
            // Clear screen first is already done above
            for (i, cell) in cells.iter().enumerate() {
                let x = (i % width) as u16;
                let y = (i / width) as u16;
                
                // BOUNDS CHECKING: Skip if outside terminal
                let target_x = x + offset_x;
                let target_y = y + offset_y;
                
                if target_x >= term_cols || target_y >= term_rows {
                    cursor_x = -1; // Invalidate cursor if we skip
                    continue;
                }
                
                // Move cursor only if not already at the correct position
                if cursor_x != target_x as i32 || cursor_y != target_y as i32 {
                    write!(buffer, "\x1b[{};{}H", target_y + 1, target_x + 1)?;
                    cursor_x = target_x as i32;
                    cursor_y = target_y as i32;
                }
                
                if Some(cell.fg) != last_fg { write!(buffer, "\x1b[38;2;{};{};{}m", cell.fg.0, cell.fg.1, cell.fg.2)?; last_fg = Some(cell.fg); }
                if Some(cell.bg) != last_bg { write!(buffer, "\x1b[48;2;{};{};{}m", cell.bg.0, cell.bg.1, cell.bg.2)?; last_bg = Some(cell.bg); }
                
                let mut b_dst = [0u8; 4];
                buffer.extend_from_slice(cell.char.encode_utf8(&mut b_dst).as_bytes());
                
                last_cells[i] = *cell;
                
                // Advance virtual cursor
                cursor_x += 1;
            }
        } else {
            // Diffing Mode
            for (i, cell) in cells.iter().enumerate() {
                if cell != &last_cells[i] {
                    let x = (i % width) as u16;
                    let y = (i / width) as u16;
                    
                    // BOUNDS CHECKING: Skip if outside terminal
                    let target_x = x + offset_x;
                    let target_y = y + offset_y;
                    
                    if target_x >= term_cols || target_y >= term_rows {
                        cursor_x = -1;
                        continue;
                    }
                    
                    // Move cursor only if not already at the correct position
                    if cursor_x != target_x as i32 || cursor_y != target_y as i32 {
                        write!(buffer, "\x1b[{};{}H", target_y + 1, target_x + 1)?;
                        cursor_x = target_x as i32;
                        cursor_y = target_y as i32;
                    }
                    
                    if Some(cell.fg) != last_fg { write!(buffer, "\x1b[38;2;{};{};{}m", cell.fg.0, cell.fg.1, cell.fg.2)?; last_fg = Some(cell.fg); }
                    if Some(cell.bg) != last_bg { write!(buffer, "\x1b[48;2;{};{};{}m", cell.bg.0, cell.bg.1, cell.bg.2)?; last_bg = Some(cell.bg); }
                    
                    let mut b_dst = [0u8; 4];
                    buffer.extend_from_slice(cell.char.encode_utf8(&mut b_dst).as_bytes());
                    
                    last_cells[i] = *cell;
                    
                    // Advance virtual cursor
                    cursor_x += 1;
                } else {
                    // If cell didn't change, our virtual cursor is no longer valid for the next position
                    // because we didn't write anything.
                    // Wait, if we skip writing, the terminal cursor DOES NOT move.
                    // So we must invalidate our virtual cursor tracker so the next write forces a MoveTo.
                    cursor_x = -1;
                }
            }
        }

        buffer.extend_from_slice(b"\x1b[0m");
        self.stdout.write_all(&buffer)?;
        self.stdout.queue(Print("\x1b[?2026l"))?;
        self.stdout.flush()?;
        
        Ok(())
    }
}

impl Drop for DisplayManager {
    fn drop(&mut self) {
        let _ = self.stdout.execute(cursor::Show);
        let _ = self.stdout.execute(LeaveAlternateScreen);
        let _ = terminal::disable_raw_mode();
    }
}
