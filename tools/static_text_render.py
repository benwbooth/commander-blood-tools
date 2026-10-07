#!/usr/bin/env python3
# /// script
# requires-python = ">=3.11"
# dependencies = []
# ///
"""Render and independently verify isolated, source-bound native text scenes.

Prepared-state evidence is kept separate from native gameplay flow coverage.
Every selected failure is retained, and interrupted batches can be resumed with
identical inputs. A successful batch is not complete-game coverage.
"""

import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess

from native_capture_storage import open_trace, trace_digest
from native_sequence_anthology import read_json, require, timeline, verify_media
from video_anthology import ROOT, asset_path, digest, inventory, resource_name, run_game, save_json


def validate_report(plan, report, exporter_hash):
    require(report["target"] == "static_text_site" and report["evidence_scope"] == "prepared_source_site",
            "not an isolated prepared text capture")
    require(report["complete_native_presentation"] is False and report["complete_game"] is False,
            "prepared scene incorrectly claims natural presentation or game completion")
    require(report["completion"] == "isolated_text_hold_completed"
            and report["decoded_master_verified"] and report["video_timestamps_verified"],
            "prepared capture has not completed native hold and media checks")
    require(report["game"] == plan["game"] and report["exporter_sha256"] == exporter_hash,
            "prepared capture game/exporter changed")
    runner = report["runner"]
    require(runner["plan"] == plan and runner["story_execution"] == "frozen"
            and runner["natural_chapter_close"] is False
            and runner["gameplay_reachability"] == "not_assessed", "prepared context differs")
    publication = runner["publication"]
    require(publication["profile"] == plan["initial_profile"] and publication["offset"] == plan["text_site"]
            and publication.get("bas", False) == (plan["source"] == "bas"), "wrong source publication")
    require(runner["authored"]["source"] == plan["source"]
            and runner["authored"]["text_site"] == plan["text_site"], "wrong authored instruction")


def expected_menu_words(authored):
    # A localized display (BBB English) is laid out by whitespace-separated words;
    # otherwise each slot is one original dictionary word.
    # History (b4&0x40) and reply (b4&0x10) lines display only section 0; the later
    # sections are the trigger list and the reply rows.
    count = 1 if authored["flags_b4"] & 0x50 else None
    if "display" in authored:
        return " ".join(authored["display"]["sections"][:count]).split()
    return [word["text"] for section in authored["sections"][:count] for word in section]


def expected_reply_words(authored):
    """Reply rows of a resume-armed (b4&0x10) line: the section after the text and
    any history candidates; BBB shows its English display labels."""
    if not authored["flags_b4"] & 0x10:
        return None
    index = 2 if authored["flags_b4"] & 0x40 else 1
    if "display" in authored:
        return authored["display"]["sections"][index].split()
    return [word["text"] for word in authored["sections"][index]]


def validate_states(plan, states, rows, expected_text, expected_words, expected_replies=None):
    frame_times = {row["start_ns"] for row in rows}
    duration = rows[-1]["start_ns"] + rows[-1]["duration_ns"]
    expected_text = " ".join(expected_text.split())
    previous = -1
    first_full = None
    count = 0
    held = False
    glyphs = 0
    replies_open = False
    for entry in states:
        time = entry["time_ns"]
        require(previous <= time <= duration, "nonmonotonic/out-of-range prepared trace")
        previous = time
        state = entry["state"]
        require(state["vm"]["resource_profile"] == plan["initial_profile"], "profile changed in prepared trace")
        require(all(state["input"][key] == 0 for key in
                    ("buttons", "previous_buttons", "press_pending", "primary_pressed")),
                "prepared scene unexpectedly contains pointer input")
        source_key = "published_bas_text_site" if plan["source"] == "bas" else "published_cod_text_site"
        other_key = "published_cod_text_site" if plan["source"] == "bas" else "published_bas_text_site"
        require(state.get(source_key) == plan["text_site"] and state.get(other_key) is None,
                "prepared trace published another source site")
        presentation = state["presentation"]
        subtitle = bool(presentation["text_display_active"])
        if subtitle:
            shown = bytes(state["subtitle_bytes"]).decode("utf-8", errors="replace")
            cursor = presentation["text_state"]["subtitle_reveal_cursor"]
            full = cursor is not None and cursor >= len(state["subtitle_bytes"])
            require(" ".join(shown.split()) == expected_text, "displayed subtitle differs from static inventory")
        else:
            # Inline menus lay out one dictionary word per slot, so compare the authored word sequence.
            menu = presentation["inline_menu"]
            full = bool(menu["display_words"]) and menu["reveal_count"] >= len(menu["display_words"])
            require(list(menu["display_words"]) == expected_words,
                    "displayed menu words differ from static inventory")
        raster = state["subtitle_raster" if subtitle else "inline_menu_raster"]
        if raster and raster["expected_pixel_count"]:
            require(raster["matching_pixel_count"] == raster["expected_pixel_count"],
                    "prepared native glyph raster mismatch")
            if full and time in frame_times:
                first_full = time if first_full is None else first_full
                count += 1
                glyphs = max(glyphs, raster["matching_pixel_count"])
        held |= presentation["text_state"]["hold_ready"]
        if expected_replies is not None and presentation["rendered_word_choices"]:
            require(presentation["rendered_word_choices"] == expected_replies,
                    "reply rows differ from the authored reply section")
            replies_open |= (held and time in frame_times
                             and presentation["retained_word_choice"]["phase"] == "Selecting")
    require(first_full is not None and count and glyphs, "no fully revealed native UI frame before encoded endpoint")
    require(held, "native text hold did not complete")
    require(expected_replies is None or replies_open, "reply rows never opened before the encoded endpoint")
    return dict(first_full_ui_ns=first_full, fully_revealed_ui_frames=count,
                max_matching_glyph_pixels=glyphs, native_hold_completed=held,
                reply_rows=expected_replies,
                scope="native UI raster with encoded intervals; not per-glyph encoded-pixel comparison")


