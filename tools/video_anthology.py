#!/usr/bin/env python3
# /// script
# requires-python = ">=3.11"
# dependencies = ["av==18.1.0"]
# ///
"""Catalog native media, record scripted routes, and assemble chaptered masters.

This records the port, not the DOS oracle. A route is not dialogue-tree coverage.
Requires built game/video-catalog binaries, ffmpeg/ffprobe, and (by default) Xvfb.
"""

import argparse
from collections import Counter, defaultdict
from contextlib import contextmanager
from datetime import datetime, timezone
from fractions import Fraction
import hashlib
import json
import os
from pathlib import Path
import re
import select
import signal
import subprocess
import sys
import tempfile

ROOT = Path(__file__).resolve().parents[1]
GAMES = {"cb": "commander-blood", "bbb": "big-bug-bang"}


def digest(path):
    with Path(path).open("rb") as source:
        return hashlib.file_digest(source, "sha256").hexdigest()


def save_json(path, value):
    path = Path(path)
    temporary = path.with_suffix(path.suffix + ".tmp")
    temporary.write_text(json.dumps(value, indent=2, ensure_ascii=True) + "\n")
    temporary.replace(path)


def command(args, **kwargs):
    return subprocess.run([str(arg) for arg in args], check=True, **kwargs)


def resource_name(name):
    return name.replace("\\", "/").upper()


def asset_path(root, relative):
    path = (root / relative).resolve()
    if not path.is_relative_to(root.resolve()):
        raise ValueError(f"resource path escapes asset store: {relative}")
    return path


def inventory(root):
    manifest_path = root / "manifest.json"
    manifest = json.loads(manifest_path.read_text())
    if manifest.get("schema_version") != 1:
        raise ValueError("unsupported asset manifest schema")
    resources = manifest["resources"]
    # Verify actual source bytes, not merely a possibly stale import manifest.
    for item in resources + manifest.get("companions", []):
        path = asset_path(root, item["path"])
        if path.stat().st_size != item["byte_count"] or digest(path) != item["sha256"]:
            raise ValueError(f"asset integrity check failed: {path}")
    return manifest, digest(manifest_path)


def categorize(name, references):
    roles = {item["role"] for item in references}
    categories = set()
    if roles & {"talk", "idle", "left", "right"}:
        categories.add("conversation")
    if "arrival" in roles or name.startswith("PL/"):
        categories.add("planetary")
    if "object" in roles or name.startswith("OB/"):
        categories.add("object")
    if "background" in roles or name.startswith("FD/"):
        categories.add("environment")
    if "sequence" in roles or name.startswith("SQ/"):
        categories.add("sequence")
    if not categories:
        categories.add("unclassified")
    return sorted(categories)


def catalog(args):
    root = args.assets.resolve()
    manifest, manifest_hash = inventory(root)
    descript = next(item for item in manifest["resources"]
                    if resource_name(item["resource_name"]) == "DESCRIPT.DES")
    database = json.loads(command([args.catalog_binary.resolve(), asset_path(root, descript["path"])],
                                  capture_output=True, text=True).stdout)
    references = defaultdict(list)
    for record in database["records"]:
        for reference in record["references"]:
            references[reference["resource"]].append({
                "record": record["name"], "kind": record["kind"], "role": reference["role"],
                "captions": record["captions"],
            })
    videos = []
    for item in manifest["resources"]:
        name = resource_name(item["resource_name"])
        if not name.endswith(".HNM"):
            continue
        refs = references[name]
        videos.append(dict(item, name=name, references=refs, categories=categorize(name, refs),
                           coverage="not_recorded", reachability="not_assessed"))
    result = {"schema": 1, "game": manifest.get("game", "commander_blood"),
              "assets": str(root), "manifest_sha256": manifest_hash,
              "catalog_binary_sha256": digest(args.catalog_binary),
              "classification": "DESCRIPT references and directory hints; not reachability proof",
              "videos": videos, "records": database["records"],
              "category_counts": dict(Counter(c for v in videos for c in v["categories"]))}
    args.out.parent.mkdir(parents=True, exist_ok=True)
    save_json(args.out, result)
    print(f"{len(videos)} native videos: {args.out}", flush=True)


