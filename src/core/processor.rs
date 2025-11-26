use rayon::prelude::*;
use std::sync::Arc;

// Represents a single character cell on the terminal
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct CellData {
    pub char: char,
    pub fg: (u8, u8, u8),
    pub bg: (u8, u8, u8),
}

pub struct FrameProcessor {
    pub width: usize,
    pub height: usize,
}

impl FrameProcessor {
    pub fn new(width: usize, height: usize) -> Self {
        Self { width, height }
    }

    // Process RGB frame into CellData grid using Half-Block rendering
    // Half-Block: 1x Horizontal, 2x Vertical (▀ character)
    // Canvas Width = Terminal Width (1x)
    // Canvas Height = Terminal Height * 2 (2x)
    pub fn process_frame(&self, frame_data: &[u8]) -> Vec<CellData> {
        let term_width = self.width;           // Canvas width IS terminal width
        let term_height = self.height / 2;     // Canvas height is 2x terminal height
        
        let rgb = frame_data; 
        let total_cells = term_width * term_height;

        (0..total_cells).into_par_iter().map(|idx| {
            let cx = idx % term_width;
            let cy = idx / term_width;

            // Map char (cx, cy) to pixels: top=(cx, 2*cy), bottom=(cx, 2*cy+1)
            let px = cx;
            let py_top = cy * 2;
            let py_bottom = cy * 2 + 1;

            let get_color = |x, y| {
                let p_idx = (y * self.width + x) * 3;
                if p_idx + 2 < rgb.len() {
                    (rgb[p_idx], rgb[p_idx+1], rgb[p_idx+2])
                } else {
                    (0, 0, 0)
                }
            };

            let top_color = get_color(px, py_top);
            let bottom_color = get_color(px, py_bottom);

            CellData {
                char: '▀', // Upper Half Block
                fg: top_color,
                bg: bottom_color,
            }
        }).collect()
    }
}
