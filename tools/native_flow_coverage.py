#!/usr/bin/env python3
# /// script
# requires-python = ">=3.11"
# dependencies = []
# ///
"""Audit normal-input flow witnesses without running or rendering the games.

Successful lineages, site evidence, and repeated wording are separate records.
An unseen site, including one with familiar wording, is never called unreachable.
"""

import argparse
from collections import Counter, defaultdict
import hashlib
import json
from pathlib import Path
import re

from native_game_flow import (normal_actions, revealed_text_sites, saved_checkpoints,
                              verify_route_completion)
from video_anthology import digest, save_json

OBSERVED = {"observed_route", "observed_ending"}
REQUIRED_FILES = {"events.jsonl", "actions.jsonl", "scenario.tsv", "game.log", "flow.md"}


def require(condition, message):
    if not condition:
        raise ValueError(message)


def read_json(path):
    return json.loads(path.read_text())


def safe_child(root, name):
    path = root / name
    require(not path.is_symlink() and path.resolve().is_relative_to(root.resolve()),
            f"unsafe evidence path: {path}")
    return path


def text_value(state):
    text = state["text"]
    return (bytes(text["content"]).decode("utf-8", "replace")
            if text["kind"] == "subtitle_bytes" else " ".join(text["content"]))


def wording(text):
    # Collapse only layout whitespace, not case, punctuation, names, or numbers.
    return " ".join(text.split())


def inline_template(site):
    if "display" in site:
        return site["display"]["text"]
    return " ".join(word["text"] if word["kind"] == "dictionary"
                    else f"<state:{word['offset']}>" for word in site["spoken_operands"])


def matches_text(site, evidence):
    expected = site["text"] if evidence.get("text_kind") == "subtitle_bytes" else site["inline_text"]
    expected, actual = wording(expected), wording(evidence["text"])
    if not site["dynamic"]:
        return expected == actual
    # Only the authored numeric placeholders may vary. This checks displayed
    # shape, not the correctness of the runtime value or original-game parity.
    parts = re.split(r"(<state:[0-9]+>)", expected)
    pattern = "".join(r"-?[0-9]+" if re.fullmatch(r"<state:[0-9]+>", part)
                      else re.escape(part) for part in parts)
    return re.fullmatch(pattern, actual) is not None


def load_catalog(directory):
    index = read_json(directory / "catalog.json")
    sites, sources = {}, {}
    for row in index["profiles"]:
        relative = row["directory"] + "/graph.json"
        path = safe_child(directory, relative)
        require(digest(path) == index["artifacts"][relative], f"changed static graph: {path}")
        graph = read_json(path)
        game, profile = row["game"], int(row["profile"].removeprefix("SCRIPT"))
        require((graph["game"], graph["profile"]) == (game, row["profile"]),
                "static graph identity changed")
        source = f"{game}/script{profile}.blood"
        expected = index["provenance"]["sources"][source]
        require(digest(safe_child(directory, row["directory"] + "/source.blood")) == expected,
                f"changed catalog source: {source}")
        sources[game, profile] = expected
        for domain in ("cod", "bas"):
            for site in (graph.get(domain) or {}).get("text_sites", []):
                key = game, profile, domain, site["offset"]
                require(key not in sites, "duplicate static text site")
                sites[key] = dict(game=game, profile=profile, domain=domain,
                                  offset=site["offset"], procedure=site.get("procedure"),
                                  record=site.get("record_name"), dynamic=site.get("dynamic", False),
                                  text=site.get("display", site)["text"], inline_text=inline_template(site),
                                  witnesses=[], rejected_presentations=[])
    return index, sites, sources


