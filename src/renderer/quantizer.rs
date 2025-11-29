/// ANSI 256-color palette quantizer
/// 
/// Converts RGB colors to ANSI 256-color indices for terminal output.
/// This reduces data size from 3 bytes (RGB) to 1 byte (index), 
/// achieving 66% data reduction.

use std::sync::OnceLock;

static COLOR_LUT: OnceLock<Vec<u8>> = OnceLock::new();

pub struct ColorQuantizer;

impl ColorQuantizer {
    /// Quantize RGB color to nearest ANSI 256-color index
    pub fn quantize_rgb(r: u8, g: u8, b: u8) -> u8 {
        // Get or initialize LUT
        let lut = COLOR_LUT.get_or_init(|| Self::build_lut());
        
        // Lookup in pre-computed table
        let idx = ((r as usize) << 16) | ((g as usize) << 8) | (b as usize);
        lut[idx]
    }
    
    /// Build look-up table mapping RGB to ANSI 256 colors
    fn build_lut() -> Vec<u8> {
        // Allocate on heap directly to avoid stack overflow (16MB)
        let mut lut = vec![0u8; 256 * 256 * 256];
        
        for r in 0..256 {
            for g in 0..256 {
                for b in 0..256 {
                    let idx = (r << 16) | (g << 8) | b;
                    lut[idx] = Self::rgb_to_ansi256(r as u8, g as u8, b as u8);
                }
            }
        }
        
        lut
    }
    
    /// Convert RGB to ANSI 256 color index
    /// 
    /// ANSI 256 color palette:
    /// - 0-15: Standard colors
    /// - 16-231: 6×6×6 color cube
    /// - 232-255: Grayscale
    fn rgb_to_ansi256(r: u8, g: u8, b: u8) -> u8 {
        // Check if grayscale
        let gray_threshold = 8;
        if (r as i16 - g as i16).abs() < gray_threshold
            && (r as i16 - b as i16).abs() < gray_threshold
            && (g as i16 - b as i16).abs() < gray_threshold
        {
            // Grays: 232-255 (24 shades)
            let gray = ((r as u16 + g as u16 + b as u16) / 3) as u8;
            if gray < 8 {
                return 16; // Black from color cube
            } else if gray > 238 {
                return 231; // White from color cube
            } else {
                return 232 + ((gray - 8) * 24 / 230);
            }
        }
        
        // Map to 6×6×6 color cube (16-231)
        let r6 = (r as u16 * 6 / 256) as u8;
        let g6 = (g as u16 * 6 / 256) as u8;
        let b6 = (b as u16 * 6 / 256) as u8;
        
        16 + 36 * r6 + 6 * g6 + b6
    }
    
    /// Get RGB values for an ANSI 256 color index (for testing)
    #[allow(dead_code)]
    pub fn ansi256_to_rgb(index: u8) -> (u8, u8, u8) {
        match index {
            // Standard 16 colors (approximations)
            0..=15 => {
                let standard = [
                    (0, 0, 0),       // 0: Black
                    (128, 0, 0),     // 1: Red
                    (0, 128, 0),     // 2: Green
                    (128, 128, 0),   // 3: Yellow
                    (0, 0, 128),     // 4: Blue
                    (128, 0, 128),   // 5: Magenta
                    (0, 128, 128),   // 6: Cyan
                    (192, 192, 192), // 7: White
                    (128, 128, 128), // 8: Bright Black
                    (255, 0, 0),     // 9: Bright Red
                    (0, 255, 0),     // 10: Bright Green
                    (255, 255, 0),   // 11: Bright Yellow
                    (0, 0, 255),     // 12: Bright Blue
                    (255, 0, 255),   // 13: Bright Magenta
                    (0, 255, 255),   // 14: Bright Cyan
                    (255, 255, 255), // 15: Bright White
                ];
                standard[index as usize]
            }
            // 6×6×6 color cube (16-231)
            16..=231 => {
                let i = index - 16;
                let r = i / 36;
                let g = (i / 6) % 6;
                let b = i % 6;
                
                let r_val = if r == 0 { 0 } else { 55 + r * 40 };
                let g_val = if g == 0 { 0 } else { 55 + g * 40 };
                let b_val = if b == 0 { 0 } else { 55 + b * 40 };
                
                (r_val, g_val, b_val)
            }
            // Grayscale (232-255)
            232..=255 => {
                let gray = 8 + (index - 232) * 10;
                (gray, gray, gray)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_quantize_black() {
        assert_eq!(ColorQuantizer::quantize_rgb(0, 0, 0), 16);
    }
    
    #[test]
    fn test_quantize_white() {
        assert_eq!(ColorQuantizer::quantize_rgb(255, 255, 255), 231);
    }
    
    #[test]
    fn test_quantize_red() {
        let idx = ColorQuantizer::quantize_rgb(255, 0, 0);
        // Should be in red range of color cube
        assert!(idx >= 16 && idx <= 231);
    }
    
    #[test]
    fn test_roundtrip_consistency() {
        // Test that quantizing is consistent
        let idx1 = ColorQuantizer::quantize_rgb(128, 64, 192);
        let idx2 = ColorQuantizer::quantize_rgb(128, 64, 192);
        assert_eq!(idx1, idx2);
    }
}
