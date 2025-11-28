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

# Parse simple CLI args
DEBUG=0
while [[ $# -gt 0 ]]; do
    case "$1" in
        --debug) DEBUG=1; shift;;
        --no-debug) DEBUG=0; shift;;
        *) break;;
    esac
done

# Ensure directories exist
mkdir -p "$VIDEO_DIR" "$AUDIO_DIR" "$FRAMES_BASE"

# Export debug flag to Rust binary
if [[ $DEBUG -eq 1 ]]; then
        export BAD_APPLE_DEBUG=1
fi

# Export char size so Rust side can convert if needed
export CHAR_WIDTH="$CHAR_WIDTH"
export CHAR_HEIGHT="$CHAR_HEIGHT"

DEBUG_LOG="$PROJECT_DIR/debug.log"
debug_log() {
        local message="$1"
        local ts
        ts=$(date +"%Y-%m-%dT%H:%M:%S%z")
        echo -e "[$ts] $message" | tee -a "$DEBUG_LOG"
}

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

# Function to print centered text
print_centered() {
    local text="$1"
    local color="${2:-$RESET}"
    local width=${TERM_WIDTH:-$(tput cols)}
    
    # Strip ANSI codes for length calculation
    local clean_text=$(echo -e "$text" | sed 's/\x1b\[[0-9;]*m//g')
    local text_len=${#clean_text}
    
    if [[ $text_len -ge $width ]]; then
        echo -e "${color}${text}${RESET}"
    else
        local padding=$(( (width - text_len) / 2 ))
        printf "%${padding}s" ""
        echo -e "${color}${text}${RESET}"
    fi
}

# 3. Cyber Banner (Dynamic Centering)
echo ""

# Function to print BIG text (Simple ASCII Art Mapper)
print_big_text() {
    local text="$1"
    local color="${2:-$CYAN}"
    
    # Only use big text if terminal is wide enough
    if [[ "$TERM_WIDTH" -lt 100 ]]; then
        print_centered "$text" "$color"
        return
    fi

    # Simple mapping for limited characters (A-Z, 0-9, space)
    # This is a simplified implementation. For full support, we'd need a huge map.
    # We will use `figlet` if available, otherwise fallback to a simple block banner.
    
    if command -v figlet &> /dev/null; then
        echo -e "$color"
        figlet -c -f slant "$text"
        echo -e "$RESET"
    else
        # Fallback: Print with spacing and border to make it "look" bigger
        echo ""
        print_centered "╔══════════════════════════════════════════════════════════════╗" "$color"
        print_centered "║ $(printf "%-60s" "   $text") ║" "$color"
        print_centered "╚══════════════════════════════════════════════════════════════╝" "$color"
        echo ""
    fi
}

# 3. Cyber Banner (Dynamic)
echo ""
print_big_text "BAD APPLE" "$CYAN"
print_centered "OpenCV ENGINE v2.0 | GPU ACCELERATED | NATIVE FPS" "$PURPLE"

# Dynamic Status Info
if [[ "$TERM_WIDTH" -gt 120 ]]; then
    echo -e "${GREEN}$(printf "%*s" $((TERM_WIDTH/2 - 10)) "")TERMINAL: ${TERM_WIDTH}x${TERM_HEIGHT}${RESET}"
    echo -e "${GREEN}$(printf "%*s" $((TERM_WIDTH/2 - 10)) "")SCREEN:   ${SCREEN_WIDTH}x${SCREEN_HEIGHT}${RESET}"
else
    print_centered "Term: ${TERM_WIDTH}x${TERM_HEIGHT} | Screen: ${SCREEN_WIDTH}x${SCREEN_HEIGHT}" "$GREEN"
fi
print_centered "──────────────────────────────────────────────────────────────" "$BLUE"
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
    ORIG_WIDTH=1920
    ORIG_HEIGHT=1080
fi

VIDEO_ASPECT=$(awk "BEGIN {printf \"%.3f\", $ORIG_WIDTH / $ORIG_HEIGHT}")

echo -e "${BLUE}📹 Video: ${ORIG_WIDTH}x${ORIG_HEIGHT} (Original)${RESET}"

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
# 7.5 Resolution Scale & Aspect Ratio Selection
# ============================================================
echo ""
echo -e "${WHITE}${BOLD}DISPLAY SETTINGS${RESET}"
echo -e "${BLUE}Aspect Ratio Mode:${RESET}"
echo -e "  ${CYAN}[1]${RESET} ${GREEN}Fit (Letterbox)${RESET} - Show entire video (Black bars)"
echo -e "  ${CYAN}[2]${RESET} ${YELLOW}Fill (Crop)${RESET}     - Fill terminal (Crop edges)"
echo -e "  ${CYAN}[3]${RESET} ${MAGENTA}Stretch${RESET}         - Distort to fill terminal"
read -p "$(echo -e ${WHITE}"Select Mode [1]: "${RESET})" ASPECT_CHOICE
ASPECT_CHOICE=${ASPECT_CHOICE:-1}

echo -e "\n${BLUE}Resolution Scale:${RESET}"
echo -e "  ${CYAN}[1]${RESET} ${GREEN}100%${RESET} (Native)"
echo -e "  ${CYAN}[2]${RESET} ${YELLOW}75%${RESET}"
echo -e "  ${CYAN}[3]${RESET} ${MAGENTA}50%${RESET}"
echo -e "  ${CYAN}[4]${RESET} ${BLUE}Manual${RESET}"
read -p "$(echo -e ${WHITE}"Select Scale [1]: "${RESET})" SCALE_CHOICE
SCALE_CHOICE=${SCALE_CHOICE:-1}

SCALE_FACTOR=1.0
MANUAL_WIDTH=0

if [[ "$SCALE_CHOICE" == "2" ]]; then SCALE_FACTOR=0.75; fi
if [[ "$SCALE_CHOICE" == "3" ]]; then SCALE_FACTOR=0.5; fi
if [[ "$SCALE_CHOICE" == "4" ]]; then read -p "Enter Target Width: " MANUAL_WIDTH; fi

# ============================================================
# ============================================================
# 8. Calculate Dimensions (pixel-based for Rust decoder & processor)
# ============================================================
# ============================================================
# Debug detected size
if [[ $DEBUG -eq 1 ]]; then
    debug_log "DEBUG: Detected Terminal Size: ${TERM_WIDTH}x${TERM_HEIGHT}"
fi

MARGIN_X=0
MARGIN_Y=0
MAX_CHARS_X=$((TERM_WIDTH - MARGIN_X))
MAX_CHARS_Y=$((TERM_HEIGHT - MARGIN_Y))

# Resolve char pixel sizes (prefer platform detection if available)
CHAR_WIDTH=${CHAR_WIDTH:-10}
CHAR_HEIGHT=${CHAR_HEIGHT:-20}

if [[ -n "$PLATFORM_JSON" ]]; then
    p_char_w=$(echo "$PLATFORM_JSON" | grep -o '"char_width": [0-9]*' | grep -o '[0-9]*')
    p_char_h=$(echo "$PLATFORM_JSON" | grep -o '"char_height": [0-9]*' | grep -o '[0-9]*')
    if [[ -n "$p_char_w" && -n "$p_char_h" ]]; then
        CHAR_WIDTH=$p_char_w
        CHAR_HEIGHT=$p_char_h
    fi
fi

if [[ $DEBUG -eq 1 ]]; then
    debug_log "CHAR SIZE resolved from platform: ${CHAR_WIDTH}x${CHAR_HEIGHT}"
fi

# If ghosy config exists, parse font-size and scale char size heuristically
if [[ -f "$GHOSY_CONFIG" ]]; then
    FONT_SIZE_CONF=$(grep -E "^\s*font-size\s*=" "$GHOSY_CONFIG" | sed -E "s/^\s*font-size\s*=\s*([0-9.]+).*$/\1/" || true)
    if [[ -n "$FONT_SIZE_CONF" ]]; then
        # Scale character dimensions by font-size as a heuristic. Keep integer pixels.
        scaled_w=$(awk -v w=$CHAR_WIDTH -v s=$FONT_SIZE_CONF 'BEGIN {printf "%d", (w * s)}')
        scaled_h=$(awk -v h=$CHAR_HEIGHT -v s=$FONT_SIZE_CONF 'BEGIN {printf "%d", (h * s)}')
        if [[ $scaled_w -gt 0 ]]; then CHAR_WIDTH=$scaled_w; fi
        if [[ $scaled_h -gt 0 ]]; then CHAR_HEIGHT=$scaled_h; fi
        echo -e "${BLUE}Using ghosy font-size=${FONT_SIZE_CONF} scaling char dim to ${CHAR_WIDTH}x${CHAR_HEIGHT}${RESET}"
    fi
fi

export CHAR_WIDTH CHAR_HEIGHT

# Query the terminal size from the Rust binary so we get the same measurement as `crossterm`
TERM_SIZE_JSON=$("$RUST_BIN" terminal-size 2>/dev/null)
if [[ -n "$TERM_SIZE_JSON" ]]; then
    NEW_TERM_COLS=$(echo "$TERM_SIZE_JSON" | grep -o '"columns"[[:space:]]*:[[:space:]]*[0-9]*' | grep -o '[0-9]*')
    if [[ -z "$NEW_TERM_COLS" ]]; then
        NEW_TERM_COLS=$(echo "$TERM_SIZE_JSON" | grep -o '"raw_columns"[[:space:]]*:[[:space:]]*[0-9]*' | grep -o '[0-9]*')
    fi
    NEW_TERM_ROWS=$(echo "$TERM_SIZE_JSON" | grep -o '"rows"[[:space:]]*:[[:space:]]*[0-9]*' | grep -o '[0-9]*')
    if [[ -z "$NEW_TERM_ROWS" ]]; then
        NEW_TERM_ROWS=$(echo "$TERM_SIZE_JSON" | grep -o '"raw_rows"[[:space:]]*:[[:space:]]*[0-9]*' | grep -o '[0-9]*')
    fi
    if [[ "$NEW_TERM_COLS" =~ ^[0-9]+$ ]] && [[ "$NEW_TERM_ROWS" =~ ^[0-9]+$ ]]; then
        TERM_WIDTH=$NEW_TERM_COLS
        TERM_HEIGHT=$NEW_TERM_ROWS
    fi
fi

# Compute terminal pixel size
TERM_PX_WIDTH=$((TERM_WIDTH * CHAR_WIDTH))
TERM_PX_HEIGHT=$((TERM_HEIGHT * CHAR_HEIGHT))

if [[ $DEBUG -eq 1 ]]; then
    debug_log "DEBUG: Terminal pixel size: ${TERM_PX_WIDTH}x${TERM_PX_HEIGHT} (CHAR ${CHAR_WIDTH}x${CHAR_HEIGHT})"
fi

# Target 16:9 box within terminal image pixels (image pixels are 1 per horizontal char and 2 per vertical char-row)
TARGET_RATIO_PHYSICAL=1.77777778
# To map to terminal image pixels (C columns x 2R image pixels) we must
# convert desired physical 16:9 aspect into image pixel aspect, accounting for char aspect
# image_aspect = (2 * display_aspect^-1?) => derived formula below
# Derived: image_aspect = (8/9) * (CHAR_HEIGHT / CHAR_WIDTH)
TARGET_RATIO=$(awk -v c_h=$CHAR_HEIGHT -v c_w=$CHAR_WIDTH 'BEGIN {printf "%.6f", (8.0/9.0) * (c_h / c_w)}')
# Compute maximum image pixels available based on char grid and half-block mapping
# Max image width (pixels) = TERM_WIDTH characters
# Max image height (pixels) = TERM_HEIGHT * 2 (2 image pixels per char row)
MAX_IMG_W=$TERM_WIDTH
MAX_IMG_H=$((TERM_HEIGHT * 2))

if (( $(awk "BEGIN {print (${MAX_IMG_W}/${MAX_IMG_H}) > ${TARGET_RATIO}}") )); then
    # Terminal is wider than target ratio -> limit by height
    BOX_IMG_H=${MAX_IMG_H}
    BOX_IMG_W=$(awk -v h=$BOX_IMG_H -v r=$TARGET_RATIO 'BEGIN {printf "%d", h * r}')
else
    BOX_IMG_W=${MAX_IMG_W}
    BOX_IMG_H=$(awk -v w=$BOX_IMG_W -v r=$TARGET_RATIO 'BEGIN {printf "%d", w / r}')
fi

echo -e "${BLUE}Target 16:9 box within terminal (image pixels columns x image rows): ${BOX_IMG_W}x${BOX_IMG_H}${RESET}"

# Get original video size (ORIG_WIDTH x ORIG_HEIGHT) in pixels
VIDEO_ORIG_W=$ORIG_WIDTH
VIDEO_ORIG_H=$ORIG_HEIGHT

if [[ -z "$VIDEO_ORIG_W" || -z "$VIDEO_ORIG_H" || "$VIDEO_ORIG_W" -eq 0 ]]; then
    # Unknown original size; default to target box
    SCALE_WIDTH=$BOX_IMG_W
    SCALE_HEIGHT=$BOX_IMG_H
else
    # Fit original video into target box without cropping
    SCALE_FACTOR_WIDTH=$(awk -v b=$BOX_IMG_W -v o=$VIDEO_ORIG_W 'BEGIN {printf "%f", b / o}')
    SCALE_FACTOR_HEIGHT=$(awk -v b=$BOX_IMG_H -v o=$VIDEO_ORIG_H 'BEGIN {printf "%f", b / o}')
    SCALE_FACTOR=$(awk -v a=$SCALE_FACTOR_WIDTH -v b=$SCALE_FACTOR_HEIGHT 'BEGIN {print (a < b) ? a : b }')
    SCALED_VIDEO_W=$(awk -v o=$VIDEO_ORIG_W -v s=$SCALE_FACTOR 'BEGIN {printf "%d", o * s}')
    SCALED_VIDEO_H=$(awk -v o=$VIDEO_ORIG_H -v s=$SCALE_FACTOR 'BEGIN {printf "%d", o * s}')
    # The final frame (decoder target) should be the BOX_IMG dimensions: 16:9 frame
    SCALE_WIDTH=$BOX_IMG_W
    SCALE_HEIGHT=$BOX_IMG_H
fi

# Ensure even height for half-block rendering
if [ $((SCALE_HEIGHT % 2)) -ne 0 ]; then
    SCALE_HEIGHT=$((SCALE_HEIGHT - 1))
fi
if [ $((SCALE_WIDTH % 2)) -ne 0 ]; then
    SCALE_WIDTH=$((SCALE_WIDTH - 1))
fi

PIXEL_WIDTH=${SCALE_WIDTH}
PIXEL_HEIGHT=${SCALE_HEIGHT}

COLUMNS_NEEDED=$PIXEL_WIDTH
ROWS_NEEDED=$((PIXEL_HEIGHT / 2))

if [[ $DEBUG -eq 1 ]]; then
    debug_log "DEBUG: Pixel Canvas: ${PIXEL_WIDTH}x${PIXEL_HEIGHT} (image pixels) -> Columns x Rows: ${COLUMNS_NEEDED}x${ROWS_NEEDED}"
fi
echo -e "${GREEN}🎯 Final Pixel Resolution: ${PIXEL_WIDTH}x${PIXEL_HEIGHT} (16:9 fit, no crop)${RESET}"

# Manual override: if user provided char-based manual width, use that (and scale to 16:9)
if [[ "$MANUAL_WIDTH" -gt 0 ]]; then
    # Manual width is provided in character columns, not in physical px.
    PIXEL_WIDTH=$((MANUAL_WIDTH))
    PIXEL_HEIGHT=$(awk -v w=$PIXEL_WIDTH -v r=$TARGET_RATIO 'BEGIN {printf "%d", w / r}')
    if [ $((PIXEL_HEIGHT % 2)) -ne 0 ]; then
        PIXEL_HEIGHT=$((PIXEL_HEIGHT - 1))
    fi
    echo -e "${YELLOW}Manual override active: pixel canvas set to ${PIXEL_WIDTH}x${PIXEL_HEIGHT}${RESET}"
fi

# Safety: Ensure PIXEL dimensions don't exceed maximum image pixels
if [[ $PIXEL_WIDTH -gt $MAX_IMG_W ]] || [[ $PIXEL_HEIGHT -gt $MAX_IMG_H ]]; then
    debug_log "Warning: PIXEL dims exceed max: ${PIXEL_WIDTH}x${PIXEL_HEIGHT} vs max ${MAX_IMG_W}x${MAX_IMG_H}. Scaling down."
    SCALE_FACTOR=$(awk -v p_w=$PIXEL_WIDTH -v p_h=$PIXEL_HEIGHT -v m_w=$MAX_IMG_W -v m_h=$MAX_IMG_H 'BEGIN {printf "%f", (m_w/p_w < m_h/p_h) ? m_w/p_w : m_h/p_h}')
    PIXEL_WIDTH=$(awk -v w=$PIXEL_WIDTH -v s=$SCALE_FACTOR 'BEGIN {printf "%d", w * s}')
    PIXEL_HEIGHT=$(awk -v h=$PIXEL_HEIGHT -v s=$SCALE_FACTOR 'BEGIN {printf "%d", h * s}')
    # Ensure even height
    if [ $((PIXEL_HEIGHT % 2)) -ne 0 ]; then
        PIXEL_HEIGHT=$((PIXEL_HEIGHT - 1))
    fi
fi

# Final clamp: if PIXEL width still too large for terminal columns, clamp to terminal columns
if [[ $PIXEL_WIDTH -gt $MAX_IMG_W ]]; then
    debug_log "Clamping PIXEL_WIDTH from ${PIXEL_WIDTH} to terminal max cols ${MAX_IMG_W}"
    SCALE_FACTOR=$(awk -v pw=$PIXEL_WIDTH -v mw=$MAX_IMG_W 'BEGIN {printf "%f", mw / pw}')
    PIXEL_WIDTH=$(awk -v w=$PIXEL_WIDTH -v s=$SCALE_FACTOR 'BEGIN {printf "%d", w * s}')
    PIXEL_HEIGHT=$(awk -v h=$PIXEL_HEIGHT -v s=$SCALE_FACTOR 'BEGIN {printf "%d", h * s}')
    if [ $((PIXEL_HEIGHT % 2)) -ne 0 ]; then
        PIXEL_HEIGHT=$((PIXEL_HEIGHT - 1))
    fi
fi

# Extra debug: Log the main computed values
if [[ $DEBUG -eq 1 ]]; then
    debug_log "FINAL CONFIG: TERM columns=${TERM_WIDTH} rows=${TERM_HEIGHT} char=${CHAR_WIDTH}x${CHAR_HEIGHT}"
    debug_log "PLATFORM/SCREEN: screen=${SCREEN_WIDTH}x${SCREEN_HEIGHT} terminal_pixels=${TERM_PX_WIDTH}x${TERM_PX_HEIGHT}"
    debug_log "MAX IMG size (image pixels) = ${MAX_IMG_W}x${MAX_IMG_H}"
    debug_log "BOX (target 16:9 box in image pixels) = ${BOX_IMG_W}x${BOX_IMG_H}"
    debug_log "VIDEO orig = ${VIDEO_ORIG_W}x${VIDEO_ORIG_H} scaled video = ${SCALED_VIDEO_W:-0}x${SCALED_VIDEO_H:-0}"
    debug_log "PIXEL canvas = ${PIXEL_WIDTH}x${PIXEL_HEIGHT} (cols x image-rows)"
fi

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
    "--width" "$PIXEL_WIDTH" \
    "--height" "$PIXEL_HEIGHT" \
    "--mode" "$MODE")

if [[ -n "$AUDIO_PATH" ]]; then
    PLAY_LIVE_CMD+=("--audio" "$AUDIO_PATH")
fi

if [[ $DEBUG -eq 1 ]]; then
    debug_log "Executing: ${PLAY_LIVE_CMD[*]}"
fi

# Execute
"${PLAY_LIVE_CMD[@]}"

echo ""
echo -e "${GREEN}${BOLD}✨ Playback Complete${RESET}"