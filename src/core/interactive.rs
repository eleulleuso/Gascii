use anyhow::{Result, Context};
use dialoguer::{theme::ColorfulTheme, Select};
use std::path::{Path, PathBuf};
use std::fs;
use crate::core::display_manager::DisplayMode;
use crate::core::player;
use opencv::prelude::*;

/// 디버그 로그 파일에 메시지를 기록합니다.
#[cfg(target_os = "macos")]
fn write_debug_log(message: &str) {
    use std::io::Write;
    let mut log_path = std::env::current_dir().unwrap_or_default();
    log_path.push("debug.log");
    
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path) 
    {
        let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
        let _ = writeln!(file, "[{}] {}", timestamp, message);
    }
}

pub fn run_interactive_mode() -> Result<()> {
    // 1. Video Selection
    let video_dir = Path::new("assets/vidio");
    if !video_dir.exists() {
        fs::create_dir_all(video_dir)?;
    }
    
    let mut videos: Vec<PathBuf> = fs::read_dir(video_dir)?
        .filter_map(|entry| entry.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().map_or(false, |ext| ext == "mp4" || ext == "mkv" || ext == "avi"))
        .collect();
    
    videos.sort();

    if videos.is_empty() {
        println!("❌ 'assets/vidio' 폴더에 비디오 파일이 없습니다.");
        return Ok(());
    }

    let video_names: Vec<String> = videos.iter()
        .map(|p| p.file_name().unwrap().to_string_lossy().to_string())
        .collect();

    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("비디오 선택")
        .default(0)
        .items(&video_names)
        .interact()?;

    let selected_video = &videos[selection];

    // 2. Audio Selection
    let audio_dir = Path::new("assets/audio");
    if !audio_dir.exists() {
        fs::create_dir_all(audio_dir)?;
    }

    let mut audios: Vec<PathBuf> = fs::read_dir(audio_dir)?
        .filter_map(|entry| entry.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().map_or(false, |ext| ext == "mp3" || ext == "wav"))
        .collect();
    
    audios.sort();

    let mut audio_options = vec!["오디오 없음 / 자동 추출".to_string()];
    audio_options.extend(audios.iter().map(|p| p.file_name().unwrap().to_string_lossy().to_string()));

    let audio_selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("오디오 선택")
        .default(0)
        .items(&audio_options)
        .interact()?;

    let selected_audio = if audio_selection == 0 {
        None
    } else {
        Some(&audios[audio_selection - 1])
    };

    // 3. Render Mode
    let modes = vec!["RGB 컬러 모드 (추천)", "ASCII 텍스트 모드"];
    let mode_selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("렌더링 모드 선택")
        .default(0)
        .items(&modes)
        .interact()?;

    let mode = if mode_selection == 0 { DisplayMode::Rgb } else { DisplayMode::Ascii };

    // 4. Aspect Ratio Mode
    let aspect_modes = vec![
        "Fit (레터박스) - 원본 비율 유지 (검은 여백)",
        "Fill (꽉 찬 화면) - 화면 채우기 (가장자리 잘림)",
        "Stretch (늘리기) - 화면에 맞게 늘리기"
    ];
    let aspect_selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("화면 비율 모드 선택")
        .default(0)
        .items(&aspect_modes)
        .interact()?;
    
    let fill = aspect_selection == 1;

    // 5. Resolution / Fullscreen
    // Get current terminal size
    let (term_cols, term_rows) = crossterm::terminal::size()?;
    println!("ℹ️  Terminal size for rendering: {}x{}", term_cols, term_rows);
    #[cfg(target_os = "macos")]
    write_debug_log(&format!("Terminal size: {}x{}", term_cols, term_rows));
    
    // We treat the terminal as a grid of "Image Pixels".
    // 1 Char Width = 1 Image Pixel Width
    // 1 Char Height = 2 Image Pixel Heights (Half-block rendering)
    // Therefore, Image Pixels are roughly square (10x10).
    
    // Use full terminal size (minus small margin for safety)
    let target_w = (term_cols as u32).saturating_sub(2);
    let target_h = term_rows as u32 * 2; // Pixel height (2x terminal rows for half-block)

    println!("\n🚀 재생 시작: {} ({}x{} 픽셀)", 
        selected_video.file_name().unwrap().to_string_lossy(),
        target_w, target_h
    );

    // Audio extraction logic if needed
    let mut final_audio_path: Option<String> = selected_audio.map(|p| p.to_string_lossy().to_string());
    
    if final_audio_path.is_none() {
        // Try to find extracted audio or extract it
        let video_stem = selected_video.file_stem().unwrap().to_string_lossy();
        let extracted_path = audio_dir.join(format!("{}_extracted.mp3", video_stem));
        
        if extracted_path.exists() {
            final_audio_path = Some(extracted_path.to_string_lossy().to_string());
        } else {
            println!("ℹ️  오디오 추출 중...");
            // Call ffmpeg
            let status = std::process::Command::new("ffmpeg")
                .arg("-i").arg(selected_video)
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

    // === START PRODUCER-CONSUMER IMPLEMENTATION ===
    
    // Initialize display manager
    let mut display = crate::core::display_manager::DisplayManager::new(mode)?;

    // Create video decoder
    // target_h is already in pixel height (term_rows * 2), so don't multiply again
    let decoder = crate::core::video_decoder::VideoDecoder::new(
        &selected_video.to_string_lossy(),
        target_w,
        target_h,  // Already pixel height!
        fill
    )?;
    
    let fps = decoder.get_fps();
    
    // Create bounded channel (120 frames = ~4-5 seconds buffer)
    let (frame_sender, frame_receiver) = crossbeam_channel::bounded(120);
    
    // Spawn decoder thread
    let decoder_handle = decoder.spawn_decoding_thread(frame_sender);
    
    // Start audio playback if available
    let audio_handle = if let Some(audio_path) = final_audio_path {
        use std::process::{Command, Stdio};
        
        let child = Command::new("ffplay")
            .arg("-nodisp")
            .arg("-autoexit")
            .arg("-hide_banner")
            .arg("-loglevel").arg("error")
            .arg(&audio_path)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn();
            
        match child {
            Ok(c) => {
                println!("🔊 오디오 재생 시작");
                Some(c)
            }
            Err(_) => {
                println!("⚠️  ffplay를 찾을 수 없습니다. 오디오 없이 재생합니다.");
                None
            }
        }
    } else {
        None
    };
    
    // Frame processor (expects pixel width and height)
    let processor = crate::core::processor::FrameProcessor::new(target_w as usize, target_h as usize);
    
    // Reusable buffer
    let mut cell_buffer = Vec::new();
    
    // Timing control
    let start_time = std::time::Instant::now();
    let frame_duration = std::time::Duration::from_secs_f64(1.0 / fps);
    let mut next_frame_time = std::time::Instant::now();
    let mut frames = 0;
    
    // CONSUMER LOOP
    for frame_data in frame_receiver {
        // Wait for correct timing
        let now = std::time::Instant::now();
        if now < next_frame_time {
            std::thread::sleep(next_frame_time - now);
        }
        next_frame_time += frame_duration;
        
        // Process frame
        processor.process_frame_into(&frame_data.buffer, &mut cell_buffer);
        
        // Render
        display.render_diff(&cell_buffer, target_w as usize)?;
        
        frames += 1;
    }
    
    // Wait for decoder thread
    let _ = decoder_handle.join();
    
    // Stop audio
    if let Some(mut audio_proc) = audio_handle {
        let _ = audio_proc.kill();
        let _ = audio_proc.wait();
    }
    
    let duration = start_time.elapsed();
    println!("\n✅ 재생 완료: {} 프레임 ({:.2}초, 평균 {:.2} FPS)", 
        frames, duration.as_secs_f64(), frames as f64 / duration.as_secs_f64());

    Ok(())
}
