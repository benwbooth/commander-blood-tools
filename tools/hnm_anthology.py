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
across the record's videos in order. A Location record without a music command keeps the
stream its holder's record loaded: the ship HUD first applies the Arche link (the holder) as its
current target and a later sub-location selection only reloads music when DESCRIPT changed it.
Object clips have no authored sound and stay silent; silent-videos.json records, per silent clip,
the static evidence for why (or the missing file that blocks it).
Frames are scaled 320x200 -> 640x480 with nearest-neighbour. Playback uses the native
cadence measured from the engine's own captures: 68 ms per decoded frame for location,
character and object lines and 136 ms for sequence lines. Character clips (PE) come from
native renders over their authored backgrounds (--clips) when available; HNM palette
changes that depend on earlier display state use the decoder's context-free result.
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


def record_music(record):
    return next((c["normalized_name"] for c in record["authored"]["commands"] if c["kind"] == "music"), None)


def record_videos(record):
    for command in record["authored"]["commands"]:
        directory = DIRECTORIES.get(command["kind"])
        if directory is not None:
            yield resource_name(f"{directory}/{command.get('normalized_name') or command['name']}")


def source_holders(locations, game):
    """Location name (lower case) -> source-default HOLDER_OR_LOCATION name, identical in every profile."""
    holders = {}
    for rows in (locations or {}).get(game, {}).values():
        for row in rows:
            if row["kind"] != "Location":
                continue
            key = row["name"].lower()
            require(holders.setdefault(key, row["holder"]) == row["holder"],
                    f"source-default holder of {row['name']} differs between profiles")
    return holders


def music_plan(inventory, game, durations, holders):
    """video name -> (music resource, offset seconds, loop, source) from the DESCRIPT records.

    The DESCRIPT 0x12 command selects music (music_voc_name_patcher, BLOODPRG 0x0077A9); a record
    without one leaves the previous selection playing. Ship HUD initialization applies the Arche
    link's record as its current target (ship_hud_coordinator initialize_ship_hud), so a
    music-less Location under a holder whose record selects music plays that music.
    """
    records = [r for r in inventory["descriptions"] if r["game"] == game]
    by_name = {r["authored"]["name"].lower(): r for r in records}
    plan = {}
    for record in records:
        music = record_music(record)
        if music is None:
            continue
        offset = 0.0
        for name in record_videos(record):
            looped = record["authored"]["kind"] == "Location"
            plan.setdefault(name, (music, offset, looped, f"DESCRIPT {record['authored']['name']}"))
            offset += durations.get(name, 0.0)
    for record in records:
        if record["authored"]["kind"] != "Location" or record_music(record) is not None:
            continue
        holder = by_name.get(holders.get(record["authored"]["name"].lower(), "").lower())
        music = record_music(holder) if holder is not None and holder["authored"]["kind"] == "Location" else None
        if music is None:
            continue
        for name in record_videos(record):
            plan.setdefault(name, (music, 0.0, True,
                                   f"DESCRIPT {record['authored']['name']} retains holder {holder['authored']['name']}"))
    return plan


PERIOD = {"SQ": 0.136}  # seconds per decoded frame; every other line plays at 68 ms


def period(name):
    return PERIOD.get(name.split("/")[0], 0.068)


def capture_index(assets_game, manifests):
    """video name -> (seconds, capture directory, start_ns) of its longest native playback.

    Intervals come from the engine's own verified chapter captures, whose audio.f32le is
    the game mix (music, voices, effects) aligned with native-state time.
    """
    from native_capture_storage import open_trace
    best = {}
    for manifest in manifests:
        for source in read_json(manifest)["sources"]:
            path = Path(source["path"])
            try:
                with open_trace(path / "native-state.jsonl") as stream:
                    rows = [(json.loads(line)) for line in stream]
            except OSError:
                continue
            current, start, last = None, 0, 0
            for row in rows + [None]:
                resource = row["state"]["video"]["active_resource"] if row else None
                now = row["time_ns"] if row else last
                if row is None or resource != current:
                    if current and now - start > best.get(current, (0,))[0]:
                        best[current] = (now - start, path, start)
                    current, start = resource, now
                last = now
    return {resource_name(k): v for k, v in best.items()}


