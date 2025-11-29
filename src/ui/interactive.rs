use anyhow::Result;
use std::path::{Path, PathBuf};
use std::fs;
use crossterm::{
    event::{self, Event, KeyCode},
};
use std::time::Duration;

// Direct module imports
use crate::renderer::{DisplayManager, DisplayMode, FrameProcessor};
use crate::renderer::cell::CellData;
use crate::decoder::VideoDecoder;
use crate::audio::AudioPlayer;
use crate::sync::MasterClock;

/// 디버그 로그 파일에 메시지를 기록합니다.

pub fn run_game(
    video_path: PathBuf,
    audio_path: Option<PathBuf>,
    mode: DisplayMode,
    fill_screen: bool
) -> Result<()> {
    // 1. Terminal Setup
    let (terminal_w, terminal_h) = {
        let size = crossterm::terminal::size()?;
        (size.0 as u32, size.1 as u32)
    };

    // Calculate target dimensions
    // We want 16:9 aspect ratio if not filling screen
    let (target_w, target_h) = if fill_screen {
        (terminal_w, terminal_h)
    } else {
        let target_ratio = 16.0 / 9.0;
        let terminal_ratio = terminal_w as f32 / terminal_h as f32;
        
        let (w, h) = if terminal_ratio > target_ratio {
            // Terminal is wider -> fit to height
            let h = terminal_h;
            let w = (h as f32 * target_ratio) as u32;
            (w, h)
        } else {
            // Terminal is taller -> fit to width
            let w = terminal_w;
            let h = (w as f32 / target_ratio) as u32;
            (w, h)
        };
        (w.saturating_sub(2), h)
    };

    println!("\n🚀 재생 시작: {} ({}x{} 픽셀, {})", 
        video_path.file_name().unwrap().to_string_lossy(),
        target_w, target_h,
        if fill_screen { "전체화면" } else { "16:9" }
    );

    // Audio extraction logic if needed
    let mut final_audio_path: Option<String> = audio_path.map(|p| p.to_string_lossy().to_string());
    
    // If audio not provided, try to find extracted audio or extract it
    if final_audio_path.is_none() {
        let audio_dir = Path::new("assets/audio");
        if !audio_dir.exists() {
            fs::create_dir_all(audio_dir)?;
        }

        let video_stem = video_path.file_stem().unwrap().to_string_lossy();
        let extracted_path = audio_dir.join(format!("{}_extracted.mp3", video_stem));
        
        if extracted_path.exists() {
            final_audio_path = Some(extracted_path.to_string_lossy().to_string());
        } else {
            println!("ℹ️  오디오 추출 중...");
            // Call ffmpeg
            let status = std::process::Command::new("ffmpeg")
                .arg("-i").arg(&video_path)
                .arg("-vn")
                .arg("-acodec").arg("libmp3lame")
                .arg("-q:a").arg("2")
                .arg(&extracted_path)
                .arg("-y")
                .arg("-hide_banner")
                .arg("-loglevel").arg("error")
                .status();
                
            if let Ok(s) = status {
                if s.success() {
                    println!("✅ 오디오 추출 완료");
                    final_audio_path = Some(extracted_path.to_string_lossy().to_string());
                } else {
                    println!("⚠️  오디오 추출 실패 (ffmpeg 에러)");
                }
            } else {
                println!("⚠️  ffmpeg를 찾을 수 없습니다. 오디오 없이 재생합니다.");
            }
        }
    }

    // === START PRODUCER-CONSUMER IMPLEMENTATION WITH SYNC ===
    
    // Initialize display manager
    let mut display = DisplayManager::new(mode)?;

    // Create video decoder
    // IMPORTANT: We use Half-Block rendering, so vertical resolution is 2x terminal rows
    let pixel_w = target_w;
    let pixel_h = target_h * 2;
    
    let decoder = VideoDecoder::new(
        &video_path.to_string_lossy(),
        pixel_w,
        pixel_h,
        fill_screen
    )?;
    
    let fps = decoder.get_fps();
    
    // Create bounded channel (120 frames = ~4-5 seconds buffer)
    let (frame_sender, frame_receiver) = crossbeam_channel::bounded(120);
    
    // Spawn decoder thread
    let decoder_handle = decoder.spawn_decoding_thread(frame_sender);
    
    // === SYNC SYSTEM ===
    let clock = MasterClock::new();
    
    // Frame processor (expects pixel width and height)
    let processor = FrameProcessor::new(pixel_w as usize, pixel_h as usize);
    
    // Reusable buffer (pre-allocate with correct size for half-block rendering)
    let term_height = (pixel_h / 2) as usize;
    let mut cell_buffer = vec![CellData { char: ' ', fg: (0,0,0), bg: (0,0,0) }; pixel_w as usize * term_height];
    
    crate::utils::logger::debug(&format!("Initialized cell buffer: {}x{} terminal = {} cells", 
        pixel_w, term_height, cell_buffer.len()));
    
    // Performance tracking
    let start_time = std::time::Instant::now();
    let frame_duration = std::time::Duration::from_secs_f64(1.0 / fps);
    
    // Adaptive frame skip with EWMA
    let mut avg_frame_time = frame_duration;
    const EWMA_ALPHA: f64 = 0.3; // Weight for new samples
    
    // CONSUMER LOOP WITH ABSOLUTE TIMING (Drift-free)
    let mut frame_idx = 0u64;
    let mut frames_dropped = 0u64;
    let mut audio_player = None; // Will start after first frame
    
    crate::utils::logger::debug("Starting render loop");
    
    loop {
        // Input
        if event::poll(Duration::from_millis(0))? {
            if let Event::Key(key) = event::read()? {
                if key.code == KeyCode::Char('q') || key.code == KeyCode::Esc {
                    break;
                }
            }
        }
        
        // Receive frame
        let frame_data = match frame_receiver.recv() {
            Ok(f) => f,
            Err(_) => break, // Channel closed (EOF)
        };
        
        // Sync Logic
        let expected_time = frame_duration.mul_f64(frame_idx as f64);
        let elapsed = clock.elapsed();
        
        // Adaptive threshold based on recent performance
        let drop_threshold = if avg_frame_time > frame_duration.mul_f64(1.2) {
            1  // Aggressive drop if consistently slow
        } else {
            3  // Conservative if normal
        };
        
        // Check how far behind we are
        let behind_frames = if elapsed > expected_time {
            ((elapsed - expected_time).as_secs_f64() / frame_duration.as_secs_f64()) as u64
        } else {
            0
        };
        
        // Drift correction: sleep until the expected time
        if elapsed < expected_time {
            std::thread::sleep(expected_time - elapsed);
        } else if behind_frames > drop_threshold {
            // More than threshold frames behind → skip this frame
            frames_dropped += 1;
            frame_idx += 1;
            continue;
        }
        
        // Process frame
        if frame_idx % 60 == 0 {
            crate::utils::logger::debug(&format!("Frame {}: buffer_len={}, processing...", 
                frame_idx, frame_data.buffer.len()));
        }
        processor.process_frame_into(&frame_data.buffer, &mut cell_buffer);
        
        // Render
        if let Err(e) = display.render_diff(&cell_buffer, target_w as usize) {
            crate::utils::logger::error(&format!("Render error: {}", e));
            return Err(e);
        }
        
        if frame_idx % 60 == 0 {
            crate::utils::logger::debug(&format!("Frame {} rendered successfully", frame_idx));
        }
        
        // Start audio AFTER first frame is rendered (for sync)
        if audio_player.is_none() {
            if let Some(audio_path) = final_audio_path.as_ref() {
                match AudioPlayer::new(audio_path) {
                    Ok(player) => {
                        crate::utils::logger::debug("Audio started (synced)");
                        audio_player = Some(player);
                    }
                    Err(e) => {
                        crate::utils::logger::error(&format!("Audio failed: {}", e));
                    }
                }
            }
        }
        
        // Update moving average frame time
        let frame_end = clock.elapsed();
        let frame_time = frame_end - elapsed;
        avg_frame_time = Duration::from_secs_f64(
            avg_frame_time.as_secs_f64() * (1.0 - EWMA_ALPHA) + 
            frame_time.as_secs_f64() * EWMA_ALPHA
        );
        
        frame_idx += 1;
    }
    
    // Cleanup
    crate::utils::logger::debug(&format!("Render loop ended. Frames: {}, Dropped: {}", frame_idx, frames_dropped));
    
    // Wait for decoder thread
    let _ = decoder_handle.join();
    
    // Stop audio
    drop(audio_player);
    
    let duration = start_time.elapsed();
    println!("\n✅ 재생 완료: (Absolute Timing - Drift-free)");
    println!("   • 렌더링: {} 프레임", frame_idx);
    println!("   • 드롭: {} 프레임", frames_dropped);
    println!("   • 재생 시간: {:.2}초", duration.as_secs_f64());
    println!("   • 평균 FPS: {:.2}", frame_idx as f64 / duration.as_secs_f64());

    Ok(())
}
    