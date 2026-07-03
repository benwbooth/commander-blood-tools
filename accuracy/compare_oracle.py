#!/usr/bin/env python3
"""Compare a generated Commander Blood frame with a DOSBox oracle capture.

The current DOSBox-X harness captures the host Xvfb root window, not a native
320x200 framebuffer. This script makes those captures comparable by cropping the
DOSBox viewport, scaling both images to the game's native 320x200 resolution,
and writing repeatable image-difference metrics.
"""

from __future__ import annotations

import argparse
import csv
import json
import math
import os
import subprocess
import tempfile
from dataclasses import dataclass
from pathlib import Path

import numpy as np
from PIL import Image, ImageChops


NATIVE_SIZE = (320, 200)
# Current accuracy/run_oracle.sh host captures are 800x600 Xvfb grabs with the
# DOSBox 320x200 game viewport aspect-corrected to 640x480 at this offset.
DEFAULT_XVFB_CROP = (80, 100, 640, 480)


@dataclass(frozen=True)
class Scenario:
    scenario_id: str
    reference: Path
    generated: Path
    generated_time: float = 0.0
    ref_crop: str = "auto"
    max_mean_abs: float | None = None
    out_dir: Path | None = None
    notes: str = ""


def parse_optional_float(value: str | None) -> float | None:
    if value is None or value.strip() == "":
        return None
    return float(value)


def load_scenarios(path: Path) -> list[Scenario]:
    text = "\n".join(
        line for line in path.read_text().splitlines() if line.strip() and not line.startswith("#")
    )
    if not text.strip():
        return []

    rows = csv.DictReader(text.splitlines(), delimiter="\t")
    scenarios: list[Scenario] = []
    for line_no, row in enumerate(rows, start=2):
        scenario_id = (row.get("scenario_id") or "").strip()
        reference = (row.get("reference") or "").strip()
        generated = (row.get("generated") or "").strip()
        if not scenario_id or not reference or not generated:
            raise ValueError(
                f"{path}:{line_no}: scenario_id, reference, and generated are required"
            )
        scenarios.append(
            Scenario(
                scenario_id=scenario_id,
                reference=Path(reference),
                generated=Path(generated),
                generated_time=float((row.get("generated_time") or "0").strip() or "0"),
                ref_crop=(row.get("ref_crop") or "auto").strip() or "auto",
                max_mean_abs=parse_optional_float(row.get("max_mean_abs")),
                out_dir=Path(row["out_dir"]) if (row.get("out_dir") or "").strip() else None,
                notes=(row.get("notes") or "").strip(),
            )
        )
    return scenarios


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


def comparison_status(metrics: dict[str, object], max_mean_abs: float | None) -> str:
    if max_mean_abs is None:
        return "unchecked"
    return "pass" if float(metrics["mean_abs"]) <= max_mean_abs else "fail"


def compare_paths(
    reference_path: Path,
    generated_path: Path,
    *,
    generated_time: float,
    ref_crop: str,
    out_dir: Path,
    max_mean_abs: float | None = None,
    scenario_id: str | None = None,
    notes: str = "",
) -> dict[str, object]:
    out_dir.mkdir(parents=True, exist_ok=True)

    reference_source = Image.open(reference_path)
    crop = parse_crop(ref_crop, reference_source.size)
    reference_source.close()
    reference_native = load_native_image(reference_path, crop=crop)

    with tempfile.TemporaryDirectory(prefix="commander-blood-compare-") as tmp:
        generated_source = extract_generated_frame(
            generated_path, generated_time, Path(tmp)
        )
        generated_native = load_native_image(generated_source)

    metrics = diff_metrics(reference_native, generated_native)
    status = comparison_status(metrics, max_mean_abs)
    metrics.update(
        {
            "scenario_id": scenario_id,
            "status": status,
            "max_mean_abs": max_mean_abs,
            "reference": str(reference_path),
            "generated": str(generated_path),
            "generated_time": generated_time,
            "reference_crop": list(crop),
            "notes": notes,
        }
    )

    reference_native.save(out_dir / "reference-native.png")
    generated_native.save(out_dir / "generated-native.png")
    save_diff(reference_native, generated_native, out_dir / "diff-x4.png")
    (out_dir / "comparison.json").write_text(json.dumps(metrics, indent=2) + "\n")

    return metrics


def run_scenarios(
    scenarios: list[Scenario],
    *,
    out_root: Path,
    scenario_ids: set[str] | None = None,
    summary_out: Path | None = None,
) -> tuple[list[dict[str, object]], int]:
    selected = [
        scenario
        for scenario in scenarios
        if scenario_ids is None or scenario.scenario_id in scenario_ids
    ]
    if scenario_ids is not None:
        found = {scenario.scenario_id for scenario in selected}
        missing = sorted(scenario_ids - found)
        if missing:
            raise ValueError(f"scenario id(s) not found: {', '.join(missing)}")

    results: list[dict[str, object]] = []
    exit_code = 0
    for scenario in selected:
        out_dir = scenario.out_dir or out_root / scenario.scenario_id
        metrics = compare_paths(
            scenario.reference,
            scenario.generated,
            generated_time=scenario.generated_time,
            ref_crop=scenario.ref_crop,
            out_dir=out_dir,
            max_mean_abs=scenario.max_mean_abs,
            scenario_id=scenario.scenario_id,
            notes=scenario.notes,
        )
        results.append(metrics)
        print(json.dumps(metrics, indent=2))
        if metrics["status"] == "fail":
            exit_code = 2

    summary = {
        "scenario_count": len(results),
        "pass_count": sum(1 for result in results if result["status"] == "pass"),
        "fail_count": sum(1 for result in results if result["status"] == "fail"),
        "unchecked_count": sum(1 for result in results if result["status"] == "unchecked"),
        "results": results,
    }
    summary_path = summary_out or out_root / "summary.json"
    summary_path.parent.mkdir(parents=True, exist_ok=True)
    summary_path.write_text(json.dumps(summary, indent=2) + "\n")
    return results, exit_code


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--reference", type=Path, help="DOSBox capture PNG")
    parser.add_argument("--generated", type=Path, help="Generated MP4 or image")
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
    parser.add_argument(
        "--scenario-file",
        type=Path,
        help="TSV file of named comparison scenarios to run in batch mode",
    )
    parser.add_argument(
        "--scenario-id",
        action="append",
        help="Only run this scenario id from --scenario-file; may be repeated",
    )
    parser.add_argument(
        "--summary-out",
        type=Path,
        help="Batch summary JSON path; defaults to <out-dir>/summary.json",
    )
    args = parser.parse_args()

    if args.scenario_file:
        out_root = args.out_dir or Path("accuracy/comparisons")
        scenarios = load_scenarios(args.scenario_file)
        _, exit_code = run_scenarios(
            scenarios,
            out_root=out_root,
            scenario_ids=set(args.scenario_id) if args.scenario_id else None,
            summary_out=args.summary_out,
        )
        return exit_code

    if args.reference is None or args.generated is None:
        parser.error("--reference and --generated are required unless --scenario-file is used")

    out_dir = args.out_dir or default_out_dir(args.reference, args.generated, args.generated_time)
    metrics = compare_paths(
        args.reference,
        args.generated,
        generated_time=args.generated_time,
        ref_crop=args.ref_crop,
        out_dir=out_dir,
        max_mean_abs=args.max_mean_abs,
    )
    print(json.dumps(metrics, indent=2))
    if metrics["status"] == "fail":
        return 2
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
