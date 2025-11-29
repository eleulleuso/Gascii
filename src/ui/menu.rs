use anyhow::Result;
use dialoguer::{theme::ColorfulTheme, Select};
use std::path::{Path, PathBuf};
use std::fs;

pub fn run_menu() -> Result<()> {
    // 1. Scan for video files
    let video_dirs = vec![Path::new("assets/video"), Path::new("assets/vidio")];
    let mut video_dir = Path::new("assets/video");
    let mut found_dir = false;

    for dir in &video_dirs {
        if dir.exists() {
            video_dir = dir;
            found_dir = true;
            break;
        }
    }
    
    if !found_dir {
        eprintln!("❌ assets/video (또는 assets/vidio) 디렉토리를 찾을 수 없습니다.");
        return Ok(());
    }

    let audio_dir = Path::new("assets/audio");

    let mut video_files: Vec<PathBuf> = fs::read_dir(video_dir)?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| {
            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
            matches!(ext.as_str(), "mp4" | "mkv" | "avi" | "mov" | "webm")
        })
        .collect();

    video_files.sort();

    if video_files.is_empty() {
        eprintln!("❌ 재생할 비디오 파일이 없습니다.");
        return Ok(());
    }

    // 2. Select Video
    let video_names: Vec<String> = video_files.iter()
        .map(|p| p.file_name().unwrap().to_string_lossy().to_string())
        .collect();

    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("📺 재생할 영상을 선택하세요")
        .default(0)
        .items(&video_names)
        .interact()?;

    let selected_video = &video_files[selection];

    // 3. Select Audio (Optional)
    // Try to find matching audio
    let video_stem = selected_video.file_stem().unwrap().to_string_lossy();
    let expected_audio = audio_dir.join(format!("{}.mp3", video_stem));
    
    let audio_path = if expected_audio.exists() {
        Some(expected_audio)
    } else {
        None
    };

    // 4. Select Mode
    let modes = vec!["RGB TrueColor (최고 화질)", "ASCII (텍스트 모드)"];
    let mode_selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("🎨 렌더링 모드 선택")
        .default(0)
        .items(&modes)
        .interact()?;

    let mode_str = if mode_selection == 0 { "rgb" } else { "ascii" };

    // 5. Select Screen Mode
    let screen_modes = vec!["전체 화면 (꽉 차게)", "원본 비율 (16:9)"];
    let screen_selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("🖥️ 화면 모드 선택")
        .default(0)
        .items(&screen_modes)
        .interact()?;

    let fill_str = if screen_selection == 0 { "true" } else { "false" };

    // Output for shell script to parse
    println!("VIDEO_PATH={}", selected_video.to_string_lossy());
    if let Some(a) = audio_path {
        println!("AUDIO_PATH={}", a.to_string_lossy());
    } else {
        println!("AUDIO_PATH=");
    }
    println!("RENDER_MODE={}", mode_str);
    println!("FILL_SCREEN={}", fill_str);

    Ok(())
}
