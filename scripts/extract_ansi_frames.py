#!/usr/bin/env python3
"""extract_ansi_frames.py
====================================================
비디오를 읽어 24-bit ANSI TrueColor 이스케이프 시퀀스를 포함한
ASCII 아트 프레임(.txt)으로 변환한다. 기본 동작은 기존
extract_ascii_frames.py 와 같지만, 각 픽셀(문자)에 전경색을
설정하여 실제 RGB 색을 표시한다.

사용 예시:
    python scripts/extract_ansi_frames.py -i assets/bad_apple.mp4 -o ansi_frames

결과는 ansi_frames/frame_00000.txt … 형태로 저장된다.
"""
import argparse
import os
import time
from concurrent.futures import ProcessPoolExecutor
from typing import List, Tuple, Optional

import cv2
import psutil
import shutil
import numpy as np

# 8-단계 밝기용 ASCII 문자(영상 측 공간 절약용)
ASCII_CHARS: List[str] = [' ', '.', '-', '+', '*', '%', '#', '@']
# TrueColor 모드에서 사용할 기본 문자 (풀블럭)
COLOR_CHAR = '█'

ESC = "\x1b"  # ANSI 이스케이프 시작문
RESET = f"{ESC}[0m"

# ---------------------------------------------------------------------------
# Utility 함수
# ---------------------------------------------------------------------------

def brightness_to_char(bright: float) -> str:
    """0.0~1.0 밝기를 대응되는 ASCII 문자로 변환"""
    idx = int(bright * (len(ASCII_CHARS) - 1))
    return ASCII_CHARS[idx]


def frame_to_ansi(frame: np.ndarray, width: int, height: int, char: str = COLOR_CHAR) -> str:
    """프레임 → ANSI TrueColor half-block 문자열로 변환 (vertical x2 resolution)"""
    # 세로 해상도 2배로 샘플링
    resized = cv2.resize(frame, (width, height * 2), interpolation=cv2.INTER_AREA)
    rgb = cv2.cvtColor(resized, cv2.COLOR_BGR2RGB)
    result_lines: List[str] = []
    for y in range(0, height * 2, 2):
        line_builder: List[str] = []
        prev_pair = None
        for x in range(width):
            r1, g1, b1 = rgb[y, x]
            r2, g2, b2 = rgb[y + 1, x]
            pair = (r1, g1, b1, r2, g2, b2)
            # 색이 바뀔 때만 이스케이프 시퀀스 출력
            if prev_pair != pair:
                # 상단 픽셀은 배경색, 하단 픽셀은 전경색으로 half-block 문자 사용
                line_builder.append(
                    f"{ESC}[48;2;{r1};{g1};{b1}m{ESC}[38;2;{r2};{g2};{b2}m"
                )
            prev_pair = pair
            line_builder.append('▄')
        result_lines.append("".join(line_builder) + RESET)
    return "\n".join(result_lines)


def _process_task(args: Tuple[int, np.ndarray, int, int, str, str]):
    idx, frame, w, h, out_dir, char = args
    try:
        ansi_art = frame_to_ansi(frame, w, h, char)
        path = os.path.join(out_dir, f"frame_{idx:05d}.txt")
        with open(path, "w", encoding="utf-8") as fp:
            fp.write(ansi_art)
    except Exception as exc:
        print(f"⚠️  Frame {idx} 처리 오류: {exc}")


# ---------------------------------------------------------------------------
# 메인 추출 루틴
# ---------------------------------------------------------------------------

