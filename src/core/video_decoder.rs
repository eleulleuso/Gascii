use anyhow::{Context, Result};
use ffmpeg_sidecar::child::FfmpegChild;
use ffmpeg_sidecar::command::FfmpegCommand;
use std::env;

pub struct VideoDecoder {
    pub child: FfmpegChild,
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
        writeln!(log_file, "=========================\n")?;
        log_file.flush()?;

        command.args(&["-vf", &filter]);
        command.args(&["-f", "rawvideo"]);
        command.args(&["-pix_fmt", "rgb24"]);
        command.output("pipe:");
        command.args(&["-loglevel", "info"]); // Changed to 'info' to capture more details
        
        println!("DEBUG: Starting video decoder (check debug.log for details)");
        
        let mut child = command.spawn().context("Failed to spawn ffmpeg")?;
        
        // Spawn stderr logger thread
        let stderr = child.as_inner_mut().stderr.take();
        if let Some(stderr_pipe) = stderr {
            std::thread::spawn(move || {
                use std::io::{BufRead, BufReader};
                use std::fs::OpenOptions;
                use std::io::Write as IoWrite;
                
                let mut log = OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open("debug.log")
                    .expect("Failed to open debug.log for stderr");
                
                writeln!(log, "\n=== FFmpeg Stderr Output ===").ok();
                log.flush().ok();
                
                let reader = BufReader::new(stderr_pipe);
                for line in reader.lines() {
                    if let Ok(l) = line {
                        writeln!(log, "[FFmpeg] {}", l).ok();
                        log.flush().ok();
                    }
                }
                writeln!(log, "=== FFmpeg Stderr End ===\n").ok();
            });
        }
        
        if child.as_inner().stdout.is_none() {
            anyhow::bail!("FFmpeg stdout not available - pipe failed");
        }
        
        // Final log entry
        let mut log_file = OpenOptions::new()
            .append(true)
            .open("debug.log")?;
        writeln!(log_file, "FFmpeg process started successfully")?;
        writeln!(log_file, "Reading video stream...\n")?;
        
        Ok(Self { child })
    }
}
