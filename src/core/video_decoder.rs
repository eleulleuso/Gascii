use anyhow::{Context, Result};
use ffmpeg_sidecar::child::FfmpegChild;
use ffmpeg_sidecar::command::FfmpegCommand;
use std::env;

pub struct VideoDecoder {
    pub child: FfmpegChild,
}

impl VideoDecoder {
    pub fn new(video_path: &str, width: u32, height: u32, fps: u32) -> Result<Self> {
        let mut command = FfmpegCommand::new();
        command.input(video_path);

        // 1. Hardware Acceleration Detection
        // Disabled for stability debugging. 
        // If software decoding works, we can re-enable it later.
        /*
        let os = env::consts::OS;
        match os {
            "macos" => {
                // macOS: VideoToolbox
                command.args(&["-hwaccel", "videotoolbox"]);
            }
            "windows" => {
                // Windows: D3D11VA or CUDA (auto is safest fallback)
                command.args(&["-hwaccel", "auto"]);
            }
            "linux" => {
                // Linux: VAAPI or CUDA (auto is safest fallback)
                command.args(&["-hwaccel", "auto"]);
            }
            _ => {}
        }
        */

        // 2. Filter Chain
        // We need to scale the video to the target terminal resolution.
        // For Quad-Block (2x2), the target resolution is width x height.
        // Note: The 'width' passed here should already be the doubled width from play.sh
        
        // Aspect Ratio Correction is handled by scaling to the target WxH.
        // We use Lanczos for high quality downscaling.
        // CRITICAL: Force output to 'rgb24' in the filter chain to ensure pixel format matches.
        // 2. Add Filters (Scale, Pad, Pixel Format, Unsharp, Interpolate)
        // We use 'crop' to handle 3D SBS sources (taking left eye).
        // We use 'minterpolate' for 120fps motion smoothing.
        // Optimization: CROP FIRST to reduce pixels for interpolation by 50%.
        let filter = format!(
            "crop=iw/2:ih:0:0,minterpolate=fps=120:mi_mode=mci:mc_mode=aobmc:me_mode=bidir,scale={}x{}:force_original_aspect_ratio=decrease,pad={}:{}:(ow-iw)/2:(oh-ih)/2,unsharp=5:5:1.0:5:5:0.0,format=rgb24",
            width, height, width, height
        );

        command.args(&["-vf", &filter]);
        
        // 3. Output Format
        // Raw RGB24 video pipe to stdout
        command.args(&["-f", "rawvideo"]);
        command.args(&["-pix_fmt", "rgb24"]);
        command.output("pipe:");

        // Debug: Print estimated command
        println!("DEBUG: Starting FFmpeg with filter: {}", filter);

        // Enable stderr capturing
        command.args(&["-loglevel", "error"]); // Only show errors
        
        // Note: ffmpeg-sidecar handles stderr internally usually, but let's be explicit if needed.
        // Actually, FfmpegCommand doesn't expose stderr pipe easily in the builder?
        // Let's check FfmpegChild.
        
        let mut child = command.spawn().context("Failed to spawn ffmpeg")?;
        
        // Ensure stdout is available
        if child.as_inner().stdout.is_none() {
            anyhow::bail!("FFmpeg stdout not available - pipe failed");
        }
        
        Ok(Self { child })
    }
}
