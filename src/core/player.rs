use anyhow::{Context, Result};
use ffmpeg_sidecar::command::FfmpegCommand;
use std::io::Read;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use std::thread;

use crate::core::display_manager::{DisplayManager, DisplayMode};
use crate::core::audio_manager::AudioManager;

pub fn play_realtime(
    video_path: &str,
    audio_path: Option<&str>,
    width: u32,
    height: u32,
    fps: u32,
    mode: DisplayMode,
) -> Result<()> {
    // 1. Initialize Display & Audio
    let mut display = DisplayManager::new(mode)?;
    let audio = AudioManager::new()?;

    // 2. Start Audio
    if let Some(path) = audio_path {
        audio.play(path)?;
    }

    // 3. Start Video Decoder (HW Accelerated)
    println!("DEBUG: Initializing Video Decoder...");
    let mut decoder = crate::core::video_decoder::VideoDecoder::new(video_path, width, height, fps)?;
    let mut stdout = decoder.child.take_stdout().context("Failed to take stdout")?;
    println!("DEBUG: Video Decoder Started. Reading stream...");
    
    // 4. Initialize Frame Processor (Rayon)
    // Note: width/height passed here are the "canvas" size (2x terminal size for QuadBlock)
    let processor = crate::core::processor::FrameProcessor::new(width as usize, height as usize);

    let frame_size = (width * height * 3) as usize;
    let mut buffer = vec![0u8; frame_size];

    // 5. Playback Loop
    let frame_duration = Duration::from_secs_f64(1.0 / fps as f64);
    let start_time = Instant::now();
    let mut frame_idx = 0;

    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();

    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
    })?;

    while running.load(Ordering::SeqCst) {
        // ... (Input polling) ...
        if crossterm::event::poll(Duration::from_millis(0))? {
             if let crossterm::event::Event::Key(key) = crossterm::event::read()? {
                 if key.code == crossterm::event::KeyCode::Char('q') {
                     break;
                 }
             }
         }

        // Read frame from FFmpeg
        match stdout.read_exact(&mut buffer) {
            Ok(_) => {
                // ... (Sync & Render) ...
                // Sync Logic (Strict Time-Based)
                let elapsed = start_time.elapsed();
                let expected_time = frame_duration * frame_idx;
                
                if expected_time > elapsed {
                    thread::sleep(expected_time - elapsed);
                }

                // Render
                match mode {
                    DisplayMode::Rgb => {
                        // 1. Process Frame (Parallel Quantization)
                        let cells = processor.process_frame(&buffer);
                        // 2. Render Diff (Optimized Output)
                        // Note: In Half-Block mode, 1 char width = 1 pixel width.
                        // So we pass the FULL width, not width / 2.
                        display.render_diff(&cells, width as usize)?;
                    },
                    DisplayMode::Ascii => {
                        // Legacy ASCII mode is disabled in Ultimate Edition.
                        // Do nothing or log warning.
                    },
                }

                frame_idx += 1;
            }
            Err(e) => {
                if frame_idx == 0 {
                    eprintln!("DEBUG: Failed to read first frame! Error: {}", e);
                    
                    // Try to read stderr from FFmpeg
                    use std::io::Read;
                    if let Some(stderr) = decoder.child.as_inner_mut().stderr.as_mut() {
                        let mut err_msg = String::new();
                        // Read up to 1KB of error log
                        let mut buf = [0u8; 1024];
                        if let Ok(n) = stderr.read(&mut buf) {
                            err_msg = String::from_utf8_lossy(&buf[..n]).to_string();
                        }
                        eprintln!("\n=== FFmpeg Error Log ===\n{}\n========================", err_msg);
                    } else {
                        eprintln!("DEBUG: Could not access FFmpeg stderr.");
                    }
                }
                break; 
            }, // EOF or Error
        }
    }

    Ok(())
}
