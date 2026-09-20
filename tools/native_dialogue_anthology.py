#!/usr/bin/env python3
"""Render source-bound dialogue branches using the native offline lifecycle.

No pointer actions, synthesized subtitle cards, or added text holds. This is a
selected-branch anthology, not a proof of all-game dialogue coverage.
"""

import argparse
import json
from pathlib import Path
import subprocess

from native_sequence_anthology import assemble, read_json, require, timeline, verify_media
from video_anthology import ROOT, command, digest, inventory, save_json


def validate_trace(plan, runner, states, rows):
    require(runner["chapter"] == plan, "captured dialogue plan differs")
    ends = {row["start_ns"] + row["duration_ns"] for row in rows}
    publications = runner["publications"]
    require(all(event["frame_end_ns"] in ends for event in publications),
            "publication outside native frame boundaries")
    require([event["frame_end_ns"] for event in publications] ==
            sorted(event["frame_end_ns"] for event in publications), "unordered publications")
    published = {event["publication"]["offset"] for event in publications
                 if event["publication"]["profile"] == plan["initial_profile"]}
    require(published == set(runner["published_cod_sites"]), "publication accounting differs")
    require(set(plan["required_cod_sites"]) <= published, "missing required publication")
    require([{key: item[key] for key in ("text_site", "word_offset")}
             for item in runner["choices"]] == plan["choices"], "different semantic choices")
    require(all(choice["requested_at_ns"] in ends for choice in runner["choices"]),
            "choice outside native frame boundaries")
    if plan["end"]["kind"] == "profile_loaded":
        require(runner["final_profile"] == plan["end"]["profile"], "wrong final profile")
    boundary = set()
    evidence = {}
    last_time = -1
    last = None
    for row in states:
        time = row["time_ns"]
        require(last_time < time < rows[-1]["start_ns"] + rows[-1]["duration_ns"],
                "unordered or out-of-range native state")
        last_time = time
        state = row["state"]
        last = state
        require(all(state["input"][field] == 0 for field in
                    ("buttons", "previous_buttons", "press_pending", "primary_pressed")),
                "dialogue capture contains pointer input")
        if state["vm"]["resource_profile"] != plan["initial_profile"]:
            continue
        site = state["published_cod_text_site"]
        if site is None:
            continue
        boundary.add(site)
        entry = evidence.setdefault(site, dict(ui_raster_frames=0, fully_revealed_ui_frames=0,
                                               max_matching_glyph_pixels=0, first_full_ui_ns=None))
        presentation = state["presentation"]
        subtitle = bool(presentation["text_display_active"])
        raster = state["subtitle_raster"] if subtitle else state["inline_menu_raster"]
        if not raster or not raster["expected_pixel_count"]:
            continue
        require(raster["matching_pixel_count"] == raster["expected_pixel_count"],
                f"native UI glyph raster mismatch at COD {site:#x}")
        entry["ui_raster_frames"] += 1
        entry["max_matching_glyph_pixels"] = max(entry["max_matching_glyph_pixels"],
                                                   raster["matching_pixel_count"])
        if subtitle:
            cursor = presentation["text_state"]["subtitle_reveal_cursor"]
            full = cursor is not None and cursor >= len(state["subtitle_bytes"])
        else:
            menu = presentation["inline_menu"]
            full = bool(menu["display_words"]) and menu["reveal_count"] >= len(menu["display_words"])
        if full:
            entry["fully_revealed_ui_frames"] += 1
            if entry["first_full_ui_ns"] is None:
                entry["first_full_ui_ns"] = time
    require(last is not None, "empty native state trace")
    require(set(plan["required_frame_boundary_cod_sites"]) <= boundary,
            "missing required frame-boundary text site")
    if plan["end"]["kind"] == "presentation_finished":
        require(not last["presentation"]["active"], "presentation did not finish")
    return dict(published_cod_sites=sorted(published), state_trace_cod_sites=sorted(boundary),
                ui_raster_evidence={str(key): value for key, value in sorted(evidence.items())},
                published_without_ui_raster=sorted(site for site in published
                    if not evidence.get(site, {}).get("ui_raster_frames")),
                published_without_full_ui_reveal=sorted(site for site in published
                    if not evidence.get(site, {}).get("fully_revealed_ui_frames")),
                raster_evidence_scope="native UI buffer before frame presentation; not by itself proof of encoded glyph visibility")


