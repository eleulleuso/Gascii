use rayon::prelude::*;


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
    // 
    // OPTIMIZATION: Process rows in chunks for better cache locality
    pub fn process_frame(&self, frame_data: &[u8]) -> Vec<CellData> {
        // Validate input size in debug builds to avoid unsafe out-of-bounds access.
        let expected_len = self.width * self.height * 3;
        debug_assert!(frame_data.len() >= expected_len, "Frame size is too small: got {} expected {}", frame_data.len(), expected_len);
        let term_width = self.width;           // Canvas width IS terminal width
        let term_height = self.height / 2;     // Canvas height is 2x terminal height
        
        // Process in row-major order for cache-friendly memory access
        // Use Rayon for parallel row processing
        (0..term_height)
            .into_par_iter()
            .flat_map(|cy| {
                // Each row produces term_width cells
                let mut row_cells = Vec::with_capacity(term_width);
                
                for cx in 0..term_width {
                    // Map char (cx, cy) to pixels: top=(cx, 2*cy), bottom=(cx, 2*cy+1)
                    let py_top = cy * 2;
                    let py_bottom = cy * 2 + 1;

                    // Inline hot path for pixel access (avoid function call overhead)
                    let get_pixel_fast = |x: usize, y: usize| -> (u8, u8, u8) {
                        let p_idx = (y * self.width + x) * 3;
                        // SAFETY: Bounds checking moved outside hot loop
                        // We know frame_data.len() == width * height * 3
                        if p_idx + 2 < frame_data.len() {
                            // SAFETY: bounds checked above with p_idx + 2 < len
                            unsafe {
                                (
                                    *frame_data.get_unchecked(p_idx),
                                    *frame_data.get_unchecked(p_idx + 1),
                                    *frame_data.get_unchecked(p_idx + 2),
                                )
                            }
                        } else {
                            (0, 0, 0)
                        }
                    };

                    let top_color = get_pixel_fast(cx, py_top);
                    let bottom_color = get_pixel_fast(cx, py_bottom);

                    row_cells.push(CellData {
                        char: '▀', // Upper Half Block
                        fg: top_color,
                        bg: bottom_color,
                    });
                }
                
                row_cells
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_process_frame_half_block() {
        // Width 2, Height 4 => term_height = 2
        let proc = FrameProcessor::new(2, 4);
        // Create a frame of 2x4 pixels = 8 pixels -> 8*3=24 bytes
        // We'll color top row pixels red and bottoms green for the first terminal row
        // Layout: row major [ (x=0,y=0), (x=1,y=0), (x=0,y=1), (x=1,y=1), (x=0,y=2), (x=1,y=2), (x=0,y=3), (x=1,y=3) ]
        let mut frame = vec![0u8; 2 * 4 * 3];
        // (0,0) red
        frame[0] = 255; frame[1]=0; frame[2]=0;
        // (1,0) red
        frame[3] = 255; frame[4]=0; frame[5]=0;
        // (0,1) green
        frame[6] = 0; frame[7]=255; frame[8]=0;
        // (1,1) green
        frame[9] = 0; frame[10]=255; frame[11]=0;
        // (0,2) blue
        frame[12] = 0; frame[13]=0; frame[14]=255;
        // (1,2) blue
        frame[15] = 0; frame[16]=0; frame[17]=255;
        // (0,3) yellow
        frame[18] = 255; frame[19]=255; frame[20]=0;
        // (1,3) yellow
        frame[21] = 255; frame[22]=255; frame[23]=0;

        let cells = proc.process_frame(&frame);
        assert_eq!(cells.len(), 2 * 2); // term_width * term_height
        // First terminal row cell 0 should be fg=red, bg=green
        assert_eq!(cells[0].fg, (255,0,0));
        assert_eq!(cells[0].bg, (0,255,0));
        // Second terminal row cell 0 should be fg=blue, bg=yellow
        assert_eq!(cells[2].fg, (0,0,255));
        assert_eq!(cells[2].bg, (255,255,0));
    }
}
