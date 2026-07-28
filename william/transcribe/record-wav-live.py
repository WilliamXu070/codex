#!/usr/bin/env python3

import argparse
import array
import json
import math
import os
import signal
import subprocess
import sys
import time
import wave


stop_requested = False


def request_stop(_signum: int, _frame: object) -> None:
    global stop_requested
    stop_requested = True


def write_level(path: str | None, rms: float, peak: float, max_rms: float, max_peak: float) -> None:
    if not path:
        return
    payload = {
        "rms": max(0.0, min(1.0, rms)),
        "peak": max(0.0, min(1.0, peak)),
        "max_rms": max(0.0, min(1.0, max_rms)),
        "max_peak": max(0.0, min(1.0, max_peak)),
        "updated_at": time.time(),
    }
    tmp = f"{path}.{os.getpid()}.tmp"
    with open(tmp, "w", encoding="utf-8") as handle:
        json.dump(payload, handle, separators=(",", ":"))
    os.replace(tmp, path)


def chunk_level(chunk: bytes) -> tuple[float, float]:
    if not chunk:
        return 0.0, 0.0
    samples = array.array("h")
    samples.frombytes(chunk[: len(chunk) - (len(chunk) % 2)])
    if sys.byteorder != "little":
        samples.byteswap()
    if not samples:
        return 0.0, 0.0
    squares = sum(sample * sample for sample in samples)
    rms = math.sqrt(squares / len(samples)) / 32768.0
    peak = max(abs(sample) for sample in samples) / 32768.0
    return rms, peak


def main() -> int:
    parser = argparse.ArgumentParser(description="Record WAV and emit live mic levels.")
    parser.add_argument("--output", required=True)
    parser.add_argument("--level-file")
    parser.add_argument("--input", default=":default")
    parser.add_argument("--sample-rate", type=int, default=16000)
    parser.add_argument("--chunk-frames", type=int, default=512)
    args = parser.parse_args()

    signal.signal(signal.SIGINT, request_stop)
    signal.signal(signal.SIGTERM, request_stop)

    command = [
        "ffmpeg",
        "-hide_banner",
        "-loglevel",
        "error",
        "-f",
        "avfoundation",
        "-i",
        args.input,
        "-ac",
        "1",
        "-ar",
        str(args.sample_rate),
        "-f",
        "s16le",
        "pipe:1",
    ]
    process = subprocess.Popen(command, stdout=subprocess.PIPE)
    chunk_bytes = max(1, args.chunk_frames) * 2
    max_rms = 0.0
    max_peak = 0.0

    try:
        with wave.open(args.output, "wb") as wav:
            wav.setnchannels(1)
            wav.setsampwidth(2)
            wav.setframerate(args.sample_rate)
            while not stop_requested:
                chunk = process.stdout.read(chunk_bytes) if process.stdout else b""
                if not chunk:
                    break
                wav.writeframes(chunk)
                rms, peak = chunk_level(chunk)
                max_rms = max(max_rms, rms)
                max_peak = max(max_peak, peak)
                write_level(args.level_file, rms, peak, max_rms, max_peak)
    finally:
        if process.poll() is None:
            process.terminate()
            try:
                process.wait(timeout=1)
            except subprocess.TimeoutExpired:
                process.kill()
                process.wait()
        write_level(args.level_file, 0.0, 0.0, max_rms, max_peak)

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
