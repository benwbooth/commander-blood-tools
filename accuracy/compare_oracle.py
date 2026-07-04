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
import glob
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

# Native-screen regions recovered from BLOODPRG.EXE's dialogue presentation
# state. These keep oracle failures actionable: wrong scene band, missing HUD
# panel, and subtitle/foreground errors should not collapse into one mean.
SCREEN_REGIONS: dict[str, tuple[int, int, int, int]] = {
    "top_bar": (0, 0, 320, 35),
    "scene_band": (0, 35, 320, 130),
    "hud_panel": (0, 165, 320, 29),
    "bottom_bar": (0, 194, 320, 6),
}


@dataclass(frozen=True)
class Scenario:
    scenario_id: str
    reference: Path
    generated: Path
    reference_manifest: Path | None = None
    generated_time: float = 0.0
    ref_crop: str = "auto"
    max_mean_abs: float | None = None
    scan_start: float | None = None
    scan_end: float | None = None
    scan_step: float | None = None
    out_dir: Path | None = None
    notes: str = ""


def parse_optional_float(value: str | None) -> float | None:
    if value is None or value.strip() == "":
        return None
    return float(value)


def load_scenarios(path: Path) -> list[Scenario]:
    text = "\n".join(
        line
        for line in path.read_text().splitlines()
        if line.strip() and not line.startswith("#")
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
                reference_manifest=(
                    Path(row["reference_manifest"])
                    if (row.get("reference_manifest") or "").strip()
                    else None
                ),
                generated_time=float(
                    (row.get("generated_time") or "0").strip() or "0"
                ),
                ref_crop=(row.get("ref_crop") or "auto").strip() or "auto",
                max_mean_abs=parse_optional_float(row.get("max_mean_abs")),
                scan_start=parse_optional_float(row.get("scan_start")),
                scan_end=parse_optional_float(row.get("scan_end")),
                scan_step=parse_optional_float(row.get("scan_step")),
                out_dir=(
                    Path(row["out_dir"])
                    if (row.get("out_dir") or "").strip()
                    else None
                ),
                notes=(row.get("notes") or "").strip(),
            )
        )
    return scenarios


@dataclass(frozen=True)
class ReferenceSource:
    path: Path
    ref_crop: str
    metadata: dict[str, object] | None = None


def load_capture_manifest(path: Path) -> dict[str, dict[str, str]]:
    rows: dict[str, dict[str, str]] = {}
    with path.open(newline="") as f:
        reader = csv.DictReader(f, delimiter="\t")
        for line_no, row in enumerate(reader, start=2):
            frame = (row.get("frame") or "").strip()
            if not frame:
                raise ValueError(f"{path}:{line_no}: frame is required")
            if frame in rows:
                raise ValueError(f"{path}:{line_no}: duplicate frame {frame}")
            rows[frame] = row
    return rows


def manifest_crop(row: dict[str, str]) -> str:
    required = ["crop_x", "crop_y", "crop_w", "crop_h"]
    missing = [field for field in required if not (row.get(field) or "").strip()]
    if missing:
        raise ValueError(
            f"capture manifest row is missing crop field(s): {', '.join(missing)}"
        )
    return ",".join(str(int(row[field])) for field in required)


def manifest_frame_path(
    row: dict[str, str], reference_manifest: Path, frame: str
) -> Path:
    raw_path = (row.get("path") or "").strip()
    if raw_path:
        path = Path(raw_path)
        return path if path.is_absolute() else reference_manifest.parent / path
    return reference_manifest.parent / frame


def resolve_reference_source(
    reference_path: Path,
    ref_crop: str,
    reference_manifest: Path | None = None,
) -> ReferenceSource:
    if reference_manifest is None:
        return ReferenceSource(reference_path, ref_crop)

    rows = load_capture_manifest(reference_manifest)
    frame = reference_path.name
    row = rows.get(frame)
    if row is None:
        raise ValueError(f"{reference_manifest}: frame {frame} not found")

    manifest_path = manifest_frame_path(row, reference_manifest, frame)
    resolved_path = (
        manifest_path if (row.get("path") or "").strip() else reference_path
    )
    if not resolved_path.exists():
        resolved_path = manifest_path
    crop_value = manifest_crop(row)
    resolved_crop = crop_value if ref_crop == "auto" else ref_crop
    metadata: dict[str, object] = {
        "manifest": str(reference_manifest),
        "frame": frame,
        "path": str(manifest_path),
        "elapsed_s": parse_optional_float(row.get("elapsed_s")),
        "epoch_s": parse_optional_float(row.get("epoch_s")),
        "display": (row.get("display") or "").strip(),
        "capture_kind": (row.get("capture_kind") or "").strip(),
        "crop": [int(part) for part in crop_value.split(",")],
        "native_size": [
            int(row.get("native_w") or NATIVE_SIZE[0]),
            int(row.get("native_h") or NATIVE_SIZE[1]),
        ],
    }
    return ReferenceSource(resolved_path, resolved_crop, metadata)