def verify_chapter(path, plan, manifest, exporter_hash):
    report = read_json(path / "report.json")
    require(report["target"] == "dialogue_chapter" and report["complete_native_presentation"]
            and not report["complete_game"], "not a completed native dialogue chapter")
    require(report["exporter_sha256"] == exporter_hash, "exporter changed")
    source = read_json(path / "source-manifest.json")
    for key in ("resources", "companions"):
        require(source[key] == manifest[key], "source asset manifest changed")
    require(report["game"] == plan["game"] == manifest.get("game", "commander_blood"), "wrong game")
    rows, duration, samples = timeline(path / "timeline.jsonl")
    require((len(rows), duration, samples) == tuple(report["runner"][key] for key in
            ("presented_frames", "duration_ns", "audio_samples")), "runner accounting mismatch")
    require(digest(path / "endpoint.rgba") == report["endpoint_rgba_sha256"], "damaged endpoint")
    with (path / "native-state.jsonl").open() as stream:
        evidence = validate_trace(plan, report["runner"], (json.loads(line) for line in stream), rows)
    hashes, _ = verify_media(path / "master.mkv", rows, duration)
    require(hashes["rgba_sha256"] == report["rgba_sha256"] and
            hashes["audio_sha256"] == report["audio_f32le_sha256"], "decoded report hash mismatch")
    require(digest(path / "audio.f32le") == hashes["audio_sha256"], "damaged raw audio")
    return dict(record=plan["title"], path=str(path), duration_ns=duration, frames=len(rows),
                audio_samples=samples, master_sha256=digest(path / "master.mkv"),
                report_sha256=digest(path / "report.json"),
                native_state_sha256=digest(path / "native-state.jsonl"), **hashes, **evidence)


def render(args):
    assets = args.assets.resolve()
    manifest, manifest_hash = inventory(assets)
    plans = [(path.resolve(), read_json(path)) for path in args.plan]
    require(plans, "no dialogue plans")
    names = [path.stem for path, _ in plans]
    require(len(set(names)) == len(names), "duplicate plan names")
    require(len({plan["title"] for _, plan in plans}) == len(plans), "duplicate chapter titles")
    args.out = args.out.resolve()
    args.out.mkdir(parents=True, exist_ok=True)
    provenance = dict(schema=1, game=manifest.get("game", "commander_blood"),
                      asset_manifest_sha256=manifest_hash, exporter_sha256=digest(args.exporter),
                      records=[dict(name=plan["title"], plan=plan, plan_sha256=digest(path))
                               for path, plan in plans])
    selection = args.out / "selection.json"
    if selection.exists():
        require(read_json(selection) == provenance, "batch inputs changed; use a new output directory")
    else:
        require(not list(args.out.iterdir()), "unrecognized nonempty output directory")
        save_json(selection, provenance)
    completed, failures = [], []
    for index, (plan_path, plan) in enumerate(plans):
        path = args.out / plan_path.stem
        print(f"[{index + 1}/{len(plans)}] {plan['title']}", flush=True)
        try:
            if not path.exists():
                with (args.out / (plan_path.stem + ".log")).open("w") as log:
                    command([args.exporter.resolve(), assets, "dialogue:" + str(plan_path), path,
                             args.max_frames], stdout=log, stderr=subprocess.STDOUT)
            completed.append(verify_chapter(path, plan, manifest, provenance["exporter_sha256"]))
            print(f"  verified {completed[-1]['duration_ns'] / 1e9:.3f}s", flush=True)
        except (ValueError, OSError, KeyError, subprocess.CalledProcessError) as error:
            failures.append(dict(record=plan["title"], error=str(error)))
            print(f"  failed: {error}", flush=True)
        save_json(args.out / "coverage.json", dict(schema=1, provenance=provenance,
                  scope="selected native dialogue branches; not all-game or gameplay reachability; preempted lines are not extended",
                  complete=len(completed) == len(plans), complete_game=False,
                  completed=completed, failures=failures))
    require(not failures, f"{len(failures)} dialogue exports failed; see coverage.json and logs")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    modes = parser.add_subparsers(dest="mode", required=True)
    capture = modes.add_parser("render")
    capture.add_argument("--assets", type=Path, required=True)
    capture.add_argument("--out", type=Path, required=True)
    capture.add_argument("--plan", type=Path, action="append", required=True)
    capture.add_argument("--exporter", type=Path, default=ROOT / "target/release/offline-presentation")
    capture.add_argument("--max-frames", type=int, default=100_000)
    capture.set_defaults(run=render)
    join = modes.add_parser("assemble")
    join.add_argument("--batch", type=Path, required=True)
    join.add_argument("--out", type=Path, required=True)
    join.set_defaults(run=assemble)
    args = parser.parse_args()
    args.run(args)


if __name__ == "__main__":
    main()
