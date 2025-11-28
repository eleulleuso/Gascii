#!/bin/bash
# -----------------------------------------------------------------------------
# Bad Apple!! Player (Rust Edition) - Cyberpunk Style
# With Screen Resolution Based Aspect Ratio
# -----------------------------------------------------------------------------

PROJECT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
RUST_BIN="$PROJECT_DIR/target/release/bad_apple"
ASSETS_DIR="$PROJECT_DIR/assets"
VIDEO_DIR="$ASSETS_DIR/vidio"
AUDIO_DIR="$ASSETS_DIR/audio"
FRAMES_BASE="$ASSETS_DIR/frames"

# Neon Colors
CYAN='\033[0;36m'
BLUE='\033[0;34m'
MAGENTA='\033[0;35m'
PURPLE='\033[1;35m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
WHITE='\033[1;37m'
BOLD='\033[1m'
RESET='\033[0m'

# Ensure directories exist
mkdir -p "$VIDEO_DIR" "$AUDIO_DIR" "$FRAMES_BASE"

# 1. Build Rust Project
if [[ ! -f "$RUST_BIN" ]]; then
    echo -e "${BLUE}⚙️  System Initialization... Building Core...${RESET}"
    export PATH="$HOME/.cargo/bin:$PATH"
    cargo build --release > /dev/null 2>&1
    if [[ $? -ne 0 ]]; then
        echo -e "${MAGENTA}❌ Critical Error: Build Failed.${RESET}"
        exit 1
    fi
fi

# Clear Screen
clear

# 1.5 Ghostty Integration (Auto-Relaunch)
# If ghosy.sh exists and we are not already in the wrapper, relaunch in Ghostty
GHOSY_CONFIG="$PROJECT_DIR/ghosy.sh"
if [[ -f "$GHOSY_CONFIG" ]] && [[ -z "$GHOSY_WRAPPER" ]]; then
    if command -v ghostty >/dev/null 2>&1; then
        echo -e "${PURPLE}👻 Ghostty Detected! Relaunching with optimized config...${RESET}"
        export GHOSY_WRAPPER=1
        # Launch Ghostty with the config and execute this script again
        exec ghostty --config-file="$GHOSY_CONFIG" -e "$0" "$@"
    fi
fi

# 2. Detect Platform Information
PLATFORM_JSON=$("$RUST_BIN" detect 2>/dev/null)

# Get CURRENT terminal size (not cached from detect)
CURRENT_SIZE=$(stty size 2>/dev/null)
if [[ -n "$CURRENT_SIZE" ]]; then
    TERM_HEIGHT=$(echo "$CURRENT_SIZE" | awk '{print $1}')
    TERM_WIDTH=$(echo "$CURRENT_SIZE" | awk '{print $2}')
else
    # Fallback to detect values
    TERM_WIDTH=$(echo "$PLATFORM_JSON" | grep -o '"terminal_width": [0-9]*' | grep -o '[0-9]*')
    TERM_HEIGHT=$(echo "$PLATFORM_JSON" | grep -o '"terminal_height": [0-9]*' | grep -o '[0-9]*')
fi

# Get screen resolution from detect (this is system-wide, not terminal-specific)
SCREEN_WIDTH=$(echo "$PLATFORM_JSON" | grep -o '"screen_width": [0-9]*' | grep -o '[0-9]*')
SCREEN_HEIGHT=$(echo "$PLATFORM_JSON" | grep -o '"screen_height": [0-9]*' | grep -o '[0-9]*')
CHAR_WIDTH=$(echo "$PLATFORM_JSON" | grep -o '"char_width": [0-9]*' | grep -o '[0-9]*')
CHAR_HEIGHT=$(echo "$PLATFORM_JSON" | grep -o '"char_height": [0-9]*' | grep -o '[0-9]*')

# Fallback defaults
# Detect terminal size
if command -v tput &> /dev/null; then
    TERM_WIDTH=$(tput cols)
    TERM_HEIGHT=$(tput lines)
else
    TERM_WIDTH=$(stty size | cut -d' ' -f2)
    TERM_HEIGHT=$(stty size | cut -d' ' -f1)
fi

# Fallback if detection fails
if [[ -z "$TERM_WIDTH" ]] || [[ "$TERM_WIDTH" -eq 0 ]]; then
    TERM_WIDTH=80
fi
if [[ -z "$TERM_HEIGHT" ]] || [[ "$TERM_HEIGHT" -eq 0 ]]; then
    TERM_HEIGHT=24
