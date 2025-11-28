use anyhow::{Result, Context};
use dialoguer::{theme::ColorfulTheme, Select, Input};
use std::path::{Path, PathBuf};
use std::fs;
use crate::core::display_manager::DisplayMode;
use crate::core::player;
use opencv::prelude::*;

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
    
    // [NEW] Resize Font to 2.5 (macOS only) for high resolution
    #[cfg(target_os = "macos")]
    {
        println!("ℹ️  Optimizing terminal resolution (Font Size -> 2.5)...");
        let _ = std::process::Command::new("osascript")
            .arg("-e")
            .arg("tell application \"Terminal\" to set font size of window 1 to 2.5")
            .output();
        
        // Wait for resize to propagate
        std::thread::sleep(std::time::Duration::from_millis(500));
    }

    // Get current terminal size (after resize)
    let (term_cols, term_rows) = crossterm::terminal::size()?;
    
    // We treat the terminal as a grid of "Image Pixels".
    // 1 Char Width = 1 Image Pixel Width
    // 1 Char Height = 2 Image Pixel Heights (Half-block rendering)
    // Therefore, Image Pixels are roughly square (10x10).
    
    let max_w = (term_cols as u32).saturating_sub(2);
    let max_h = term_rows as u32 * 2;

    let (mut target_w, mut target_h) = (max_w, max_h);

    if aspect_selection == 0 { // Fit (Original Ratio)
        // Probe video for aspect ratio
        let mut video_w = 1920.0;
        let mut video_h = 1080.0;
        
        // Use OpenCV to get video dimensions
        if let Ok(mut capture) = opencv::videoio::VideoCapture::from_file(selected_video.to_str().unwrap(), opencv::videoio::CAP_ANY) {
             if let Ok(w) = capture.get(opencv::videoio::CAP_PROP_FRAME_WIDTH) {
                 if w > 0.0 { video_w = w; }
             }
             if let Ok(h) = capture.get(opencv::videoio::CAP_PROP_FRAME_HEIGHT) {
                 if h > 0.0 { video_h = h; }
             }
        }

        let target_ratio = video_w / video_h;
        let current_ratio = max_w as f64 / max_h as f64;
        
        if current_ratio > target_ratio {
            // Terminal is wider than video -> Limit by height
            target_h = max_h;
            target_w = (max_h as f64 * target_ratio) as u32;
        } else {
            // Terminal is taller/narrower than video -> Limit by width
            target_w = max_w;
            target_h = (max_w as f64 / target_ratio) as u32;
        }
    } else {
        // Fill (1) or Stretch (2)
        // Use full available terminal space
        // Fill mode logic in player.rs will handle cropping if needed
        // Stretch mode will just stretch to this size
        target_w = max_w;
        target_h = max_h;
    }

    // Ensure even dimensions for half-block rendering
    if target_w % 2 != 0 { target_w -= 1; }
    if target_h % 2 != 0 { target_h -= 1; }

    println!("\n🚀 재생 시작: {} ({}x{})", 
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

    let video_path_str = selected_video.to_string_lossy();
    player::play_realtime(
        &video_path_str,
        final_audio_path.as_deref(),
        target_w,
        target_h,
        0, // 0 means native fps
        mode,
        fill
    )?;

    Ok(())
}
