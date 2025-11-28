#!/bin/bash
# -----------------------------------------------------------------------------
# Bad Apple!! Player (Rust Edition) - Interactive Launcher
# -----------------------------------------------------------------------------

set -e

PROJECT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Ensure cargo is installed
if ! command -v cargo &> /dev/null; then
    echo "❌ Error: Rust (cargo) is not installed."
    echo "Please install Rust from https://rustup.rs/"
    exit 1
fi

# macOS Font Resizing (AppleScript)
if [[ "$(uname)" == "Darwin" ]]; then
    osascript -e '
    tell application "Terminal"
        set font size of window 1 to 2.5
    end tell
    ' 2>/dev/null || true
fi

# Build and Run in one go
# We use --release for performance
# We pass 'interactive' to trigger the new menu mode
echo "🚀 Launching Bad Apple Player..."
cd "$PROJECT_DIR"
cargo run --quiet --release -- interactive