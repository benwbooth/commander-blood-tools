#!/usr/bin/env python3
# /// script
# requires-python = ">=3.11"
# dependencies = []
# ///
"""Render every HNM video of one game, in full, into one chaptered master.

Frames come from the recovered decoder's lossless WebM derivatives (media-v1/video-v1),
checked against the asset manifest and the full video catalog. HNM files carry no sound
records (verified per frame), so the stream is silent; sequences and dialogue with
audio are covered by the other masters. Frames are scaled 320x200 -> 640x480 with
nearest-neighbour and played at the derivative's nominal 25 fps; HNM palette changes
that depend on earlier display state use the decoder's context-free result.
"""

import argparse
import json
from pathlib import Path
import subprocess

from native_sequence_anthology import read_json, require
from video_anthology import digest, inventory, resource_name, save_json

CATEGORY_ORDER = ("sequence", "planetary", "object", "conversation", "unclassified")
WIDTH, HEIGHT = 640, 480


def ordered(catalog):
    def key(video):
        category = min((CATEGORY_ORDER.index(c) for c in video["categories"] if c in CATEGORY_ORDER),
                       default=len(CATEGORY_ORDER))
        return category, video["name"]
    return sorted(catalog["videos"], key=key)


def transcode(source, target, frames, rate):
    duration = frames / rate
    subprocess.run(
        ["ffmpeg", "-v", "error", "-y", "-i", str(source), "-f", "lavfi", "-t", f"{duration:.6f}",
         "-i", "anullsrc=r=48000:cl=mono", "-map", "0:v", "-map", "1:a",
         "-vf", f"scale={WIDTH}:{HEIGHT}:flags=neighbor,format=gbrp", "-r", str(rate),
         "-c:v", "libvpx-vp9", "-lossless", "1", "-profile:v", "1", "-pix_fmt", "gbrp", "-c:a", "pcm_f32le",
         str(target)], check=True)


def build(assets, catalog_path, out):
    require(not out.exists(), f"output already exists: {out}")
    catalog = read_json(catalog_path)
    manifest, manifest_hash = inventory(assets)
    require(manifest_hash == catalog["manifest_sha256"], "catalog belongs to another asset manifest")
    video_dir = assets / "media-v1" / "video-v1"
    derived = read_json(video_dir / "manifest.json")
    by_name = {resource_name(v["source_resource_name"]): v for v in derived["videos"]}
    videos = ordered(catalog)
    require({v["name"] for v in videos} == set(by_name),
            "video catalog and derivative manifest cover different HNM sets")
    rate = derived["nominal_frame_rate"]
    require(rate[1] == 1, "unexpected nominal frame rate")
    out.mkdir(parents=True)
    parts, chapters, start = [], [";FFMETADATA1"], 0
    for index, video in enumerate(videos):
        entry = by_name[video["name"]]
        require(entry["source_sha256"] == video["sha256"], f"derivative is stale: {video['name']}")
        webm = video_dir / entry["webm_path"]
        require(digest(webm) == entry["webm_sha256"], f"derivative changed: {video['name']}")
        part = out / "parts" / f"{index:04d}.mkv"
        part.parent.mkdir(exist_ok=True)
        transcode(webm, part, entry["frame_count"], rate[0])
        length = round(entry["frame_count"] / rate[0] * 1e9)
        refs = sorted({f"{r['record']}:{r.get('kind', '')}" for r in video["references"]})
        title = f"{video['name']} [{','.join(video['categories'])}] {' '.join(refs)[:120]}"
        chapters += ["[CHAPTER]", "TIMEBASE=1/1000000000", f"START={start}", f"END={start + length}", f"title={title}"]
        start += length
        parts.append(part)
    (out / "concat.txt").write_text("".join(f"file '{p.resolve()}'\n" for p in parts))
    (out / "chapters.ffmeta").write_text("\n".join(chapters) + "\n")
    master = out / "all-hnm-videos.mkv"
    subprocess.run(["ffmpeg", "-v", "error", "-f", "concat", "-safe", "0", "-i", str(out / "concat.txt"),
                    "-i", str(out / "chapters.ffmeta"), "-map_metadata", "1", "-map", "0", "-c", "copy",
                    str(master)], check=True)
    probe = subprocess.run(["ffprobe", "-v", "error", "-show_entries", "format=duration", "-of", "json", str(master)],
                           check=True, capture_output=True, text=True).stdout
    duration = float(json.loads(probe)["format"]["duration"])
    require(abs(duration - start / 1e9) < 1.0 + 0.001 * len(parts), "stitched duration differs from the frame census")
    save_json(out / "manifest.json", dict(
        schema=1, game=manifest.get("game", "commander_blood"), scope="every HNM video, silent, decoder derivative",
        asset_manifest_sha256=manifest_hash, catalog_sha256=digest(catalog_path), videos=len(videos),
        frames=sum(by_name[v["name"]]["frame_count"] for v in videos), duration_s=duration,
        master=master.name, master_sha256=digest(master), tool_sha256=digest(__file__),
        video_names=[v["name"] for v in videos]))
    for part in parts:
        part.unlink()
    (out / "parts").rmdir()
    print(f"{len(videos)} videos, {duration / 60:.1f} min")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--assets", type=Path, required=True)
    parser.add_argument("--catalog", type=Path, required=True)
    parser.add_argument("--out", type=Path, required=True)
    args = parser.parse_args()
    build(args.assets.resolve(), args.catalog, args.out)


if __name__ == "__main__":
    main()
