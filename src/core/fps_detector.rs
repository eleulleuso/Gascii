use anyhow::{Context, Result};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicU32, Ordering};

/// Detects the actual FPS of a video file from FFmpeg output
pub struct FpsDetector {
    detected_fps: Arc<AtomicU32>, // Store as u32 (fps * 1000 for precision)
}

impl FpsDetector {
    pub fn new() -> Self {
        Self {
            detected_fps: Arc::new(AtomicU32::new(0)),
        }
    }

    /// Parse FPS from FFmpeg output line
    /// Example: "Stream #0:0[0x1](und): Video: h264 ... 30 fps, 30 tbr ..."
    pub fn parse_fps_from_line(&self, line: &str) {
        // Look for "XX fps" pattern in stream info
        if line.contains("Video:") && line.contains(" fps") {
            // Find "XXX fps" pattern
            if let Some(fps_idx) = line.find(" fps") {
                // Extract number before " fps"
                let before_fps = &line[..fps_idx];
                if let Some(last_space) = before_fps.rfind(' ') {
                    let fps_str = before_fps[last_space + 1..].trim();
                    if let Ok(fps_float) = fps_str.parse::<f32>() {
                        // Store as millifps (fps * 1000)
                        let millifps = (fps_float * 1000.0) as u32;
                        self.detected_fps.store(millifps, Ordering::SeqCst);
                        println!("Auto-detected video FPS: {:.2}", fps_float);
                    }
                }
            }
        }
    }

    /// Get detected FPS, returns None if not yet detected
    pub fn get_fps(&self) -> Option<f32> {
        let millifps = self.detected_fps.load(Ordering::SeqCst);
        if millifps > 0 {
            Some(millifps as f32 / 1000.0)
        } else {
            None
        }
    }

    /// Get FPS or fallback to default
    pub fn get_fps_or(&self, default: u32) -> f32 {
        self.get_fps().unwrap_or(default as f32)
    }
}
