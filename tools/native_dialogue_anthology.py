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
from video_anthology import ROOT, command, digest, inventory, resource_name, save_json


def validate_trace(plan, runner, states, rows):
    require(runner["chapter"] == plan, "captured dialogue plan differs")
    ends = {row["start_ns"] + row["duration_ns"] for row in rows}
    publications = runner["publications"]
    require(all(event["frame_end_ns"] in ends for event in publications),
            "publication outside native frame boundaries")
    require([event["frame_end_ns"] for event in publications] ==
            sorted(event["frame_end_ns"] for event in publications), "unordered publications")
    published = {event["publication"]["offset"] for event in publications
                 if event["publication"]["profile"] == plan["initial_profile"]
                 and not event["publication"].get("bas", False)}
    published_bas = {event["publication"]["offset"] for event in publications
                     if event["publication"]["profile"] == plan["initial_profile"]
                     and event["publication"].get("bas", False)}
    require(published == set(runner["published_cod_sites"]), "publication accounting differs")
    require(published_bas == set(runner.get("published_bas_sites", [])), "BAS publication accounting differs")
    require(set(plan["required_cod_sites"]) <= published, "missing required publication")
    require(set(plan.get("required_bas_sites", [])) <= published_bas, "missing required BAS publication")
    unpublished = set(plan.get("expected_unpublished_cod_sites", []))
    require(not published & unpublished, "published a site declared absent on this branch")
    selected = [{key: value for key, value in item.items() if key != "requested_at_ns"}
                for item in runner["choices"]]
    count = len(plan["choices"])
    retries = plan.get("max_exit_retries", 0)
    require(not retries or (count and plan["choices"][-1].get("source") == "bas_menu"),
            "exit retries require a final BAS menu choice")
    require(0 <= retries <= 8 and count <= len(selected) <= count + retries
            and selected[:count] == plan["choices"]
            and all(choice == plan["choices"][-1] for choice in selected[count:]),
            "different semantic choices or exceeded authored exit retry limit")
    require(all(choice["requested_at_ns"] in ends for choice in runner["choices"]),
            "choice outside native frame boundaries")
    frame_starts = {row["start_ns"] + row["duration_ns"]: row["start_ns"] for row in rows}
    inventory_requests = [dict(choice, offer_frame_ns=frame_starts[choice["requested_at_ns"]])
                          for choice in runner["choices"] if choice.get("source") == "inventory"]
    inventory_evidence = [dict(item=choice["inventory_item"], text_site=choice["text_site"],
                               offered_at_ns=None, transferred_at_ns=None)
                          for choice in inventory_requests]
    if plan["end"]["kind"] == "profile_loaded":
        require(runner["final_profile"] == plan["end"]["profile"], "wrong final profile")
    if plan["initial_profile"] != 0:
        require(runner["profile_selected_at_ns"] is not None and
                0 < runner["profile_selected_at_ns"] <= runner["bootstrap_duration_ns"],
                "missing native profile selection provenance")
    if plan.get("contact_procedure") is not None:
        preparation = runner.get("contact_preparation")
        require(preparation is not None and preparation["procedure_offset"] == plan["contact_procedure"],
                "missing prepared-contact provenance")
        if plan["game"] == "big_bug_bang":
            require(preparation.get("cod_sha256") == plan["cod_sha256"]
                    and preparation.get("guard_source") == "typed_cod_outer_guard",
                    "BBB contact preparation is not bound to the authored COD guard")
        else:
            require(preparation["manifest_sha256"] == digest(ROOT / "re/vm/contact-manifest/contact-manifest.json"),
                    "contact preparation manifest changed")
        encounter = preparation.get("encounter_guard")
        if plan.get("contact_encounter_guard") is not None:
            require(isinstance(encounter, dict)
                    and encounter.get("offset") == plan["contact_encounter_guard"]
                    and isinstance(encounter.get("at_presentation"), int)
                    and 0 < encounter["at_presentation"] <= 65535
                    and encounter.get("before_entry") == encounter["at_presentation"] - 1,
                    "missing source-bound encounter preparation")
        else:
            require(encounter is None, "unexpected encounter preparation")
    if plan.get("entry") == "travel":
        preparation = runner.get("travel_preparation")
        require(isinstance(preparation, dict) and preparation.get("setup") == plan["travel_setup"],
                "missing prepared-travel provenance")
    boundary = set()
    evidence = {}
    boundary_bas = set()
    evidence_bas = {}
    last_time = -1
    last = None
    sequences = []
    previous_resource = None
    staged_actor_checked = False
    staged_inventory_checked = False
    for row in states:
        time = row["time_ns"]
        require(last_time < time < rows[-1]["start_ns"] + rows[-1]["duration_ns"],
                "unordered or out-of-range native state")
        last_time = time
        state = row["state"]
        last = state
        active = state.get("video", {}).get("active_resource")
        active = resource_name(active) if active else None
        if active != previous_resource and active and active.startswith("SQ/"):
            sequences.append(dict(resource=active, state_time_ns=time))
        previous_resource = active
        require(all(state["input"][field] == 0 for field in
                    ("buttons", "previous_buttons", "press_pending", "primary_pressed")),
                "dialogue capture contains pointer input")
        if state["vm"]["resource_profile"] != plan["initial_profile"]:
            continue
        if plan.get("travel_setup", {}).get("stage_actor_at_destination") and not staged_actor_checked:
            actors = [row for row in state.get("persistent", {}).get("object_locations", [])
                      if row["name"] == plan["target"]]
            require(len(actors) == 1 and actors[0]["kind"] == "Actor"
                    and actors[0]["target_name"] == plan["travel_setup"]["destination"]
                    and actors[0].get("sequel_simulation_flags", 0) & 4,
                    "native trace does not show the staged travel actor at its destination")
            staged_actor_checked = True
        staged_inventory = plan.get("travel_setup", {}).get("stage_aboard_inventory", [])
        if staged_inventory and not staged_inventory_checked:
            for offset in staged_inventory:
                items = [item for item in state.get("persistent", {}).get("object_locations", [])
                         if item.get("source_offset") == offset]
                require(len(items) == 1 and items[0]["kind"] == "InventoryItem"
                        and items[0]["relation"] == "sentinel" and items[0]["holder_raw"] == 65535,
                        "native trace does not show staged inventory aboard")
            staged_inventory_checked = True
        for request, transfer in zip(inventory_requests, inventory_evidence):
            if time == request["offer_frame_ns"]:
                offered = state["presentation"].get("inventory_choice")
                require(offered and offered["text_site"] == request["text_site"]
                        and offered["recipient"] == plan["target"]
                        and request["inventory_item"] in offered["offered_items"]
                        and state["presentation"]["retained_word_choice"]["phase"] == "Selecting",
                        "inventory selection was not offered by the native chooser")
                transfer["offered_at_ns"] = time
            if time >= request["requested_at_ns"] and transfer["transferred_at_ns"] is None:
                items = [item for item in state.get("persistent", {}).get("object_locations", [])
                         if item.get("source_offset") == request["inventory_item"]]
                if (len(items) == 1 and items[0]["kind"] == "InventoryItem"
                        and items[0]["relation"] == "object" and items[0]["target_name"] == plan["target"]):
                    transfer["transferred_at_ns"] = time
        site = state["published_cod_text_site"]
        bas_site = state.get("published_bas_text_site")
        require(site is None or bas_site is None, "ambiguous COD/BAS publication source")
        is_bas = bas_site is not None
        if is_bas:
            site = bas_site
        if site is None:
            continue
        (boundary_bas if is_bas else boundary).add(site)
        entry = (evidence_bas if is_bas else evidence).setdefault(site, dict(ui_raster_frames=0, fully_revealed_ui_frames=0,
                                               max_matching_glyph_pixels=0, first_full_ui_ns=None))
        presentation = state["presentation"]
        subtitle = bool(presentation["text_display_active"])
        raster = state["subtitle_raster"] if subtitle else state["inline_menu_raster"]
        if not raster or not raster["expected_pixel_count"]:
            continue
        require(raster["matching_pixel_count"] == raster["expected_pixel_count"],
                f"native UI glyph raster mismatch at {'BAS' if is_bas else 'COD'} {site:#x}")
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
    require(not plan.get("travel_setup", {}).get("stage_actor_at_destination") or staged_actor_checked,
            "no native state for staged travel actor")
    require(not plan.get("travel_setup", {}).get("stage_aboard_inventory") or staged_inventory_checked,
            "no native state for staged inventory")
    require(all(item["offered_at_ns"] is not None and item["transferred_at_ns"] is not None
                for item in inventory_evidence), "missing native inventory offer or transfer")
    require(set(plan["required_frame_boundary_cod_sites"]) <= boundary,
            "missing required frame-boundary text site")
    require(set(plan.get("required_frame_boundary_bas_sites", [])) <= boundary_bas,
            "missing required frame-boundary BAS text site")
    if plan["end"]["kind"] == "presentation_finished":
        require(not last["presentation"]["active"], "presentation did not finish")
    if plan.get("entry", "radio") == "contact":
        require(runner["contact_transition_closed"] and
                last["contact_transition"]["phase"] == "Inactive" and
                not last["presentation"]["navigation_rebuild_pending"],
                "contact transition did not finish")
    if plan.get("entry") == "travel":
        require(runner["travel_transition_closed"] and last["presentation"]["ship_flags"] == 0
                and not last["presentation"]["text_state"]["sequence_active"]
                and not last["presentation"]["navigation_rebuild_pending"],
                "travel transition did not finish")
    return dict(published_cod_sites=sorted(published), state_trace_cod_sites=sorted(boundary),
                published_bas_sites=sorted(published_bas), state_trace_bas_sites=sorted(boundary_bas),
                bas_ui_raster_evidence={str(key): value for key, value in sorted(evidence_bas.items())},
                published_bas_without_ui_raster=sorted(site for site in published_bas
                    if not evidence_bas.get(site, {}).get("ui_raster_frames")),
                published_bas_without_full_ui_reveal=sorted(site for site in published_bas
                    if not evidence_bas.get(site, {}).get("fully_revealed_ui_frames")),
                expected_unpublished_cod_sites=sorted(unpublished),
                observed_sequence_resources=sequences,
                ui_raster_evidence={str(key): value for key, value in sorted(evidence.items())},
                published_without_ui_raster=sorted(site for site in published
                    if not evidence.get(site, {}).get("ui_raster_frames")),
                published_without_full_ui_reveal=sorted(site for site in published
                    if not evidence.get(site, {}).get("fully_revealed_ui_frames")),
                raster_evidence_scope="native UI buffer before frame presentation; not by itself proof of encoded glyph visibility",
                **(dict(inventory_transfers=inventory_evidence) if inventory_evidence else {}))


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