def parse_scan_range(value: str) -> tuple[float, float, float]:
    parts = value.split(":")
    if len(parts) != 3:
        raise ValueError("--scan-generated must be START:END:STEP")
    start, end, step = (float(part) for part in parts)
    validate_scan_range(start, end, step)
    return start, end, step


def validate_scan_range(start: float, end: float, step: float) -> None:
    if start < 0 or end < 0:
        raise ValueError("scan range cannot contain negative timestamps")
    if end < start:
        raise ValueError("scan end must be greater than or equal to start")
    if step <= 0:
        raise ValueError("scan step must be positive")


def scan_times(start: float, end: float, step: float) -> list[float]:
    validate_scan_range(start, end, step)
    times: list[float] = []
    time_sec = start
    epsilon = step / 1000.0
    while time_sec <= end + epsilon:
        times.append(round(time_sec, 6))
        time_sec += step
    return times


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


def load_reference_native(
    reference_path: Path, ref_crop: str
) -> tuple[Image.Image, tuple[int, int, int, int]]:
    reference_source = Image.open(reference_path)
    crop = parse_crop(ref_crop, reference_source.size)
    reference_source.close()
    return load_native_image(reference_path, crop=crop), crop


def extract_generated_frame(generated: Path, time_sec: float, out_path: Path) -> Path:
    suffix = generated.suffix.lower()
    if suffix in {".png", ".jpg", ".jpeg", ".bmp"}:
        return generated

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
            str(out_path),
        ],
        check=True,
    )
    if not out_path.exists():
        raise ValueError(f"ffmpeg did not produce a frame at {time_sec:.6f}s")
    return out_path


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


def region_metrics(reference: Image.Image, generated: Image.Image) -> dict[str, object]:
    regions: dict[str, object] = {}
    for name, (x, y, w, h) in SCREEN_REGIONS.items():
        ref_region = reference.crop((x, y, x + w, y + h))
        gen_region = generated.crop((x, y, x + w, y + h))
        metrics = diff_metrics(ref_region, gen_region)
        metrics["x"] = x
        metrics["y"] = y
        metrics["width"] = w
        metrics["height"] = h
        regions[name] = metrics
    return regions


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


def default_candidate_search_out_dir(reference: Path) -> Path:
    safe_reference = "".join(
        ch.lower() if ch.isalnum() else "_" for ch in reference.stem
    ).strip("_")
    return Path("accuracy/comparisons") / f"{safe_reference}__candidate_search"


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
    reference_manifest: Path | None = None,
    max_mean_abs: float | None = None,
    scenario_id: str | None = None,
    notes: str = "",
    extra_metrics: dict[str, object] | None = None,
) -> dict[str, object]:
    out_dir.mkdir(parents=True, exist_ok=True)

    reference_source = resolve_reference_source(
        reference_path, ref_crop, reference_manifest
    )
    reference_native, crop = load_reference_native(
        reference_source.path, reference_source.ref_crop
    )

    with tempfile.TemporaryDirectory(prefix="commander-blood-compare-") as tmp:
        generated_source = extract_generated_frame(
            generated_path, generated_time, Path(tmp) / "generated-source-frame.png"
        )
        generated_native = load_native_image(generated_source)

    metrics = diff_metrics(reference_native, generated_native)
    metrics["regions"] = region_metrics(reference_native, generated_native)
    status = comparison_status(metrics, max_mean_abs)
    metrics.update(
        {
            "scenario_id": scenario_id,
            "status": status,
            "max_mean_abs": max_mean_abs,
            "reference": str(reference_source.path),
            "generated": str(generated_path),
            "generated_time": generated_time,
            "reference_crop": list(crop),
            "notes": notes,
        }
    )
    if reference_source.metadata is not None:
        metrics["reference_manifest"] = reference_source.metadata
    if extra_metrics is not None:
        metrics.update(extra_metrics)

    reference_native.save(out_dir / "reference-native.png")
    generated_native.save(out_dir / "generated-native.png")
    save_diff(reference_native, generated_native, out_dir / "diff-x4.png")
    (out_dir / "comparison.json").write_text(json.dumps(metrics, indent=2) + "\n")

    return metrics


