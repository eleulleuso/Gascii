use rayon::prelude::*;
use super::cell::CellData;

pub struct FrameProcessor {
    pub width: usize,
    pub height: usize,
}

impl FrameProcessor {
    pub fn new(width: usize, height: usize) -> Self {
        Self { width, height }
    }

    pub fn process_frame(&self, pixel_data: &[u8]) -> Vec<CellData> {
        let mut cells = vec![CellData { char: ' ', fg: (0,0,0), bg: (0,0,0) }; self.width * (self.height / 2)];
        self.process_frame_into(pixel_data, &mut cells);
        cells
    }

    pub fn process_frame_into(&self, pixel_data: &[u8], cells: &mut [CellData]) {
        let w = self.width;
        let h = self.height; // This is the pixel height
        let term_height = h / 2; // Terminal height is half pixel height

        // Ensure cells buffer is correct size
        if cells.len() != w * term_height {
            return;
        }

        // Parallel processing using Rayon
        let chunk_size = if w * term_height > 10000 { 
            2000 
        } else { 
            (w * term_height / rayon::current_num_threads().max(1)).max(1) 
        };

        cells.par_chunks_mut(chunk_size)
            .enumerate()
            .for_each(|(chunk_idx, chunk)| {
                let start_idx = chunk_idx * chunk_size;
                
                for (i, cell) in chunk.iter_mut().enumerate() {
                    let idx = start_idx + i;
                    let cx = idx % w;
                    let cy = idx / w; // This `cy` is the terminal character row

                    // Map char coordinate (cx, cy) to pixel coordinates
                    // Each char represents 2 vertical pixels (Upper Half Block)
                    // Top pixel: (cx, cy * 2)
                    // Bottom pixel: (cx, cy * 2 + 1)
                    
                    let py_top = cy * 2;
                    let py_bottom = cy * 2 + 1;

                    // Helper to get pixel color safely
                    let get_pixel = |x: usize, y: usize| -> (u8, u8, u8) {
                        let offset = (y * w + x) * 3;
                        if offset + 2 < pixel_data.len() {
                            (
                                pixel_data[offset],
                                pixel_data[offset + 1],
                                pixel_data[offset + 2]
                            )
                        } else {
                            (0, 0, 0)
                        }
                    };

                    let top_color = get_pixel(cx, py_top);
                    let bottom_color = get_pixel(cx, py_bottom);

                    *cell = CellData {
                        char: '▀', // Upper Half Block
                        fg: top_color,
                        bg: bottom_color,
                    };
                }
            });
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