@contextmanager
def display_environment(visible):
    env = os.environ.copy()
    if visible:
        yield env
        return
    with tempfile.TemporaryFile() as log:
        server = subprocess.Popen(["Xvfb", "-displayfd", "1", "-screen", "0", "1280x960x24",
                                   "-nolisten", "tcp"], stdout=subprocess.PIPE, stderr=log)
        try:
            if not select.select([server.stdout], [], [], 10)[0]:
                raise RuntimeError("private X display startup timed out")
            number = server.stdout.readline().decode().strip()
            if not number.isdecimal():
                raise RuntimeError("Xvfb did not return a display number")
            env.update(DISPLAY=":" + number, SDL_VIDEODRIVER="x11", SDL_AUDIODRIVER="dummy",
                       WGPU_BACKEND="vulkan")
            env.pop("WAYLAND_DISPLAY", None)
            yield env
        finally:
            server.terminate()
            try:
                server.wait(timeout=5)
            except subprocess.TimeoutExpired:
                server.kill()
                server.wait()
            server.stdout.close()


def run_game(args, env, log, timeout):
    process = subprocess.Popen([str(arg) for arg in args], env=env, stdout=log,
                               stderr=subprocess.STDOUT, start_new_session=True)
    try:
        code = process.wait(timeout=timeout)
        if code:
            raise RuntimeError(f"game exited with status {code}; see game.log")
    finally:
        # Include an encoder child even if the game crashed before reaping it.
        try:
            os.killpg(process.pid, signal.SIGTERM)
        except ProcessLookupError:
            pass
        try:
            process.wait(timeout=5)
        except subprocess.TimeoutExpired:
            os.killpg(process.pid, signal.SIGKILL)
            process.wait()


def probe(path):
    return json.loads(command(["ffprobe", "-v", "error", "-show_streams", "-show_format",
                               "-show_chapters", "-show_data_hash", "sha256", "-of", "json", path],
                              capture_output=True, text=True).stdout)


def verify_movie(path):
    info = probe(path)
    kinds = [stream["codec_type"] for stream in info["streams"]]
    if kinds != ["video", "audio"] or float(info["format"]["duration"]) <= 0:
        raise ValueError(f"invalid master streams: {path}")
    command(["ffmpeg", "-nostdin", "-v", "error", "-xerror", "-i", path,
             "-map", "0:v:0", "-map", "0:a:0", "-f", "null", "-"], stdout=subprocess.DEVNULL)
    return info


def metadata_text(chapters):
    def escape(text):
        return str(text).replace("\\", "\\\\").replace("=", "\\=").replace(";", "\\;").replace("#", "\\#").replace("\n", " ").replace("\r", " ")
    lines = [";FFMETADATA1"]
    for chapter in chapters:
        lines += ["[CHAPTER]", "TIMEBASE=1/1000000000", f"START={chapter['start_ns']}",
                  f"END={chapter['end_ns']}", "title=" + escape(chapter["title"])]
    return "\n".join(lines) + "\n"


def transcript(scenes, duration):
    turns = []
    current = None
    for scene in scenes:
        state = scene["state"]
        text = (state.get("subtitle") or state.get("inline_dialogue") or "").replace("\r\n", "\n").replace("\r", "\n").strip()
        if current and text == current["text"]:
            continue
        if current:
            current["end_ns"] = min(scene["time_ns"], duration)
            if current["end_ns"] > current["start_ns"]:
                turns.append(current)
        current = dict(start_ns=scene["time_ns"], text=text, resource=state.get("resource"),
                       profile=state.get("profile")) if text else None
    if current and duration > current["start_ns"]:
        turns.append(dict(current, end_ns=duration))
    return turns


def srt_text(turns):
    def stamp(ns):
        ms = ns // 1_000_000
        return f"{ms // 3600000:02}:{ms // 60000 % 60:02}:{ms // 1000 % 60:02},{ms % 1000:03}"
    return "".join(f"{i}\n{stamp(t['start_ns'])} --> {stamp(t['end_ns'])}\n{t['text']}\n\n"
                   for i, t in enumerate(turns, 1))


