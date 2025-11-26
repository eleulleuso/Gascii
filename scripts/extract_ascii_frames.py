#!/usr/bin/env python3

# -------------------------------
# Imports
# -------------------------------
import cv2
import numpy as np
import os
import argparse
from concurrent.futures import ProcessPoolExecutor
from typing import List, Tuple
import time
import multiprocessing as mp

# -------------------------------
# Globals (worker-local)
# -------------------------------
_cap = None          # cv2.VideoCapture per worker
_width = 0
_height = 0
_output_dir = ""
_multiplier = 1.0

# -------------------------------
# Worker initializer
# -------------------------------
def _init_worker(video_path: str, width: int, height: int, output_dir: str, multiplier: float):
    global _cap, _width, _height, _output_dir, _multiplier
    _cap = cv2.VideoCapture(video_path)
    _width = width
    _height = height
    _output_dir = output_dir
    _multiplier = multiplier
    # OpenCV 내부 스레드를 1로 제한 (각 프로세스 당)
    cv2.setNumThreads(1)
    try:
        cv2.ocl.setUseOpenCL(False)
    except AttributeError:
        pass

# -------------------------------
# Worker task (index → txt)
# -------------------------------
def _process_index(frame_idx: int):
    global _cap, _width, _height, _output_dir, _multiplier
    if _cap is None or not _cap.isOpened():
        return False
    # 원본 비디오 프레임 번호 계산
    src_idx = int(frame_idx / _multiplier)
    _cap.set(cv2.CAP_PROP_POS_FRAMES, src_idx)
    ret, frame = _cap.read()
    if not ret:
        return False
    ascii_frame = convert_frame_to_ascii(frame, _width, _height)
    out_path = os.path.join(_output_dir, f"frame_{frame_idx:05d}.txt")
    with open(out_path, "w", encoding="utf-8") as fp:
        fp.write(ascii_frame)
    return True

# 🔥 최적화된 ASCII 문자셋 (8단계 그라데이션)
ASCII_CHARS = [' ', '.', '-', '+', '*', '%', '#', '@']
    
def get_char_by_brightness(brightness: float) -> str:
    """Convert brightness value to ASCII char with optimized mapping"""
    index = int(brightness * (len(ASCII_CHARS) - 1))
    return ASCII_CHARS[index]

def convert_frame_to_ascii(frame: np.ndarray, width: int, height: int) -> str:
    """Convert a frame to ASCII art with optimized brightness calculation"""
    # Resize frame with INTER_AREA for better downsampling performance
    resized = cv2.resize(frame, (width, height), interpolation=cv2.INTER_AREA)
        
    # Convert to grayscale if needed
    if len(resized.shape) == 3:
        gray = cv2.cvtColor(resized, cv2.COLOR_BGR2GRAY)
    else:
        gray = resized
    
    # 🔥 Enhance contrast and details
    clahe = cv2.createCLAHE(clipLimit=3.0, tileGridSize=(8,8))
    enhanced = clahe.apply(gray)
        
    # Normalize brightness
    brightness = enhanced.astype(float) / 255.0
    
    # Convert to ASCII
    ascii_frame = ''
    for row in brightness:
        for pixel in row:
            ascii_frame += get_char_by_brightness(pixel)
        ascii_frame += '\n'
    
    return ascii_frame

# 기존 numpy 전송 방식 함수 제거

def extract_frames(input_path: str, output_dir: str, width: int = 265, height: int = 65,
                  fps: int = 120, workers: int = None) -> None:
    """Extract ASCII frames from video with progress tracking"""
    
    # Create output directory if not exists
    os.makedirs(output_dir, exist_ok=True)
    # Prevent BLAS/OpenBLAS thread oversubscription
    os.environ["OMP_NUM_THREADS"] = "1"
    os.environ["OPENBLAS_NUM_THREADS"] = "1"
    cv2.setNumThreads(1)
    try:
        cv2.ocl.setUseOpenCL(False)
    except AttributeError:
        pass

    # Adjust default workers to CPU count if unspecified
    cpu_total = os.cpu_count() or 1
    workers = workers or cpu_total

    # Resource-based resolution adjustment
    try:
        import psutil
        mem_info = psutil.virtual_memory()
        mem_limit = mem_info.total * 0.85
        total_pixels = width * height
        if total_pixels > mem_limit:
            scale = (mem_limit / total_pixels) ** 0.5
            new_w = max(40, int(width * scale))
            new_h = max(20, int(height * scale))
            print(f"🔧 리소스 제약에 따라 해상도를 {width}x{height}→{new_w}x{new_h}로 조정합니다.")
            width, height = new_w, new_h
    except ImportError:
        pass
    
    # Open video file
    cap = cv2.VideoCapture(input_path)
    if not cap.isOpened():
        raise ValueError(f"Could not open video file: {input_path}")
    
    # Set video properties for 4K processing
    cap.set(cv2.CAP_PROP_FRAME_WIDTH, 3840)
    cap.set(cv2.CAP_PROP_FRAME_HEIGHT, 2160)
    
    # Calculate total frames for target FPS
    total_frames = int(cap.get(cv2.CAP_PROP_FRAME_COUNT))
    original_fps = cap.get(cv2.CAP_PROP_FPS)
    frame_multiplier = fps / original_fps if original_fps else 1
    target_frames = int(total_frames * frame_multiplier)

    # Initialize progress tracking
    start_time = time.time()
    processed_frames = 0
 
    print(f'🎬 Processing video: {input_path}')
    print(f'📊 Original FPS: {original_fps:.2f}, Target FPS: {fps}')
    print(f'🎯 Frames to process: {target_frames}')
    print(f'📐 Output size: {width}x{height}')
    
    # 프레임 인덱스 기반 분산 처리 → 대용량 numpy 직렬화 제거
    with ProcessPoolExecutor(
        max_workers=workers,
        initializer=_init_worker,
        initargs=(input_path, width, height, output_dir, frame_multiplier),
    ) as executor:
        for _ in executor.map(_process_index, range(target_frames), chunksize=workers * 4):
            processed_frames += 1
            if processed_frames % 100 == 0:
                elapsed = time.time() - start_time
                fps_current = processed_frames / elapsed if elapsed else 0
                remaining = (target_frames - processed_frames) / fps_current if fps_current else 0
                print(
                    f"\r📈 Progress: {processed_frames}/{target_frames} frames "
                    f"({processed_frames/target_frames*100:.1f}%) | "
                    f"Speed: {fps_current:.1f} fps | "
                    f"ETA: {remaining:.1f}s",
                    end="",
                    flush=True,
                )
    
    cap.release()
    print(f"\n✅ Completed! Generated {processed_frames} ASCII frames")

def main():
    parser = argparse.ArgumentParser(description='Extract ASCII frames from video')
    parser.add_argument('--input', '-i', required=True, help='Input video file')
    parser.add_argument('--output', '-o', default='ascii_frames', help='Output directory')
    parser.add_argument('--width', '-w', type=int, default=265, help='ASCII frame width')
    parser.add_argument('--height', '-ht', type=int, default=65, help='ASCII frame height')
    parser.add_argument('--fps', '-f', type=int, default=120, help='Target FPS')
    parser.add_argument('--workers', type=int, default=None, help='Number of worker threads')
    parser.add_argument('--verbose', '-v', action='store_true', help='Show verbose output')
    
    args = parser.parse_args()
    
    try:
        extract_frames(args.input, args.output, args.width, args.height, args.fps, args.workers)
    except Exception as e:
        print(f'❌ Error: {e}')
        exit(1)

if __name__ == '__main__':
    main()