def chapter_plans(plan_paths, plan_set=None):
    paths = list(plan_paths or [])
    if plan_set is not None:
        planned = read_json(plan_set)
        require(planned["schema"] == 1 and isinstance(planned["plans"], list)
                and all(isinstance(name, str) for name in planned["plans"]), "invalid chapter plan set")
        paths.extend(plan_set.parent / name for name in planned["plans"])
    plans = [(path.resolve(), read_json(path)) for path in paths]
    require(plans, "no dialogue plans")
    names = [path.stem for path, _ in plans]
    require(len(set(names)) == len(names), "duplicate plan names")
    require(len({plan["title"] for _, plan in plans}) == len(plans), "duplicate chapter titles")
    return plans


def reusable_chapters(batch_paths, provenance):
    candidates = []
    for batch in batch_paths or []:
        selection = read_json(batch / "selection.json")
        coverage = read_json(batch / "coverage.json")
        require(coverage["provenance"] == selection, "reuse batch selection changed")
        require(all(selection[key] == provenance[key] for key in
                    ("game", "asset_manifest_sha256", "exporter_sha256")),
                "reuse batch source or exporter differs")
        completed = {entry["record"]: entry for entry in coverage["completed"]}
        require(len(completed) == len(coverage["completed"]), "ambiguous reuse chapter")
        records = selection["records"]
        require(len({record["name"] for record in records}) == len(records)
                and completed.keys() <= {record["name"] for record in records},
                "reuse chapter is not in its selection")
        for record in records:
            if record["name"] in completed:
                require(record["name"] == record["plan"]["title"], "reuse chapter title differs")
                candidates.append((record["plan"], completed[record["name"]]))
    return candidates