fi
SCREEN_WIDTH=${SCREEN_WIDTH:-1920}
SCREEN_HEIGHT=${SCREEN_HEIGHT:-1080}
CHAR_WIDTH=${CHAR_WIDTH:-10}
CHAR_HEIGHT=${CHAR_HEIGHT:-20}

# Calculate screen aspect ratio (this is what we'll use for frames)
SCREEN_ASPECT=$(awk "BEGIN {printf \"%.3f\", $SCREEN_WIDTH / $SCREEN_HEIGHT}")

# 3. Cyber Banner
echo -e "${CYAN}${BOLD}"
echo " ╔══════════════════════════════════════════════════════════════╗"
echo " ║  ____            _        _                _      _          ║"
echo " ║ |  _ \          | |      / \   _ __  _ __ | | ___| |         ║"
echo " ║ | |_) | __ _  __| |     / _ \ | '_ \| '_ \| |/ _ \ |         ║"
echo " ║ |  _ < / _\` |/ _\` |    / ___ \| |_) | |_) | |  __/_|       ║"
echo " ║ |_| \_\__,_|\__,_|   /_/   \_\ .__/| .__/|_|\___(_)          ║"
echo " ║                              |_|   |_|                       ║"
echo " ╚══════════════════════════════════════════════════════════════╝"
echo -e "${RESET}"
echo -e "${PURPLE}   OpenCV ENGINE v2.0${RESET} ${BLUE}|${RESET} ${WHITE}GPU ACCELERATED${RESET} ${BLUE}|${RESET} ${CYAN}NATIVE FPS${RESET}"
echo -e "${GREEN}   터미널: ${TERM_WIDTH}x${TERM_HEIGHT}${RESET}"
echo -e "${GREEN}   스크린: ${SCREEN_WIDTH}x${SCREEN_HEIGHT} (${SCREEN_ASPECT})${RESET}"
echo -e "${BLUE} ──────────────────────────────────────────────────────────────${RESET}"
echo ""

