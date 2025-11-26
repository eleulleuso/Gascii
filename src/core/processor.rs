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

    // Process a raw RGB frame into a grid of CellData using Rayon
    // SWITCHED TO SMART BLOCK RENDERING (2x2)
    // This provides 2x Horizontal and 2x Vertical resolution (4x total).
    // It uses Quarter Blocks, Half Blocks, and Geometric Triangles for Anti-aliasing.
    pub fn process_frame(&self, frame_data: &[u8]) -> Vec<CellData> {
        // In Smart Block mode:
        // Canvas Width = Terminal Width * 2 (2x)
        // Canvas Height = Terminal Height * 2 (2x)
        // So a single Cell corresponds to a 2x2 pixel block.
        
        let term_width = self.width / 2;      // Canvas width is 2x terminal width
        let term_height = self.height / 2;    // Canvas height is 2x terminal height
        
        let rgb = frame_data; 
        
        let total_cells = term_width * term_height;

        (0..total_cells).into_par_iter().map(|idx| {
            let cx = idx % term_width;
            let cy = idx / term_width;

            // Map char grid (cx, cy) to pixel grid (2*cx, 2*cy)
            // We read 2x2 pixels:
            // TL (0,0)  TR (1,0)
            // BL (0,1)  BR (1,1)
            
            let px = cx * 2;
            let py = cy * 2;

            // Helper to get pixel color safely
            let get_color = |x, y| {
                let p_idx = (y * self.width + x) * 3;
                if p_idx + 2 < rgb.len() {
                    (rgb[p_idx], rgb[p_idx+1], rgb[p_idx+2])
                } else {
                    (0, 0, 0)
                }
            };

            let c_tl = get_color(px, py);
            let c_tr = get_color(px+1, py);
            let c_bl = get_color(px, py+1);
            let c_br = get_color(px+1, py+1);

            // Smart Block Logic:
            // 1. Calculate average color (for fallback)
            // 2. Cluster colors to find best 2 dominant colors (FG, BG)
            // 3. Match 2x2 pattern to Unicode shapes
            
            // Simple Euclidean distance squared
            let dist_sq = |c1: (u8,u8,u8), c2: (u8,u8,u8)| {
                let dr = c1.0 as i32 - c2.0 as i32;
                let dg = c1.1 as i32 - c2.1 as i32;
                let db = c1.2 as i32 - c2.2 as i32;
                dr*dr + dg*dg + db*db
            };

            // Check if 4 pixels are similar (Solid Block)
            let d_tr = dist_sq(c_tl, c_tr);
            let d_bl = dist_sq(c_tl, c_bl);
            let d_br = dist_sq(c_tl, c_br);
            let threshold = 30 * 30; // Tolerance

            if d_tr < threshold && d_bl < threshold && d_br < threshold {
                return CellData { char: '█', fg: c_tl, bg: c_tl };
            }

            // Check Half Block (Top vs Bottom) - Standard High Quality
            // Avg Top vs Avg Bottom
            let top_avg = ((c_tl.0 as u32 + c_tr.0 as u32)/2, (c_tl.1 as u32 + c_tr.1 as u32)/2, (c_tl.2 as u32 + c_tr.2 as u32)/2);
            let bot_avg = ((c_bl.0 as u32 + c_br.0 as u32)/2, (c_bl.1 as u32 + c_br.1 as u32)/2, (c_bl.2 as u32 + c_br.2 as u32)/2);
            let top_c = (top_avg.0 as u8, top_avg.1 as u8, top_avg.2 as u8);
            let bot_c = (bot_avg.0 as u8, bot_avg.1 as u8, bot_avg.2 as u8);
            
            // Check Vertical Block (Left vs Right)
            let left_avg = ((c_tl.0 as u32 + c_bl.0 as u32)/2, (c_tl.1 as u32 + c_bl.1 as u32)/2, (c_tl.2 as u32 + c_bl.2 as u32)/2);
            let right_avg = ((c_tr.0 as u32 + c_br.0 as u32)/2, (c_tr.1 as u32 + c_br.1 as u32)/2, (c_tr.2 as u32 + c_br.2 as u32)/2);
            let left_c = (left_avg.0 as u8, left_avg.1 as u8, left_avg.2 as u8);
            let right_c = (right_avg.0 as u8, right_avg.1 as u8, right_avg.2 as u8);

            // Check Diagonal (Triangles)
            // TL+BR vs TR+BL ? No, usually it's a split.
            // Split 1: TL is different, others are same (Upper Left Triangle ◤)
            // Split 2: TR is different (Upper Right Triangle ◥)
            // Split 3: BL is different (Lower Left Triangle ◣)
            // Split 4: BR is different (Lower Right Triangle ◢)
            
            // We need a robust way to pick the best shape.
            // Calculate error for each shape and pick minimum.
            
            let err = |p: (u8,u8,u8), target: (u8,u8,u8)| dist_sq(p, target);
            
            // 1. Half Block Error
            let err_half = err(c_tl, top_c) + err(c_tr, top_c) + err(c_bl, bot_c) + err(c_br, bot_c);
            
            // 2. Vertical Block Error
            let err_vert = err(c_tl, left_c) + err(c_bl, left_c) + err(c_tr, right_c) + err(c_br, right_c);
            
            // 3. Diagonal 1 (TL/BR split? No, usually triangles are corner vs rest)
            // Let's test 4 triangle cases.
            // ◤ (TL is FG, Rest is BG)
            let bg_rest_tl = ((c_tr.0 as u32 + c_bl.0 as u32 + c_br.0 as u32)/3, (c_tr.1 as u32 + c_bl.1 as u32 + c_br.1 as u32)/3, (c_tr.2 as u32 + c_bl.2 as u32 + c_br.2 as u32)/3);
            let bg_rest_tl_c = (bg_rest_tl.0 as u8, bg_rest_tl.1 as u8, bg_rest_tl.2 as u8);
            let err_tri_tl = err(c_tl, c_tl) + err(c_tr, bg_rest_tl_c) + err(c_bl, bg_rest_tl_c) + err(c_br, bg_rest_tl_c);

            // ◥ (TR is FG, Rest is BG)
            let bg_rest_tr = ((c_tl.0 as u32 + c_bl.0 as u32 + c_br.0 as u32)/3, (c_tl.1 as u32 + c_bl.1 as u32 + c_br.1 as u32)/3, (c_tl.2 as u32 + c_bl.2 as u32 + c_br.2 as u32)/3);
            let bg_rest_tr_c = (bg_rest_tr.0 as u8, bg_rest_tr.1 as u8, bg_rest_tr.2 as u8);
            let err_tri_tr = err(c_tr, c_tr) + err(c_tl, bg_rest_tr_c) + err(c_bl, bg_rest_tr_c) + err(c_br, bg_rest_tr_c);

            // ◣ (BL is FG, Rest is BG)
            let bg_rest_bl = ((c_tl.0 as u32 + c_tr.0 as u32 + c_br.0 as u32)/3, (c_tl.1 as u32 + c_tr.1 as u32 + c_br.1 as u32)/3, (c_tl.2 as u32 + c_tr.2 as u32 + c_br.2 as u32)/3);
            let bg_rest_bl_c = (bg_rest_bl.0 as u8, bg_rest_bl.1 as u8, bg_rest_bl.2 as u8);
            let err_tri_bl = err(c_bl, c_bl) + err(c_tl, bg_rest_bl_c) + err(c_tr, bg_rest_bl_c) + err(c_br, bg_rest_bl_c);

            // ◢ (BR is FG, Rest is BG)
            let bg_rest_br = ((c_tl.0 as u32 + c_tr.0 as u32 + c_bl.0 as u32)/3, (c_tl.1 as u32 + c_tr.1 as u32 + c_bl.1 as u32)/3, (c_tl.2 as u32 + c_tr.2 as u32 + c_bl.2 as u32)/3);
            let bg_rest_br_c = (bg_rest_br.0 as u8, bg_rest_br.1 as u8, bg_rest_br.2 as u8);
            let err_tri_br = err(c_br, c_br) + err(c_tl, bg_rest_br_c) + err(c_tr, bg_rest_br_c) + err(c_bl, bg_rest_br_c);

            // Find minimum error
            let mut min_err = err_half;
            let mut best_char = '▀';
            let mut best_fg = top_c;
            let mut best_bg = bot_c;

            if err_vert < min_err {
                min_err = err_vert;
                best_char = '▌'; // Left Half Block
                best_fg = left_c;
                best_bg = right_c;
            }
            
            // Check Triangles (Geometric AA)
            // Note: Triangles use FG for the triangle part, BG for the rest.
            if err_tri_tl < min_err { min_err = err_tri_tl; best_char = '◤'; best_fg = c_tl; best_bg = bg_rest_tl_c; }
            if err_tri_tr < min_err { min_err = err_tri_tr; best_char = '◥'; best_fg = c_tr; best_bg = bg_rest_tr_c; }
            if err_tri_bl < min_err { min_err = err_tri_bl; best_char = '◣'; best_fg = c_bl; best_bg = bg_rest_bl_c; }
            if err_tri_br < min_err { min_err = err_tri_br; best_char = '◢'; best_fg = c_br; best_bg = bg_rest_br_c; }

            // Check Quadrants (Detail)
            // ▘ (TL only)
            // ▝ (TR only)
            // ▖ (BL only)
            // ▗ (BR only)
            // These are mathematically same as triangles (1 vs 3), but different shape.
            // Triangles are better for diagonals, Quadrants for corners.
            // Let's prefer Triangles for AA, but if we want detail, maybe Quadrants?
            // Actually, '◤' is visually very similar to '▘' but with a diagonal cut.
            // Let's stick to Triangles for the "Geometric" feel requested.

            CellData {
                char: best_char,
                fg: best_fg,
                bg: best_bg,
            }
        }).collect()
    }
}