def render(args):
    plans = chapter_plans(args.plan, args.plan_set)
    assets = args.assets.resolve()
    manifest, manifest_hash = inventory(assets)
    args.out = args.out.resolve()
    args.out.mkdir(parents=True, exist_ok=True)
    provenance = dict(schema=1, game=manifest.get("game", "commander_blood"),
                      asset_manifest_sha256=manifest_hash, exporter_sha256=digest(args.exporter),
                      records=[dict(name=plan["title"], plan=plan, plan_sha256=digest(path))
                               for path, plan in plans])
    reusable = reusable_chapters(args.reuse_batch, provenance)
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
            verified = None
            if not path.exists():
                reused = next((entry for candidate, entry in reusable if candidate == plan), None)
                if reused is not None:
                    verified = verify_chapter(Path(reused["path"]), plan, manifest, provenance["exporter_sha256"])
                    require(verified == reused, "reused chapter evidence changed")
                    path.symlink_to(Path(reused["path"]).resolve(), target_is_directory=True)
                    verified = {**verified, "path": str(path)}
                else:
                    with (args.out / (plan_path.stem + ".log")).open("w") as log:
                        command([args.exporter.resolve(), assets, "dialogue:" + str(plan_path), path,
                                 args.max_frames], stdout=log, stderr=subprocess.STDOUT)
            if verified is None:
                verified = verify_chapter(path, plan, manifest, provenance["exporter_sha256"])
            completed.append(verified)
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
    capture.add_argument("--plan", type=Path, action="append")
    capture.add_argument("--plan-set", type=Path, help="planning.json from the static BAS planner")
    capture.add_argument("--reuse-batch", type=Path, action="append",
                         help="reuse identical completed chapters, with full source/trace/media verification")
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
