use rayon::prelude::*;
use super::cell::CellData;
use super::quantizer::ColorQuantizer;

// 8x8 Bayer Matrix for Ordered Dithering
// Values are 0..63
const BAYER_8X8: [u8; 64] = [
    0, 32,  8, 40,  2, 34, 10, 42,
    48, 16, 56, 24, 50, 18, 58, 26,
    12, 44,  4, 36, 14, 46,  6, 38,
    60, 28, 52, 20, 62, 30, 54, 22,
    3, 35, 11, 43,  1, 33,  9, 41,
    51, 19, 59, 27, 49, 17, 57, 25,
    15, 47,  7, 39, 13, 45,  5, 37,
    63, 31, 55, 23, 61, 29, 53, 21,
];

pub struct FrameProcessor {
    pub width: usize,
    pub height: usize,
}

impl FrameProcessor {
    pub fn new(width: usize, height: usize) -> Self {
        Self { width, height }
    }

    pub fn process_frame(&self, pixel_data: &[u8]) -> Vec<CellData> {
        let mut cells = vec![CellData { char: ' ', fg: 0, bg: 0 }; self.width * (self.height / 2)];
        self.process_frame_into(pixel_data, &mut cells);
        cells
    }

    pub fn process_frame_into(&self, pixel_data: &[u8], cells: &mut [CellData]) {
        let w = self.width;
        let h = self.height; // This is the pixel height
        let term_height = h / 2; // Terminal height is half pixel height

        // Ensure cells buffer is correct size
        if cells.len() != w * term_height {
            // In a real scenario, we might want to resize or error out.
            // For now, we assume the caller provides the correct size.
            return;
        }

        // Parallel processing using Rayon
        // We use a chunk size that balances load balancing with overhead.
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

                    // Helper to get pixel color safely with dithering
                    let get_pixel_dithered = |x: usize, y: usize| -> (u8, u8, u8) {
                        let offset = (y * w + x) * 3;
                        if offset + 2 < pixel_data.len() {
                            let r = pixel_data[offset];
                            let g = pixel_data[offset + 1];
                            let b = pixel_data[offset + 2];
                            
                            // Apply Ordered Dithering
                            // Threshold from Bayer Matrix (0..63)
                            let threshold = BAYER_8X8[(y % 8) * 8 + (x % 8)];
                            
                            // Normalize threshold to -0.5 to 0.5 range (approx) 
                            // and scale by spread factor.
                            // Factor 32.0 means the noise spreads across +/- 16 values.
                            // This is enough to bridge the gap between 256 colors.
                            let spread = 32.0;
                            let adjustment = (threshold as f32 - 31.5) / 63.0 * spread;
                            
                            let r_d = (r as f32 + adjustment).clamp(0.0, 255.0) as u8;
                            let g_d = (g as f32 + adjustment).clamp(0.0, 255.0) as u8;
                            let b_d = (b as f32 + adjustment).clamp(0.0, 255.0) as u8;
                            
                            (r_d, g_d, b_d)
                        } else {
                            (0, 0, 0)
                        }
                    };

                    let top_color = get_pixel_dithered(cx, py_top);
                    let bottom_color = get_pixel_dithered(cx, py_bottom);

                    // Quantize RGB to ANSI 256 colors
                    let fg_idx = ColorQuantizer::quantize_rgb(top_color.0, top_color.1, top_color.2);
                    let bg_idx = ColorQuantizer::quantize_rgb(bottom_color.0, bottom_color.1, bottom_color.2);

                    *cell = CellData {
                        char: '▀', // Upper Half Block
                        fg: fg_idx,
                        bg: bg_idx,
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
        
        // Expected quantized colors
        // Note: Dithering might slightly alter values, but for pure colors (255,0,0) it should map to the primary color index
        // Red (255,0,0) -> Index 196 (in 6x6x6 cube) or 9 (bright red) depending on quantization logic.
        // Let's use the quantizer to get expected values.
        let expected_red = ColorQuantizer::quantize_rgb(255, 0, 0);
        let expected_green = ColorQuantizer::quantize_rgb(0, 255, 0);
        let expected_blue = ColorQuantizer::quantize_rgb(0, 0, 255);
        let expected_yellow = ColorQuantizer::quantize_rgb(255, 255, 0);

        // First terminal row cell 0 should be fg=red, bg=green
        assert_eq!(cells[0].fg, expected_red);
        assert_eq!(cells[0].bg, expected_green);
        // Second terminal row cell 0 should be fg=blue, bg=yellow
        assert_eq!(cells[2].fg, expected_blue);
        assert_eq!(cells[2].bg, expected_yellow);
    }
}
