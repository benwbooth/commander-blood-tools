#!/usr/bin/env python3
"""Write capture-manifest.tsv metadata for an existing oracle frame directory.

This is for older `accuracy/captures/frame_NN.png` sets captured before
`run_oracle.sh` started writing manifests. It does not inspect image pixels; it
records the crop/native metadata needed by `compare_oracle.py`.
"""

from __future__ import annotations

import argparse
import csv
import re
from pathlib import Path


DEFAULT_CROP = (80, 100, 640, 480)
DEFAULT_NATIVE_SIZE = (320, 200)
FRAME_RE = re.compile(r"^frame_(\d+)\.png$", re.IGNORECASE)
MANIFEST_FIELDS = [
    "frame",
    "path",
    "elapsed_s",
    "epoch_s",
    "display",
    "capture_kind",
    "crop_x",
    "crop_y",
    "crop_w",
    "crop_h",
    "native_w",
    "native_h",
]


def parse_int_tuple(value: str, *, count: int, label: str) -> tuple[int, ...]:
    parts = value.split(",")
    if len(parts) != count:
        raise argparse.ArgumentTypeError(
            f"{label} must contain {count} comma-separated integers"
        )
    try:
        parsed = tuple(int(part) for part in parts)
    except ValueError as exc:
        raise argparse.ArgumentTypeError(f"{label} must contain integers") from exc
    if any(part < 0 for part in parsed):
        raise argparse.ArgumentTypeError(f"{label} values must be non-negative")
    dimension_values = parsed[2:] if count > 2 else parsed
    if any(part == 0 for part in dimension_values):
        raise argparse.ArgumentTypeError(f"{label} dimensions must be positive")
    return parsed


def frame_sort_key(path: Path) -> tuple[int, str]:
    match = FRAME_RE.match(path.name)
    if match is None:
        return (1_000_000_000, path.name)
    return (int(match.group(1)), path.name)


def discover_frames(capture_dir: Path) -> list[Path]:
    return sorted(
        (path for path in capture_dir.glob("frame_*.png") if path.is_file()),
        key=frame_sort_key,
    )


def write_capture_manifest(
    capture_dir: Path,
    manifest_path: Path,
    *,
    crop: tuple[int, int, int, int] = DEFAULT_CROP,
    native_size: tuple[int, int] = DEFAULT_NATIVE_SIZE,
    interval_s: float = 1.0,
    display: str = "",
    capture_kind: str = "host-root-retrofit",
    epoch_s: float = 0.0,
) -> int:
    frames = discover_frames(capture_dir)
    if not frames:
        raise ValueError(f"no frame_*.png files found in {capture_dir}")
    if interval_s <= 0:
        raise ValueError("interval_s must be positive")

    manifest_path.parent.mkdir(parents=True, exist_ok=True)
    with manifest_path.open("w", newline="") as f:
        writer = csv.DictWriter(
            f,
            fieldnames=MANIFEST_FIELDS,
            delimiter="\t",
            lineterminator="\n",
        )
        writer.writeheader()
        for index, frame in enumerate(frames, start=1):
            writer.writerow(
                {
                    "frame": frame.name,
                    "path": str(frame.resolve()),
                    "elapsed_s": format_number(index * interval_s),
                    "epoch_s": format_number(epoch_s),
                    "display": display,
                    "capture_kind": capture_kind,
                    "crop_x": crop[0],
                    "crop_y": crop[1],
                    "crop_w": crop[2],
                    "crop_h": crop[3],
                    "native_w": native_size[0],
                    "native_h": native_size[1],
                }
            )
    return len(frames)


def format_number(value: float) -> str:
    if float(value).is_integer():
        return str(int(value))
    return f"{value:.6f}".rstrip("0").rstrip(".")


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "capture_dir",
        nargs="?",
        type=Path,
        default=Path("accuracy/captures"),
        help="Directory containing frame_NN.png captures; default accuracy/captures",
    )
    parser.add_argument(
        "--manifest",
        type=Path,
        help="Output manifest path; default <capture_dir>/capture-manifest.tsv",
    )
    parser.add_argument(
        "--crop",
        type=lambda value: parse_int_tuple(value, count=4, label="crop"),
        default=DEFAULT_CROP,
        help="Reference crop as x,y,w,h; default 80,100,640,480",
    )
    parser.add_argument(
        "--native-size",
        type=lambda value: parse_int_tuple(value, count=2, label="native-size"),
        default=DEFAULT_NATIVE_SIZE,
        help="Native output size as w,h; default 320,200",
    )
    parser.add_argument(
        "--interval",
        type=float,
        default=1.0,
        help="Elapsed seconds between frame_NN captures; default 1",
    )
    parser.add_argument(
        "--display",
        default="",
        help="Display label to store in the manifest when known",
    )
    parser.add_argument(
        "--capture-kind",
        default="host-root-retrofit",
        help="Capture kind label; default host-root-retrofit",
    )
    parser.add_argument(
        "--epoch",
        type=float,
        default=0.0,
        help="Epoch timestamp to store when original capture time is unknown; default 0",
    )
    return parser


def main(argv: list[str] | None = None) -> int:
    parser = build_parser()
    args = parser.parse_args(argv)
    manifest = args.manifest or args.capture_dir / "capture-manifest.tsv"
    count = write_capture_manifest(
        args.capture_dir,
        manifest,
        crop=args.crop,
        native_size=args.native_size,
        interval_s=args.interval,
        display=args.display,
        capture_kind=args.capture_kind,
        epoch_s=args.epoch,
    )
    print(f"wrote {manifest} for {count} frame(s)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
