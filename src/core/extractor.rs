use anyhow::{Context, Result};
use ffmpeg_sidecar::command::FfmpegCommand;
use std::fs;
use std::path::Path;
use std::time::Instant;
use std::io::Read;

pub fn extract_frames(input: &str, output_dir: &str, width: u32, height: u32, fps: u32) -> Result<()> {
    let start_time = Instant::now();
    fs::create_dir_all(output_dir).context("Failed to create output directory")?;

    println!("🚀 Starting extraction: {} -> {} ({}x{} @ {}fps)", input, output_dir, width, height, fps);

    // 1. Setup FFmpeg
    let mut ffmpeg = FfmpegCommand::new()
        .input(input)
        .args(&["-vf", &format!("fps={},scale={}:{}", fps, width, height * 2)])
        .args(&["-f", "image2pipe"])
        .args(&["-pix_fmt", "rgb24"])
        .args(&["-vcodec", "rawvideo"])
        .output("-")
        .spawn()
        .context("Failed to spawn ffmpeg")?;

    // 2. Read frames
    let frame_size = (width * (height * 2) * 3) as usize;
    let mut stdout = ffmpeg.take_stdout().context("Failed to take stdout")?;
    
    // Buffer for all frames (packed)
    // Packed size = width * height * 2 / 8 bytes per frame
    let packed_frame_size = ((width * (height * 2)) as usize + 7) / 8;
    let mut all_frames_packed = Vec::new();
    
    let mut global_idx = 0;
    let mut buffer = vec![0u8; frame_size];

    loop {
        match stdout.read_exact(&mut buffer) {
            Ok(_) => {
                // Process frame immediately: RGB -> 1-bit Packed
                let mut packed_frame = vec![0u8; packed_frame_size];
                let mut bit_idx = 0;
                
                for chunk in buffer.chunks_exact(3) {
                    let r = chunk[0] as u32;
                    let g = chunk[1] as u32;
                    let b = chunk[2] as u32;
                    // Luminance
                    let gray = (r * 299 + g * 587 + b * 114) / 1000;
                    
                    if gray > 128 {
                        let byte_pos = bit_idx / 8;
                        let bit_pos = 7 - (bit_idx % 8); // MSB first
                        packed_frame[byte_pos] |= 1 << bit_pos;
                    }
                    bit_idx += 1;
                }
                
                all_frames_packed.extend_from_slice(&packed_frame);
                global_idx += 1;
                
                if global_idx % 100 == 0 {
                    print!("\rProcessed {} frames...", global_idx);
                    use std::io::Write;
                    std::io::stdout().flush()?;
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                break;
            }
            Err(e) => return Err(e.into()),
        }
    }

    println!("\nCompressing {} frames...", global_idx);

    // 3. Write Single Compressed File
    let output_path = Path::new(output_dir).join("video.bin");
    let mut file = std::fs::File::create(&output_path)?;
    
    use std::io::Write;
    // Header: Width(u16), Height(u16), FrameCount(u32)
    file.write_all(&(width as u16).to_le_bytes())?;
    file.write_all(&(height as u16).to_le_bytes())?;
    file.write_all(&(global_idx as u32).to_le_bytes())?;
    
    // Compress all data
    let compressed_data = lz4::block::compress(&all_frames_packed, None, true)?;
    file.write_all(&compressed_data)?;

    let duration = start_time.elapsed();
    println!("✅ Done! Saved to {:?} (Size: {:.2} MB)", output_path, compressed_data.len() as f64 / 1024.0 / 1024.0);
    println!("Extracted {} frames in {:.2}s ({:.1} fps)", global_idx, duration.as_secs_f64(), global_idx as f64 / duration.as_secs_f64());

    Ok(())
}

// Removed process_batch as we are now streaming to a single buffer