def extract_ansi_frames(
    input_path: str,
    output_dir: str = "ansi_frames",
    width: int = 265,
    height: int = 65,
    target_fps: int = 120,
    workers: Optional[int] = None,
):
    # 기존 프레임 디렉터리 삭제
    if os.path.exists(output_dir):
        shutil.rmtree(output_dir)
    os.makedirs(output_dir, exist_ok=True)
    # 터미널 크기 동적 조정은 play.sh에서 처리합니다.

    # OpenCV 및 BLAS 스레드 오버서브스크립션 방지
    os.environ["OMP_NUM_THREADS"] = "1"
    os.environ["OPENBLAS_NUM_THREADS"] = "1"
    cv2.setNumThreads(1)
    cv2.ocl.setUseOpenCL(False)

    # 하드웨어 기반 워커 수 조정 (전체 코어 사용)
    cpu_total = os.cpu_count() or 1
    cpu_allowed = cpu_total  # 전체 코어 사용
    mem_info = psutil.virtual_memory()  # 메모리 전체 중 85%만 사용
    mem_limit = mem_info.total * 0.85
    # ProcessPoolExecutor 워커 수: 전체 코어
    actual_workers = workers or cpu_total
    try:
        gpu_count = cv2.cuda.getCudaEnabledDeviceCount()
        gpu_factor = gpu_count if gpu_count > 0 else 1
    except Exception:
        gpu_factor = 1
    # CPU 및 메모리 기반 픽셀 허용 계산 (픽셀당 3바이트 가정)
    cpu_allow = cpu_allowed * 100000 * gpu_factor
    mem_allow = mem_limit / 3
    allow_pixels = min(cpu_allow, mem_allow)
    total_pixels = width * height
    if total_pixels > allow_pixels:
        scale = (allow_pixels / total_pixels) ** 0.5
        new_w = max(40, int(width * scale))
        new_h = max(20, int(height * scale))
        print(
            f"🔧 리소스 제약에 따라 해상도를 {width}x{height}→{new_w}x{new_h}로 조정합니다. "
            f"(CPU: {cpu_allow:.0f}, MEM: {mem_allow:.0f})"
        )
        width, height = new_w, new_h

    # 비디오 캡처 및 처리 시작
    cap = cv2.VideoCapture(input_path)
    if not cap.isOpened():
        raise ValueError(f"비디오 파일을 열 수 없습니다: {input_path}")

    # 원본 FPS 및 프레임 수
    orig_fps = cap.get(cv2.CAP_PROP_FPS)
    total_frames = int(cap.get(cv2.CAP_PROP_FRAME_COUNT))

    multiplier = target_fps / orig_fps if orig_fps else 1
    est_out_frames = int(total_frames * multiplier)

    print(
        f"🎬 {input_path} | 원본 {orig_fps:.2f}fps → 목표 {target_fps}fps\n"
        f"📐 출력 크기: {width}x{height} | 예상 프레임: {est_out_frames}"
    )

    start = time.time()
    processed = 0
    import multiprocessing

    def gen_tasks():
        idx = 0
        input_idx = 0
        skip_factor = max(int(round(orig_fps / target_fps)), 1) if multiplier < 1 else 1
        while True:
            ret, frame = cap.read()
            if not ret:
                break
            if multiplier > 1:
                reps = int(multiplier)
                for _ in range(reps):
                    yield (idx, frame, width, height, output_dir, COLOR_CHAR)
                    idx += 1
            else:
                if input_idx % skip_factor == 0:
                    yield (idx, frame, width, height, output_dir, COLOR_CHAR)
                    idx += 1
            input_idx += 1

    with multiprocessing.Pool(processes=actual_workers) as pool:
        for _ in pool.imap_unordered(_process_task, gen_tasks(), chunksize=actual_workers * 2):
            processed += 1
            if processed % 100 == 0:
                elapsed = time.time() - start
                spd = processed / elapsed
                eta = (est_out_frames - processed) / spd if spd else 0
                pct = processed / est_out_frames * 100
                print(f"\r📈 {processed}/{est_out_frames} ({pct:5.1f}%) | 속도 {spd:6.1f}fps | ETA {eta:6.1f}s", end="", flush=True)
    cap.release()
    print(f"\n✅ 완료! 총 {processed} ANSI 프레임 생성.")

# ---------------------------------------------------------------------------
# CLI 엔트리
# ---------------------------------------------------------------------------

def main():
    parser = argparse.ArgumentParser(
        description="MP4 → ANSI TrueColor ASCII 프레임 (.txt) 추출기"
    )
    parser.add_argument("-i", "--input", required=True, help="입력 비디오 경로")
    parser.add_argument("-o", "--output", default="ansi_frames", help="출력 디렉터리")
    parser.add_argument("-w", "--width", type=int, default=265, help="가로 문자 수")
    parser.add_argument("-ht", "--height", type=int, default=65, help="세로 문자 수")
    parser.add_argument("-f", "--fps", type=int, default=60, help="목표 FPS (반복 저장)")
    parser.add_argument("--char", default='█', help="컬러 픽셀에 사용할 문자 (기본: 풀블럭)")
    parser.add_argument("--workers", type=int, default=None, help="쓰레드 수 (기본 CPU 수)")

    args = parser.parse_args()

    try:
        extract_ansi_frames(
            input_path=args.input,
            output_dir=args.output,
            width=args.width,
            height=args.height,
            target_fps=args.fps,
            workers=args.workers,
        )
    except Exception as err:
        print(f"❌ 오류: {err}")
        exit(1)

if __name__ == "__main__":
    main() 