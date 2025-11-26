#!/usr/bin/env python3
import argparse
import os
import cv2
import shutil
import subprocess
import time

def extract_png_frames(input_path: str, output_dir: str, width: int, height: int, fps: int):
    # 기존 프레임 디렉터리 초기화
    if os.path.exists(output_dir):
        shutil.rmtree(output_dir)
    os.makedirs(output_dir, exist_ok=True)

    cap = cv2.VideoCapture(input_path)
    if not cap.isOpened():
        raise ValueError(f"비디오 파일을 열 수 없습니다: {input_path}")

    orig_fps = cap.get(cv2.CAP_PROP_FPS)
    total_frames = int(cap.get(cv2.CAP_PROP_FRAME_COUNT))
    multiplier = fps / orig_fps if orig_fps else 1
    est_out_frames = int(total_frames * multiplier)

    # ffmpeg가 설치되어 있으면 ffmpeg로 빠르게 추출
    if shutil.which("ffmpeg"):  # 시스템에 ffmpeg가 있으면
        print(f"🔧 ffmpeg 모드: 고속 PNG 추출 시작")
        # fps와 scale 필터 적용
        cmd = [
            "ffmpeg", "-y",
            "-i", input_path,
            "-vf", f"fps={fps},scale={width}:{height}",
            os.path.join(output_dir, "frame_%05d.png")
        ]
        subprocess.run(cmd, check=True)
        print(f"✅ ffmpeg 완료! 총 약 {est_out_frames}장 프레임 생성.")
        return

    # 진행 로그
    start = time.time()
    print(f"🎬 {input_path} | 원본 {orig_fps:.2f}fps → 목표 {fps}fps")
    print(f"📐 출력 크기: {width}x{height} | 예상 프레임: {est_out_frames}")

    # Python 루프 (fallback) - 느리지만 호환성 보장
    frame_idx = 0
    out_idx = 0
    while True:
        ret, frame = cap.read()
        if not ret:
            break
        reps = int(multiplier) if multiplier > 1 else 1
        for _ in range(reps):
            resized = cv2.resize(frame, (width, height), interpolation=cv2.INTER_AREA)
            path = os.path.join(output_dir, f"frame_{out_idx:05d}.png")
            cv2.imwrite(path, resized)
            out_idx += 1
            if out_idx % 100 == 0:
                elapsed = time.time() - start
                spd = out_idx / elapsed if elapsed else 0
                eta = (est_out_frames - out_idx) / spd if spd else 0
                pct = out_idx / est_out_frames * 100
                print(f"\r📈 {out_idx}/{est_out_frames} ({pct:5.1f}%) | 속도 {spd:6.1f}fps | ETA {eta:6.1f}s", end="", flush=True)
    cap.release()
    print(f"\n✅ PNG 프레임 생성 완료. 총 {out_idx}장")

if __name__ == '__main__':
    parser = argparse.ArgumentParser(description="MP4 → PNG 프레임 추출기")
    parser.add_argument("-i", "--input", required=True, help="입력 비디오 경로")
    parser.add_argument("-o", "--output", default="png_frames", help="출력 디렉터리")
    parser.add_argument("-w", "--width", type=int, default=140, help="가로 픽셀 수")
    parser.add_argument("-ht", "--height", type=int, default=50, help="세로 픽셀 수")
    parser.add_argument("-f", "--fps", type=int, default=60, help="목표 FPS")
    args = parser.parse_args()
    extract_png_frames(args.input, args.output, args.width, args.height, args.fps)