def scan_generated_times(
    reference_path: Path,
    generated_path: Path,
    *,
    start: float,
    end: float,
    step: float,
    ref_crop: str,
    out_dir: Path,
    reference_manifest: Path | None = None,
    max_mean_abs: float | None = None,
    scenario_id: str | None = None,
    notes: str = "",
) -> dict[str, object]:
    if max_mean_abs is not None:
        label = f" for scenario {scenario_id!r}" if scenario_id else ""
        raise ValueError(
            "thresholded oracle comparisons must use a fixed generated timestamp"
            f"{label}; clear scan_start/scan_end/scan_step before setting max_mean_abs"
        )

    times = scan_times(start, end, step)
    if not times:
        raise ValueError("scan range produced no timestamps")

    reference_source = resolve_reference_source(
        reference_path, ref_crop, reference_manifest
    )
    reference_native, crop = load_reference_native(
        reference_source.path, reference_source.ref_crop
    )

    scan_results = scan_generated_against_reference(
        reference_native, generated_path, times
    )

    best = min(scan_results, key=lambda result: float(result["mean_abs"]))
    scan_summary = {
        "scan_start": start,
        "scan_end": end,
        "scan_step": step,
        "scan_count": len(scan_results),
        "best_generated_time": best["generated_time"],
        "best_mean_abs": best["mean_abs"],
    }
    out_dir.mkdir(parents=True, exist_ok=True)
    (out_dir / "scan.json").write_text(
        json.dumps({"summary": scan_summary, "results": scan_results}, indent=2) + "\n"
    )

    return compare_paths(
        reference_source.path,
        generated_path,
        generated_time=float(best["generated_time"]),
        ref_crop=reference_source.ref_crop,
        out_dir=out_dir,
        reference_manifest=reference_manifest,
        max_mean_abs=max_mean_abs,
        scenario_id=scenario_id,
        notes=notes,
        extra_metrics=scan_summary,
    )


def scan_generated_against_reference(
    reference_native: Image.Image, generated_path: Path, times: list[float]
) -> list[dict[str, object]]:
    scan_results: list[dict[str, object]] = []
    with tempfile.TemporaryDirectory(prefix="commander-blood-scan-") as tmp:
        tmp_dir = Path(tmp)
        for index, time_sec in enumerate(times):
            frame_path = tmp_dir / f"generated-source-frame-{index:05}.png"
            try:
                generated_source = extract_generated_frame(
                    generated_path, time_sec, frame_path
                )
            except (subprocess.CalledProcessError, ValueError):
                continue
            generated_native = load_native_image(generated_source)
            metrics = diff_metrics(reference_native, generated_native)
            scan_results.append(
                {
                    "generated_time": time_sec,
                    "mean_abs": metrics["mean_abs"],
                    "rmse": metrics["rmse"],
                    "max_abs": metrics["max_abs"],
                    "exact_pixel_percent": metrics["exact_pixel_percent"],
                    "mean_abs_rgb": metrics["mean_abs_rgb"],
                }
            )
    if not scan_results:
        raise ValueError(f"no frames extracted from {generated_path}")
    return scan_results


def candidate_paths_from_globs(patterns: list[str]) -> list[Path]:
    candidates: set[Path] = set()
    for pattern in patterns:
        for match in glob.glob(pattern):
            path = Path(match)
            if path.is_file():
                candidates.add(path)
    return sorted(candidates, key=lambda path: str(path))


