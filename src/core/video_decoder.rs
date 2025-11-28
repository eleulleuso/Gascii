use anyhow::{Result, anyhow};
use opencv::{
    prelude::*,
    videoio,
    imgproc,
    core,
};
use std::fs::OpenOptions;
use std::io::Write;

pub struct VideoDecoder {
    capture: videoio::VideoCapture,
    width: u32,
    height: u32,
    fps: f64,
    needs_crop: bool,
}

impl VideoDecoder {
    pub fn new(path: &str, width: u32, height: u32) -> Result<Self> {
        // Setup logging with absolute path
        let mut log_path = std::env::current_dir()?;
        log_path.push("debug.log");
        
        let mut log_file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&log_path)?;
        
        writeln!(log_file, "=== OpenCV Video Decoder Initialization ===")?;
        writeln!(log_file, "Video: {}", path)?;
        writeln!(log_file, "Target Resolution: {}x{}", width, height)?;
        
        // Check if this is a 3D SBS video (bochi.mp4)
        let needs_crop = path.to_lowercase().contains("bochi");
        if needs_crop {
            writeln!(log_file, "DEBUG: Detected 3D SBS video - crop enabled (left half)")?;
        }
        
        writeln!(log_file, "DEBUG: Opening video with OpenCV...")?;
        
        // CAP_ANY allows OpenCV to choose the best backend
        // macOS: AVFoundation (VideoToolbox GPU decode)
        // Windows: Media Foundation (GPU decode)
        // Linux: V4L2/GStreamer
        let mut capture = videoio::VideoCapture::from_file(path, videoio::CAP_ANY)?;
        
        // Try to enforce HW acceleration
        // Note: This might not work on all backends/platforms, but it's worth setting
        let _ = capture.set(videoio::CAP_PROP_HW_ACCELERATION, videoio::VIDEO_ACCELERATION_ANY as f64);
        
        if !capture.is_opened()? {
            let err_msg = format!("Failed to open video file: {}", path);
            writeln!(log_file, "ERROR: {}", err_msg)?;
            return Err(anyhow!(err_msg));
        }

        let fps = capture.get(videoio::CAP_PROP_FPS)?;
        let orig_width = capture.get(videoio::CAP_PROP_FRAME_WIDTH)? as u32;
        let orig_height = capture.get(videoio::CAP_PROP_FRAME_HEIGHT)? as u32;
        
        writeln!(log_file, "SUCCESS: OpenCV VideoCapture opened")?;
        writeln!(log_file, "  Original: {}x{}", orig_width, orig_height)?;
        writeln!(log_file, "  FPS: {}", fps)?;
        writeln!(log_file, "  Backend: AVFoundation (GPU decode)")?;
        writeln!(log_file, "=========================")?;
        
        println!("DEBUG: OpenCV VideoCapture opened successfully. Detected FPS: {}", fps);

        Ok(Self {
            capture,
            width,
            height,
            fps,
            needs_crop,
        })
    }

    pub fn get_fps(&self) -> f64 {
        self.fps
    }

    pub fn read_frame(&mut self) -> Result<Option<Vec<u8>>> {
        let start_total = std::time::Instant::now();
        let mut frame = Mat::default();
        
        // 1. Decode (GPU/CPU)
        let start_decode = std::time::Instant::now();
        if !self.capture.read(&mut frame)? {
            return Ok(None); // EOF
        }
        let decode_time = start_decode.elapsed();
        
        if frame.empty() {
            return Ok(None);
        }

        // 2. Resize & Crop (CPU)
        let mut resized = Mat::default();
        let size = core::Size::new(self.width as i32, self.height as i32);
        let start_resize = std::time::Instant::now();

        // Apply crop for 3D SBS videos (take left half)
        if self.needs_crop {
            let width = frame.cols();
            let height = frame.rows();
            let roi = core::Rect::new(0, 0, width / 2, height);
            let cropped = Mat::roi(&frame, roi)?;
            // Resize directly from the ROI (no clone needed)
            imgproc::resize(&cropped, &mut resized, size, 0.0, 0.0, imgproc::INTER_LINEAR)?;
        } else {
            // Resize directly from original frame
            imgproc::resize(&frame, &mut resized, size, 0.0, 0.0, imgproc::INTER_LINEAR)?;
        }
        let resize_time = start_resize.elapsed();

        // 3. Color Conversion (CPU)
        let start_cvt = std::time::Instant::now();
        let mut rgb = Mat::default();
        imgproc::cvt_color(&resized, &mut rgb, imgproc::COLOR_BGR2RGB, 0, 
                          core::AlgorithmHint::ALGO_HINT_DEFAULT)?;
        let cvt_time = start_cvt.elapsed();

        // Return raw bytes
        if !rgb.is_continuous() {
            return Err(anyhow!("Frame data is not continuous"));
        }
        
        let data = rgb.data_bytes()?;
        let total_time = start_total.elapsed();

        // Log slow frames (> 10ms) to debug.log
        if total_time.as_millis() > 10 {
            use std::fs::OpenOptions;
            use std::io::Write;
            let mut log_path = std::env::current_dir().unwrap_or_default();
            log_path.push("debug.log");

            if let Ok(mut file) = OpenOptions::new().append(true).open(log_path) {
                let _ = writeln!(file, "SLOW FRAME: Total={}us | Decode={}us | Resize={}us | Cvt={}us", 
                    total_time.as_micros(),
                    decode_time.as_micros(),
                    resize_time.as_micros(),
                    cvt_time.as_micros()
                );
            }
        }

        Ok(Some(data.to_vec()))
    }
}