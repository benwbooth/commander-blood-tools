#!/usr/bin/env python3
"""Compare a generated Commander Blood frame with a DOSBox oracle capture.

The current DOSBox-X harness captures the host Xvfb root window, not a native
320x200 framebuffer. This script makes those captures comparable by cropping the
DOSBox viewport, scaling both images to the game's native 320x200 resolution,
and writing repeatable image-difference metrics.
"""

from __future__ import annotations

import argparse
import json
import math
import os
import subprocess
import tempfile
from pathlib import Path

import numpy as np
from PIL import Image, ImageChops


NATIVE_SIZE = (320, 200)
# Current accuracy/run_oracle.sh host captures are 800x600 Xvfb grabs with the
# DOSBox 320x200 game viewport aspect-corrected to 640x480 at this offset.
DEFAULT_XVFB_CROP = (80, 100, 640, 480)


def parse_crop(value: str | None, image_size: tuple[int, int]) -> tuple[int, int, int, int]:
    if not value or value == "auto":
        if image_size == (800, 600):
            return DEFAULT_XVFB_CROP
        return (0, 0, image_size[0], image_size[1])

    parts = value.split(",")
    if len(parts) != 4:
        raise ValueError("--ref-crop must be 'x,y,w,h', 'auto', or omitted")
    x, y, w, h = (int(part) for part in parts)
    if x < 0 or y < 0 or w <= 0 or h <= 0:
        raise ValueError("--ref-crop contains invalid dimensions")
    if x + w > image_size[0] or y + h > image_size[1]:
        raise ValueError("--ref-crop extends outside reference image")
    return (x, y, w, h)


def load_native_image(path: Path, *, crop: tuple[int, int, int, int] | None = None) -> Image.Image:
    image = Image.open(path).convert("RGB")
    if crop is not None:
        x, y, w, h = crop
        image = image.crop((x, y, x + w, y + h))
    return image.resize(NATIVE_SIZE, Image.Resampling.NEAREST)


def extract_generated_frame(generated: Path, time_sec: float, out_dir: Path) -> Path:
    suffix = generated.suffix.lower()
    if suffix in {".png", ".jpg", ".jpeg", ".bmp"}:
        return generated

    frame_path = out_dir / "generated-source-frame.png"
    ffmpeg = os.environ.get("FFMPEG", "ffmpeg")
    subprocess.run(
        [
            ffmpeg,
            "-y",
            "-loglevel",
            "error",
            "-ss",
            f"{time_sec:.6f}",
            "-i",
            str(generated),
            "-frames:v",
            "1",
            str(frame_path),
        ],
        check=True,
    )
    return frame_path


def diff_metrics(reference: Image.Image, generated: Image.Image) -> dict[str, object]:
    ref = np.asarray(reference, dtype=np.int16)
    gen = np.asarray(generated, dtype=np.int16)
    delta = gen - ref
    abs_delta = np.abs(delta)
    exact_pixels = np.all(abs_delta == 0, axis=2)
    per_channel_mean = abs_delta.mean(axis=(0, 1))

    return {
        "width": NATIVE_SIZE[0],
        "height": NATIVE_SIZE[1],
        "mean_abs": float(abs_delta.mean()),
        "rmse": float(math.sqrt(np.mean(delta.astype(np.float64) ** 2))),
        "max_abs": int(abs_delta.max()),
        "exact_pixel_percent": float(exact_pixels.mean() * 100.0),
        "mean_abs_rgb": [float(value) for value in per_channel_mean],
    }


def save_diff(reference: Image.Image, generated: Image.Image, out_path: Path) -> None:
    diff = ImageChops.difference(reference, generated)
    # Amplify subtle channel differences without saturating large differences too
    # early; this is diagnostic, not the source of truth for metrics.
    diff = diff.point(lambda value: min(value * 4, 255))
    diff.save(out_path)


def default_out_dir(reference: Path, generated: Path, time_sec: float) -> Path:
    safe_generated = "".join(
        ch.lower() if ch.isalnum() else "_" for ch in generated.stem
    ).strip("_")
    safe_reference = "".join(
        ch.lower() if ch.isalnum() else "_" for ch in reference.stem
    ).strip("_")
    return Path("accuracy/comparisons") / f"{safe_reference}__{safe_generated}__t{time_sec:.2f}"


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--reference", required=True, type=Path, help="DOSBox capture PNG")
    parser.add_argument("--generated", required=True, type=Path, help="Generated MP4 or image")
    parser.add_argument(
        "--generated-time",
        type=float,
        default=0.0,
        help="Timestamp in seconds to sample from generated MP4",
    )
    parser.add_argument(
        "--ref-crop",
        default="auto",
        help="Reference crop as x,y,w,h; default auto uses 80,100,640,480 for 800x600 captures",
    )
    parser.add_argument("--out-dir", type=Path, help="Directory for normalized frames and metrics")
    parser.add_argument(
        "--max-mean-abs",
        type=float,
        help="Optional failure threshold for mean absolute RGB error",
    )
    args = parser.parse_args()

    reference_path = args.reference
    generated_path = args.generated
    out_dir = args.out_dir or default_out_dir(reference_path, generated_path, args.generated_time)
    out_dir.mkdir(parents=True, exist_ok=True)

    reference_source = Image.open(reference_path)
    crop = parse_crop(args.ref_crop, reference_source.size)
    reference_source.close()
    reference_native = load_native_image(reference_path, crop=crop)

    with tempfile.TemporaryDirectory(prefix="commander-blood-compare-") as tmp:
        generated_source = extract_generated_frame(
            generated_path, args.generated_time, Path(tmp)
        )
        generated_native = load_native_image(generated_source)

    metrics = diff_metrics(reference_native, generated_native)
    metrics.update(
        {
            "reference": str(reference_path),
            "generated": str(generated_path),
            "generated_time": args.generated_time,
            "reference_crop": list(crop),
        }
    )

    reference_native.save(out_dir / "reference-native.png")
    generated_native.save(out_dir / "generated-native.png")
    save_diff(reference_native, generated_native, out_dir / "diff-x4.png")
    (out_dir / "comparison.json").write_text(json.dumps(metrics, indent=2) + "\n")

    print(json.dumps(metrics, indent=2))
    if args.max_mean_abs is not None and metrics["mean_abs"] > args.max_mean_abs:
        return 2
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