def search_candidate_videos(
    reference_path: Path,
    candidates: list[Path],
    *,
    start: float,
    end: float,
    step: float,
    ref_crop: str,
    out_dir: Path,
    reference_manifest: Path | None = None,
    top_n: int = 20,
) -> dict[str, object]:
    if not candidates:
        raise ValueError("candidate search found no files")
    if top_n <= 0:
        raise ValueError("candidate top count must be positive")

    times = scan_times(start, end, step)
    reference_source = resolve_reference_source(
        reference_path, ref_crop, reference_manifest
    )
    reference_native, crop = load_reference_native(
        reference_source.path, reference_source.ref_crop
    )
    rows: list[dict[str, object]] = []
    errors: list[dict[str, object]] = []
    for candidate in candidates:
        try:
            scan_results = scan_generated_against_reference(
                reference_native, candidate, times
            )
        except (subprocess.CalledProcessError, OSError, ValueError) as exc:
            errors.append({"generated": str(candidate), "error": str(exc)})
            continue
        best = min(scan_results, key=lambda result: float(result["mean_abs"]))
        rows.append(
            {
                "generated": str(candidate),
                "best_generated_time": best["generated_time"],
                "best_mean_abs": best["mean_abs"],
                "best_rmse": best["rmse"],
                "best_max_abs": best["max_abs"],
                "best_exact_pixel_percent": best["exact_pixel_percent"],
                "best_mean_abs_rgb": best["mean_abs_rgb"],
                "scan_count": len(scan_results),
            }
        )

    if not rows:
        raise ValueError("candidate search produced no comparable frames")

    ranked = sorted(rows, key=lambda row: float(row["best_mean_abs"]))
    for rank, row in enumerate(ranked, start=1):
        row["rank"] = rank

    summary = {
        "reference": str(reference_source.path),
        "reference_crop": list(crop),
        "candidate_count": len(candidates),
        "candidate_error_count": len(errors),
        "scan_start": start,
        "scan_end": end,
        "scan_step": step,
        "top_count": min(top_n, len(ranked)),
        "best": ranked[0],
        "top": ranked[:top_n],
        "errors": errors,
    }
    if reference_source.metadata is not None:
        summary["reference_manifest"] = reference_source.metadata
    out_dir.mkdir(parents=True, exist_ok=True)
    (out_dir / "candidate-search.json").write_text(
        json.dumps(summary, indent=2) + "\n"
    )
    compare_paths(
        reference_source.path,
        Path(str(ranked[0]["generated"])),
        generated_time=float(ranked[0]["best_generated_time"]),
        ref_crop=reference_source.ref_crop,
        out_dir=out_dir / "best",
        reference_manifest=reference_manifest,
        extra_metrics={
            "candidate_rank": ranked[0]["rank"],
            "candidate_count": len(candidates),
            "candidate_search_out": str(out_dir / "candidate-search.json"),
        },
    )
    return summary


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
        if (
            scenario.scan_start is not None
            or scenario.scan_end is not None
            or scenario.scan_step is not None
        ):
            scan_start = (
                scenario.scan_start
                if scenario.scan_start is not None
                else scenario.generated_time
            )
            scan_end = (
                scenario.scan_end if scenario.scan_end is not None else scan_start
            )
            scan_step = (
                scenario.scan_step if scenario.scan_step is not None else 1.0
            )
            metrics = scan_generated_times(
                scenario.reference,
                scenario.generated,
                start=scan_start,
                end=scan_end,
                step=scan_step,
                ref_crop=scenario.ref_crop,
                out_dir=out_dir,
                reference_manifest=scenario.reference_manifest,
                max_mean_abs=scenario.max_mean_abs,
                scenario_id=scenario.scenario_id,
                notes=scenario.notes,
            )
        else:
            metrics = compare_paths(
                scenario.reference,
                scenario.generated,
                generated_time=scenario.generated_time,
                ref_crop=scenario.ref_crop,
                out_dir=out_dir,
                reference_manifest=scenario.reference_manifest,
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
    parser.add_argument(
        "--reference-manifest",
        type=Path,
        help="Capture manifest TSV; resolves --reference frame names and crop metadata",
    )
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
    parser.add_argument(
        "--scan-generated",
        help="Scan generated MP4 timestamps as START:END:STEP and compare the best frame",
    )
    parser.add_argument(
        "--candidate-glob",
        action="append",
        help="Glob of generated MP4/image candidates to rank against --reference",
    )
    parser.add_argument(
        "--candidate-top",
        type=int,
        default=20,
        help="Number of ranked candidate matches to write; default 20",
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
        if args.reference is not None and args.candidate_glob:
            start, end, step = (
                parse_scan_range(args.scan_generated)
                if args.scan_generated
                else (args.generated_time, args.generated_time, 1.0)
            )
            out_dir = args.out_dir or default_candidate_search_out_dir(args.reference)
            summary = search_candidate_videos(
                args.reference,
                candidate_paths_from_globs(args.candidate_glob),
                start=start,
                end=end,
                step=step,
                ref_crop=args.ref_crop,
                out_dir=out_dir,
                reference_manifest=args.reference_manifest,
                top_n=args.candidate_top,
            )
            print(json.dumps(summary, indent=2))
            return 0
        parser.error(
            "--reference and --generated are required unless --scenario-file or "
            "--candidate-glob is used"
        )

    out_dir = args.out_dir or default_out_dir(args.reference, args.generated, args.generated_time)
    if args.scan_generated:
        start, end, step = parse_scan_range(args.scan_generated)
        metrics = scan_generated_times(
            args.reference,
            args.generated,
            start=start,
            end=end,
            step=step,
            ref_crop=args.ref_crop,
            out_dir=out_dir,
            reference_manifest=args.reference_manifest,
            max_mean_abs=args.max_mean_abs,
        )
    else:
        metrics = compare_paths(
            args.reference,
            args.generated,
            generated_time=args.generated_time,
            ref_crop=args.ref_crop,
            out_dir=out_dir,
            reference_manifest=args.reference_manifest,
            max_mean_abs=args.max_mean_abs,
        )
    print(json.dumps(metrics, indent=2))
    if metrics["status"] == "fail":
        return 2
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
