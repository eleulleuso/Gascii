use anyhow::{Context, Result};
use ffmpeg_sidecar::child::FfmpegChild;
use ffmpeg_sidecar::command::FfmpegCommand;
use std::io::Write;
use std::thread::JoinHandle;
use std::sync::Arc;
use crate::core::fps_detector::FpsDetector;

pub struct VideoDecoder {
    pub child: FfmpegChild,
    pub stderr_handle: Option<JoinHandle<()>>,
    pub fps_detector: Arc<FpsDetector>,
}

impl VideoDecoder {
    pub fn new(video_path: &str, width: u32, height: u32, _fps: u32) -> Result<Self> {
        let mut command = FfmpegCommand::new();
        
        let os = std::env::consts::OS;
        let mut filter = String::new();
        
        // Hack for specific 3D video (bochi.mp4)
        let crop_filter = if video_path.to_lowercase().contains("bochi") {
            "crop=iw/2:ih:0:0,"
        } else {
            ""
        };

        if os == "macos" {
            println!("DEBUG: Enabling macOS Hardware Acceleration (videotoolbox via filter)");
            command.args(&["-init_hw_device", "videotoolbox=vt"]);
            command.args(&["-filter_hw_device", "vt"]);
            
            // CRITICAL FIX: Use scale_videotoolbox (full name) and format=pix_fmts=rgb24
            filter = format!(
                "{}hwupload,scale_videotoolbox=w={}:h={},hwdownload,format=pix_fmts=rgb24",
                crop_filter, width, height
            );
        } else if os == "windows" {
            println!("DEBUG: Enabling Windows Hardware Acceleration (d3d11va via filter)");
            command.args(&["-init_hw_device", "d3d11va=d3d11"]);
            command.args(&["-filter_hw_device", "d3d11"]);
            
            filter = format!(
                "{}hwupload,scale_d3d11=w={}:h={},hwdownload,format=pix_fmts=rgb24",
                crop_filter, width, height
            );
        } else if os == "linux" {
            println!("DEBUG: Enabling Linux Hardware Acceleration (vaapi via filter)");
            command.args(&["-init_hw_device", "vaapi=va:/dev/dri/renderD128"]);
            command.args(&["-filter_hw_device", "va"]);
            
            filter = format!(
                "{}hwupload,scale_vaapi=w={}:h={},hwdownload,format=pix_fmts=rgb24",
                crop_filter, width, height
            );
        } else {
            println!("DEBUG: Using CPU Decoding (Fallback)");
            filter = format!(
                "scale={}:{}:flags=lanczos:force_original_aspect_ratio=decrease,pad={}:{}:(ow-iw)/2:(oh-ih)/2,format=pix_fmts=rgb24",
                width, height, width, height
            );
        }

        command.input(video_path);
        command.args(&["-vf", &filter]);
        command.args(&["-f", "rawvideo"]);
        command.args(&["-pix_fmt", "rgb24"]);

        // Logging setup
        use std::fs::OpenOptions;
        let log_file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open("debug.log");

        if let Ok(mut file) = log_file {
            writeln!(file, "=== Video Decoder Initialization ===").ok();
            writeln!(file, "Video: {}", video_path).ok();
            writeln!(file, "Target Resolution: {}x{}", width, height).ok();
            writeln!(file, "=== FFmpeg Filter Chain ===").ok();
            writeln!(file, "{}", filter).ok();
            writeln!(file, "=========================\n").ok();
        }

        command.output("pipe:");
        command.args(&["-loglevel", "info"]);
        
        println!("DEBUG: Starting video decoder (check debug.log for details)");
        
        let mut child = command.spawn().context("Failed to spawn ffmpeg process")?;
        
        let fps_detector = Arc::new(FpsDetector::new());
        let fps_detector_clone = Arc::clone(&fps_detector);

        // Spawn stderr reader thread
        let stderr_handle = if let Some(stderr) = child.as_inner_mut().stderr.take() {
            use std::io::BufRead;
            use std::io::BufReader;
            let reader = BufReader::new(stderr);
            
            Some(std::thread::spawn(move || {
                let mut log_file = OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open("debug.log")
                    .ok();

                for line in reader.lines() {
                    if let Ok(line_str) = line {
                        fps_detector_clone.parse_fps_from_line(&line_str);
                        
                        if let Some(ref mut log) = log_file {
                            let level = if line_str.contains("error") || line_str.contains("Error") {
                                "error"
                            } else if line_str.contains("warning") || line_str.contains("Warning") {
                                "warning"
                            } else {
                                "info"
                            };
                            writeln!(log, "[FFmpeg] [{}] {}", level, line_str).ok();
                        }
                    }
                }
                if let Some(ref mut log) = log_file {
                    writeln!(log, "=== FFmpeg Stderr End ===").ok();
                }
            }))
        } else {
            None
        };

        Ok(Self {
            child,
            stderr_handle,
            fps_detector,
        })
    }
}