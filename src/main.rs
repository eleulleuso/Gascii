mod core;
mod utils;

use clap::{Parser, Subcommand};
use anyhow::{Result, Context};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use std::thread;
use crossterm::event::{self, Event, KeyCode};

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
        #[arg(short, long, default_value_t = 265)]
        width: u32,
        #[arg(short, long, default_value_t = 65)]
        height: u32,
        #[arg(short, long, default_value_t = 0)]
        fps: u32,
        #[arg(short, long, value_enum, default_value_t = DisplayMode::Rgb)]
        mode: DisplayMode,
    },
    /// Detect platform info
    Detect,
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
        Commands::PlayLive { video, audio, width, height, fps, mode } => {
            crate::core::player::play_realtime(video, audio.as_deref(), *width, *height, *fps, *mode)?;
        }
        Commands::Detect => {
            let info = crate::utils::platform::PlatformInfo::detect()?;
            println!("{}", serde_json::to_string_pretty(&info)?);
        }
    }

    Ok(())
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

    // 4. Playback Loop
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
        if let Some(frame_data) = frames.get_frame(i) {
            display.render_frame(frame_data.as_slice())?;
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
