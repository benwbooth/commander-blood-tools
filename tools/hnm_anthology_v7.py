#!/usr/bin/env python3
# /// script
# requires-python = ">=3.11"
# dependencies = []
# ///
"""Join every HNM video of one game, each as the game itself presents it, into one master.

The per-video plan (tools/video_play_plan.py) decides how each video is shown:

* engine_clip: the native engine render of the clip in its real scene (contact
  transition over FRIGO.FD, the bridge, or the place the commander travelled to), from
  hnm_clips_render coverage; no pointer is drawn in offline exports.
* opening / credits: the native opening reel (line 0) and credits (line 1) exports.
* descript_sequence: cut from the native sequence-panel render of the DESCRIPT record
  that plays it (native-sequences-v2), at the frames its native state shows the clip.
* plain_decode: videos this game never presents, or presents only in a ship state the
  static exporter cannot build yet; decoded full screen over black, silent.

Chapters are titled with the video name, its render method and its scene.
"""

import argparse
import json
from pathlib import Path
import subprocess

from hnm_anthology import ordered, period, transcode
from native_sequence_anthology import read_json, require
from play_order_master import cut
from video_anthology import digest, inventory, resource_name, save_json


def sequence_interval(record_dir, name):
    """(start_ns, end_ns) of the first run of frames whose native state shows `name`."""
    starts = [json.loads(line)["start_ns"] for line in (record_dir / "timeline.jsonl").open()]
    end_total = starts[-1] + json.loads(list((record_dir / "timeline.jsonl").open())[-1])["duration_ns"]
    first = last = None
    for index, line in enumerate((record_dir / "native-state.jsonl").open()):
        active = json.loads(line)["state"].get("video", {}).get("active_resource")
        if active and resource_name(active.replace("\\", "/")) == name:
            first = index if first is None else first
            last = index
        elif first is not None:
            break
    if first is None:
        return None
    end = starts[last + 1] if last + 1 < len(starts) else end_total
    return starts[min(first, len(starts) - 1)], end


def build(args):
    out = args.out
    require(not out.exists(), f"output already exists: {out}")
    plan = read_json(args.plan)
    videos_plan = {v["name"]: v for v in plan["videos"]}
    catalog = read_json(args.catalog)
    assets = args.assets.resolve()
    manifest, manifest_hash = inventory(assets)
    video_dir = assets / "media-v1" / "video-v1"
    derived = {resource_name(v["source_resource_name"]): v for v in read_json(video_dir / "manifest.json")["videos"]}
    rate = read_json(video_dir / "manifest.json")["nominal_frame_rate"][0]
    clips = {}
    for directory in args.clips:
        for batch in sorted(directory.glob("*/coverage.json")):
            for entry in read_json(batch)["completed"]:
                clips[entry["clip"]] = entry
    out.mkdir(parents=True)
    (out / "parts").mkdir()
    parts, chapters, start, used = [], [";FFMETADATA1"], 0, {}
    for index, video in enumerate(ordered(catalog)):
        name = video["name"]
        entry = videos_plan[name]
        method = entry["render"]["method"]
        part = out / "parts" / f"{index:04d}.mkv"
        length = None
        if method == "engine_clip" and name in clips:
            clip = clips[name]
            require(digest(Path(clip["path"]) / "master.mkv") == clip["master_sha256"], f"clip changed: {name}")
            part, length = Path(clip["path"]) / "master.mkv", round(clip["duration_ns"])
        elif method in ("opening", "credits"):
            film = args.films / method / "master.mkv"
            require(film.exists(), f"missing {method} export")
            part = film
            length = round(read_json(args.films / method / "report.json")["runner"]["duration_ns"])
        elif method == "descript_sequence":
            record_dir = args.sequences / entry["render"]["record"]
            interval = sequence_interval(record_dir, name) if record_dir.exists() else None
            if interval:
                cut(interval[0], interval[1], part, record_dir / "master.mkv")
                length = interval[1] - interval[0]
        if length is None:
            # Never presented by the game, presented only in a ship state the exporter
            # cannot build, or an engine render that failed verification.
            if method != "plain_decode":
                method = f"plain_decode (planned {method}, no verified render)"
            source = derived[name]
            transcode(video_dir / source["webm_path"], video_dir / source["mask_webm_path"], part,
                      source["frame_count"], rate, None, period(name))
            length = round(source["frame_count"] * period(name) * 1e9)
        used[name] = method
        scene = entry.get("scene", {}).get("context") or ("not played by this game" if not entry["played"] else "")
        title = f"{name} [{method}{', ' + scene if scene else ''}]"
        chapters += ["[CHAPTER]", "TIMEBASE=1/1000000000", f"START={start}", f"END={start + length}",
                     f"title={title}"]
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
    require(abs(duration - start / 1e9) < 1.0 + 0.001 * len(parts), "stitched duration differs from the chapters")
    from collections import Counter
    save_json(out / "manifest.json", dict(
        schema=1, game=manifest.get("game", "commander_blood"),
        scope="every HNM video, each as the game presents it (video-play-plan)",
        asset_manifest_sha256=manifest_hash, plan_sha256=digest(args.plan), catalog_sha256=digest(args.catalog),
        methods=dict(Counter(used.values())), videos=len(parts), duration_s=duration,
        master=master.name, master_sha256=digest(master), tool_sha256=digest(__file__),
        video_methods=used))
    for part in (p for p in parts if p.parent == out / "parts"):
        part.unlink()
    (out / "parts").rmdir()
    print(f"{len(parts)} videos, {duration / 60:.1f} min, {dict(Counter(used.values()))}")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--assets", type=Path, required=True)
    parser.add_argument("--catalog", type=Path, required=True)
    parser.add_argument("--plan", type=Path, required=True)
    parser.add_argument("--clips", type=Path, action="append", default=[])
    parser.add_argument("--films", type=Path, required=True, help="directory with opening/ and credits/ exports")
    parser.add_argument("--sequences", type=Path, required=True, help="native-sequences-v2/<game> record renders")
    parser.add_argument("--out", type=Path, required=True)
    build(parser.parse_args())


if __name__ == "__main__":
    main()
