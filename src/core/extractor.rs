#![allow(dead_code, unused_variables, unused_imports)]
use anyhow::{Context, Result};
use std::fs;
use std::path::Path;
use std::time::Instant;

pub fn extract_frames(_input: &str, _output_dir: &str, _width: u32, _height: u32, _fps: u32) -> Result<()> {
    // FFmpeg extraction functionality has been disabled
    // Please use OpenCV VideoDecoder for video playback
    unimplemented!("Use OpenCV VideoDecoder for video decoding instead")
}