def timestamp_audio(capture, rate):
    import av

    destination = capture / "audio-timestamped.mka"
    if destination.exists():
        raise ValueError(f"timestamped audio already exists: {destination}")
    count = 0
    previous_time = -1
    expected_sample = None
    with av.open(str(destination), "w", format="matroska") as output, (capture / "audio.f32le").open("rb") as raw, (capture / "timeline.jsonl").open() as timeline:
        stream = output.add_stream("pcm_f32le", rate=rate)
        stream.layout = "mono"
        stream.time_base = Fraction(1, rate)
        for line in timeline:
            event = json.loads(line)
            if event["kind"] != "audio" or not event["samples"]:
                continue
            if event["time_ns"] < previous_time or (expected_sample is not None and event["sample"] != expected_sample):
                raise ValueError("nonmonotonic or discontinuous audio timeline")
            previous_time = event["time_ns"]
            expected_sample = event["sample"] + event["samples"]
            raw.seek(event["sample"] * 4)
            encoded = raw.read(event["samples"] * 4)
            if len(encoded) != event["samples"] * 4:
                raise ValueError("audio timeline extends beyond sample data")
            frame = av.AudioFrame(format="flt", layout="mono", samples=event["samples"])
            frame.planes[0].update(encoded)
            frame.sample_rate = rate
            frame.time_base = Fraction(1, rate)
            frame.pts = round(event["time_ns"] * rate / 1_000_000_000)
            output.mux(stream.encode(frame))
            count += 1
        output.mux(stream.encode(None))
    if not count:
        raise ValueError("audio timeline has no packets")
    return destination


def verify_route_complete(attempt):
    last_boundary = None
    with (attempt / "runtime-trace.jsonl").open() as trace:
        for line in trace:
            last_boundary = json.loads(line)
    actions = [line for line in (attempt / "scenario.tsv").read_text().splitlines()
               if line.strip() and not line.lstrip().startswith("#")]
    if not last_boundary or last_boundary.get("action_index") != len(actions):
        raise ValueError("game exited before completing the scripted route")


def finalize(attempt, job, mp4=False):
    capture = attempt / "capture"
    verify_route_complete(attempt)
    recording = json.loads((capture / "recording.json").read_text())
    if not recording["audio_rate"] or not recording["audio_samples"]:
        raise ValueError("capture has no mixed audio")
    if (capture / "audio.f32le").stat().st_size != recording["audio_samples"] * 4:
        raise ValueError("truncated audio capture")
    duration = round(recording["frames"] * 1_000_000_000 / recording["fps"])
    scenes = [json.loads(line) for line in (capture / "scenes.jsonl").read_text().splitlines()]
    observed = {resource_name(s["state"].get("resource") or "") for s in scenes}
    missing = set(map(resource_name, job.get("expected_resources", []))) - observed
    if missing:
        raise ValueError(f"route missed required scenes: {sorted(missing)}")
    if "expected_final_profile" in job and (not scenes or scenes[-1]["state"]["profile"] != job["expected_final_profile"]):
        raise ValueError("route ended in an unexpected profile")
    turns = transcript(scenes, duration)
    for expected in job.get("expected_subtitles", []):
        if not any(expected in turn["text"] for turn in turns):
            raise ValueError(f"route missed required subtitle: {expected}")
    save_json(attempt / "transcript.json", turns)
    (attempt / "transcript.srt").write_text(srt_text(turns))
    chapters = [dict(start_ns=0, end_ns=duration, title=job["title"])]
    (attempt / "chapters.ffmeta").write_text(metadata_text(chapters))
    timestamped = timestamp_audio(capture, recording["audio_rate"])
    command(["ffmpeg", "-nostdin", "-v", "error", "-n", "-copyts", "-i", capture / "video.mkv",
             "-i", timestamped,
             "-f", "ffmetadata", "-i", attempt / "chapters.ffmeta", "-map", "0:v:0", "-map", "1:a:0",
             "-map_metadata", "2", "-map_chapters", "2", "-c:v", "copy", "-c:a", "flac",
             "-af", f"aresample={recording['audio_rate']}:async=1000:first_pts=0,apad",
             "-t", f"{duration / 1e9:.9f}", attempt / "master.mkv"])
    info = verify_movie(attempt / "master.mkv")
    if abs(float(info["format"]["duration"]) - duration / 1e9) > 0.1:
        raise ValueError("master duration differs from captured video")
    if mp4:
        command(["ffmpeg", "-nostdin", "-v", "error", "-n", "-i", attempt / "master.mkv",
                 "-map", "0:v:0", "-map", "0:a:0", "-map_chapters", "0",
                 "-c:v", "libx264", "-crf", "18", "-preset", "fast", "-pix_fmt", "yuv420p",
                 "-c:a", "aac", "-b:a", "192k", "-movflags", "+faststart", attempt / "viewing.mp4"])
    return dict(duration_ns=duration, observed_resources=sorted(observed - {""}),
                subtitle_count=len(turns), media=info,
                audio_alignment="callback timestamps, FFmpeg async resampling; original float samples retained",
                uncorrected_audio_duration_difference_ns=duration - round(recording["audio_samples"] * 1e9 / recording["audio_rate"]))