def captured_track(entry, seconds, target):
    """Write `seconds` of the captured game mix starting where the clip began."""
    length, path, start = entry
    samples = int(seconds * 48000)
    with open(path / "audio.f32le", "rb") as stream:
        stream.seek(int(start * 48000 / 1e9) * 4)
        data = stream.read(samples * 4)
    data += bytes(samples * 4 - len(data))
    target.write_bytes(data)
    return target


SILENCE_PEAK = 1e-3
_master_audio = {}


def audible(master, start_ns, end_ns):
    """Whether the master's audio peaks above SILENCE_PEAK inside one chapter."""
    import numpy as np
    if master not in _master_audio:
        raw = subprocess.run(["ffmpeg", "-v", "error", "-i", str(master), "-map", "0:a", "-f", "f32le",
                              "-ac", "1", "-ar", "48000", "-"], check=True, capture_output=True).stdout
        _master_audio.clear()
        _master_audio[master] = np.frombuffer(raw, dtype=np.float32)
    segment = _master_audio[master][int(start_ns * 48000 / 1e9):int(end_ns * 48000 / 1e9)]
    return len(segment) > 0 and float(np.abs(segment).max()) > SILENCE_PEAK


def stem_references(assets, manifest, stem):
    """Non-HNM resources and companions whose bytes name `stem` (case-insensitive)."""
    needle = stem.lower().encode()
    hits = []
    for item in manifest["resources"] + manifest.get("companions", []):
        if item["path"].upper().endswith(".HNM"):
            continue
        if needle in (assets / item["path"]).read_bytes().lower():
            hits.append(item.get("resource_name") or item["filename"])
    return hits


def silence_evidence(video, game, inventory, holders, music, audio_index, assets, manifest, catalog_names):
    """Static reason a clip has no sound in the master, from DESCRIPT, script state and resources."""
    name = video["name"]
    records = {r["authored"]["name"].lower(): r for r in inventory["descriptions"] if r["game"] == game}
    if name in music:
        resource, _, _, source = music[name]
        if audio_index.get("MU/" + resource.upper()) is None:
            return (f"blocked: {source} selects {resource}, which is absent from the game's archive "
                    f"({len(manifest['resources'])} indexed resources, no MU/{resource.upper()})")
    reasons = []
    for reference in video["references"]:
        record = records.get(reference["record"].lower())
        if record is None:
            continue
        authored = record["authored"]
        kinds = sorted({c["kind"] for c in authored["commands"]})
        if authored["kind"] == "Object":
            reasons.append(
                f"DESCRIPT Object '{authored['name']}' (directory {record['directory_index']}) carries only {kinds}: "
                "no 0x12 music and no sound bank; the inventory-description lookup presents it on presentation "
                "line 43 with PresentationMusicChange::Retained, so no sound is authored for the clip (whatever "
                "navigation stream was already playing continues)")
        elif authored["kind"] == "Character":
            talk = sum(1 for c in authored["commands"] if c["kind"] == "talk_clip")
            reasons.append(
                f"DESCRIPT Character '{authored['name']}' names it as its {reference['role']} video (line "
                f"{39 if reference['role'] == 'right' else 40}), but its {talk} talk clips fill lines 9..{8 + talk}; "
                "talk entries 31/32 overwrite lines 39/40 (opcode 07 after 09/0A, last write wins), so the engine "
                "never presents this clip" if talk >= 31 else
                f"DESCRIPT Character '{authored['name']}' {reference['role']} video; no native render or capture")
        elif authored["kind"] == "Location":
            holder_name = holders.get(authored["name"].lower())
            holder = records.get((holder_name or "").lower())
            if holder_name is None:
                reasons.append(
                    f"DESCRIPT Location '{authored['name']}' has no 0x12 music command, and its script object is "
                    "itself a ship-HUD holder (celestial body or navigation entity, not a Location) applied as the "
                    "Arche-link target, so it keeps whatever music an earlier record left playing (play-history "
                    "dependent, not authored for this clip)")
            else:
                reasons.append(
                    f"DESCRIPT Location '{authored['name']}' has no 0x12 music command and its source-default holder "
                    f"'{holder_name}' {'has no DESCRIPT record' if holder is None else 'record selects no music either'}; "
                    "the ship HUD applies the holder's record first and then retains the stream, so the sound is "
                    "whatever music an earlier record left playing (play-history dependent, not authored for this clip)")
        else:
            reasons.append(f"DESCRIPT {authored['kind']} '{authored['name']}' authors no music for it")
    if reasons:
        return "; ".join(sorted(set(reasons)))
    stem = name.split("/")[1].rsplit(".", 1)[0]
    hits = stem_references(assets, manifest, stem)
    if not hits:
        return (f"unreferenced: no DESCRIPT record, executable asset table, script or dictionary names "
                f"'{stem}' (searched every non-HNM resource and companion), so the game never plays it")
    others = sorted(n for n in catalog_names if n != name and n.split("/")[1].rsplit(".", 1)[0] == stem)
    if others:
        return (f"never played from {name.split('/')[0]}/: '{stem}' is named only in {hits}, where DESCRIPT "
                f"sequence commands and script sequence requests resolve it to {others}")
    return f"no sound found; '{stem}' is named only in {hits}"


