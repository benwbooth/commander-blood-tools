#!/usr/bin/env python3
"""Render DESCRIPT chapters and assemble verified native variable-rate masters.

This covers authored sequence records, not all game dialogue or gameplay routes.
Uses the production offline exporter, FFmpeg, and the typed video-catalog binary.
"""

import argparse
from fractions import Fraction
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import tempfile

from video_anthology import ROOT, command, digest, inventory, metadata_text, resource_name, save_json


def require(condition, message):
    if not condition:
        raise ValueError(message)


def read_json(path):
    return json.loads(Path(path).read_text())


def timeline(path):
    with Path(path).open() as source:
        rows = [json.loads(line) for line in source]
    time_ns = samples = 0
    for frame, row in enumerate(rows):
        require(row["frame"] == frame and row["start_ns"] == time_ns
                and row["sample_start"] == samples, "non-contiguous native timeline")
        duration = row["duration_ns"]
        require(duration > 0 and duration % 1_000_000 == 0, "invalid native frame duration")
        time_ns += duration
        samples += row["sample_count"]
        require(samples * 1_000_000_000 == time_ns * 48_000, "native sample-clock mismatch")
    require(rows, "empty native capture")
    return rows, time_ns, samples


def verify_media(path, rows, duration_ns, width=640, height=480):
    """Decode every pixel/sample and compare every native interval, not just headers."""
    hashes = {}
    for video in (True, False):
        args = ["ffmpeg", "-nostdin", "-v", "error", "-i", path]
        if video:
            args += ["-map", "0:v:0", "-fps_mode", "passthrough", "-pix_fmt", "rgba",
                     "-enc_time_base:v", "1:1000", "-f", "rawvideo"]
        else:
            args += ["-map", "0:a:0", "-c:a", "pcm_f32le", "-f", "f32le"]
        args += ["pipe:1"]
        key = "rgba_sha256" if video else "audio_sha256"
        whole = hashlib.sha256()
        with subprocess.Popen(args, stdout=subprocess.PIPE) as process:
            try:
                for index, row in enumerate(rows):
                    count = width * height * 4 if video else row["sample_count"] * 4
                    data = process.stdout.read(count)
                    require(len(data) == count, f"truncated {key} interval {index}: {path}")
                    require(hashlib.sha256(data).hexdigest() == row[key],
                            f"changed {key} interval {index}: {path}")
                    whole.update(data)
                require(not process.stdout.read(1), f"extra decoded {key} data: {path}")
                require(process.wait() == 0, f"decoder failed: {path}")
            finally:
                if process.poll() is None:
                    process.kill()
                    process.wait()
        hashes[key] = whole.hexdigest()
    probe = json.loads(command([
        "ffprobe", "-v", "error", "-select_streams", "v:0", "-show_frames",
        "-show_streams", "-show_format", "-show_chapters", "-show_entries",
        "frame=best_effort_timestamp:stream=time_base:format=duration:chapter=start_time,end_time,tags",
        "-of", "json", path], capture_output=True, text=True).stdout)
    validate_timing(probe, rows, duration_ns)
    return hashes, probe.get("chapters", [])