def check_artifacts(attempt, state):
    artifacts = state.get("artifacts", {})
    if not artifacts or "master.mkv" not in artifacts:
        return False
    return all(asset_path(attempt, name).is_file() and digest(asset_path(attempt, name)) == sha
               for name, sha in artifacts.items())


def load_jobs(path):
    manifest = json.loads(path.read_text())
    if manifest.get("schema") != 1:
        raise ValueError("unsupported jobs schema")
    jobs = manifest["jobs"]
    seen = set()
    for job in jobs:
        key = job["id"]
        if not re.fullmatch(r"[a-z0-9][a-z0-9_-]*", key) or key in seen:
            raise ValueError(f"invalid or duplicate job id: {key}")
        seen.add(key)
        if job["game"] not in GAMES or not job["title"] or not job["category"]:
            raise ValueError(f"invalid job: {key}")
    return jobs


def record(args):
    import av

    jobs = load_jobs(args.jobs)
    assets = {game: path.resolve() for game, path in (("cb", args.cb_assets), ("bbb", args.bbb_assets)) if path}
    selected = [job for job in jobs if not args.job or job["id"] in args.job]
    if not selected or (args.job and set(args.job) - {job["id"] for job in selected}):
        raise ValueError("no matching job or unknown --job")
    inputs = {}
    for game in {job["game"] for job in selected}:
        if game not in assets:
            raise ValueError(f"--{game}-assets is required")
        manifest, sha = inventory(assets[game])
        actual_game = manifest.get("game", "commander_blood")
        if actual_game != {"cb": "commander_blood", "bbb": "big_bug_bang"}[game]:
            raise ValueError(f"wrong asset store for {game}: {actual_game}")
        inputs[game] = sha
    args.out.mkdir(parents=True, exist_ok=True)
    for job in selected:
        binary = (args.bin_dir / GAMES[job["game"]]).resolve()
        scenario = (args.jobs.resolve().parent / job["scenario"]).resolve()
        provenance = dict(job=job, scenario_sha256=digest(scenario), binary_sha256=digest(binary),
                          assets_manifest_sha256=inputs[job["game"]], runner_sha256=digest(__file__),
                          assets=str(assets[job["game"]]), binary=str(binary), visible=args.visible,
                          mp4=args.mp4, pyav=av.__version__,
                          ffmpeg=command(["ffmpeg", "-version"], capture_output=True, text=True).stdout.splitlines()[0])
        fingerprint = hashlib.sha256(json.dumps(provenance, sort_keys=True).encode()).hexdigest()
        job_dir = args.out / job["id"]
        job_dir.mkdir(exist_ok=True)
        resumed = False
        for status in sorted(job_dir.glob("*/status.json"), reverse=True):
            state = json.loads(status.read_text())
            if state.get("status") == "complete" and state.get("fingerprint") == fingerprint and check_artifacts(status.parent, state):
                verify_movie(status.parent / "master.mkv")
                print(f"verified cached {job['id']}: {status.parent}", flush=True)
                resumed = True
                break
        if resumed:
            continue
        attempt = Path(tempfile.mkdtemp(prefix="attempt-", dir=job_dir)).resolve()
        state = dict(schema=1, status="recording", fingerprint=fingerprint, provenance=provenance,
                     started=datetime.now(timezone.utc).isoformat())
        save_json(attempt / "status.json", state)
        (attempt / "scenario.tsv").write_bytes(scenario.read_bytes())
        print(f"recording {job['id']}: {attempt}", flush=True)
        try:
            with display_environment(args.visible) as env, (attempt / "game.log").open("w") as log:
                run_game([binary, "--data", assets[job["game"]], "--write-data", attempt / "writable",
                          "--record", attempt / "capture", "--scenario", attempt / "scenario.tsv",
                          "--trace", attempt / "runtime-trace.jsonl", "--oracle-packed-second", job.get("packed_second", 39)],
                         env, log, job.get("timeout_seconds", 900))
            state["verification"] = finalize(attempt, job, args.mp4)
            files = [attempt / name for name in ("master.mkv", "transcript.json", "transcript.srt", "chapters.ffmeta", "scenario.tsv", "runtime-trace.jsonl")]
            files += list((attempt / "capture").iterdir())
            if args.mp4:
                files.append(attempt / "viewing.mp4")
            state["artifacts"] = {str(path.relative_to(attempt)): digest(path) for path in files if path.is_file()}
            state["status"] = "complete"
            print(f"completed {job['id']}: {attempt / 'master.mkv'}", flush=True)
        except BaseException as error:
            state.update(status="failed", error=str(error))
            raise
        finally:
            save_json(attempt / "status.json", state)


