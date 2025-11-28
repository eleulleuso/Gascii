mod core;
mod utils;

use clap::{Parser, Subcommand};
use anyhow::{Result, Context};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use std::thread;
use crossterm::event::{self, Event, KeyCode};
use serde_json::json;

use crate::core::display_manager::{DisplayManager, DisplayMode};
use crate::core::audio_manager::AudioManager;
use crate::core::frame_manager::FrameManager;
use crate::core::extractor;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Extract frames from video
    Extract {
        #[arg(short, long)]
        input: String,
        #[arg(short, long)]
        output_dir: String,
        #[arg(short, long, default_value_t = 265)]
        width: u32,
        #[arg(short, long, default_value_t = 65)]
        height: u32,
        #[arg(short, long, default_value_t = 60)]
        fps: u32,
    },
    /// Play the animation
    Play {
        #[arg(short, long)]
        frames_dir: String,
        #[arg(short, long)]
        audio: Option<String>,
        #[arg(short, long, default_value_t = 60)]
        fps: u32,
        #[arg(short, long, value_enum, default_value_t = DisplayMode::Rgb)]
        mode: DisplayMode,
    },
    /// Play video directly (real-time, no extraction)
    PlayLive {
        #[arg(short, long)]
        video: String,
        #[arg(short, long)]
        audio: Option<String>,
        #[arg(short, long, default_value_t = 265, help = "Requested width in pixels for video scaling (applies to the decoder and processor)")]
        width: u32,
        #[arg(short, long, default_value_t = 65, help = "Requested height in pixels for video scaling (applies to the decoder and processor)")]
        height: u32,
        #[arg(short, long, default_value_t = 0)]
        fps: u32,
        #[arg(short, long, value_enum, default_value_t = DisplayMode::Rgb)]
        mode: DisplayMode,
        #[arg(short, long, default_value_t = false, help = "If true, Fill mode: crop to fill 16:9 box (center crop)")]
        fill: bool,
    },
    /// Detect platform info
    Detect,
    /// Query the terminal size as crossterm sees it
    TerminalSize,
    /// Interactive Mode (Menu)
    Interactive,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Extract { input, output_dir, width, height, fps } => {
            extractor::extract_frames(input, output_dir, *width, *height, *fps)?;
        }
        Commands::Play { frames_dir, audio, fps, mode } => {
            play_animation(frames_dir, audio.as_deref(), *fps, *mode)?;
        }
        Commands::PlayLive { video, audio, width, height, fps, mode, fill } => {
            crate::core::player::play_realtime(video, audio.as_deref(), *width, *height, *fps, *mode, *fill)?;
        }
        Commands::Detect => {
            let info = crate::utils::platform::PlatformInfo::detect()?;
            println!("{}", serde_json::to_string_pretty(&info)?);
        }
        Commands::TerminalSize => {
            let (raw_cols, raw_rows) = crossterm::terminal::size()?;
            let (cols, rows) = normalize_terminal_size(raw_cols, raw_rows);
            println!("{}", json!({
                "columns": cols,
                "rows": rows,
                "raw_columns": raw_cols,
                "raw_rows": raw_rows,
            }));
        }
        Commands::Interactive => {
            crate::core::interactive::run_interactive_mode()?;
        }
    }

    Ok(())
}

fn normalize_terminal_size(raw_cols: u16, raw_rows: u16) -> (u16, u16) {
    let char_width = std::env::var("CHAR_WIDTH").ok().and_then(|v| v.parse::<u16>().ok());
    let char_height = std::env::var("CHAR_HEIGHT").ok().and_then(|v| v.parse::<u16>().ok());
    if let (Some(cw), Some(ch)) = (char_width, char_height) {
        if cw > 0 && ch > 0 {
            // If the reported size is unusually large, treat as pixels and convert to cols/rows
            if raw_cols >= cw.saturating_mul(32) && raw_rows >= ch.saturating_mul(16) {
                return (raw_cols / cw.max(1), raw_rows / ch.max(1));
            }
        }
    }
    (raw_cols, raw_rows)
}

fn play_animation(frames_dir: &str, audio_path: Option<&str>, fps: u32, mode: DisplayMode) -> Result<()> {
    // 1. Initialize Managers
    let mut display = DisplayManager::new(mode)?;
    let mut frames = FrameManager::new();
    let audio = AudioManager::new()?;

    // 2. Load Frames
    // Extractor always produces .bin (raw RGB with header)
    let ext = "bin";
    frames.load_frames(frames_dir, ext)?;

    if frames.frame_count() == 0 {
        anyhow::bail!("No frames found in {}", frames_dir);
    }

    // 3. Start Audio
    if let Some(path) = audio_path {
        audio.play(path)?;
    }

    // 4. Initialize Frame Processor (based on first frame header) and Playback Loop
    // We will infer width/height from the first frame header if possible
    let mut processor_opt: Option<crate::core::processor::FrameProcessor> = None;
    let frame_duration = Duration::from_secs_f64(1.0 / fps as f64);
    let start_time = Instant::now();
    
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();

    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
    }).context("Error registering Ctrl-C handler")?;

    for i in 0..frames.frame_count() {
        if !running.load(Ordering::SeqCst) {
            break;
        }

        // Sync
        let elapsed = start_time.elapsed();
        let expected_time = frame_duration * i as u32;

        if elapsed < expected_time {
            thread::sleep(expected_time - elapsed);
        } else if elapsed > expected_time + Duration::from_millis(50) {
            continue; // Skip frame
        }

        // Render
        if let Some(frame_data_arc) = frames.get_frame(i) {
            // frame_data is [width(u16)][height(u16)][R,G,B...]
            let frame_slice = frame_data_arc.as_slice();
            if frame_slice.len() >= 4 {
                let w = u16::from_le_bytes([frame_slice[0], frame_slice[1]]) as usize;
                let h = u16::from_le_bytes([frame_slice[2], frame_slice[3]]) as usize;
                let pixel_data = &frame_slice[4..];

                // Initialize processor if not set
                if processor_opt.is_none() {
                    processor_opt = Some(crate::core::processor::FrameProcessor::new(w, h));
                }

                if let Some(processor) = processor_opt.as_ref() {
                    let cells = processor.process_frame(pixel_data);
                    display.render_diff(&cells, w)?;
                }
            }
        }

        // Input
        if event::poll(Duration::from_millis(0))? {
            if let Event::Key(key) = event::read()? {
                if key.code == KeyCode::Char('q') {
                    break;
                }
            }
        }
    }

    Ok(())
}
