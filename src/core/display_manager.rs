use anyhow::Result;
use crossterm::{
    cursor,
    style::Print,
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
        
        // === STRONGER V-SYNC ENFORCEMENT ===
        // Enable synchronized updates mode (DECSM 2026)
        // This ensures terminal waits for complete frame before rendering
        stdout.execute(Print("\x1b[?2026h"))?;
        
        // Disable cursor blinking (reduces screen tearing)
        stdout.execute(Print("\x1b[?12l"))?;
        
        // Request high refresh rate mode if supported
        stdout.execute(Print("\x1b[?1049h"))?; // Alternative screen buffer
        
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
        
        // OPTIMIZATION: Pre-allocate buffer with a more accurate size estimate
        // Each cell update takes approx 15-20 bytes (cursor move + color + char)
        // If full redraw, size is large. If diff, size is small.
        // We use a safe upper bound estimate to avoid reallocations.
        let estimated_size = if force_redraw { cells.len() * 20 } else { cells.len() * 5 };
        let mut buffer = Vec::with_capacity(estimated_size);
        
        let mut last_fg: Option<(u8, u8, u8)> = None;
        let mut last_bg: Option<(u8, u8, u8)> = None;
        
        // Calculate centering offsets dynamically
        let (term_cols, term_rows) = terminal::size().unwrap_or((80, 24));
        let content_width = width as u16;
        let content_height = (cells.len() / width) as u16;
        
        let offset_x = if term_cols > content_width { (term_cols - content_width) / 2 } else { 0 };
        let offset_y = if term_rows > content_height { (term_rows - content_height) / 2 } else { 0 };

        // Track virtual cursor position to minimize MoveTo commands
        let mut cursor_x: i32 = -1;
        let mut cursor_y: i32 = -1;

        // OPTIMIZATION: Unified loop for both redraw and diff
        // This reduces code duplication and allows for better branch prediction
        for (i, cell) in cells.iter().enumerate() {
            if force_redraw || cell != &last_cells[i] {
                let x = (i % width) as u16;
                let y = (i / width) as u16;
                
                let target_x = x + offset_x;
                let target_y = y + offset_y;
                
                // BOUNDS CHECKING: Skip if outside terminal
                if target_x >= term_cols || target_y >= term_rows {
                    cursor_x = -1;
                    continue;
                }
                
                // Move cursor only if not already at the correct position
                if cursor_x != target_x as i32 || cursor_y != target_y as i32 {
                    // OPTIMIZATION: Use direct byte pushing instead of write! macro for cursor move
                    buffer.extend_from_slice(b"\x1b[");
                    buffer.extend_from_slice((target_y + 1).to_string().as_bytes());
                    buffer.extend_from_slice(b";");
                    buffer.extend_from_slice((target_x + 1).to_string().as_bytes());
                    buffer.extend_from_slice(b"H");
                    
                    cursor_x = target_x as i32;
                    cursor_y = target_y as i32;
                }
                
                // Color updates
                if Some(cell.fg) != last_fg { 
                    // OPTIMIZATION: Direct byte pushing for colors
                    buffer.extend_from_slice(b"\x1b[38;2;");
                    buffer.extend_from_slice(cell.fg.0.to_string().as_bytes());
                    buffer.extend_from_slice(b";");
                    buffer.extend_from_slice(cell.fg.1.to_string().as_bytes());
                    buffer.extend_from_slice(b";");
                    buffer.extend_from_slice(cell.fg.2.to_string().as_bytes());
                    buffer.extend_from_slice(b"m");
                    last_fg = Some(cell.fg); 
                }
                if Some(cell.bg) != last_bg { 
                    buffer.extend_from_slice(b"\x1b[48;2;");
                    buffer.extend_from_slice(cell.bg.0.to_string().as_bytes());
                    buffer.extend_from_slice(b";");
                    buffer.extend_from_slice(cell.bg.1.to_string().as_bytes());
                    buffer.extend_from_slice(b";");
                    buffer.extend_from_slice(cell.bg.2.to_string().as_bytes());
                    buffer.extend_from_slice(b"m");
                    last_bg = Some(cell.bg); 
                }
                
                // Write character
                let mut b_dst = [0u8; 4];
                buffer.extend_from_slice(cell.char.encode_utf8(&mut b_dst).as_bytes());
                
                last_cells[i] = *cell;
                
                // Advance virtual cursor
                cursor_x += 1;
            } else {
                // If cell didn't change, invalidate cursor tracker
                cursor_x = -1;
            }
        }

        buffer.extend_from_slice(b"\x1b[0m");
        self.stdout.write_all(&buffer)?;
        self.stdout.flush()?;
        
        // End VSync AFTER flush to ensure complete frame is ready
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
