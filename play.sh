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



# Check if running in Ghostty
if [[ "$TERM_PROGRAM" == "ghostty" || -n "$GHOSTTY_RESOURCES_DIR" ]]; then
    # Check if we are already running with our custom config
    # We can use a custom environment variable to track this
    if [[ -z "$GASCII_OPTIMIZED" ]]; then
        echo "👻 Ghostty detected. Relaunching with optimized configuration..."
        export GASCII_OPTIMIZED=1
        
        # Launch a new Ghostty instance with the config
        # We assume 'ghostty' is in the PATH
        if command -v ghostty &> /dev/null; then
            exec ghostty --config-file="$PROJECT_DIR/Gascii.config" -e "$0"
        else
            echo "⚠️  Ghostty command not found in PATH. Continuing with current settings."
        fi
    else
        echo "✅ Running in optimized Ghostty environment."
    fi
fi

# Build and Run in one go
# We use --release for performance
# We pass 'interactive' to trigger the new menu mode
echo "🚀 Launching Gascii Player..."
cd "$PROJECT_DIR"
cargo run --quiet --release -- interactive