def validate_timing(probe, rows, duration_ns):
    require(probe["streams"][0]["time_base"] == "1/1000", "unexpected video time base")
    actual = [frame["best_effort_timestamp"] for frame in probe["frames"]]
    require(actual == [row["start_ns"] // 1_000_000 for row in rows], "encoded video timestamps differ")
    endpoint = Fraction(probe["format"]["duration"]) * 1_000_000_000
    require(abs(endpoint - duration_ns) < 1000, "encoded endpoint differs")


def validate_playlist(record, report, states, available=None):
    expected = [resource_name(item["resource"]) for item in record["references"]
                if item["role"] == "sequence"]
    authored = [resource_name("SQ/" + name) for name in report["runner"]["authored_videos"]]
    require(expected == authored, "exported playlist differs from authored DESCRIPT order")
    missing = [name for name in expected if available is not None and name not in available]
    playable = [name for name in expected if name not in missing]
    observed = []
    previous = None
    cue_times = {}
    last_sequence_frame = 0
    for state in states:
        video = state["state"]["video"]
        name = video["active_resource"]
        name = resource_name(name) if name else None
        caption = state["state"].get("sequence_caption", {})
        identity = (name, caption.get("remaining_scene_lines"))
        if identity != previous and name:
            observed.append(name)
        previous = identity
        metrics = video.get("queue_metrics")
        if metrics:
            current = metrics["sequence_index"]
            require(current >= last_sequence_frame or last_sequence_frame > 65_000,
                    "native sequence clock restarted between clips")
            last_sequence_frame = current
        cue = caption.get("cue_index")
        if cue is not None:
            cue_times.setdefault(cue, state["time_ns"])
    require(observed == playable, f"observed native playlist differs: {observed} != {playable}")
    cues = report["runner"].get("caption_cues", [])
    require(all(0 <= index < len(cues) for index in cue_times), "invalid observed caption cue")
    unseen = [dict(index=index, **cue) for index, cue in enumerate(cues) if index not in cue_times]
    require(not any(cue["display_text"] and 0 <= cue["authored_frame"] < last_sequence_frame
                    for cue in unseen), "a reachable nonblank authored caption was not drawn")
    return dict(observed_clips=observed, missing_authored_resources=missing,
                caption_first_native_state_ns=cue_times, unshown_caption_cues=unseen,
                final_sequence_frame=last_sequence_frame)


def verify_chapter(path, record, manifest, exporter_hash):
    report = read_json(path / "report.json")
    require(report["target"] == "descript_sequence" and report["complete_native_presentation"]
            and not report["complete_game"], "not a completed standalone sequence")
    require(report["exporter_sha256"] == exporter_hash, "exporter changed; use a new output directory")
    require(report["runner"]["sequence_record"] == record["name"], "wrong sequence record")
    require(report["runner"]["selected_record_at_ns"] is not None, "missing explicit selection provenance")
    source = read_json(path / "source-manifest.json")
    for key in ("resources", "companions"):
        require(source[key] == manifest[key], "source asset manifest changed")
    require(report["game"] == manifest.get("game", "commander_blood"), "wrong game")
    rows, duration, samples = timeline(path / "timeline.jsonl")
    require((len(rows), duration, samples) == tuple(report["runner"][key] for key in
            ("presented_frames", "duration_ns", "audio_samples")), "runner accounting mismatch")
    require(digest(path / "endpoint.rgba") == report["endpoint_rgba_sha256"], "damaged endpoint")
    with (path / "native-state.jsonl").open() as stream:
        trace = validate_playlist(record, report, (json.loads(line) for line in stream),
                                  {resource_name(item["resource_name"]) for item in manifest["resources"]})
    hashes, _ = verify_media(path / "master.mkv", rows, duration)
    require(hashes["rgba_sha256"] == report["rgba_sha256"]
            and hashes["audio_sha256"] == report["audio_f32le_sha256"], "decoded report hash mismatch")
    require(digest(path / "audio.f32le") == hashes["audio_sha256"], "damaged raw audio")
    return dict(record=record["name"], path=str(path), duration_ns=duration, frames=len(rows),
                audio_samples=samples, caption_cues=len(report["runner"]["caption_cues"]),
                master_sha256=digest(path / "master.mkv"), **hashes, **trace)


def render(args):
    assets = args.assets.resolve()
    manifest, manifest_hash = inventory(assets)
    descript = next(item for item in manifest["resources"]
                    if resource_name(item["resource_name"]) == "DESCRIPT.DES")
    catalog = json.loads(command([args.catalog_binary.resolve(), assets / descript["path"]],
                                 capture_output=True, text=True).stdout)
    records = [record for record in catalog["records"] if record["kind"] == "Sequence"]
    require(records, "no authored sequence records")
    names = [record["name"] for record in records]
    require(len(names) == len(set(names)), "duplicate sequence record names")
    require(all(name and name not in (".", "..") and "/" not in name and "\\" not in name
                for name in names), "unsafe sequence output name")
    args.out = args.out.resolve()
    args.out.mkdir(parents=True, exist_ok=True)
    provenance = dict(schema=1, game=manifest.get("game", "commander_blood"),
                      asset_manifest_sha256=manifest_hash, exporter_sha256=digest(args.exporter),
                      catalog_binary_sha256=digest(args.catalog_binary), records=records)
    provenance_path = args.out / "selection.json"
    if provenance_path.exists():
        require(read_json(provenance_path) == provenance, "batch inputs changed; use a new output directory")
    else:
        require(not list(args.out.iterdir()), "unrecognized nonempty batch output directory")
        save_json(provenance_path, provenance)
    completed = []
    failures = []
    for index, record in enumerate(records):
        path = args.out / record["name"]
        print(f"[{index + 1}/{len(records)}] {record['name']}", flush=True)
        try:
            if not path.exists():
                with (args.out / (record["name"] + ".log")).open("w") as log:
                    command([args.exporter.resolve(), assets, "sequence:" + record["name"], path,
                             args.max_frames], stdout=log, stderr=subprocess.STDOUT)
            completed.append(verify_chapter(path, record, manifest, provenance["exporter_sha256"]))
            print(f"  verified {completed[-1]['duration_ns'] / 1e9:.3f}s", flush=True)
        except (ValueError, OSError, KeyError, subprocess.CalledProcessError) as error:
            failures.append(dict(record=record["name"], error=str(error)))
            print(f"  failed: {error}", flush=True)
        save_json(args.out / "coverage.json", dict(schema=1, provenance=provenance,
                  scope="DESCRIPT sequence records only; not complete game or gameplay reachability",
                  complete=len(completed) == len(records), completed=completed, failures=failures))
    require(not failures, f"{len(failures)} sequence exports failed; see coverage.json and logs")


def concatenate_timelines(chapters):
    result = []
    offset_ns = offset_samples = 0
    for rows, duration, samples in chapters:
        for row in rows:
            result.append(dict(row, frame=len(result), start_ns=row["start_ns"] + offset_ns,
                               sample_start=row["sample_start"] + offset_samples))
        offset_ns += duration
        offset_samples += samples
    return result, offset_ns, offset_samples


def assembly_entries(batches):
    require(batches, "no native batches")
    entries = []
    game = None
    seen = set()
    for batch in batches:
        coverage = read_json(batch / "coverage.json")
        require(coverage["complete"] and not coverage["failures"], "native batch is incomplete")
        require([entry["record"] for entry in coverage["completed"]] ==
                [record["name"] for record in coverage["provenance"]["records"]],
                "batch chapter order differs")
        current_game = coverage["provenance"]["game"]
        require(game is None or game == current_game, "native batches belong to different games")
        game = current_game
        for entry in coverage["completed"]:
            path = Path(entry["path"]).resolve()
            require(path not in seen, "duplicate native chapter in assembly")
            seen.add(path)
            entries.append(entry)
    return game, entries


def replace_travel_entries(base, replacements):
    original = base["sources"]
    require([entry["record"] for entry in original] ==
            [chapter["title"] for chapter in base["chapters"]],
            "base anthology chapter order differs from its sources")
    require(len({entry["record"] for entry in original}) == len(original),
            "base anthology has duplicate chapter titles")
    old_travel = {}
    for entry in original:
        if "report_sha256" not in entry:
            continue
        report_path = Path(entry["path"]) / "report.json"
        require(digest(report_path) == entry["report_sha256"], "base dialogue report changed")
        report = read_json(report_path)
        plan = report["runner"]["chapter"]
        require(plan["title"] == entry["record"], "base dialogue title differs from its plan")
        if plan.get("entry") == "travel":
            old_travel[entry["record"]] = plan
    require(old_travel, "base anthology has no travel chapters")
    game, newer = assembly_entries(replacements)
    require(game == base["game"], "replacement chapters belong to another game")
    replacements_by_title = {}
    for entry in newer:
        title = entry["record"]
        require(title in old_travel and title not in replacements_by_title,
                "replacement is not a unique base travel chapter")
        report_path = Path(entry["path"]) / "report.json"
        require(digest(report_path) == entry["report_sha256"], "replacement dialogue report changed")
        report = read_json(report_path)
        require(report["runner"]["chapter"] == old_travel[title],
                "replacement travel plan changed")
        require("travel_music" in report["runner"], "replacement has no travel music provenance")
        replacements_by_title[title] = entry
    missing = old_travel.keys() - replacements_by_title.keys()
    require(not missing, f"missing {len(missing)} travel replacements: {sorted(missing)[:5]}")
    return game, [replacements_by_title.get(entry["record"], entry) for entry in original]


def dialogue_source_order(entries):
    sequences, dialogue = [], []
    seen_dialogue = False
    for index, entry in enumerate(entries):
        if "report_sha256" not in entry:
            require(not seen_dialogue, "sequence batch follows dialogue batch")
            sequences.append(entry)
            continue
        seen_dialogue = True
        path = Path(entry["path"])
        report_path = path / "report.json"
        require(digest(report_path) == entry["report_sha256"], "dialogue report changed")
        report = read_json(report_path)
        require(report["target"] == "dialogue_chapter", "source-ordered entry is not dialogue")
        plan = report["runner"]["chapter"]
        require(plan["title"] == entry["record"], "dialogue title differs from source plan")
        anchor = plan.get("contact_procedure")
        if anchor is None:
            anchor = (plan.get("travel_setup") or {}).get("procedure_offset")
        if anchor is None:
            sites = plan.get("required_cod_sites") or entry.get("published_cod_sites") or []
            require(sites, "dialogue has no source anchor")
            anchor = min(sites)
        require(isinstance(anchor, int) and anchor >= 0, "dialogue has no source anchor")
        dialogue.append(((plan["initial_profile"], anchor, index), entry))
    require(dialogue, "no dialogue chapters to source-order")
    return sequences + [entry for _, entry in sorted(dialogue)]


def assemble(args):
    base_path = getattr(args, "base_manifest", None)
    replacement_batches = getattr(args, "replacement_batch", None) or []
    batches = args.batch or []
    if base_path is not None:
        require(not batches and replacement_batches,
                "base assembly requires replacement batches and no ordinary batches")
        base = read_json(base_path)
        game, entries = replace_travel_entries(base, replacement_batches)
    else:
        require(batches and not replacement_batches, "ordinary assembly requires batches")
        game, entries = assembly_entries(batches)
    source_order = getattr(args, "dialogue_source_order", False)
    require(not (base_path and source_order), "base assembly already has an authored chapter order")
    if source_order:
        entries = dialogue_source_order(entries)
    require(not args.out.exists(), f"output already exists: {args.out}")
    args.out.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="native-assembly-", dir=args.out.parent) as temp:
        temp = Path(temp)
        listing = []
        chapters = []
        timelines = []
        offset = 0
        with (temp / "audio.f32le").open("wb") as audio:
            for entry in entries:
                path = Path(entry["path"])
                require(digest(path / "master.mkv") == entry["master_sha256"], "chapter master changed")
                require(digest(path / "audio.f32le") == entry["audio_sha256"], "chapter PCM changed")
                native = timeline(path / "timeline.jsonl")
                require(native[1] == entry["duration_ns"] and len(native[0]) == entry["frames"],
                        "chapter timing changed")
                timelines.append(native)
                name = str(path / "master.mkv")
                require("\n" not in name and "\r" not in name, "newline in chapter path")
                name = name.replace("'", "'\\''")
                listing += [f"file '{name}'", f"duration {native[1] // 1_000_000_000}.{native[1] % 1_000_000_000:09d}"]
                chapters.append(dict(start_ns=offset, end_ns=offset + native[1], title=entry["record"]))
                offset += native[1]
                with (path / "audio.f32le").open("rb") as source:
                    shutil.copyfileobj(source, audio)
        rows, duration, samples = concatenate_timelines(timelines)
        (temp / "concat.txt").write_text("\n".join(listing) + "\n")
        (temp / "chapters.ffmeta").write_text(metadata_text(chapters))
        packet_durations = (f"setts=duration='if(eq(N,{len(rows) - 1}),"
                            f"{rows[-1]['duration_ns']}/(1000000000*TB),NEXT_PTS-PTS)'")
        command(["ffmpeg", "-nostdin", "-v", "error", "-n", "-f", "concat", "-safe", "0", "-i", temp / "concat.txt",
                 "-f", "f32le", "-ar", "48000", "-ac", "1", "-i", temp / "audio.f32le",
                 "-f", "ffmetadata", "-i", temp / "chapters.ffmeta", "-map", "0:v:0", "-map", "1:a:0",
                 "-map_metadata", "2", "-map_chapters", "2", "-c:v", "copy", "-bsf:v", packet_durations,
                 "-c:a", "pcm_f32le", temp / "master.mkv"])
        hashes, encoded_chapters = verify_media(temp / "master.mkv", rows, duration)
        require(len(encoded_chapters) == len(chapters), "encoded chapter count differs")
        for actual, expected in zip(encoded_chapters, chapters):
            for boundary in ("start", "end"):
                require(Fraction(actual[boundary + "_time"]) * 1_000_000_000 == expected[boundary + "_ns"],
                        "encoded chapter boundary differs")
            require(actual["tags"]["title"] == expected["title"], "encoded chapter title differs")
        scope = (base["scope"] + "; every travel chapter recaptured with selected music"
                 if base_path else "verified native sequence and dialogue chapters; not complete-game coverage"
                 if len(batches) > 1 else read_json(batches[0] / "coverage.json")["scope"])
        manifest = dict(schema=1, game=game, scope=scope,
                        sources=entries, chapters=chapters, frames=len(rows), duration_ns=duration,
                        audio_samples=samples, master_sha256=digest(temp / "master.mkv"), **hashes,
                        **(dict(ordering=base.get("ordering")) if base_path else
                           dict(ordering="sequence_block_then_dialogue_source_offset")
                           if source_order else {}),
                        **(dict(supersedes_manifest=str(base_path.resolve()),
                                replacement_batches=[str(path.resolve()) for path in replacement_batches])
                           if base_path else {}),
                        **(dict(source_batches=[str(batch.resolve()) for batch in batches])
                           if len(batches) > 1 else {}))
        require(not args.out.exists(), "assembly output appeared during verification")
        (temp / "master.mkv").rename(args.out)
        save_json(args.out.with_suffix(".manifest.json"), manifest)
    print(f"Verified {len(entries)} chapters, {duration / 1e9:.3f}s: {args.out}", flush=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    modes = parser.add_subparsers(dest="mode", required=True)
    capture = modes.add_parser("render")
    capture.add_argument("--assets", type=Path, required=True)
    capture.add_argument("--out", type=Path, required=True)
    capture.add_argument("--exporter", type=Path, default=ROOT / "target/release/offline-presentation")
    capture.add_argument("--catalog-binary", type=Path, default=ROOT / "target/release/video-catalog")
    capture.add_argument("--max-frames", type=int, default=100_000)
    capture.set_defaults(run=render)
    join = modes.add_parser("assemble")
    join.add_argument("--batch", type=Path, action="append")
    join.add_argument("--base-manifest", type=Path)
    join.add_argument("--replacement-batch", type=Path, action="append")
    join.add_argument("--out", type=Path, required=True)
    join.add_argument("--dialogue-source-order", action="store_true",
                      help="stable-sort verified dialogue by profile and source procedure offset")
    join.set_defaults(run=assemble)
    args = parser.parse_args()
    args.run(args)


if __name__ == "__main__":
    main()
