#!/bin/bash

# Get project directory
PROJECT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$PROJECT_DIR"

# Build the project
echo "🔨 Building Bad Apple..."
cargo build --release
if [ $? -ne 0 ]; then
    echo "❌ Build failed"
    exit 1
fi

# 1. Run Interactive Menu (Normal Font)
# This runs in the current terminal, so the font size is readable.
echo "🖥️  Launching Menu..."

# Capture output to a temporary file
MENU_OUTPUT=$(mktemp)
./target/release/bad_apple menu > "$MENU_OUTPUT"

# Check if menu was cancelled (empty output or error)
if [ ! -s "$MENU_OUTPUT" ]; then
    echo "❌ Menu cancelled or no output"
    rm "$MENU_OUTPUT"
    exit 0
fi

# Read variables from output
source "$MENU_OUTPUT"
rm "$MENU_OUTPUT"

# Debug output
echo "Selected Video: $VIDEO_PATH"
echo "Selected Audio: $AUDIO_PATH"
echo "Render Mode: $RENDER_MODE"
echo "Fill Screen: $FILL_SCREEN"

# 2. Launch Ghostty for Playback (Optimized Font)
echo "🚀 Launching Ghostty for Playback..."

# Construct arguments
ARGS="--play-live --video \"$VIDEO_PATH\" --mode $RENDER_MODE"

if [ -n "$AUDIO_PATH" ]; then
    ARGS="$ARGS --audio \"$AUDIO_PATH\""
fi

if [ "$FILL_SCREEN" = "true" ]; then
    ARGS="$ARGS --fill"
fi

# Find Ghostty binary
GHOSTTY_BIN="ghostty"
if ! command -v ghostty &> /dev/null; then
    if [ -f "/Applications/Ghostty.app/Contents/MacOS/ghostty" ]; then
        GHOSTTY_BIN="/Applications/Ghostty.app/Contents/MacOS/ghostty"
    else
        echo "❌ Ghostty not found. Please install Ghostty or add it to your PATH."
        exit 1
    fi
fi

# Launch Ghostty
# We use 'sh -c' to ensure arguments are parsed correctly inside the terminal
# Use absolute path to binary to avoid CWD issues
BINARY_PATH="$PROJECT_DIR/target/release/bad_apple"

"$GHOSTTY_BIN" \
    --config-file=Gascii.config \
    -e "sh -c '$BINARY_PATH $ARGS; echo \"Press Enter to exit...\"; read'"