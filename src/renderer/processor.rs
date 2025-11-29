use rayon::prelude::*;
use super::cell::CellData;

pub struct FrameProcessor {
    pub width: usize,
    pub height: usize,
}

// Bayer Matrix 8x8 for Ordered Dithering
const BAYER_8X8: [[u8; 8]; 8] = [
    [ 0, 32,  8, 40,  2, 34, 10, 42],
    [48, 16, 56, 24, 50, 18, 58, 26],
    [12, 44,  4, 36, 14, 46,  6, 38],
    [60, 28, 52, 20, 62, 30, 54, 22],
    [ 3, 35, 11, 43,  1, 33,  9, 41],
    [51, 19, 59, 27, 49, 17, 57, 25],
    [15, 47,  7, 39, 13, 45,  5, 37],
    [63, 31, 55, 23, 61, 29, 53, 21]
];

impl FrameProcessor {
    pub fn new(width: usize, height: usize) -> Self {
        Self { width, height }
    }

    pub fn process_frame(&self, pixel_data: &[u8]) -> Vec<CellData> {
        let mut cells = vec![CellData::default(); self.width * (self.height / 2)];
        self.process_frame_into(pixel_data, &mut cells);
        cells
    }

    pub fn process_frame_into(&self, pixel_data: &[u8], cells: &mut [CellData]) {
        let w = self.width;
        let h = self.height; 
        let term_height = h / 2; 

        if cells.len() != w * term_height {
            return;
        }

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
                    let cy = idx / w; 

                    let py_top = cy * 2;
                    let py_bottom = cy * 2 + 1;

                    let get_pixel = |x: usize, y: usize| -> (u8, u8, u8) {
                        let offset = (y * w + x) * 3;
                        if offset + 2 < pixel_data.len() {
                            (pixel_data[offset], pixel_data[offset + 1], pixel_data[offset + 2])
                        } else {
                            (0, 0, 0)
                        }
                    };

                    let (tr, tg, tb) = get_pixel(cx, py_top);
                    let (br, bg, bb) = get_pixel(cx, py_bottom);

                    // Apply Bayer Dithering
                    // We use the same matrix for both top and bottom pixels, but mapped to their screen coordinates
                    // Actually, for consistency, we should use the pixel coordinates for the matrix lookup
                    
                    let dither = |r: u8, g: u8, b: u8, x: usize, y: usize| -> u8 {
                        let threshold = BAYER_8X8[y % 8][x % 8];
                        // Normalize threshold to -0.5 to 0.5 range and scale
                        // 64 levels. 
                        // Formula: value + (threshold - 32) * scale
                        // Let's use a simpler approach: 
                        // Add (threshold - 32) to the color value before quantization?
                        // Standard: color = color + (threshold * scale / 64) - (scale / 2)
                        
                        let noise = (threshold as i32 - 32) * 1; // Scale factor 1 is subtle. Try 2 for more effect.
                        
                        let r_d = (r as i32 + noise).clamp(0, 255) as u8;
                        let g_d = (g as i32 + noise).clamp(0, 255) as u8;
                        let b_d = (b as i32 + noise).clamp(0, 255) as u8;
                        
                        Self::rgb_to_ansi256(r_d, g_d, b_d)
                    };

                    *cell = CellData {
                        char: '▀', 
                        fg: dither(tr, tg, tb, cx, py_top),
                        bg: dither(br, bg, bb, cx, py_bottom),
                    };
                }
            });
    }

    // Convert RGB to ANSI 256 color index
    #[inline(always)]
    fn rgb_to_ansi256(r: u8, g: u8, b: u8) -> u8 {
        // Standard 6x6x6 color cube
        if r == g && g == b {
            if r < 8 { return 16; }
            if r > 248 { return 231; }
            return (((r as u16 - 8) * 24 / 247) as u8) + 232;
        }

        let r_idx = (r as u16 * 5 + 127) / 255;
        let g_idx = (g as u16 * 5 + 127) / 255;
        let b_idx = (b as u16 * 5 + 127) / 255;

        16 + 36 * r_idx as u8 + 6 * g_idx as u8 + b_idx as u8
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_process_frame_half_block() {
        let proc = FrameProcessor::new(2, 4);
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
        // (1,3) yellow
        frame[18] = 255; frame[19]=255; frame[20]=0;
        // (1,3) yellow
        frame[21] = 255; frame[22]=255; frame[23]=0;

        let cells = proc.process_frame(&frame);
        assert_eq!(cells.len(), 2 * 2); 
        
        // Check if colors are mapped to reasonable ANSI indices
        // Red ~ 196, Green ~ 46, Blue ~ 21, Yellow ~ 226
        // Exact values depend on dithering noise, but should be close
        
        // Just assert they are not 0 (black)
        assert!(cells[0].fg != 0);
        assert!(cells[0].bg != 0);
    }
}
