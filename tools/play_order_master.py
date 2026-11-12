#!/usr/bin/env python3
# /// script
# requires-python = ">=3.11"
# dependencies = []
# ///
"""Assemble one game's clips into a single chaptered master in static play order.

The order is play_order.py's. Each entry takes the most faithful render that exists:

* text sites: the verified prepared-scene render from the static-text ledger (actor
  clip animated over its authored background, native fonts, voices);
* DESCRIPT sequence records: the engine's native capture of the whole record
  (its videos in order under the record's music); its videos are then covered;
* other videos: their chapter of the all-HNM master (native clip renders, captured or
  authored audio), cut frame-exact and re-encoded losslessly. A character clip whose
  line is rendered as a verified site is already shown there and is not repeated.

Videos no script names (engine-played: hyperspace, cryobox, logos, TV channels) follow
the game as a final block of chapters, so nothing is dropped. Streams are copied
except for the cut HNM chapters.
"""

import argparse
from concurrent.futures import ThreadPoolExecutor
import json
import os
from pathlib import Path
import re
import subprocess

from native_sequence_anthology import read_json, require
from video_anthology import digest, save_json


def chapters(ffmeta):
    out, current = [], {}
    for line in ffmeta.read_text().splitlines():
        if line == "[CHAPTER]":
            current = {}
            out.append(current)
        elif "=" in line and out:
            key, value = line.split("=", 1)
            current[key] = value
    return [(int(c["START"]), int(c["END"])) for c in out]


def probe_duration(path):
    out = subprocess.run(["ffprobe", "-v", "error", "-show_entries", "format=duration", "-of", "json", str(path)],
                         check=True, capture_output=True, text=True).stdout
    return float(json.loads(out)["format"]["duration"])


def cut(start_ns, end_ns, target, master):
    if target.exists():
        return target
    temporary = target.with_suffix(".tmp.mkv")
    subprocess.run(
        ["ffmpeg", "-v", "error", "-y", "-ss", f"{start_ns / 1e9:.6f}", "-i", str(master),
         "-t", f"{(end_ns - start_ns) / 1e9:.6f}", "-map", "0:v", "-map", "0:a", "-fps_mode", "passthrough",
         "-c:v", "libvpx-vp9", "-lossless", "1", "-profile:v", "1", "-pix_fmt", "gbrp", "-row-mt", "1",
         "-c:a", "pcm_f32le", str(temporary)], check=True)
    temporary.rename(target)
    return target


def silent_names(path):
    data = read_json(path)
    return {v["name"] for v in data["videos"]} if isinstance(data, dict) else set(data)


