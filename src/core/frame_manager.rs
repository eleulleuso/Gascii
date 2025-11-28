use anyhow::Result;
use crate::utils::file_utils;
use std::sync::Arc;

pub struct FrameManager {
    frames: Vec<Arc<Vec<u8>>>,
}

impl FrameManager {
    pub fn new() -> Self {
        Self { frames: Vec::new() }
    }

    pub fn load_frames(&mut self, dir: &str, _extension: &str) -> Result<usize> {
        let path = std::path::Path::new(dir).join("video.bin");
        println!("Loading video data from {:?}...", path);
        
        if !path.exists() {
            return Ok(0);
        }

        let data = file_utils::read_file(&path)?;
        
        if data.len() < 8 {
            return Ok(0);
        }

        // Header: Width(u16), Height(u16), FrameCount(u32)
        let width = u16::from_le_bytes([data[0], data[1]]) as usize;
        let height = u16::from_le_bytes([data[2], data[3]]) as usize;
        let frame_count = u32::from_le_bytes([data[4], data[5], data[6], data[7]]) as usize;
        
        let compressed_body = &data[8..];
        
        println!("Decompressing data ({} frames, {}x{})...", frame_count, width, height);
        
        // Calculate expected unpacked size (1 bit per pixel)
        // Note: The extractor packed it as (width * height * 2 + 7) / 8 bytes per frame
        // We need to decompress to that size first
        let packed_frame_size = ((width * (height * 2)) + 7) / 8;
        let total_packed_size = packed_frame_size * frame_count;
        
        let decompressed_packed = lz4::block::decompress(compressed_body, Some(total_packed_size as i32))?;

        if decompressed_packed.len() < total_packed_size {
            anyhow::bail!("Decompressed data length {} shorter than expected {}", decompressed_packed.len(), total_packed_size);
        }
        
        println!("Unpacking frames...");
        self.frames.reserve(frame_count);
        
        // Unpack each frame to RGB (or Grayscale) for the renderer
        // Renderer expects: [Width(u16)][Height(u16)][R,G,B, R,G,B...]
        // To save memory, let's just store [Width(u16)][Height(u16)][Gray, Gray...] (1 byte per pixel)
        // But DisplayManager expects RGB (3 bytes). Let's stick to RGB for compatibility for now, 
        // or update DisplayManager. Updating DisplayManager is better but risky.
        // Let's generate RGB frames to be safe and compatible with existing DisplayManager.
        // It uses more RAM but we solved the DISK size issue.
        
        let pixels_per_frame = width * (height * 2);
        
        for i in 0..frame_count {
            let packed_start = i * packed_frame_size;
            let packed_frame = &decompressed_packed[packed_start..packed_start + packed_frame_size];
            
            // Create frame buffer compatible with DisplayManager
            // Header (4 bytes) + RGB Data (pixels * 3)
            let mut frame_data = Vec::with_capacity(4 + pixels_per_frame * 3);
            frame_data.extend_from_slice(&(width as u16).to_le_bytes());
            frame_data.extend_from_slice(&(height as u16).to_le_bytes());
            
            let mut bit_idx = 0;
            for _ in 0..pixels_per_frame {
                let byte_pos = bit_idx / 8;
                let bit_pos = 7 - (bit_idx % 8);
                
                let is_white = (packed_frame[byte_pos] >> bit_pos) & 1 == 1;
                let val = if is_white { 255 } else { 0 };
                
                // Push RGB
                frame_data.push(val);
                frame_data.push(val);
                frame_data.push(val);
                
                bit_idx += 1;
            }
            
            self.frames.push(Arc::new(frame_data));
        }

        println!("Loaded {} frames.", self.frames.len());
        Ok(self.frames.len())
    }

    pub fn get_frame(&self, index: usize) -> Option<Arc<Vec<u8>>> {
        self.frames.get(index).map(|v| Arc::clone(v))
    }

    pub fn frame_count(&self) -> usize {
        self.frames.len()
    }
}