# 4. Video Selection
echo -e "${WHITE}${BOLD}SOURCE SELECTION${RESET}"
VIDEO_FILES=("$VIDEO_DIR"/*.mp4)
count=0
valid_videos=()

if [[ ${#VIDEO_FILES[@]} -eq 0 ]] || [[ ! -e "${VIDEO_FILES[0]}" ]]; then
    echo -e "${MAGENTA}❌ No video files found in 'assets/vidio'.${RESET}"
    exit 1
fi

for file in "${VIDEO_FILES[@]}"; do
    filename=$(basename "$file")
    echo -e "  ${CYAN}[$((count+1))]${RESET} $filename"
    valid_videos+=("$file")
    ((count++))
done
echo -e "${BLUE} ──────────────────────────────────────────────────────────────${RESET}"
read -p "$(echo -e ${WHITE}"Select Video [1]: "${RESET})" VIDEO_CHOICE
VIDEO_CHOICE=${VIDEO_CHOICE:-1}

if [[ "$VIDEO_CHOICE" -gt 0 ]] && [[ "$VIDEO_CHOICE" -le "$count" ]]; then
    VIDEO_PATH="${valid_videos[$((VIDEO_CHOICE-1))]}"
else
    echo -e "${MAGENTA}❌ Invalid Selection.${RESET}"
    exit 1
fi

VIDEO_NAME=$(basename "$VIDEO_PATH" .mp4)

# 5. Get Video Metadata (Aspect Ratio)
if command -v ffprobe > /dev/null 2>&1; then
    VIDEO_INFO=$(ffprobe -v error -select_streams v:0 -show_entries stream=width,height -of csv=s=x:p=0 "$VIDEO_PATH" 2>/dev/null)
    ORIG_WIDTH=$(echo "$VIDEO_INFO" | cut -d'x' -f1)
    ORIG_HEIGHT=$(echo "$VIDEO_INFO" | cut -d'x' -f2)
else
    # Fallback: Assume 16:9 aspect ratio (standard)
    ORIG_WIDTH=16
    ORIG_HEIGHT=9
fi

# FORCE 16:9 Aspect Ratio as per user request
# This ensures consistent widescreen display without excessive cropping
ORIG_WIDTH=16
ORIG_HEIGHT=9
VIDEO_ASPECT=$(awk "BEGIN {printf \"%.3f\", 16.0 / 9.0}")

echo -e "${BLUE}📹 Video: ${ORIG_WIDTH}x${ORIG_HEIGHT} (Forced 16:9)${RESET}"

# 6. Audio Selection
echo ""
echo -e "${WHITE}${BOLD}AUDIO STREAM${RESET}"
AUDIO_FILES=("$AUDIO_DIR"/*.mp3 "$AUDIO_DIR"/*.wav)
count=0
valid_audios=()

for file in "${AUDIO_FILES[@]}"; do
    if [[ -f "$file" ]]; then
        filename=$(basename "$file")
        echo -e "  ${CYAN}[$((count+1))]${RESET} $filename"
        valid_audios+=("$file")
        ((count++))
    fi
done
echo -e "  ${CYAN}[0]${RESET} No Audio / Auto-Extract"
echo -e "${BLUE} ──────────────────────────────────────────────────────────────${RESET}"
read -p "$(echo -e ${WHITE}"Select Audio [0]: "${RESET})" AUDIO_CHOICE
AUDIO_CHOICE=${AUDIO_CHOICE:-0}

AUDIO_PATH=""
if [[ "$AUDIO_CHOICE" -gt 0 ]] && [[ "$AUDIO_CHOICE" -le "$count" ]]; then
    AUDIO_PATH="${valid_audios[$((AUDIO_CHOICE-1))]}"
else
    EXTRACTED_AUDIO="$AUDIO_DIR/${VIDEO_NAME}_extracted.mp3"
    if [[ ! -f "$EXTRACTED_AUDIO" ]]; then
        echo -e "${BLUE}ℹ️  Extracting audio stream...${RESET}"
        if command -v ffmpeg > /dev/null 2>&1; then
            ffmpeg -i "$VIDEO_PATH" -vn -acodec libmp3lame -q:a 2 "$EXTRACTED_AUDIO" -y -hide_banner -loglevel error
            if [[ $? -eq 0 ]]; then
                AUDIO_PATH="$EXTRACTED_AUDIO"
                echo -e "${GREEN}✅ Audio Extracted.${RESET}"
            else
                echo -e "${YELLOW}⚠️  No Audio Stream Detected.${RESET}"
            fi
        else
            echo -e "${YELLOW}⚠️  FFmpeg not found.${RESET}"
        fi
    else
        AUDIO_PATH="$EXTRACTED_AUDIO"
        echo -e "${GREEN}✅ Using Cached Audio.${RESET}"
    fi
fi


# ============================================================
# 7. Mode Selection
# ============================================================
echo ""
echo -e "${WHITE}${BOLD}RENDER MODE${RESET}"
echo -e "  ${CYAN}[1]${RESET} ${PURPLE}RGB ULTRA${RESET} (120FPS, TrueColor)"
echo -e "  ${CYAN}[2]${RESET} ${GREEN}ASCII RETRO${RESET} (Classic Text)"
echo -e "${BLUE} ──────────────────────────────────────────────────────────────${RESET}"
read -p "$(echo -e ${WHITE}"Select Mode [1]: "${RESET})" MODE_CHOICE
MODE_CHOICE=${MODE_CHOICE:-1}

if [[ "$MODE_CHOICE" == "1" ]]; then
    MODE="rgb"
else
    MODE="ascii"
fi

# ============================================================
# 7.5 Resolution Scale Selection
# ============================================================
echo ""
echo -e "${WHITE}${BOLD}RESOLUTION SCALE${RESET}"
echo -e "  ${CYAN}[1]${RESET} ${GREEN}100%${RESET} (Native - Best Quality)"
echo -e "  ${CYAN}[2]${RESET} ${YELLOW}75%${RESET}  (Balanced)"
echo -e "  ${CYAN}[3]${RESET} ${MAGENTA}50%${RESET}  (Performance/Retro)"
echo -e "  ${CYAN}[4]${RESET} ${BLUE}Manual${RESET} (Enter Width)"
echo -e "${BLUE} ──────────────────────────────────────────────────────────────${RESET}"
read -p "$(echo -e ${WHITE}"Select Scale [1]: "${RESET})" SCALE_CHOICE
SCALE_CHOICE=${SCALE_CHOICE:-1}

SCALE_FACTOR=1.0
MANUAL_WIDTH=0

if [[ "$SCALE_CHOICE" == "2" ]]; then
    SCALE_FACTOR=0.75
elif [[ "$SCALE_CHOICE" == "3" ]]; then
    SCALE_FACTOR=0.5
elif [[ "$SCALE_CHOICE" == "4" ]]; then
    read -p "Enter Target Width (e.g., 200): " MANUAL_WIDTH
fi

# ============================================================
# 8. Calculate Dimensions
# ============================================================
# Debug detected size
echo -e "${YELLOW}DEBUG: Detected Terminal Size: ${TERM_WIDTH}x${TERM_HEIGHT}${RESET}"

# Minimal safety margin to prevent edge artifacts
MARGIN_X=0
MARGIN_Y=0

# Ensure margins don't exceed terminal size
if [[ $MARGIN_X -ge $TERM_WIDTH ]]; then MARGIN_X=0; fi
if [[ $MARGIN_Y -ge $TERM_HEIGHT ]]; then MARGIN_Y=0; fi

# RGB uses half-blocks (▄), so each terminal ROW displays 2 pixels vertically
MAX_CHARS_X=$((TERM_WIDTH - MARGIN_X))
MAX_CHARS_Y=$((TERM_HEIGHT - MARGIN_Y))

if [[ "$MANUAL_WIDTH" -gt 0 ]]; then
    WIDTH=$MANUAL_WIDTH
    # Calculate height based on aspect ratio (approx 16:9) or just use terminal height ratio
    # Let's just use the terminal's aspect ratio to fill height proportionally
    # Or better, just use the terminal height scaled by the width ratio?
    # Actually, if user sets width, we should probably auto-calc height to keep aspect ratio?
    # But for "Stretch", we want to fill.
    # Let's assume if they set Manual Width, they want to fit that width.
    # We'll set height to fit the terminal aspect or just use available height.
    
    # Simple approach: If manual width is set, use it. Calculate height to maintain 16:9 roughly?
    # No, user wants to control size.
    # Let's just set WIDTH to manual, and HEIGHT to proportional terminal height?
    # Let's default HEIGHT to MAX_CHARS_Y (full height) but scaled if WIDTH is scaled.
    
    # Actually, let's just ask for Height too if Manual.
    read -p "Enter Target Height (e.g., 50): " MANUAL_HEIGHT
    HEIGHT=$MANUAL_HEIGHT
else
    # Target: FILL TERMINAL (Stretch)
    # We use the full available terminal space.
    WIDTH=$(awk -v w=$MAX_CHARS_X -v s=$SCALE_FACTOR "BEGIN {printf \"%.0f\", w * s}")
    HEIGHT=$(awk -v h=$MAX_CHARS_Y -v s=$SCALE_FACTOR "BEGIN {printf \"%.0f\", h * s}")

    if [[ "$MODE" == "rgb" ]]; then
        # In RGB mode, we use half-blocks, so vertical resolution is doubled
        HEIGHT=$(awk -v h=$MAX_CHARS_Y -v s=$SCALE_FACTOR "BEGIN {printf \"%.0f\", (h * 2) * s}")
    fi
fi

# Ensure even dimensions for block characters
WIDTH=$((WIDTH / 2 * 2))
HEIGHT=$((HEIGHT / 2 * 2))

# DEBUG: Print calculated dimensions
echo -e "${YELLOW}DEBUG: Terminal ${TERM_WIDTH}x${TERM_HEIGHT} -> Canvas ${WIDTH}x${HEIGHT} (Stretch)${RESET}"
echo -e "${GREEN}🎯 Canvas Size: ${WIDTH}x${HEIGHT} (Full Screen)${RESET}"

# ──────────────────────────────────────────────────────────────
# 9. LAUNCH RUST PLAYER (Real-time)
# ──────────────────────────────────────────────────────────────

echo ""
echo -e "${BOLD}🚀 LAUNCHING OpenCV PLAYBACK${RESET}"
echo "   (Hardware-Accelerated Video Decoding)"
echo ""

# Build if needed
# ALWAYS Build to ensure latest code is used
echo -e "${YELLOW}Compiling latest version...${RESET}"
cargo build --release
if [[ $? -ne 0 ]]; then
    echo -e "${MAGENTA}❌ Build Failed!${RESET}"
    exit 1
fi

# Debug Info
ls -l "$RUST_BIN"
echo "Binary Hash: $(shasum "$RUST_BIN" | awk '{print $1}')"

# Construct the command array
PLAY_LIVE_CMD=("$RUST_BIN" "play-live" \
    "--video" "$VIDEO_PATH" \
    "--width" "$WIDTH" \
    "--height" "$HEIGHT" \
    "--width" "$WIDTH" \
    "--height" "$HEIGHT" \
    "--mode" "$MODE")

if [[ -n "$AUDIO_PATH" ]]; then
    PLAY_LIVE_CMD+=("--audio" "$AUDIO_PATH")
fi

# Execute
"${PLAY_LIVE_CMD[@]}"

echo ""
echo -e "${GREEN}${BOLD}✨ Playback Complete${RESET}"