def build(order_path, ledgers, hnm_dir, sequence_dir, inventory_path, out, jobs, voiced=None, silent_before=None):
    order = read_json(order_path)
    game = order["game"]
    sites = {}
    for ledger in ledgers:
        for row in read_json(ledger)["sites"]:
            if row["status"] == "verified_prepared_scene" and row["id"].startswith(game + "."):
                sites[row["id"]] = row["detail"]
    hnm = read_json(hnm_dir / "manifest.json")
    hnm_master = hnm_dir / hnm["master"]
    spans = dict(zip(hnm["video_names"], chapters(hnm_dir / "chapters.ffmeta")))
    require(len(spans) == hnm["videos"], "HNM chapters and video names differ")
    source = {name: hnm_master for name in spans}
    if voiced is not None:
        # Clips a later all-HNM build gave sound to: take that build's chapter for them.
        other = read_json(voiced / "manifest.json")
        other_spans = dict(zip(other["video_names"], chapters(voiced / "chapters.ffmeta")))
        newly = silent_names(silent_before) - silent_names(voiced / "silent-videos.json")
        for name in sorted(newly & set(other_spans)):
            spans[name], source[name] = other_spans[name], voiced / other["master"]
    texts = {s["id"]: (s["authored"].get("record_name") or "", (s["authored"].get("text") or "").replace("\n", " "))
             for s in read_json(inventory_path)["sites"] if s["game"] == game}
    out.mkdir(parents=True, exist_ok=True)
    cuts = out / "cuts"
    cuts.mkdir(exist_ok=True)

    plan, covered, missing_sites, cut_jobs = [], set(), [], []
    for entry in order["order"]:
        kind, ident = entry["kind"], entry["id"]
        if kind == "site":
            detail = sites.get(ident)
            if detail is None:
                missing_sites.append(ident)
                continue
            actor, text = texts.get(ident, ("", ""))
            profile = ident.split(".")[1].upper()
            plan.append(dict(kind="site", id=ident, path=str(Path(detail["path"]) / "master.mkv"),
                             duration_ns=detail["duration_ns"], title=f"{profile} {actor}: {text[:70]}"))
        elif kind == "sequence_record":
            capture = sequence_dir / ident / "master.mkv"
            if capture.exists() and ident not in covered:
                covered.add(ident)
                covered.update(entry["videos"])
                plan.append(dict(kind="sequence_record", id=ident, path=str(capture), duration_ns=None,
                                 title=f"sequence {ident} ({entry['why']})"))
        elif kind == "video":
            if ident in covered or entry.get("shown_in_site") in sites:
                continue
            covered.add(ident)
            start, end = spans[ident]
            target = cuts / (ident.replace("/", "_") + ".mkv")
            cut_jobs.append((start, end, target, source[ident]))
            plan.append(dict(kind="video", id=ident, path=str(target), duration_ns=end - start,
                             title=f"{ident} ({entry['why']})"))
    engine_played = [name for name in order["unplaced_videos"] if name not in covered]
    for name in engine_played:
        start, end = spans[name]
        target = cuts / (name.replace("/", "_") + ".mkv")
        cut_jobs.append((start, end, target, source[name]))
        plan.append(dict(kind="engine_video", id=name, path=str(target), duration_ns=end - start,
                         title=f"{name} (played by the engine; no script names it)"))

    with ThreadPoolExecutor(jobs) as pool:
        list(pool.map(lambda job: cut(*job), cut_jobs))

    meta, start = [";FFMETADATA1"], 0
    for item in plan:
        # The concat demuxer advances by each file's container duration; a lossless cut
        # can run a frame past its chapter, so cut parts are measured, not assumed.
        if item["kind"] in ("video", "engine_video") or not item["duration_ns"]:
            item["duration_ns"] = round(probe_duration(Path(item["path"])) * 1e9)
        length = item["duration_ns"]
        item["duration_ns"] = length
        title = re.sub(r"[=;#\\\n]", " ", item["title"])
        meta += ["[CHAPTER]", "TIMEBASE=1/1000000000", f"START={start}", f"END={start + length}", f"title={title}"]
        start += length
    (out / "chapters.ffmeta").write_text("\n".join(meta) + "\n")
    (out / "concat.txt").write_text("".join(f"file '{Path(i['path']).resolve()}'\n" for i in plan))
    master = out / f"{game}-play-order.mkv"
    subprocess.run(["ffmpeg", "-v", "error", "-y", "-f", "concat", "-safe", "0", "-i", str(out / "concat.txt"),
                    "-i", str(out / "chapters.ffmeta"), "-map_metadata", "1", "-map", "0", "-c", "copy",
                    str(master)], check=True)
    duration = probe_duration(master)
    require(abs(duration - start / 1e9) < 2.0 + 0.002 * len(plan), "stitched duration differs from the parts")
    save_json(out / "manifest.json", dict(
        schema=1, game=game, scope="every rendered text site and HNM video in static play order",
        order=str(order_path.resolve()), order_sha256=digest(order_path),
        ledgers=[str(Path(l).resolve()) for l in ledgers], hnm_master_sha256=hnm["master_sha256"],
        voiced_from={n: str(m) for n, m in source.items() if m != hnm_master},
        assembler_sha256=digest(__file__), chapters=len(plan),
        counts={k: sum(1 for i in plan if i["kind"] == k) for k in ("site", "sequence_record", "video", "engine_video")},
        sites_without_render=missing_sites, duration_s=duration, master=master.name,
        plan=[{k: v for k, v in i.items() if k != "path"} for i in plan]))
    print(f"{master}: {duration / 3600:.2f} h, {len(plan)} chapters, {len(missing_sites)} sites lack a render, "
          f"{len(engine_played)} engine-played videos at the end")


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--order", type=Path, required=True)
    parser.add_argument("--ledger", type=Path, action="append", required=True,
                        help="static-text ledger; later ledgers override earlier ones")
    parser.add_argument("--hnm", type=Path, required=True, help="all-HNM master directory")
    parser.add_argument("--sequences", type=Path, required=True, help="native sequence captures (<record>/master.mkv)")
    parser.add_argument("--inventory", type=Path, required=True)
    parser.add_argument("--out", type=Path, required=True)
    parser.add_argument("--hnm-voiced", type=Path, help="later all-HNM build whose newly voiced clips replace the base's")
    parser.add_argument("--silent-before", type=Path, help="the silent-videos.json those clips were listed in")
    parser.add_argument("--jobs", type=int, default=max(1, (os.cpu_count() or 2) // 2))
    args = parser.parse_args()
    build(args.order, args.ledger, args.hnm, args.sequences, args.inventory, args.out, args.jobs,
          args.hnm_voiced, args.silent_before)


if __name__ == "__main__":
    main()
