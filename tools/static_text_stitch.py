#!/usr/bin/env python3
# /// script
# requires-python = ">=3.11"
# dependencies = []
# ///
"""Concatenate the verified prepared scenes of one game into a chaptered master.

Streams are copied, never re-encoded. Scenes are ordered by profile, source and
offset; each is a chapter titled with its site id and record. The result carries
no more coverage than the ledger: prepared scenes, not gameplay.
"""

import argparse
import json
from pathlib import Path
import subprocess

from native_sequence_anthology import read_json, require
from video_anthology import digest, save_json


def order(identity):
    game, profile, source, offset = identity.split(".")
    return game, int(profile.removeprefix("script")), source, int(offset, 16)


def probe_duration(path):
    out = subprocess.run(["ffprobe", "-v", "error", "-show_entries", "format=duration", "-of", "json", str(path)],
                         check=True, capture_output=True, text=True).stdout
    return float(json.loads(out)["format"]["duration"])


def stitch(ledger_path, game, out):
    require(not out.exists(), f"output already exists: {out}")
    ledger = read_json(ledger_path)
    rows = sorted((row for row in ledger["sites"]
                   if row["status"] == "verified_prepared_scene" and row["id"].startswith(game + ".")),
                  key=lambda row: order(row["id"]))
    require(rows, "no verified scenes for this game")
    out.mkdir(parents=True)
    concat, chapters, start = [], [";FFMETADATA1"], 0
    for row in rows:
        detail = row["detail"]
        master = Path(detail["path"]) / "master.mkv"
        require(digest(master) == detail["master_sha256"], f"scene changed since the ledger: {row['id']}")
        escaped = str(master).replace("'", "'\\''")
        concat.append(f"file '{escaped}'")
        end = start + detail["duration_ns"]
        title = f"{row['id']} {detail['record']}".replace("\n", " ")
        chapters += ["[CHAPTER]", "TIMEBASE=1/1000000000", f"START={start}", f"END={end}", f"title={title}"]
        start = end
    (out / "concat.txt").write_text("\n".join(concat) + "\n")
    (out / "chapters.ffmeta").write_text("\n".join(chapters) + "\n")
    master = out / f"{game}-static-text-anthology.mkv"
    subprocess.run(["ffmpeg", "-v", "error", "-f", "concat", "-safe", "0", "-i", str(out / "concat.txt"),
                    "-i", str(out / "chapters.ffmeta"), "-map_metadata", "1", "-map", "0", "-c", "copy",
                    str(master)], check=True)
    duration = probe_duration(master)
    require(abs(duration - start / 1e9) < 1.0 + 0.001 * len(rows),
            f"stitched duration {duration:.3f}s differs from ledger {start / 1e9:.3f}s")
    save_json(out / "manifest.json", dict(
        schema=1, game=game, scope="prepared static text scenes; not gameplay coverage",
        ledger_sha256=digest(ledger_path), scenes=len(rows), ledger_duration_s=start / 1e9,
        stitched_duration_s=duration, master=master.name, master_sha256=digest(master),
        stitcher_sha256=digest(__file__), scene_ids=[row["id"] for row in rows]))
    print(f"{game}: {len(rows)} scenes, {duration / 3600:.2f} h")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--ledger", type=Path, required=True)
    parser.add_argument("--game", choices=("cb", "bbb"), required=True)
    parser.add_argument("--out", type=Path, required=True)
    args = parser.parse_args()
    stitch(args.ledger, args.game, args.out)


if __name__ == "__main__":
    main()
