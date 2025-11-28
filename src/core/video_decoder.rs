use anyhow::{Result, anyhow};
use opencv::{
    prelude::*,
    videoio,
    imgproc,
    core,
};

pub struct VideoDecoder {
    capture: videoio::VideoCapture,
    width: u32,
    height: u32,
    fps: f64,
}

impl VideoDecoder {
    pub fn new(path: &str, width: u32, height: u32) -> Result<Self> {
        println!("DEBUG: Opening video with OpenCV: {}", path);
        
        // CAP_ANY allows OpenCV to choose the best backend (AVFoundation on macOS, MSMF on Windows)
        let capture = videoio::VideoCapture::from_file(path, videoio::CAP_ANY)?;
        
        if !capture.is_opened()? {
            return Err(anyhow!("Failed to open video file: {}", path));
        }

        let fps = capture.get(videoio::CAP_PROP_FPS)?;
        println!("DEBUG: OpenCV VideoCapture opened successfully. Detected FPS: {}", fps);

        Ok(Self {
            capture,
            width,
            height,
            fps,
        })
    }

    pub fn get_fps(&self) -> f64 {
        self.fps
    }

    pub fn read_frame(&mut self) -> Result<Option<Vec<u8>>> {
        let mut frame = Mat::default();
        
        // Read frame (decoding happens here, likely on GPU if backend supports it)
        if !self.capture.read(&mut frame)? {
            return Ok(None); // EOF
        }
        
        if frame.empty() {
            return Ok(None);
        }

        // Resize (CPU)
        let mut resized = Mat::default();
        let size = core::Size::new(self.width as i32, self.height as i32);
        imgproc::resize(&frame, &mut resized, size, 0.0, 0.0, imgproc::INTER_LINEAR)?;

        // Convert BGR to RGB (CPU)
        let mut rgb = Mat::default();
        imgproc::cvt_color(&resized, &mut rgb, imgproc::COLOR_BGR2RGB, 0, core::AlgorithmHint::ALGO_HINT_DEFAULT)?;

        // Return raw bytes
        if !rgb.is_continuous() {
            return Err(anyhow!("Frame data is not continuous"));
        }
        
        let data = rgb.data_bytes()?;
        Ok(Some(data.to_vec()))
    }
}