use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

/// Detects the actual FPS of a video file from FFmpeg output
pub struct FpsDetector {
    fps: Arc<AtomicU32>, // Store as u32 * 100 to handle 23.98 -> 2398
}

impl FpsDetector {
    pub fn new() -> Self {
        Self {
            fps: Arc::new(AtomicU32::new(0)),
        }
    }

    pub fn parse_fps_from_line(&self, line: &str) {
        // Look for "Stream #0:0... 23.98 fps,"
        if line.contains("Stream #") && line.contains("Video:") && line.contains("fps") {
            // Simple parsing logic
            let parts: Vec<&str> = line.split(',').collect();
            for part in parts {
                if part.contains("fps") {
                    let fps_str = part.replace("fps", "").trim().to_string();
                    if let Ok(fps_val) = fps_str.parse::<f32>() {
                        let stored_val = (fps_val * 100.0) as u32;
                        self.fps.store(stored_val, Ordering::SeqCst);
                        println!("Auto-detected video FPS: {}", fps_val);
                    }
                }
            }
        }
    }

    pub fn get_fps(&self) -> Option<f32> {
        let val = self.fps.load(Ordering::SeqCst);
        if val == 0 {
            None
        } else {
            Some(val as f32 / 100.0)
        }
    }
    
    pub fn get_fps_or(&self, default: u32) -> f32 {
        self.get_fps().unwrap_or(default as f32)
    }
}
