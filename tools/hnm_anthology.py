#!/usr/bin/env python3
# /// script
# requires-python = ">=3.11"
# dependencies = []
# ///
"""Render every HNM video of one game, in full, into one chaptered master.

Frames come from the recovered decoder's lossless WebM derivatives (media-v1/video-v1),
checked against the asset manifest and the full video catalog. Each frame shows only
the pixels that frame owns (the derivative's mask) over black, so overlay clips are
erased between frames as the game redraws their background. HNM files carry no sound
records (verified per frame); the audio is the music of the DESCRIPT record that
presents the video: Location arrival music loops under the clip, Sequence music runs
across the record's videos in order. Character and object clips have no authored music
and stay silent. Frames are scaled 320x200 -> 640x480 with nearest-neighbour and played
at the derivative's nominal 25 fps; HNM palette changes that depend on earlier display
state use the decoder's context-free result.
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


DIRECTORIES = {"location_video": "PL", "sequence_video": "SQ", "object_video": "OB"}


def music_plan(inventory, game, durations):
    """video name -> (music resource, offset seconds, loop) from the DESCRIPT records."""
    plan = {}
    for record in inventory["descriptions"]:
        if record["game"] != game:
            continue
        commands = record["authored"]["commands"]
        music = next((c["normalized_name"] for c in commands if c["kind"] == "music"), None)
        if music is None:
            continue
        offset = 0.0
        for command in commands:
            directory = DIRECTORIES.get(command["kind"])
            if directory is None:
                continue
            name = resource_name(f"{directory}/{command.get('normalized_name') or command['name']}")
            looped = record["authored"]["kind"] == "Location"
            plan.setdefault(name, (music, offset, looped))
            offset += durations.get(name, 0.0)
    return plan


def transcode(source, mask, target, frames, rate, music):
    duration = frames / rate
    audio = ["-f", "lavfi", "-t", f"{duration:.6f}", "-i", "anullsrc=r=48000:cl=mono"]
    if music is not None:
        path, offset, looped = music
        audio = (["-stream_loop", "-1"] if looped else []) + ["-ss", f"{offset:.6f}", "-i", str(path)]
    graph = ("[1:v]format=gray[m];[0:v]format=gbrp[v];"
             f"color=c=black:s=320x200:r={rate},format=gbrp[b];[b][v][m]maskedmerge,"
             f"scale={WIDTH}:{HEIGHT}:flags=neighbor,format=gbrp[out];"
             f"[2:a]aresample=48000,aformat=sample_fmts=fltp:channel_layouts=mono,apad[a]")
    subprocess.run(
        ["ffmpeg", "-v", "error", "-y", "-i", str(source), "-i", str(mask), *audio,
         "-filter_complex", graph, "-map", "[out]", "-map", "[a]", "-t", f"{duration:.6f}", "-r", str(rate),
         "-c:v", "libvpx-vp9", "-lossless", "1", "-profile:v", "1", "-pix_fmt", "gbrp", "-c:a", "pcm_f32le",
         str(target)], check=True)


def build(assets, catalog_path, inventory_path, out):
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
    game = "bbb" if manifest.get("game") == "big_bug_bang" else "cb"
    durations = {name: entry["frame_count"] / rate[0] for name, entry in by_name.items()}
    music = music_plan(read_json(inventory_path), game, durations)
    audio_index = {resource_name(a["source_resource_name"]): a for a in read_json(assets / "media-v1" / "manifest.json")["audio"]}
    out.mkdir(parents=True)
    parts, chapters, start = [], [";FFMETADATA1"], 0
    for index, video in enumerate(videos):
        entry = by_name[video["name"]]
        require(entry["source_sha256"] == video["sha256"], f"derivative is stale: {video['name']}")
        webm = video_dir / entry["webm_path"]
        require(digest(webm) == entry["webm_sha256"], f"derivative changed: {video['name']}")
        part = out / "parts" / f"{index:04d}.mkv"
        part.parent.mkdir(exist_ok=True)
        track = None
        if video["name"] in music:
            resource, offset, looped = music[video["name"]]
            wav = audio_index["MU/" + resource.upper()]
            wav_path = assets / "media-v1" / wav["path"]
            require(digest(wav_path) == wav["sha256"], f"music derivative changed: {resource}")
            track = (wav_path, offset, looped)
        mask = video_dir / entry["mask_webm_path"]
        require(digest(mask) == entry["mask_webm_sha256"], f"mask derivative changed: {video['name']}")
        transcode(webm, mask, part, entry["frame_count"], rate[0], track)
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
        schema=1, game=manifest.get("game", "commander_blood"), scope="every HNM video, masked per frame, record music where authored",
        asset_manifest_sha256=manifest_hash, videos_with_music=sum(1 for v in videos if v["name"] in music), catalog_sha256=digest(catalog_path), videos=len(videos),
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
    parser.add_argument("--inventory", type=Path, required=True)
    parser.add_argument("--out", type=Path, required=True)
    args = parser.parse_args()
    build(args.assets.resolve(), args.catalog, args.inventory, args.out)


if __name__ == "__main__":
    main()