def scan_events(path, summary):
    """Re-audit old traces too; their original hashed summaries remain untouched."""
    state, last_frame, events = None, -1, 0
    full, saves, loads, active_full = {}, {}, [], {}
    published, observed_resources = set(), set()
    with path.open() as stream:
        for line in stream:
            event = json.loads(line)
            frame = event["frame"]
            require(event["event"] == events and frame > last_frame,
                    f"nonmonotonic compact events: {path}")
            last_frame, events = frame, events + 1
            if event["state"] is not None:
                state = event["state"]
            require(state is not None, "first compact event has no state")
            require("CHEAT MODE" not in text_value(state).upper(), "observed script cheat menu")
            operations = state.get("save_load") or {}
            loaded = operations.get("completed_loads", 0)
            require(loaded in (0, 1), "more than one checkpoint load")
            if loaded and not loads:
                loads.append(dict(frame=frame, slot=operations["active_slot"]))
            require(not loads or loaded == 1, "checkpoint load counter regressed")
            saved = operations.get("completed_saves", 0)
            if saved > len(saves):
                require(saved == len(saves) + 1, "save counter jumped")
                saves[saved] = dict(frame=frame, slot=operations["active_slot"])
            require(saved == len(saves), "save counter regressed")
            if state["profile"] is not None:
                for domain in ("cod", "bas"):
                    offset = state[f"{domain}_site"]
                    if offset is not None:
                        published.add((state["profile"] + 1, domain, offset))
            raster = event.get("text_raster") or {}
            expected = raster.get("expected_pixel_count", 0)
            require(not expected or expected == raster.get("matching_pixel_count"),
                    f"text raster mismatch: {path}:{events}")
            next_full = {}
            if expected:
                for profile, domain, offset in revealed_text_sites(state):
                    key = profile + 1, domain, offset
                    detail = dict(actor=state["actor"], text=text_value(state), text_kind=state["text"]["kind"],
                                  video=state["active_video"])
                    next_full[key] = detail
                    if active_full.get(key) != detail:
                        full.setdefault(key, []).append(dict(frame=frame, **detail))
            active_full = next_full
            if state["active_video"] and event["decoded_frame_count"] > 0:
                observed_resources.add(state["active_video"])
    require(events == summary["events"], "compact event count differs")
    require(last_frame < summary["observed_frames"] and state == summary["final_state"],
            "final event state differs")
    return dict(full=full, published=published, saves=saves, loads=loads,
                observed_resources=observed_resources)


def read_witness(path, source_hashes):
    path = path.resolve()
    raw = path.read_bytes()
    manifest = json.loads(raw)
    require(manifest.get("schema") == 1 and manifest.get("status") in OBSERVED
            and manifest.get("state_injection") is False, f"not a normal-input witness: {path}")
    require("lifecycle" in (manifest.get("observations", {}).get("final_state") or {}),
            "legacy witness lacks required audit fields")
    require(REQUIRED_FILES.issubset(manifest["files"]), "missing hashed evidence")
    for name, expected in manifest["files"].items():
        require(digest(safe_child(path.parent, name)) == expected, f"changed evidence: {path.parent / name}")
    source = (path.parent / "scenario.tsv").read_text()
    actions = normal_actions(source)
    require(hashlib.sha256(source.encode()).hexdigest() == manifest["provenance"]["scenario_sha256"],
            "scenario provenance differs")
    game = manifest["game"]
    prefix = "re/vm/big-bug-bang-profiles" if game == "bbb" else "re/vm/profiles"
    for (source_game, profile), expected in source_hashes.items():
        if source_game == game:
            name = f"{prefix}/script{profile}.blood"
            require(manifest["provenance"]["sources"].get(name) == expected,
                    f"flow and static catalog sources differ: {name}")
    completed = []
    with (path.parent / "actions.jsonl").open() as stream:
        for line in stream:
            action = json.loads(line)
            if action["phase"] == "after":
                require(action["action_index"] == len(completed) + 1, "completed action order differs")
                completed.append(action["action"])
    summary = manifest["observations"]
    evidence = scan_events(path.parent / "events.jsonl", summary)
    ending = manifest.get("observed_ending")
    verified_ending = verify_route_completion(
        actions, completed, 0, summary["final_state"],
        ending_offset=ending["code_offset"] if ending and game == "bbb" else None,
        expect_cb_ending=bool(ending and game == "cb"),
        sequence_runs=summary.get("sequence_runs"), final_objects=summary.get("final_objects"))
    require((ending is not None) == (manifest["status"] == "observed_ending"), "ending status differs")
    if ending:
        require(all(verified_ending.get(key) == value for key, value in ending.items()),
                "ending summary differs")
    slots = {row["slot"]: row["saved_at_frame"] for row in manifest.get("checkpoints", [])}
    actual = saved_checkpoints(path.parent / "writable", slots)
    require(actual == manifest.get("checkpoints", []), "checkpoint bytes or directory changed")
    for checkpoint in actual:
        require(dict(frame=checkpoint["saved_at_frame"], slot=checkpoint["slot"])
                in evidence["saves"].values(), "checkpoint has no native save event")
    require(len(evidence["loads"]) == int(bool(manifest.get("predecessor"))), "unvalidated load")
    return dict(path=path, sha256=hashlib.sha256(raw).hexdigest(), manifest=manifest,
                evidence=evidence, ending=verified_ending)