def transcode(source, mask, target, frames, rate, music, frame_period):
    duration = frames * frame_period
    stretch = frame_period * rate
    audio = ["-f", "lavfi", "-t", f"{duration:.6f}", "-i", "anullsrc=r=48000:cl=mono"]
    if isinstance(music, Path):
        audio = ["-f", "f32le", "-ar", "48000", "-ac", "1", "-i", str(music)]
    elif music is not None:
        path, offset, looped = music
        audio = (["-stream_loop", "-1"] if looped else []) + ["-ss", f"{offset:.6f}", "-i", str(path)]
    graph = ("[1:v]format=gray[m];[0:v]format=gbrp[v];"
             f"color=c=black:s=320x200:r={rate},format=gbrp[b];[b][v][m]maskedmerge,"
             f"scale={WIDTH}:{HEIGHT}:flags=neighbor,setpts=PTS*{stretch:.6f},format=gbrp[out];"
             f"[2:a]aresample=48000,aformat=sample_fmts=fltp:channel_layouts=mono,apad[a]")
    subprocess.run(
        ["ffmpeg", "-v", "error", "-y", "-i", str(source), "-i", str(mask), *audio,
         "-filter_complex", graph, "-map", "[out]", "-map", "[a]", "-t", f"{duration:.6f}", "-fps_mode", "passthrough",
         "-c:v", "libvpx-vp9", "-lossless", "1", "-profile:v", "1", "-pix_fmt", "gbrp", "-c:a", "pcm_f32le",
         str(target)], check=True)