def verify_capture(path, plan, site, manifest, assets, exporter_hash):
    report = read_json(path / "report.json")
    validate_report(plan, report, exporter_hash)
    source = read_json(path / "source-manifest.json")
    require(source == dict(manifest, game=manifest.get("game", "commander_blood")),
            "prepared capture uses another asset manifest")
    runner = report["runner"]
    authored = runner["authored"]
    resource = f"SCRIPT{plan['initial_profile'] + 1}.{plan['source'].upper()}"
    matches = [row for row in manifest["resources"] if resource_name(row["resource_name"]) == resource]
    require(len(matches) == 1, "no unique original instruction resource")
    code = asset_path(assets, matches[0]["path"]).read_bytes()
    require(hashlib.sha256(code).hexdigest() == plan[plan["source"] + "_sha256"],
            "source instruction resource differs from plan")
    instruction = bytes(authored["instruction_bytes"])
    require(authored["end_offset"] == plan["text_site"] + len(instruction)
            and code[plan["text_site"]:authored["end_offset"]] == instruction
            and hashlib.sha256(instruction).hexdigest() == authored["instruction_sha256"],
            "captured instruction bytes differ from original resource")
    original = site["authored"]
    require(authored["line_record_offset"] == original["record_offset"]
            and authored["control_bits"] == (original["flags_b4"] | original["flags_b5"] << 8),
            "captured instruction operands differ from inventory")
    rows, duration, samples = timeline(path / "timeline.jsonl")
    require((len(rows), duration, samples) == tuple(runner[key] for key in
            ("presented_frames", "duration_ns", "audio_samples")), "prepared timeline differs from runner")
    require(digest(path / "endpoint.rgba") == report["endpoint_rgba_sha256"], "prepared endpoint changed")
    text = original.get("display", original)["text"]
    with open_trace(path / "native-state.jsonl") as stream:
        evidence = validate_states(plan, (json.loads(line) for line in stream), rows, text,
                                   expected_menu_words(original), expected_reply_words(original))
    hashes, _ = verify_media(path / "master.mkv", rows, duration)
    require(hashes["rgba_sha256"] == report["rgba_sha256"]
            and hashes["audio_sha256"] == report["audio_f32le_sha256"], "prepared media report differs")
    return dict(id=site["id"], record=plan["title"], path=str(path.resolve()), status="verified_prepared_scene",
                gameplay_reachability="not_assessed", duration_ns=duration, frames=len(rows), audio_samples=samples,
                master_sha256=digest(path / "master.mkv"), report_sha256=digest(path / "report.json"),
                native_state_sha256=trace_digest(path / "native-state.jsonl"),
                ui_raster_evidence=evidence, **hashes)