def check_lineages(witnesses):
    done, active = set(), set()

    def visit(path):
        if path in done:
            return
        require(path not in active and len(active) < 100, "cyclic or too-deep lineage")
        require(path in witnesses, f"missing valid predecessor: {path}")
        active.add(path)
        row = witnesses[path]
        manifest = row["manifest"]
        parent = manifest.get("predecessor")
        if parent:
            parent_path = Path(parent["manifest"]).resolve()
            visit(parent_path)
            previous = witnesses[parent_path]
            require(previous["sha256"] == parent["manifest_sha256"], "predecessor manifest changed")
            before = previous["manifest"]
            require(before["game"] == manifest["game"] and before["status"] == "observed_route",
                    "wrong game or ending used as predecessor")
            require(manifest["observations"].get("loaded_checkpoint") is True
                    and row["evidence"]["loads"][0]["slot"] == parent["slot"], "wrong loaded slot")
            require(any(c["slot"] == parent["slot"] for c in before["checkpoints"]),
                    "parent has no witnessed save in loaded slot")
            for field in ("asset_manifest_sha256", "sources"):
                require(before["provenance"][field] == manifest["provenance"][field],
                        f"lineage changes {field}")
            old, new = (m["provenance"]["binary_sha256"] for m in (before, manifest))
            update = parent.get("runtime_update")
            if old != new:
                require(isinstance(update, dict) and isinstance(update.get("reason"), str)
                        and update["reason"].strip()
                        and update.get("previous_binary_sha256") == old
                        and update.get("current_binary_sha256") == new, "unrecorded runtime change")
            else:
                require(update is None, "spurious runtime change")
        active.remove(path)
        done.add(path)

    for path in witnesses:
        visit(path)


def lineage_segments(leaf, witnesses):
    """Exclude parent actions after the exact save the next segment loaded."""
    result, end = [], None
    while leaf is not None:
        row = witnesses[leaf]
        manifest, evidence = row["manifest"], row["evidence"]
        start = evidence["loads"][0]["frame"] if evidence["loads"] else 0
        end = manifest["observations"]["observed_frames"] - 1 if end is None else end
        require(start <= end, "loaded segment starts after its consumed save")
        result.append(dict(witness=str(leaf), name=leaf.parent.name,
                           start_frame=start, end_frame=end))
        parent = manifest.get("predecessor")
        if parent is None:
            break
        leaf = Path(parent["manifest"]).resolve()
        end = next(c["saved_at_frame"] for c in witnesses[leaf]["manifest"]["checkpoints"]
                   if c["slot"] == parent["slot"])
    return list(reversed(result))


