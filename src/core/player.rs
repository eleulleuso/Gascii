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

    // 3. Start Video Decoder
    println!("Initializing video decoder...");
    let mut decoder = crate::core::video_decoder::VideoDecoder::new(video_path, width, height, fps)?;
    let mut stdout = decoder.child.take_stdout().context("Failed to take stdout")?;
    println!("Video decoder started. Check debug.log for details.");
    
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
                use std::fs::OpenOptions;
                use std::io::Write as IoWrite;
                
                if let Ok(mut log) = OpenOptions::new().append(true).open("debug.log") {
                    writeln!(log, "\n=== Playback Error ===").ok();
                    writeln!(log, "read_exact() failed at frame {}", frame_idx).ok();
                    writeln!(log, "Error: {}", e).ok();
                    
                    if frame_idx == 0 {
                        writeln!(log, "\nCRITICAL: Failed to read first frame!").ok();
                        writeln!(log, "This means FFmpeg produced no output.").ok();
                        writeln!(log, "Possible causes:").ok();
                        writeln!(log, "  1. Invalid filter syntax").ok();
                        writeln!(log, "  2. Video codec not supported").ok();
                        writeln!(log, "  3. File read error").ok();
                        writeln!(log, "  4. stereo3d filter requires Side-by-Side input").ok();
                        eprintln!("\n❌ ERROR: Failed to start playback. Check debug.log for details.");
                    } else {
                        writeln!(log, "Total frames rendered: {}", frame_idx).ok();
                        eprintln!("\n✓ Playback ended at frame {}. Check debug.log for details.", frame_idx);
                    }
                    writeln!(log, "=====================\n").ok();
                }
                break; 
            },
        }
    }

    Ok(())
}