def render(args):
    require(args.max_frames > 0 and args.timeout > 0, "frame and timeout limits must be positive")
    planning = read_json(args.plan_set)
    require(planning["schema"] == 1 and planning["mode"] == "static_text", "not a static text plan set")
    inventory_path = Path(planning["inventory"])
    require(digest(inventory_path) == planning["inventory_sha256"], "static inventory changed")
    content = read_json(inventory_path)
    sites = {site["id"]: site for site in content["sites"]}
    assets = args.assets.resolve()
    manifest, manifest_hash = inventory(assets)
    game = manifest.get("game", "commander_blood")
    candidates = []
    for row in planning["sites"]:
        if row["status"] != "planned":
            continue
        path = asset_path(args.plan_set.parent, row["plan"])
        require(digest(path) == planning["plan_sha256"][row["plan"]], "source plan changed")
        plan = read_json(path)
        site = sites.get(row["id"])
        require(site is not None and row["id"] ==
                f"{site['game']}.script{plan['initial_profile'] + 1}.{plan['source']}.{plan['text_site']:08x}",
                "plan source identity differs from inventory")
        require(all(plan[key] == site["source_hashes"][key] for key in ("cod_sha256", "dic_sha256"))
                and plan["context"]["actor_offset"] == site["authored"]["record_offset"],
                "plan source bindings differ from inventory")
        if plan["game"] == game:
            candidates.append((row["id"], path, plan))
    requested = set(args.site or [])
    require(requested <= {identity for identity, _, _ in candidates}, "requested site is not an eligible plan for this game")
    selected = [entry for entry in candidates if not requested or entry[0] in requested]
    if args.limit is not None:
        require(args.limit > 0, "limit must be positive")
        selected = selected[:args.limit]
    require(selected, "no static text plans selected")
    exporter_hash = digest(args.exporter)
    provenance = dict(schema=1, mode="static_text", game=game, asset_manifest_sha256=manifest_hash,
                      plan_set_sha256=digest(args.plan_set), inventory_sha256=planning["inventory_sha256"],
                      exporter_sha256=exporter_hash, verifier_sha256=digest(__file__),
                      eligible_sites=len(candidates), selected_sites=len(selected),
                      records=[dict(id=identity, plan=plan, plan_sha256=digest(path)) for identity, path, plan in selected])
    args.out.mkdir(parents=True, exist_ok=True)
    selection_path = args.out / "selection.json"
    if selection_path.exists():
        require(read_json(selection_path) == provenance, "batch inputs changed; use a new output directory")
    else:
        require(not list(args.out.iterdir()), "unrecognized nonempty prepared batch directory")
        save_json(selection_path, provenance)
    completed, failures = [], []
    for index, (identity, plan_path, plan) in enumerate(selected):
        path = args.out / identity
        print(f"[{index + 1}/{len(selected)}] {identity}", flush=True)
        try:
            require(digest(args.exporter) == exporter_hash, "exporter changed during batch")
            require(digest(plan_path) == provenance["records"][index]["plan_sha256"], "plan changed during batch")
            if not path.exists():
                with (args.out / (identity + ".log")).open("w") as log:
                    run_game([args.exporter.resolve(), assets, "static-text:" + str(plan_path), path.resolve(),
                              args.max_frames], os.environ.copy(), log, args.timeout)
            entry = verify_capture(path, plan, sites[identity], manifest, assets, exporter_hash)
            completed.append(entry)
            print(f"  verified prepared scene, {entry['duration_ns'] / 1e9:.3f}s", flush=True)
        except (ValueError, RuntimeError, OSError, KeyError, subprocess.SubprocessError) as error:
            failures.append(dict(id=identity, error=str(error), log=str(args.out / (identity + ".log"))))
            print(f"  failed: {error}", flush=True)
        save_json(args.out / "coverage.json", dict(schema=1, mode="static_text", provenance=provenance,
                  scope="selected isolated native text scenes; explicit prepared context, not normal gameplay",
                  complete_selection=len(completed) == len(selected), complete_game=False,
                  completed=completed, failures=failures))
    require(not failures, f"{len(failures)} prepared scenes failed; see coverage.json and logs")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--assets", type=Path, required=True)
    parser.add_argument("--plan-set", type=Path, required=True)
    parser.add_argument("--out", type=Path, required=True)
    parser.add_argument("--exporter", type=Path, default=ROOT / "target/release/offline-presentation")
    parser.add_argument("--site", action="append")
    parser.add_argument("--limit", type=int)
    parser.add_argument("--max-frames", type=int, default=10000)
    parser.add_argument("--timeout", type=int, default=900)
    render(parser.parse_args())


if __name__ == "__main__":
    main()