def join_sites(sites, witnesses, main_routes):
    main_ranges = {Path(segment["witness"]): (segment["start_frame"], segment["end_frame"])
                   for route in main_routes.values() for segment in route}
    for path, row in witnesses.items():
        game = row["manifest"]["game"]
        for identity, occurrences in row["evidence"]["full"].items():
            key = (game, *identity)
            require(key in sites, f"unknown fully revealed source site: {key}")
            matching = [e for e in occurrences if matches_text(sites[key], e)]
            rejected = [e for e in occurrences if not matches_text(sites[key], e)]
            if rejected:
                sites[key]["rejected_presentations"].append(dict(witness=str(path), **rejected[0],
                    reason="rendered text does not match the attributed source site's spoken text"))
            if not matching:
                continue
            bounds = main_ranges.get(path)
            inside = [e for e in matching if bounds and bounds[0] <= e["frame"] <= bounds[1]]
            evidence = (inside or matching)[0]
            sites[key]["witnesses"].append(dict(witness=str(path), **evidence,
                on_successful_route=bool(inside)))
    known_wording = {(site["game"], wording(site["text"])) for site in sites.values()
                     if site["witnesses"] and wording(site["text"]) and not site["dynamic"]}
    for site in sites.values():
        text = wording(site["text"])
        if any(w["on_successful_route"] for w in site["witnesses"]):
            site["status"] = "witnessed_on_successful_route"
        elif site["witnesses"]:
            site["status"] = "witnessed_on_other_normal_route"
        elif not text or text in {"...", "."}:
            site["status"] = "unobserved_empty_or_control_text"
        elif not site["dynamic"] and (site["game"], text) in known_wording:
            site["status"] = "unobserved_site_with_witnessed_wording"
        else:
            site["status"] = "unobserved_wording_or_dynamic_site"


def build(flow_root, catalog, successes, output):
    require(not output.exists(), "audit output must be new")
    index, sites, sources = load_catalog(catalog)
    witnesses, excluded = {}, []
    paths = sorted(flow_root.glob("*/flow.json"))
    require(paths, "no flow manifests")
    for path in paths:
        manifest = read_json(path)
        reason = ("not_completed" if manifest.get("status") not in OBSERVED else
                  "legacy_trace_without_required_audit_fields" if "lifecycle" not in
                  (manifest.get("observations", {}).get("final_state") or {}) else None)
        if reason:
            excluded.append(dict(path=str(path.resolve()), sha256=digest(path),
                                 status=manifest.get("status"), reason=reason))
            continue
        print(f"Auditing {path.parent.name}", flush=True)
        row = read_witness(path, sources)
        witnesses[path.resolve()] = row
    check_lineages(witnesses)
    main_routes = {}
    for game, path in successes.items():
        path = path.resolve()
        require(path in witnesses and witnesses[path]["manifest"]["game"] == game, "missing successful leaf")
        ending = witnesses[path]["ending"] or {}
        require(ending.get("kind") == {"cb": "cb_concert", "bbb": "bbb_success"}[game],
                "selected leaf does not witness the successful ending")
        main_routes[game] = lineage_segments(path, witnesses)
    require(set(main_routes) == {"cb", "bbb"}, "select one successful leaf for each game")
    join_sites(sites, witnesses, main_routes)
    counts = {game: dict(Counter(site["status"] for site in sites.values() if site["game"] == game))
              for game in main_routes}
    records = []
    for path, row in witnesses.items():
        manifest = row["manifest"]
        records.append(dict(path=str(path), name=path.parent.name, sha256=row["sha256"],
                            game=manifest["game"], status=manifest["status"],
                            predecessor=manifest.get("predecessor"), ending=row["ending"],
                            resources=manifest["observations"]["decoded_video_resources"],
                            sequence_runs=manifest["observations"].get("sequence_runs", []),
                            fully_revealed_sites=len(row["evidence"]["full"])))
    result = dict(schema=1, scope="hash-bound normal native route evidence, not DOS parity or complete branch coverage",
                  time_basis="native frame boundaries, not video timestamps or durations",
                  wording_scope="identical wording is not proof of the unobserved site's scene or reachability",
                  sequence_scope="recorded sequence runs, not a proof of full source frame or caption coverage",
                  all_normal_branches_complete=False, render_ready=False,
                  catalog_sha256=digest(catalog / "catalog.json"), auditor_sha256=digest(Path(__file__)),
                  counts=counts, successful_routes=main_routes, witnesses=records, excluded=excluded,
                  sites=[sites[key] for key in sorted(sites)])
    output.mkdir(parents=True)
    save_json(output / "coverage.json", result)
    for game in main_routes:
        (output / f"{game}.md").write_text(report_markdown(result, game))
    print(json.dumps(counts), flush=True)
    return result


