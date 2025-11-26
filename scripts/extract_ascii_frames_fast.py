#!/usr/bin/env python3
"""extract_ascii_frames_fast.py
------------------------------------------------------------
FFmpeg 로 비디오를 한 번만 디코딩/스케일/FPS 변환하여 bgr24 RAW 스트림으로
읽어들이고, 파이썬 단일 프로세스에서 ASCII 프레임(txt)으로 저장한다.
장점:
  • 프로세스 간 numpy 직렬화 없음 → 메모리·IPC 병목 제거
  • FFmpeg C-레벨 멀티스레드 디코딩으로 최대 10× 속도
  • 코드 단순화, 플랫폼 deadlock 회피
"""
import argparse
import os
import subprocess
import sys
import time
from typing import List
import itertools

import cv2
import numpy as np

# 간단한 10단계 ASCII 문자 램프 (명확한 대비)
ASCII_GRADIENT = " .:-=+*#%@"
# 기본 8단계(속도)와 10단계(고품질) LUT 준비
LUT_8 = np.array(list(' .:-=*#@'), dtype='<U1')
LUT_10 = np.array(list(ASCII_GRADIENT), dtype='<U1')

# LUT → 256-level 문자 매핑 테이블 사전 계산 (uint8 인덱스 → 문자)
CHAR_MAP_8 = np.array([LUT_8[(i * (len(LUT_8)-1)) // 255] for i in range(256)], dtype='<U1')
CHAR_MAP_10 = np.array([LUT_10[(i * (len(LUT_10)-1)) // 255] for i in range(256)], dtype='<U1')

# 전역 설정값 (인자에 의해 갱신)
USE_HIGH = True
USE_DITHER = 'none'  # 기본적으로 디더링 끄고 테스트

# 8×8 Bayer 행렬 (ordered dither)
_BAYER_8 = (
    np.array([
        [ 0, 48, 12, 60,  3, 51, 15, 63],
        [32, 16, 44, 28, 35, 19, 47, 31],
        [ 8, 56,  4, 52, 11, 59,  7, 55],
        [40, 24, 36, 20, 43, 27, 39, 23],
        [ 2, 50, 14, 62,  1, 49, 13, 61],
        [34, 18, 46, 30, 33, 17, 45, 29],
        [10, 58,  6, 54,  9, 57,  5, 53],
        [42, 26, 38, 22, 41, 25, 37, 21]
    ], dtype=np.uint8)
)


# ------------------------------------------------------------
# 디더링 구현
# ------------------------------------------------------------

def _fs_dither(gray: np.ndarray, lut_chars: np.ndarray) -> List[str]:
    """ Floyd-Steinberg 오차확산. 반환: ASCII 문자열 리스트(행 단위) """
    h, w = gray.shape
    gray_f = gray.astype(np.int16)
    levels = len(lut_chars) - 1
    ascii_rows: List[str] = []

    for y in range(h):
        row_chars = []
        for x in range(w):
            old = gray_f[y, x]
            new_level = (old * levels + 127) // 255  # rounding
            new_val = (new_level * 255) // levels
            error = old - new_val
            row_chars.append(str(lut_chars[new_level]))

            # 오차 분배
            if x + 1 < w:
                gray_f[y, x+1] += error * 7 // 16
            if y + 1 < h:
                if x > 0:
                    gray_f[y+1, x-1] += error * 3 // 16
                gray_f[y+1, x] += error * 5 // 16
                if x + 1 < w:
                    gray_f[y+1, x+1] += error * 1 // 16

        ascii_rows.append("".join(row_chars))
    return ascii_rows


def _ordered_dither(gray: np.ndarray, lut_chars: np.ndarray) -> List[str]:
    """8×8 Bayer ordered dither"""
    h, w = gray.shape
    levels = len(lut_chars) - 1

    # Bayer 행렬을 프레임 크기에 맞게 타일링
    bayer_h = (h + 7) // 8
    bayer_w = (w + 7) // 8
    tiled_threshold = np.tile(_BAYER_8, (bayer_h, bayer_w))[:h, :w]

    # threshold 정규화 및 적용
    dithered = gray.astype(np.int16) + (tiled_threshold * 4) - 128
    dithered = np.clip(dithered, 0, 255).astype(np.uint8)

    # LUT 적용
    indices = (dithered * levels) // 255
    indices = np.clip(indices, 0, levels)
    ascii_img = lut_chars[indices]

    return ["".join(row) for row in ascii_img]


def bgr_frame_to_ascii(frame: np.ndarray) -> str:
    """BGR 프레임 → ASCII 문자열 (디더링·고품질 지원)"""
    gray = cv2.cvtColor(frame, cv2.COLOR_BGR2GRAY)

    # 대비 향상 (CLAHE)
    clahe = cv2.createCLAHE(clipLimit=3.0, tileGridSize=(8, 8))
    enhanced = clahe.apply(gray)

    # 블러링으로 노이즈 감소 (선택적)
    enhanced = cv2.GaussianBlur(enhanced, (3, 3), 0)

    # 고품질 LUT 사용
    lut_chars = CHAR_MAP_10  # 간소화된 고품질 문자 셋 사용

    # 디더링 적용 (ordered dither로 품질 향상)
    if USE_DITHER == 'fs':
        ascii_rows = _fs_dither(enhanced, lut_chars)
    elif USE_DITHER == 'ordered':
        ascii_rows = _ordered_dither(enhanced, lut_chars)
    else:
        # 기본 변환도 개선
        ascii_img = lut_chars[enhanced]
        ascii_rows = ["".join(row) for row in ascii_img]

    # 파일 말미 개행 추가 → 플레이어에서 라인 깨짐 방지
    return "\n".join(ascii_rows) + "\n"


def get_video_info(path: str):
    """비디오 파일의 FPS와 총 프레임 수를 가져옴"""
    cmd = [
        'ffprobe', '-v', 'error', '-select_streams', 'v:0',
        '-show_entries', 'stream=r_frame_rate,duration',
        '-of', 'csv=p=0', path
    ]
    try:
        result = subprocess.run(cmd, capture_output=True, text=True, timeout=10)
        if result.returncode == 0:
            parts = result.stdout.strip().split(',')
            if len(parts) >= 2:
                # r_frame_rate는 "30/1" 형식
                rate_parts = parts[0].split('/')
                if len(rate_parts) == 2:
                    fps = float(rate_parts[0]) / float(rate_parts[1])
                    duration = float(parts[1]) if parts[1] != 'N/A' else 0
                    return fps, duration
    except (subprocess.TimeoutExpired, ValueError, IndexError):
        pass
    return 30.0, 0.0  # 기본값


def stream_frames_ffmpeg(path: str, width: int, height: int, fps: int):
    """FFmpeg subprocess 로부터 BGR raw frame 을 yield"""
    # 비디오 정보 확인
    orig_fps, duration = get_video_info(path)
    print(f'🎥 원본 FPS: {orig_fps:.1f}, 길이: {duration:.1f}s')

    # 요청된 FPS 그대로 사용 (강제 120 FPS)
    print(f'🎯 타겟 FPS: {fps} (강제 적용)')

    cmd = [
        'ffmpeg', '-loglevel', 'error', '-i', path,
        '-vf', f'fps={fps},scale={width}:{height}',
        '-f', 'rawvideo', '-pix_fmt', 'bgr24', '-'
    ]
    proc = subprocess.Popen(cmd, stdout=subprocess.PIPE)
    frame_bytes = width * height * 3
    if proc.stdout is None:
        raise RuntimeError('Failed to open ffmpeg stdout')

    while True:
        buf = proc.stdout.read(frame_bytes)
        if len(buf) < frame_bytes:
            break
        yield np.frombuffer(buf, dtype=np.uint8).reshape((height, width, 3))

    proc.stdout.close()
    proc.wait()


def extract_frames(input_path: str, output_dir: str, width: int, height: int, fps: int):
    # 출력 디렉토리 생성 및 권한 확인
    try:
        os.makedirs(output_dir, exist_ok=True)
        # 디렉토리 쓰기 권한 확인
        test_file = os.path.join(output_dir, '.test_write')
        with open(test_file, 'w') as f:
            f.write('test')
        os.remove(test_file)
    except (OSError, PermissionError) as e:
        print(f'❌ 출력 디렉토리 생성/권한 오류: {e}')
        sys.exit(1)

    start = time.time()
    processed = 0
    last_report_time = start

    print(f'🎬 {input_path}')
    print(f'📐 {width}x{height} @ {fps}fps')

    try:
        for frame in stream_frames_ffmpeg(input_path, width, height, fps):
            ascii_txt = bgr_frame_to_ascii(frame)
            out_path = os.path.join(output_dir, f'frame_{processed:05d}.txt')

            try:
                with open(out_path, 'w', encoding='utf-8') as fp:
                    fp.write(ascii_txt)
            except (OSError, PermissionError) as e:
                print(f'\n❌ 파일 저장 오류 ({out_path}): {e}')
                continue

            processed += 1

            # 1초마다 진행 상황 보고 (속도 계산 개선)
            current_time = time.time()
            if current_time - last_report_time >= 1.0:
                elapsed = current_time - start
                speed = processed / elapsed if elapsed > 0 else 0
                print(f'\r📈 {processed} frames | {speed:.1f} fps', end='', flush=True)
                last_report_time = current_time

    except KeyboardInterrupt:
        print(f'\n⚠️ 사용자 중단: {processed} 프레임 처리됨')
    except Exception as e:
        print(f'\n❌ 처리 중 오류: {e}')

    duration = time.time() - start
    if duration > 0:
        avg_fps = processed / duration
        print(f'\n✅ 완료! 총 {processed} 프레임, {duration:.1f}s, 평균 {avg_fps:.1f} fps')
    else:
        print(f'\n✅ 완료! 총 {processed} 프레임')


def main():
    p = argparse.ArgumentParser(description='Fast ASCII frame extractor (FFmpeg)')
    p.add_argument('-i', '--input', required=True, help='Input video path')
    p.add_argument('-o', '--output', default='ascii_frames', help='Output directory')
    p.add_argument('-w', '--width', type=int, default=140, help='Frame width')
    p.add_argument('-ht', '--height', type=int, default=50, help='Frame height')
    p.add_argument('-f', '--fps', type=int, default=60, help='Target FPS')
    p.add_argument('--quality', choices=['fast', 'high'], default='high', help='ASCII LUT quality')
    p.add_argument('--dither', choices=['none', 'fs', 'ordered'], default='none', help='Dithering algorithm')
    args = p.parse_args()

    global USE_HIGH, USE_DITHER
    USE_HIGH = args.quality == 'high'
    USE_DITHER = args.dither
    extract_frames(args.input, args.output, args.width, args.height, args.fps)


if __name__ == '__main__':
    main() 