def build(assets, catalog_path, inventory_path, locations_path, clip_dirs, capture_manifests, out):
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
    durations = {name: entry["frame_count"] * period(name) for name, entry in by_name.items()}
    inventory_data = read_json(inventory_path)
    holders = source_holders(read_json(locations_path) if locations_path else None, game)
    music = music_plan(inventory_data, game, durations, holders)
    audio_index = {resource_name(a["source_resource_name"]): a for a in read_json(assets / "media-v1" / "manifest.json")["audio"]}
    clips = {}
    for directory in clip_dirs:
        for batch in sorted(directory.glob("*/coverage.json")) or [directory / "coverage.json"]:
            for entry in read_json(batch)["completed"]:
                clips[entry["clip"]] = entry
    out.mkdir(parents=True)
    parts, chapters, start = [], [";FFMETADATA1"], 0
    missing_music = set()
    native_clips = captured_audio = 0
    captures = capture_index(game, [Path(m) for m in capture_manifests])
    sources, spans = {}, {}
    for index, video in enumerate(videos):
        entry = by_name[video["name"]]
        require(entry["source_sha256"] == video["sha256"], f"derivative is stale: {video['name']}")
        webm = video_dir / entry["webm_path"]
        require(digest(webm) == entry["webm_sha256"], f"derivative changed: {video['name']}")
        part = out / "parts" / f"{index:04d}.mkv"
        part.parent.mkdir(exist_ok=True)
        track = None
        if video["name"] in music:
            resource, offset, looped, _ = music[video["name"]]
            wav = audio_index.get("MU/" + resource.upper())
            if wav is None:
                # DESCRIPT names music the original archive does not contain; stay silent.
                missing_music.add(resource)
                wav_path = None
            else:
                wav_path = assets / "media-v1" / wav["path"]
            if wav is not None:
                require(digest(wav_path) == wav["sha256"], f"music derivative changed: {resource}")
                track = (wav_path, offset, looped)
                sources[video["name"]] = f"music {resource} ({music[video['name']][3]})"
        clip = clips.get(video["name"])
        capture = captures.get(video["name"])
        if clip is not None:
            require(digest(Path(clip["path"]) / "master.mkv") == clip["master_sha256"], f"clip changed: {video['name']}")
            length = round(clip["duration_ns"])
            native_clips += 1
            if capture is not None:
                # Keep the native clip render's picture; lay the captured game audio under it.
                raw = captured_track(capture, length / 1e9, out / "parts" / f"{index:04d}.f32le")
                subprocess.run(["ffmpeg", "-v", "error", "-y", "-i", str(Path(clip["path"]) / "master.mkv"),
                                "-f", "f32le", "-ar", "48000", "-ac", "1", "-i", str(raw),
                                "-map", "0:v", "-map", "1:a", "-c:v", "copy", "-c:a", "pcm_f32le", "-shortest", str(part)],
                               check=True)
                raw.unlink()
                captured_audio += 1
                sources[video["name"]] = "native clip render; captured game audio"
            else:
                part = Path(clip["path"]) / "master.mkv"
                sources[video["name"]] = "native clip render audio"
        elif capture is not None:
            mask = video_dir / entry["mask_webm_path"]
            require(digest(mask) == entry["mask_webm_sha256"], f"mask derivative changed: {video['name']}")
            seconds = entry["frame_count"] * period(video["name"])
            raw = captured_track(capture, seconds, out / "parts" / f"{index:04d}.f32le")
            transcode(webm, mask, part, entry["frame_count"], rate[0], raw, period(video["name"]))
            raw.unlink()
            length = round(seconds * 1e9)
            captured_audio += 1
            sources[video["name"]] = "captured game audio"
        else:
            mask = video_dir / entry["mask_webm_path"]
            require(digest(mask) == entry["mask_webm_sha256"], f"mask derivative changed: {video['name']}")
            transcode(webm, mask, part, entry["frame_count"], rate[0], track, period(video["name"]))
            length = round(entry["frame_count"] * period(video["name"]) * 1e9)
        refs = sorted({f"{r['record']}:{r.get('kind', '')}" for r in video["references"]})
        title = f"{video['name']} [{','.join(video['categories'])}] {' '.join(refs)[:120]}"
        chapters += ["[CHAPTER]", "TIMEBASE=1/1000000000", f"START={start}", f"END={start + length}", f"title={title}"]
        spans[video["name"]] = (start, start + length)
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
        asset_manifest_sha256=manifest_hash, native_clip_renders=native_clips,
        videos_with_captured_game_audio=captured_audio, videos_with_music=sum(1 for v in videos if v["name"] in music and not all(
            m in missing_music for m in [music[v["name"]][0]])), music_missing_from_archive=sorted(missing_music), catalog_sha256=digest(catalog_path), videos=len(videos),
        frames=sum(by_name[v["name"]]["frame_count"] for v in videos), duration_s=duration,
        master=master.name, master_sha256=digest(master), tool_sha256=digest(__file__),
        video_names=[v["name"] for v in videos], audio_sources=sources))
    silent = [dict(name=v["name"], categories=v["categories"], planned_audio=sources.get(v["name"]),
                   evidence=silence_evidence(v, game, inventory_data, holders, music, audio_index, assets, manifest,
                                             {c['name'] for c in videos}))
              for v in videos if not audible(master, *spans[v["name"]])]
    save_json(out / "silent-videos.json", dict(
        schema=2, game=manifest.get("game", "commander_blood"), threshold_peak=SILENCE_PEAK,
        silent=len(silent), videos=silent))
    for part in (p for p in parts if p.parent == out / "parts"):
        part.unlink()
    (out / "parts").rmdir()
    print(f"{len(videos)} videos, {duration / 60:.1f} min")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--assets", type=Path, required=True)
    parser.add_argument("--catalog", type=Path, required=True)
    parser.add_argument("--inventory", type=Path, required=True)
    parser.add_argument("--locations", type=Path,
                        help="source-default holder chains (object-locations.json)")
    parser.add_argument("--clips", type=Path, action="append", default=[],
                        help="directory of hnm_clips_render shard directories")
    parser.add_argument("--captures", type=Path, action="append", default=[],
                        help="anthology manifest whose source chapters carry native-state and audio captures")
    parser.add_argument("--out", type=Path, required=True)
    args = parser.parse_args()
    build(args.assets.resolve(), args.catalog, args.inventory, args.locations, args.clips, args.captures, args.out)


if __name__ == "__main__":
    main()
