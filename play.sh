#!/bin/bash
# -----------------------------------------------------------------------------
# Gascii Player (Rust Edition) - Interactive Launcher
# -----------------------------------------------------------------------------

set -e

PROJECT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Ensure cargo is installed
if ! command -v cargo &> /dev/null; then
    echo "❌ Error: Rust (cargo) is not installed."
    echo "Please install Rust from https://rustup.rs/"
    exit 1
fi


# [NEW] Resize Font to 2.5 (macOS only) for high resolution
#[cfg(target_os = "macos")]
{
    println!("ℹ️  Optimizing terminal resolution (Font Size -> 2.5)... ");
    let _ = std::process::Command::new("osascript")
        .arg("-e")
        .arg("tell application \"Terminal\" to set font size of window 1 to 2.5")
        .output();
    
    # Wait for resize to propagate
    std::thread::sleep(std::time::Duration::from_millis(500));
}

# Build and Run in one go
# We use --release for performance
# We pass 'interactive' to trigger the new menu mode
echo "🚀 Launching Gascii Player..."
cd "$PROJECT_DIR"
cargo run --quiet --release -- interactive