def assemble(args):
    sources = []
    signature = None
    game = None
    chapters = []
    offset = 0
    for attempt in args.attempt:
        attempt = attempt.resolve()
        state = json.loads((attempt / "status.json").read_text())
        if state["status"] != "complete" or not check_artifacts(attempt, state):
            raise ValueError(f"incomplete or damaged capture: {attempt}")
        job = state["provenance"]["job"]
        if game is not None and game != job["game"]:
            raise ValueError("assemble CB and BBB separately")
        game = job["game"]
        info = verify_movie(attempt / "master.mkv")
        current = [dict({key: s.get(key) for key in ("codec_name", "codec_type", "width", "height", "pix_fmt", "r_frame_rate", "time_base", "sample_rate", "channels", "bits_per_raw_sample")},
                        video_extradata_hash=s.get("extradata_hash") if s["codec_type"] == "video" else None)
                   for s in info["streams"]]
        if signature is not None and signature != current:
            raise ValueError("master stream formats differ; cannot losslessly concatenate")
        signature = current
        duration = state["verification"]["duration_ns"]
        chapters.append(dict(start_ns=offset, end_ns=offset + duration, title=job["title"]))
        offset += duration
        sources.append(dict(path=str(attempt / "master.mkv"), sha256=state["artifacts"]["master.mkv"], duration_ns=duration))
    if args.out.exists():
        raise ValueError(f"output already exists: {args.out}")
    args.out.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="anthology-") as temporary:
        temporary = Path(temporary)
        listing = temporary / "concat.txt"
        lines = []
        for source in sources:
            name = source["path"]
            if "\n" in name or "\r" in name:
                raise ValueError("newline in input path")
            escaped = name.replace("'", "'\\''")
            lines += [f"file '{escaped}'", f"duration {source['duration_ns'] / 1e9:.9f}"]
        listing.write_text("\n".join(lines) + "\n")
        metadata = temporary / "chapters.ffmeta"
        metadata.write_text(metadata_text(chapters))
        command(["ffmpeg", "-nostdin", "-v", "error", "-n", "-f", "concat", "-safe", "0", "-i", listing,
                 "-f", "ffmetadata", "-i", metadata, "-map", "0:v:0", "-map", "0:a:0", "-map_metadata", "1",
                 "-map_chapters", "1", "-c:v", "copy", "-c:a", "flac", args.out])
    info = verify_movie(args.out)
    if len(info["chapters"]) != len(chapters):
        raise ValueError("assembled chapter count does not match source clips")
    save_json(args.out.with_suffix(".manifest.json"), dict(schema=1, game=game, sources=sources,
                                                         chapters=chapters, sha256=digest(args.out)))
    print(f"assembled {len(sources)} chapters: {args.out}")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    subs = parser.add_subparsers(dest="command", required=True)
    cat = subs.add_parser("catalog")
    cat.add_argument("--assets", type=Path, required=True)
    cat.add_argument("--catalog-binary", type=Path, default=ROOT / "target/release/video-catalog")
    cat.add_argument("--out", type=Path, required=True)
    cat.set_defaults(function=catalog)
    run = subs.add_parser("record")
    run.add_argument("--jobs", type=Path, required=True)
    run.add_argument("--job", action="append")
    run.add_argument("--bin-dir", type=Path, default=ROOT / "target/release")
    run.add_argument("--cb-assets", type=Path)
    run.add_argument("--bbb-assets", type=Path)
    run.add_argument("--out", type=Path, required=True)
    run.add_argument("--visible", action="store_true", help="use the current desktop and audio device")
    run.add_argument("--mp4", action="store_true", help="also encode an H.264/AAC viewing copy")
    run.set_defaults(function=record)
    join = subs.add_parser("assemble")
    join.add_argument("--attempt", type=Path, action="append", required=True, help="completed attempt directory, in chapter order")
    join.add_argument("--out", type=Path, required=True)
    join.set_defaults(function=assemble)
    args = parser.parse_args()
    args.function(args)


if __name__ == "__main__":
    try:
        main()
    except (OSError, ValueError, RuntimeError, subprocess.SubprocessError) as error:
        sys.exit(str(error))
