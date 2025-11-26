#!/usr/bin/env python3
"""extract_ansi_frames_optimized.py
====================================================
Optimized ANSI TrueColor Frame Extractor with Delta Encoding
"""
import argparse
import os
import time
import shutil
import sys
from typing import List, Tuple, Optional

import cv2
import numpy as np

# TrueColor 모드에서 사용할 기본 문자 (풀블럭)
COLOR_CHAR = '█'
ESC = "\x1b"
RESET = f"{ESC}[0m"

def frame_to_ansi_diff(
    current_frame: np.ndarray, 
    prev_frame: Optional[np.ndarray], 
    width: int, 
    height: int
) -> str:
    """
    Generates an ANSI string that updates only the changed parts from prev_frame to current_frame.
    """
    # Resize to width x (height * 2) for half-block rendering
    # We do this resizing here to ensure consistency
    resized = cv2.resize(current_frame, (width, height * 2), interpolation=cv2.INTER_AREA)
    rgb = cv2.cvtColor(resized, cv2.COLOR_BGR2RGB)
    
    # If no previous frame, we must draw everything (Full Frame)
    if prev_frame is None:
        prev_rgb = np.zeros_like(rgb) - 1 # Impossible colors to force update
    else:
        # Resize previous frame similarly (or we could cache the resized version)
        # Assuming prev_frame passed in is already the resized RGB version would be faster,
        # but for simplicity of interface, let's assume we manage state outside or resize here.
        # To optimize, the caller should pass the resized RGB array of the previous frame.
        pass

    # We'll assume the caller manages the 'prev_rgb' state to avoid re-resizing/converting.
    # Let's adjust the signature in the main loop.
    return "" # Placeholder, logic moved to main loop for efficiency

