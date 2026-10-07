#!/usr/bin/env python3
# /// script
# requires-python = ">=3.11"
# dependencies = []
# ///
"""Join a corrected sequence/travel anthology and a static-text master, copying streams.

Chapters of both inputs are kept; the second input's chapters are offset by the
first input's duration. Claims nothing beyond the two inputs.
"""

import argparse
import json
from pathlib import Path
import re
import subprocess

from native_sequence_anthology import require
from video_anthology import digest, save_json


def chapters(path):
    out = subprocess.run(["ffprobe", "-v", "error", "-show_chapters", "-of", "json", str(path)],
                         check=True, capture_output=True, text=True).stdout
    return [(int(c["start"]), int(c["end"]), c["tags"].get("title", "")) for c in json.loads(out)["chapters"]]


def duration(path):
    out = subprocess.run(["ffprobe", "-v", "error", "-show_entries", "format=duration", "-of", "json", str(path)],
                         check=True, capture_output=True, text=True).stdout
    return float(json.loads(out)["format"]["duration"])


def combine(first, second, out):
    require(not out.exists(), f"output already exists: {out}")
    out.mkdir(parents=True)
    d1 = duration(first)
    # ffprobe reports chapter times in the container timebase (1/1e9 for these inputs).
    first_chapters = chapters(first)
    second_chapters = chapters(second)
    shift = round(d1 * 1e9)
    meta = [";FFMETADATA1"]
    for start, end, title in first_chapters:
        meta += ["[CHAPTER]", "TIMEBASE=1/1000000000", f"START={start}", f"END={end}", f"title={title}"]
    for start, end, title in second_chapters:
        meta += ["[CHAPTER]", "TIMEBASE=1/1000000000", f"START={start + shift}", f"END={end + shift}", f"title={title}"]
    (out / "chapters.ffmeta").write_text("\n".join(meta) + "\n")
    (out / "concat.txt").write_text(f"file '{first.resolve()}'\nfile '{second.resolve()}'\n")
    master = out / (re.sub(r"\.mkv$", "", second.name) + "-with-sequences.mkv")
    subprocess.run(["ffmpeg", "-v", "error", "-f", "concat", "-safe", "0", "-i", str(out / "concat.txt"),
                    "-i", str(out / "chapters.ffmeta"), "-map_metadata", "1", "-map", "0", "-c", "copy",
                    str(master)], check=True)
    total = duration(master)
    require(abs(total - (d1 + duration(second))) < 2.0, "combined duration differs from the inputs")
    save_json(out / "manifest.json", dict(
        schema=1, first=str(first.resolve()), first_sha256=digest(first), second=str(second.resolve()),
        second_sha256=digest(second), chapters=len(first_chapters) + len(second_chapters),
        duration_s=total, master=master.name, master_sha256=digest(master)))
    print(f"{master}: {total / 3600:.2f} h, {len(first_chapters) + len(second_chapters)} chapters")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--first", type=Path, required=True)
    parser.add_argument("--second", type=Path, required=True)
    parser.add_argument("--out", type=Path, required=True)
    args = parser.parse_args()
    combine(args.first, args.second, args.out)


if __name__ == "__main__":
    main()