def report_markdown(result, game):
    lines = [f"# {game.upper()} Normal-Flow Audit", "",
             "Successful normal-input route plus other observed branches. Not complete or render-ready.",
             "Frame numbers identify native evidence; they are not video timestamps or durations.", "",
             "## Successful Route", "",
             "The next segment loads the preceding segment's exact witnessed save. Startup reloads and",
             "actions after a consumed save are outside the listed segment ranges.", ""]
    records = {row["path"]: row for row in result["witnesses"]}
    for number, segment in enumerate(result["successful_routes"][game], 1):
        root = Path(segment["witness"]).parent
        lines.append(f"{number}. [{segment['name']}]({root / 'flow.md'}), "
                     f"boundaries {segment['start_frame']}--{segment['end_frame']}.")
    lines += ["", "## Coverage", "", "| Evidence | Sites |", "| --- | ---: |"]
    lines += [f"| {status} | {count} |" for status, count in sorted(result["counts"][game].items())]
    lines += ["", "Counts include UI and control text. Equal wording does not imply equal visuals,",
              "speaker, state, or reachability. Unobserved sites remain unresolved."]
    groups = defaultdict(Counter)
    for site in result["sites"]:
        if site["game"] == game and site["status"].startswith("unobserved_"):
            groups[site["profile"], site.get("record") or "Unattributed", site["domain"]][site["status"]] += 1
    columns = ("unobserved_wording_or_dynamic_site", "unobserved_site_with_witnessed_wording",
               "unobserved_empty_or_control_text")
    ranked = sorted(groups, key=lambda key: (-groups[key][columns[0]], -sum(groups[key].values()), key))
    lines += ["", "## Unresolved Groups", "",
              "Up to twenty source groups, ordered by unobserved new or dynamic wording.",
              "These are source-site counts, not distinct clips, feasible branches, or duration estimates.",
              "The full per-site evidence and unresolved list remain in `coverage.json`.", "",
              "| Profile | Source Actor | Domain | New/Dynamic | Known Wording | Empty/Control |",
              "| --- | --- | --- | ---: | ---: | ---: |"]
    for key in ranked[:20]:
        profile, actor, domain = key
        actor = actor.replace("|", "\\|")
        values = " | ".join(str(groups[key][status]) for status in columns)
        lines.append(f"| SCRIPT{profile} | {actor} | {domain.upper()} | {values} |")
    if not ranked:
        lines += ["", "No unobserved source text sites; this alone is not full scene-coverage proof."]
    lines += ["", "## Other Witnesses", ""]
    main = {row["witness"] for row in result["successful_routes"][game]}
    contributions = defaultdict(set)
    for site in result["sites"]:
        if site["game"] == game and site["status"] == "witnessed_on_other_normal_route":
            for witness in site["witnesses"]:
                contributions[witness["witness"]].add((site["profile"], site["domain"], site["offset"]))
    for path, record in sorted(records.items()):
        if record["game"] != game or path in main:
            continue
        lines.append(f"- [{record['name']}]({Path(path).parent / 'flow.md'}): "
                     f"{len(contributions[path])} sites outside the successful route; "
                     f"{len(record['sequence_runs'])} recorded sequence runs including startup/repeats.")
    lines += ["", "These are witnesses, not an automatically selected edit list. Repeated material",
              "must be deduplicated by its full scene, not only its filename or wording.", ""]
    return "\n".join(lines)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--flows", type=Path, required=True)
    parser.add_argument("--catalog", type=Path, required=True)
    parser.add_argument("--cb-success", type=Path, required=True)
    parser.add_argument("--bbb-success", type=Path, required=True)
    parser.add_argument("--out", type=Path, required=True)
    args = parser.parse_args()
    build(args.flows, args.catalog, {"cb": args.cb_success, "bbb": args.bbb_success}, args.out)


if __name__ == "__main__":
    main()