def extract_ansi_frames_optimized(
    input_path: str,
    output_dir: str = "ansi_frames",
    width: int = 265,
    height: int = 65,
    target_fps: int = 60,
):
    # 1. Setup
    if os.path.exists(output_dir):
        shutil.rmtree(output_dir)
    os.makedirs(output_dir, exist_ok=True)

    # 2. Open Video
    cap = cv2.VideoCapture(input_path)
    if not cap.isOpened():
        raise ValueError(f"Cannot open video: {input_path}")

    orig_fps = cap.get(cv2.CAP_PROP_FPS)
    total_frames = int(cap.get(cv2.CAP_PROP_FRAME_COUNT))
    
    if target_fps <= 0: target_fps = 60
    
    # Calculate skip/repeat
    multiplier = target_fps / orig_fps if orig_fps else 1.0
    est_out_frames = int(total_frames * multiplier)

    print(f"🎬 Optimizing {input_path} for RGB Mode")
    print(f"   Original: {orig_fps:.2f} fps | Target: {target_fps} fps")
    print(f"   Resolution: {width}x{height} (Internal: {width}x{height*2})")
    print(f"   Mode: Delta Encoding (Sequential Processing)")

    # 3. Processing Loop
    frame_idx = 0
    input_idx = 0
    
    # State for Delta Encoding
    # We store the PREVIOUS frame's RGB data (resized)
    prev_rgb = None
    
    # We also track the cursor position to optimize moves
    # But since each frame is a separate file sent to 'writev', 
    # the cursor resets to Home (1,1) at the start of each frame due to display_manager.c
    # So we assume cursor starts at (1,1) for each frame.
    
    start_time = time.time()
    
    while True:
        ret, frame = cap.read()
        if not ret:
            break

        # Frame Timing Logic
        # Simple approach: if multiplier > 1, repeat frame. If < 1, skip frames.
        # To be precise, we should use a time accumulator.
        
        current_out_frame_idx = int(input_idx * multiplier)
        next_out_frame_idx = int((input_idx + 1) * multiplier)
        frames_to_generate = next_out_frame_idx - current_out_frame_idx
        
        if frames_to_generate > 0:
            # Process this frame
            # 1. Resize & Convert
            resized = cv2.resize(frame, (width, height * 2), interpolation=cv2.INTER_AREA)
            rgb = cv2.cvtColor(resized, cv2.COLOR_BGR2RGB) # Shape: (h*2, w, 3)
            
            # 2. Generate ANSI Diff
            ansi_parts = []
            
            # Optimization: Track current active color to avoid redundant codes
            cur_fg = None
            cur_bg = None
            
            # Optimization: Track cursor to avoid redundant moves
            # (1-based coordinates)
            cursor_y, cursor_x = 1, 1 
            
            # We iterate by character blocks (2 vertical pixels)
            for y in range(height):
                # Row optimization: If the whole row is identical, skip it
                # (Can be done with numpy comparison)
                y2 = y * 2
                
                # Extract row data
                row_current = rgb[y2:y2+2, :]
                
                if prev_rgb is not None:
                    row_prev = prev_rgb[y2:y2+2, :]
                    if np.array_equal(row_current, row_prev):
                        # Entire row is unchanged -> Skip
                        # But we must update our virtual cursor position
                        # If we just skip, the cursor remains at the end of previous drawing
                        # We don't need to track cursor precisely if we always use absolute moves for new blocks
                        continue
                
                # If row has changes, iterate columns
                for x in range(width):
                    # Get colors
                    top_pixel = rgb[y2, x]     # [R, G, B]
                    bottom_pixel = rgb[y2+1, x] # [R, G, B]
                    
                    # Check against previous
                    changed = True
                    if prev_rgb is not None:
                        prev_top = prev_rgb[y2, x]
                        prev_bottom = prev_rgb[y2+1, x]
                        if np.array_equal(top_pixel, prev_top) and np.array_equal(bottom_pixel, prev_bottom):
                            changed = False
                    
                    if not changed:
                        continue
                        
                    # It changed! We need to draw.
                    
                    # 1. Move Cursor (if not already there)
                    # Target position is (y+1, x+1)
                    target_y, target_x = y + 1, x + 1
                    
                    # Heuristic: If we are at (y, x), we don't need move? 
                    # No, we can't easily track where the terminal cursor is across frames 
                    # because we don't know if the previous frame finished writing exactly here.
                    # BUT, within this frame generation, we know where we last wrote.
                    
                    if cursor_y == target_y and cursor_x == target_x:
                        pass # Already there
                    else:
                        # Move cursor
                        ansi_parts.append(f"{ESC}[{target_y};{target_x}H")
                        cursor_y, cursor_x = target_y, target_x
                    
                    # 2. Set Colors
                    # Top pixel -> Background, Bottom pixel -> Foreground (using half block ▄)
                    # Format: \x1b[48;2;R;G;Bm (Back) \x1b[38;2;R;G;Bm (Fore)
                    
                    # Check if colors changed from last active
                    r1, g1, b1 = top_pixel
                    r2, g2, b2 = bottom_pixel
                    
                    new_bg = (r1, g1, b1)
                    new_fg = (r2, g2, b2)
                    
                    if cur_bg != new_bg:
                        ansi_parts.append(f"{ESC}[48;2;{r1};{g1};{b1}m")
                        cur_bg = new_bg
                    
                    if cur_fg != new_fg:
                        ansi_parts.append(f"{ESC}[38;2;{r2};{g2};{b2}m")
                        cur_fg = new_fg
                        
                    # 3. Draw Character
                    ansi_parts.append('▄')
                    cursor_x += 1
            
            # End of Frame
            # Reset attributes to avoid bleeding into next frame's initial cursor moves?
            # No, display_manager resets cursor but not necessarily colors. 
            # Safe to reset at end of frame? 
            # Actually, if we reset, we lose the state optimization for the next frame's start.
            # But since each file is independent, we can't assume state carries over between files 
            # UNLESS the terminal preserves it. The terminal DOES preserve it.
            # However, to be safe and prevent "color bleeding" if a frame is dropped or skipped,
            # we usually append RESET. But for "Hardcore Optimization", we want to avoid RESET if possible.
            # Let's append RESET only if we drew something.
            if ansi_parts:
                ansi_parts.append(RESET)
                
            frame_data = "".join(ansi_parts)
            
            # If frame_data is empty (no changes), we still need a valid file.
            # An empty file might be ignored by the player or cause issues.
            # We should at least have one byte? 
            # display_manager.c: if (frame_size == 0) return ERR_SUCCESS;
            # So empty file is fine! It just won't draw anything, keeping previous screen. Perfect.
            
            # Save Frame(s)
            # If we need to repeat this frame (multiplier > 1), we save the SAME content
            # BUT wait. If we repeat the frame, the *second* repeat has NO changes relative to the first repeat.
            # So the first file has the diff, the second file should be EMPTY.
            
            # Logic:
            # 1. Generate diff from prev_rgb to current rgb.
            # 2. Save as frame_N.
            # 3. Update prev_rgb = current rgb.
            # 4. If we need to repeat, frame_N+1 should be diff of current vs current (= Empty).
            
            # Save first instance
            out_name = os.path.join(output_dir, f"frame_{frame_idx:05d}.txt")
            with open(out_name, "w", encoding="utf-8") as f:
                f.write(frame_data)
            frame_idx += 1
            
            # Update state
            prev_rgb = rgb
            
            # Handle repeats
            for _ in range(frames_to_generate - 1):
                # Empty frame (no changes)
                out_name = os.path.join(output_dir, f"frame_{frame_idx:05d}.txt")
                with open(out_name, "w", encoding="utf-8") as f:
                    f.write("") # Empty file
                frame_idx += 1
                
            # Progress
            if frame_idx % 100 == 0:
                elapsed = time.time() - start_time
                print(f"\rGenerating Frame {frame_idx}/{est_out_frames}...", end="", flush=True)

        input_idx += 1

    cap.release()
    print(f"\n✅ Done! Generated {frame_idx} optimized frames.")

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("-i", "--input", required=True)
    parser.add_argument("-o", "--output", default="ansi_frames")
    parser.add_argument("-w", "--width", type=int, default=265)
    parser.add_argument("-ht", "--height", type=int, default=65)
    parser.add_argument("-f", "--fps", type=int, default=60)
    args = parser.parse_args()

    extract_ansi_frames_optimized(
        args.input, args.output, args.width, args.height, args.fps
    )

if __name__ == "__main__":
    main()
