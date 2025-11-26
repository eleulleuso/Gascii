use anyhow::{Context, Result};
use ffmpeg_sidecar::child::FfmpegChild;
use ffmpeg_sidecar::command::FfmpegCommand;
use std::io::{BufReader, Write};
use std::thread::{self, JoinHandle};
use std::sync::Arc;
use crate::core::fps_detector::FpsDetector;

pub struct VideoDecoder {
    pub child: FfmpegChild,
    pub stderr_handle: Option<JoinHandle<()>>,
    pub fps_detector: Arc<FpsDetector>,
}

impl VideoDecoder {
    pub fn new(video_path: &str, width: u32, height: u32, fps: u32) -> Result<Self> {
        use std::fs::OpenOptions;
        use std::io::Write as IoWrite;
        
        // Create/truncate log file
        let mut log_file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open("debug.log")
            .context("Failed to create debug.log")?;
        
        writeln!(log_file, "=== Video Decoder Initialization ===")?;
        writeln!(log_file, "BINARY VERSION 3.0 (STEREO3D FIX)")?;
        writeln!(log_file, "Video: {}", video_path)?;
        writeln!(log_file, "Target Resolution: {}x{}", width, height)?;
        writeln!(log_file, "Target FPS: {}", fps)?;
        
        let mut command = FfmpegCommand::new();
        
        // 1. Hardware Acceleration (macOS M-series Optimization)
        // CRITICAL: Must be applied BEFORE input file to act as a Decoder
        if std::env::consts::OS == "macos" {
            println!("DEBUG: Enabling macOS Hardware Acceleration (videotoolbox)");
            command.args(&["-hwaccel", "videotoolbox"]);
        }

        // Input file
        command.input(video_path);

        // 2. Filter Chain (Native FPS - No Interpolation)
        // PERFORMANCE FIX: Removed minterpolate (was causing 0.397x speed bottleneck)
        // Video will play at native 24fps with perfect audio sync
        let filter = format!(
            "scale={}:{}:force_original_aspect_ratio=decrease,pad={}:{}:(ow-iw)/2:(oh-ih)/2,format=rgb24",
            width, height, width, height
        );
        
        writeln!(log_file, "\n=== FFmpeg Filter Chain ===")?;
        writeln!(log_file, "{}", filter)?;
        
        // Log full command for debugging
        writeln!(log_file, "\n=== Full FFmpeg Command ===")?;
        writeln!(log_file, "ffmpeg -hwaccel videotoolbox -i {} -vf \"{}\" -f rawvideo -pix_fmt rgb24 pipe:", 
                 video_path, filter)?;
        writeln!(log_file, "=========================\n")?;
        log_file.flush()?;

        command.args(&["-vf", &filter]);
        command.args(&["-f", "rawvideo"]);
        command.args(&["-pix_fmt", "rgb24"]);
        command.output("pipe:");
        command.args(&["-loglevel", "info"]); // Changed to 'info' to capture more details
        
        println!("DEBUG: Starting video decoder (check debug.log for details)");
        
        let mut child = command.spawn().context("Failed to spawn ffmpeg")?;
        
        // Create FPS detector
        let fps_detector = Arc::new(FpsDetector::new());
        let fps_detector_clone = Arc::clone(&fps_detector);

        // Spawn stderr reader thread to capture and parse FFmpeg output
        let stderr_handle = if let Some(stderr) = child.take_stderr() {
            use std::io::BufRead;
            use std::fs::OpenOptions;
            use std::io::Write as IoWrite;
            use std::io::BufReader;
            use std::thread;
            
            Some(thread::spawn(move || {
                let reader = BufReader::new(stderr);
                let mut log_file = OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open("debug.log")
                    .ok();
                
                if let Some(ref mut log) = log_file {
                    writeln!(log, "\n=== FFmpeg Stderr Output ===").ok();
                }
                
                for line in reader.lines() {
                    if let Ok(line_str) = line {
                        // Parse FPS from stream info
                        fps_detector_clone.parse_fps_from_line(&line_str);
                        
                        if let Some(ref mut log) = log_file {
                            // Format: [FFmpeg] [level] message
                            let level = if line_str.contains("error") || line_str.contains("Error") {
                                "error"
                            } else if line_str.contains("warning") || line_str.contains("Warning") {
                                "warning"
                            } else if line_str.contains("fatal") || line_str.contains("Fatal") {
                                "fatal"
                            } else {
                                "info"
                            };
                            writeln!(log, "[FFmpeg] [{}] {}", level, line_str).ok();
                        }
                    }
                }
                
                if let Some(ref mut log) = log_file {
                    writeln!(log, "=== FFmpeg Stderr End ===\n").ok();
                }
            }))
        } else {
            None
        };
        
        if child.as_inner().stdout.is_none() {
            anyhow::bail!("FFmpeg stdout not available - pipe failed");
        }
        
        // Final log entry
        let mut log_file = OpenOptions::new()
            .append(true)
            .open("debug.log")?;
        writeln!(log_file, "FFmpeg process started successfully")?;
        writeln!(log_file, "Reading video stream...\n")?;
        
        Ok(VideoDecoder {
            child,
            stderr_handle,
        })
    }
}
