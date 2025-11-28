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
        
        // 1. Hardware Acceleration & Filter Chain Strategy
        // We detect the OS and apply the best available hardware acceleration.
        // NOTE: We use explicit device initialization (-init_hw_device) instead of -hwaccel
        // to avoid argument conflicts and ensure hwupload has a valid device reference.
        
        let os = std::env::consts::OS;
        let mut filter = String::new();
        
        // Hack for specific 3D video (bochi.mp4) requested by user
        let crop_filter = if video_path.to_lowercase().contains("bochi") {
            "crop=iw/2:ih:0:0,"
        } else {
            ""
        };

        if os == "macos" {
            println!("DEBUG: Enabling macOS Hardware Acceleration (videotoolbox via filter)");
            // macOS: VideoToolbox
            // 1. Init device named 'vt'
            // 2. Bind 'vt' to filter graph
            command.args(&["-init_hw_device", "videotoolbox=vt"]);
            command.args(&["-filter_hw_device", "vt"]);
            
            filter = format!(
                "{}hwupload,scale_vt=w={}:h={},hwdownload,format=pix_fmts=rgb24",
                crop_filter, width, height
            );
        } else if os == "windows" {
            println!("DEBUG: Enabling Windows Hardware Acceleration (d3d11va via filter)");
            // Windows: D3D11VA
            command.args(&["-init_hw_device", "d3d11va=d3d11"]);
            command.args(&["-filter_hw_device", "d3d11"]);
            
            filter = format!(
                "{}hwupload,scale_d3d11=w={}:h={},hwdownload,format=pix_fmts=rgb24",
                crop_filter, width, height
            );
        } else if os == "linux" {
            println!("DEBUG: Enabling Linux Hardware Acceleration (vaapi via filter)");
            // Linux: VAAPI
            command.args(&["-init_hw_device", "vaapi=va:/dev/dri/renderD128"]);
            command.args(&["-filter_hw_device", "va"]);
            
            filter = format!(
                "{}hwupload,scale_vaapi=w={}:h={},hwdownload,format=pix_fmts=rgb24",
                crop_filter, width, height
            );
        } else {
            println!("DEBUG: Using CPU Decoding (Fallback)");
            // Fallback: CPU Lanczos
            filter = format!(
                "scale={}:{}:flags=lanczos:force_original_aspect_ratio=decrease,pad={}:{}:(ow-iw)/2:(oh-ih)/2,format=pix_fmts=rgb24",
                width, height, width, height
            );
        }

        // 2. Apply Input and Arguments
        // CRITICAL: Must specify input BEFORE output options
        command.input(video_path);
        
        command.args(&["-vf", &filter]);
        command.args(&["-f", "rawvideo"]);
        command.args(&["-pix_fmt", "rgb24"]);
        
        // 3. Logging
        writeln!(log_file, "\n=== FFmpeg Filter Chain ===")?;
        writeln!(log_file, "{}", filter)?;
        
        writeln!(log_file, "\n=== Full FFmpeg Command ===")?;
        writeln!(log_file, "ffmpeg -i {} -vf \"{}\" -f rawvideo -pix_fmt rgb24 pipe:", 
                 video_path, filter)?;
        writeln!(log_file, "=========================\n")?;
        log_file.flush()?;
        command.output("pipe:");
        command.args(&["-loglevel", "info"]); // Changed to 'info' to capture more details
        
        println!("DEBUG: Starting video decoder (check debug.log for details)");
        
        let mut child = command.spawn().context("Failed to spawn ffmpeg")?;
        
        // Create FPS detector
        let fps_detector = Arc::new(FpsDetector::new());
        let fps_detector_clone = Arc::clone(&fps_detector);

        // Spawn stderr reader thread to capture and parse FFmpeg output
        let stderr_handle = if let Some(stderr) = child.as_inner_mut().stderr.take() {
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
            fps_detector,
        })
    }
}
