use anyhow::{Result, Context};
use dialoguer::{theme::ColorfulTheme, Select};
use std::path::{Path, PathBuf};
use std::fs;
use crate::core::display_manager::DisplayMode;
use crate::core::player;
use opencv::prelude::*;

/// 터미널 종류를 감지합니다.
#[cfg(target_os = "macos")]
#[derive(Debug, Clone, Copy)]
enum TerminalType {
    AppleTerminal,
    ITerm2,
    Kitty,
    Ghostty,
    Unknown,
}

#[cfg(target_os = "macos")]
impl TerminalType {
    fn detect() -> Self {
        if let Ok(term_program) = std::env::var("TERM_PROGRAM") {
            match term_program.as_str() {
                "Apple_Terminal" => return Self::AppleTerminal,
                "iTerm.app" => return Self::ITerm2,
                _ => {}
            }
        }
        
        // Check for Kitty
        if std::env::var("KITTY_WINDOW_ID").is_ok() {
            return Self::Kitty;
        }
        
        // Check for Ghostty
        if std::env::var("GHOSTTY_RESOURCES_DIR").is_ok() {
            return Self::Ghostty;
        }
        
        Self::Unknown
    }
}

/// AppleScript를 실행하고 stdout을 문자열로 반환합니다.
#[cfg(target_os = "macos")]
fn run_applescript(script: &str) -> Result<String> {
    let output = std::process::Command::new("osascript")
        .arg("-e")
        .arg(script)
        .output()
        .context("Failed to run osascript")?;

    if !output.status.success() {
        anyhow::bail!("osascript failed: {}", String::from_utf8_lossy(&output.stderr));
    }
    
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// 터미널 설정을 변경하고, Drop 시 원래대로 복구하는 가드입니다.
#[cfg(target_os = "macos")]
struct TerminalSettingsGuard {
    terminal_type: TerminalType,
    original_font_size: Option<String>,
    original_font_family: Option<String>,
}

#[cfg(target_os = "macos")]
impl TerminalSettingsGuard {
    /// 새 설정을 적용하고 가드를 생성합니다.
    fn new(new_family: &str, new_size: f32) -> Result<Self> {
        let terminal_type = TerminalType::detect();
        println!("ℹ️  Detected terminal: {:?}", terminal_type);
        
        match terminal_type {
            TerminalType::AppleTerminal => Self::setup_apple_terminal(new_family, new_size, terminal_type),
            TerminalType::ITerm2 => Self::setup_iterm2(new_family, new_size, terminal_type),
            TerminalType::Kitty => Self::setup_kitty(new_size, terminal_type),
            TerminalType::Ghostty => Self::setup_ghostty(new_size, terminal_type),
            TerminalType::Unknown => {
                println!("⚠️  Unknown terminal type. Font settings may not apply.");
                Ok(Self {
                    terminal_type,
                    original_font_size: None,
                    original_font_family: None,
                })
            }
        }
    }
    
    fn setup_apple_terminal(new_family: &str, new_size: f32, terminal_type: TerminalType) -> Result<Self> {
        let original_font_size = run_applescript(
            "tell application \"Terminal\" to get font size of window 1"
        ).ok();
        let original_font_family = run_applescript(
            "tell application \"Terminal\" to get font name of window 1"
        ).ok();

        let set_script = format!(
            "tell application \"Terminal\"
                set font name of window 1 to \"{}\"
                set font size of window 1 to {}
            end tell",
            new_family, new_size
        );
        run_applescript(&set_script)?;
        
        println!("ℹ️  Terminal settings applied (Font: {}, Size: {})", new_family, new_size);

        Ok(Self { terminal_type, original_font_size, original_font_family })
    }
    
    fn setup_iterm2(new_family: &str, new_size: f32, terminal_type: TerminalType) -> Result<Self> {
        // iTerm2는 현재 세션의 프로파일을 복제하고 수정하는 방식
        let original_font_size = run_applescript(
            "tell application \"iTerm2\"
                tell current session of current window
                    get font size
                end tell
            end tell"
        ).ok();
        
        let original_font_family = run_applescript(
            "tell application \"iTerm2\"
                tell current session of current window
                    get font
                end tell
            end tell"
        ).ok();

        let set_script = format!(
            "tell application \"iTerm2\"
                tell current session of current window
                    set font to \"{}\"
                    set font size to {}
                end tell
            end tell",
            new_family, new_size
        );
        
        if let Err(e) = run_applescript(&set_script) {
            println!("⚠️  iTerm2 font setting failed: {}. Continuing anyway...", e);
        } else {
            println!("ℹ️  iTerm2 settings applied (Font: {}, Size: {})", new_family, new_size);
        }

        Ok(Self { terminal_type, original_font_size, original_font_family })
    }
    
    fn setup_kitty(new_size: f32, terminal_type: TerminalType) -> Result<Self> {
        // Kitty의 원래 폰트 크기를 가져오는 방법이 없으므로, None으로 설정
        let original_font_size = None;
        let original_font_family = None;

        // Kitty remote control로 폰트 크기 변경
        let result = std::process::Command::new("kitty")
            .arg("@")
            .arg("set-font-size")
            .arg(new_size.to_string())
            .output();
            
        match result {
            Ok(output) if output.status.success() => {
                println!("ℹ️  Kitty font size set to {}", new_size);
            }
            _ => {
                println!("⚠️  Kitty font setting failed. Ensure 'allow_remote_control yes' is in kitty.conf");
            }
        }

        Ok(Self { terminal_type, original_font_size, original_font_family })
    }
    
    fn setup_ghostty(new_size: f32, terminal_type: TerminalType) -> Result<Self> {
        // Ghostty는 escape sequence로 폰트 크기 변경
        // OSC 50 sequence: ESC ] 50 ; font-size=SIZE ST
        print!("\x1b]50;font-size={}\x07", new_size);
        std::io::Write::flush(&mut std::io::stdout())?;
        
        println!("ℹ️  Ghostty font size set to {}", new_size);

        Ok(Self {
            terminal_type,
            original_font_size: None,
            original_font_family: None,
        })
    }
}

/// 이 구조체가 범위를 벗어날 때 (함수가 끝날 때) 'drop'이 호출됩니다.
#[cfg(target_os = "macos")]
impl Drop for TerminalSettingsGuard {
    fn drop(&mut self) {
        println!("\nℹ️  Restoring original terminal settings...");
        
        match self.terminal_type {
            TerminalType::AppleTerminal => {
                if let (Some(size), Some(family)) = (&self.original_font_size, &self.original_font_family) {
                    let restore_script = format!(
                        "tell application \"Terminal\"
                            set font name of window 1 to \"{}\"
                            set font size of window 1 to {}
                        end tell",
                        family, size
                    );
                    let _ = run_applescript(&restore_script);
                }
            }
            TerminalType::ITerm2 => {
                if let (Some(size), Some(family)) = (&self.original_font_size, &self.original_font_family) {
                    let restore_script = format!(
                        "tell application \"iTerm2\"
                            tell current session of current window
                                set font to \"{}\"
                                set font size to {}
                            end tell
                        end tell",
                        family, size
                    );
                    let _ = run_applescript(&restore_script);
                }
            }
            TerminalType::Kitty => {
                // Kitty는 원래 크기를 모르므로 기본값(11)으로 복구
                let _ = std::process::Command::new("kitty")
                    .arg("@")
                    .arg("set-font-size")
                    .arg("11")
                    .output();
            }
            TerminalType::Ghostty => {
                // Ghostty는 원래 크기를 모르므로 기본값(12)으로 복구
                print!("\x1b]50;font-size=12\x07");
                let _ = std::io::Write::flush(&mut std::io::stdout());
            }
            TerminalType::Unknown => {
                // No action
            }
        }
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
    
    // [NEW] 터미널 설정을 변경하고, 복구 가드를 생성합니다.
    #[cfg(target_os = "macos")]
    let _settings_guard = TerminalSettingsGuard::new("D2Coding", 2.5)
        .context("Failed to set terminal settings")?;
    // (이 변수가 생성되는 시점에 폰트가 바뀌고, 함수가 끝나면 자동으로 복구됩니다)

    // Wait for resize to propagate
    #[cfg(target_os = "macos")]
    std::thread::sleep(std::time::Duration::from_millis(500));
    
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
