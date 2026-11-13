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


def chapter_titles(ffmeta):
    return [line.split("=", 1)[1] for line in ffmeta.read_text().splitlines() if line.startswith("title=")]


def video_names(manifest, directory):
    if "video_names" in manifest:
        return manifest["video_names"]
    return [title.split()[0] for title in chapter_titles(directory / "chapters.ffmeta")]


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


def build(order_path, ledgers, hnm_dir, sequence_dir, inventory_path, out, jobs, voiced=None, silent_before=None,
          play_plan=None):
    order = read_json(order_path)
    game = order["game"]
    sites = {}
    for ledger in ledgers:
        for row in read_json(ledger)["sites"]:
            if row["status"] == "verified_prepared_scene" and row["id"].startswith(game + "."):
                sites[row["id"]] = row["detail"]
    hnm = read_json(hnm_dir / "manifest.json")
    hnm_master = hnm_dir / hnm["master"]
    spans = dict(zip(video_names(hnm, hnm_dir), chapters(hnm_dir / "chapters.ffmeta")))
    require(len(spans) == hnm["videos"], "HNM chapters and video names differ")
    source = {name: hnm_master for name in spans}
    if voiced is not None:
        # Clips a later all-HNM build gave sound to: take that build's chapter for them.
        other = read_json(voiced / "manifest.json")
        other_spans = dict(zip(video_names(other, voiced), chapters(voiced / "chapters.ffmeta")))
        newly = silent_names(silent_before) - silent_names(voiced / "silent-videos.json")
        for name in sorted(newly & set(other_spans)):
            spans[name], source[name] = other_spans[name], voiced / other["master"]
    texts = {s["id"]: (s["authored"].get("record_name") or "", (s["authored"].get("text") or "").replace("\n", " "))
             for s in read_json(inventory_path)["sites"] if s["game"] == game}
    out.mkdir(parents=True, exist_ok=True)
    cuts = out / "cuts"
    cuts.mkdir(exist_ok=True)

    plan, covered, missing_sites, cut_jobs = [], set(), [], []
    fixed = {}
    if play_plan is not None:
        # The opening reel and the credits are executable films with fixed places, even
        # where a sequence record also names the file (BBB 42micro names MICROFOL).
        for video in read_json(play_plan)["videos"]:
            method = (video.get("render") or {}).get("method")
            if method in ("opening", "credits") and video["name"] in spans:
                fixed[video["name"]] = method
    for entry in order["order"]:
        if entry["id"] in fixed:
            continue
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
            capture = sequence_dir / ident / "master.mkv" if sequence_dir else None
            if capture and capture.exists() and ident not in covered:
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
    def video_item(name, kind, why):
        start, end = spans[name]
        target = cuts / (name.replace("/", "_") + ".mkv")
        cut_jobs.append((start, end, target, source[name]))
        covered.add(name)
        return dict(kind=kind, id=name, path=str(target), duration_ns=end - start, title=f"{name} ({why})")

    never_played = []
    if play_plan is not None:
        # Executable-played films go where their code path first runs (video-play-plan):
        # the opening reel before anything, the ship cycle before the first bridge line,
        # the contact transition around the first contact call, hyperspace before the first
        # travel, the credits after the game. Films no game path plays close the master.
        scenes = {e["id"]: (e.get("scene") or {}).get("during") for e in order["order"] if e["kind"] == "site"}
        places_of = {e["id"]: tuple((e.get("scene") or {}).get("place") or (e.get("actor"),))
                     for e in order["order"] if e["kind"] == "site"}
        by_line, opening, credits = {}, [], []
        for video in read_json(play_plan)["videos"]:
            name = video["name"]
            if name in covered or name not in spans:
                continue
            if not video.get("played", True):
                never_played.append(name)
                continue
            method = (video.get("render") or {}).get("method")
            line = (video.get("trigger") or {}).get("line")
            if name in fixed:
                (opening if fixed[name] == "opening" else credits).append(name)
            elif name in order["unplaced_videos"] and line is not None:
                by_line.setdefault(line, []).append(name)

        def first_index(during):
            return next((i for i, item in enumerate(plan) if item["kind"] == "site" and scenes.get(item["id"]) == during),
                        None)

        def insert(index, names, why, after=False):
            if index is None or not names:
                return
            items = [video_item(n, "engine_video", why) for n in names]
            plan[index + (1 if after else 0):index + (1 if after else 0)] = items

        contact = first_index("contact")
        if contact is not None:
            end = contact
            while end + 1 < len(plan) and plan[end + 1]["kind"] == "site" and scenes.get(plan[end + 1]["id"]) == "contact":
                end += 1
            insert(end, by_line.pop(42, []), "contact transition close", after=True)
            insert(contact, by_line.pop(41, []), "contact transition open")
        # The hyperspace film index cycles with each travel: the n-th film before the first
        # line at the n-th place travelled to.
        places, travels = set(), []
        for index, item in enumerate(plan):
            if item["kind"] == "site" and scenes.get(item["id"]) == "travel":
                place = places_of.get(item["id"])
                if place not in places:
                    places.add(place)
                    travels.append(index)
        for index, name in reversed(list(zip(travels, sorted(by_line.pop(6, []))))):
            insert(index, [name], "hyperspace on a travel")
        insert(first_index("bridge"), by_line.pop(4, []) + by_line.pop(5, []), "ship presentation cycle")
        plan[0:0] = [video_item(n, "engine_video", "opening reel") for n in opening]
        plan.extend(video_item(n, "engine_video", "credits") for n in credits)
    engine_played = [name for name in order["unplaced_videos"] if name not in covered and name not in never_played]
    for name in engine_played:
        plan.append(video_item(name, "engine_video", "played by the engine; no script names it"))
    for name in never_played:
        plan.append(video_item(name, "unplayed_video", "no path in this game plays it"))

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
        counts={k: sum(1 for i in plan if i["kind"] == k)
                for k in ("site", "sequence_record", "video", "engine_video", "unplayed_video")},
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
    parser.add_argument("--sequences", type=Path, help="native sequence captures (<record>/master.mkv) to play whole")
    parser.add_argument("--play-plan", type=Path, help="video-play-plan/<game>.json: place executable-played films")
    parser.add_argument("--inventory", type=Path, required=True)
    parser.add_argument("--out", type=Path, required=True)
    parser.add_argument("--hnm-voiced", type=Path, help="later all-HNM build whose newly voiced clips replace the base's")
    parser.add_argument("--silent-before", type=Path, help="the silent-videos.json those clips were listed in")
    parser.add_argument("--jobs", type=int, default=max(1, (os.cpu_count() or 2) // 2))
    args = parser.parse_args()
    build(args.order, args.ledger, args.hnm, args.sequences, args.inventory, args.out, args.jobs,
          args.hnm_voiced, args.silent_before, args.play_plan)


if __name__ == "__main__":
    main()
