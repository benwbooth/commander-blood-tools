#!/usr/bin/env python3
"""Join verified native chapters to every statically catalogued dialogue site.

UI-raster evidence is not an encoded-visibility claim. A site absent from one
selected branch is not classified as globally unreachable or complete.
"""

import argparse
import json
from collections import Counter
from pathlib import Path

from native_dialogue_anthology import validate_trace
from native_sequence_anthology import read_json, require, timeline
from video_anthology import digest, save_json

GAMES = {"commander_blood": "cb", "big_bug_bang": "bbb"}


def add_graph(graph, profiles, sites):
    key = (graph["game"], graph["profile"].upper())
    require(key not in profiles, "duplicate static profile")
    profiles[key] = graph["resources"]
    for kind in ("cod", "bas"):
        if graph[kind] is None:
            continue
        for site in graph[kind]["text_sites"]:
            identity = (*key, kind, site["offset"])
            require(identity not in sites, "duplicate static text site")
            sites[identity] = dict(game=key[0], profile=key[1], kind=kind,
                                   offset=site["offset"], procedure=site.get("procedure"),
                                   record=site.get("record_name"),
                                   text=site.get("display", site)["text"],
                                   publications=[], ui_partial=[], ui_full=[], absent_branches=[])


def add_chapter(plan, chapter, profiles, sites, capture):
    key = (GAMES[plan["game"]], f"SCRIPT{plan['initial_profile'] + 1}")
    require(key in profiles, "chapter profile missing from static catalog")
    require(all(profiles[key][field] == plan[field] for field in ("cod_sha256", "dic_sha256")),
            "chapter source hashes differ from static catalog")
    if chapter.get("published_bas_sites"):
        require(plan.get("bas_sha256") is not None and
                profiles[key].get("bas_sha256") == plan["bas_sha256"],
                "chapter BAS hash differs from static catalog")
    for kind in ("cod", "bas"):
        published = set(chapter.get(f"published_{kind}_sites", []))
        absent = set(chapter.get(f"expected_unpublished_{kind}_sites", []))
        require(not published & absent, "chapter site is both published and absent")
        for offset in published | absent:
            identity = (*key, kind, offset)
            require(identity in sites, f"chapter references an unknown static {kind.upper()} site")
            site = sites[identity]
            if offset in absent:
                site["absent_branches"].append(capture)
                continue
            site["publications"].append(capture)
            raster = chapter["bas_ui_raster_evidence" if kind == "bas" else "ui_raster_evidence"].get(str(offset), {})
            if raster.get("ui_raster_frames", 0):
                site["ui_partial"].append(capture)
            if raster.get("fully_revealed_ui_frames", 0):
                require(raster.get("ui_raster_frames", 0) >= raster["fully_revealed_ui_frames"],
                        "full UI reveal exceeds raster frame count")
                site["ui_full"].append(capture)


def summarize(sites):
    counts = {}
    for site in sites.values():
        status = ("ui_fully_revealed" if site["ui_full"] else "ui_partly_revealed"
                  if site["ui_partial"] else "published_without_ui_raster" if site["publications"]
                  else "not_published_on_selected_branches" if site["absent_branches"] else "uncovered")
        site["evidence_status"] = status
        counts.setdefault(site["game"], Counter())[status] += 1
    return {game: dict(total=sum(count.values()), **dict(sorted(count.items())))
            for game, count in sorted(counts.items())}


def build(catalog, batches, output):
    index = read_json(catalog / "catalog.json")
    profiles, sites = {}, {}
    for profile in index["profiles"]:
        relative = profile["directory"] + "/graph.json"
        path = catalog / relative
        require(digest(path) == index["artifacts"][relative], "static graph changed")
        graph = read_json(path)
        require((graph["game"], graph["profile"]) == (profile["game"], profile["profile"]),
                "static profile identity changed")
        add_graph(graph, profiles, sites)
    for game, total in index["totals"].items():
        require(sum(site["game"] == game for site in sites.values()) == total["text_sites"],
                "static site census differs")
    captures = []
    seen = set()
    for batch in batches:
        coverage = read_json(batch / "coverage.json")
        require(coverage["complete"] and not coverage["failures"], "incomplete native chapter batch")
        records = coverage["provenance"]["records"]
        require([row["name"] for row in records] == [row["record"] for row in coverage["completed"]],
                "native batch chapter order differs")
        for record, chapter in zip(records, coverage["completed"]):
            path = Path(chapter["path"]).resolve()
            require(path not in seen, "duplicate capture path")
            seen.add(path)
            for name, field in [("report.json", "report_sha256"),
                                ("native-state.jsonl", "native_state_sha256"),
                                ("master.mkv", "master_sha256"), ("audio.f32le", "audio_sha256")]:
                require(digest(path / name) == chapter[field], f"changed chapter artifact: {path / name}")
            report = read_json(path / "report.json")
            plan = record["plan"]
            require(report["runner"]["chapter"] == plan and report["target"] == "dialogue_chapter"
                    and report["complete_native_presentation"] and not report["complete_game"]
                    and report["decoded_master_verified"] and report["video_timestamps_verified"],
                    "chapter has no matching verified native report")
            require(report["exporter_sha256"] == coverage["provenance"]["exporter_sha256"],
                    "chapter exporter differs from batch")
            require(report["runner"]["published_cod_sites"] == chapter["published_cod_sites"],
                    "chapter publication summary differs")
            rows, duration, samples = timeline(path / "timeline.jsonl")
            require((len(rows), duration, samples) ==
                    (chapter["frames"], chapter["duration_ns"], chapter["audio_samples"]),
                    "chapter timeline accounting differs")
            with (path / "native-state.jsonl").open() as stream:
                evidence = validate_trace(plan, report["runner"],
                                          (json.loads(line) for line in stream), rows)
            for field in ("published_cod_sites", "state_trace_cod_sites", "ui_raster_evidence",
                          "published_without_ui_raster", "published_without_full_ui_reveal"):
                require(chapter[field] == evidence[field], "chapter trace evidence differs: " + field)
            for field in ("published_bas_sites", "state_trace_bas_sites", "published_bas_without_ui_raster",
                          "published_bas_without_full_ui_reveal", "bas_ui_raster_evidence"):
                require(chapter.get(field, {} if field == "bas_ui_raster_evidence" else []) == evidence[field],
                        "chapter trace evidence differs: " + field)
            require(chapter.get("expected_unpublished_cod_sites", []) ==
                    evidence["expected_unpublished_cod_sites"], "chapter absent-site evidence differs")
            require(chapter.get("inventory_transfers", []) == evidence.get("inventory_transfers", []),
                    "chapter inventory transfer evidence differs")
            require(chapter.get("inventory_cancellations", []) == evidence.get("inventory_cancellations", []),
                    "chapter inventory cancellation evidence differs")
            require(chapter.get("travel_encounter") == evidence.get("travel_encounter"),
                    "chapter travel encounter evidence differs")
            capture = len(captures)
            add_chapter(plan, chapter, profiles, sites, capture)
            captures.append(dict(title=record["name"], path=str(path),
                                 master_sha256=chapter["master_sha256"],
                                 duration_ns=chapter["duration_ns"]))
    counts = summarize(sites)
    save_json(output, dict(schema=1,
              scope="static dialogue census joined to hash-bound native chapter evidence; not full-game completion or DOS parity",
              ui_evidence_scope="native UI buffer before presentation; not a proof of encoded glyph visibility",
              absent_scope="not published in these selected branches, not globally unreachable",
              catalog_sha256=digest(catalog / "catalog.json"), counts=counts, captures=captures,
              sites=[sites[key] for key in sorted(sites)]))
    for game, count in counts.items():
        print(f"{game}: {count}")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--catalog", type=Path, required=True)
    parser.add_argument("--batch", type=Path, action="append", required=True)
    parser.add_argument("--out", type=Path, required=True)
    args = parser.parse_args()
    build(args.catalog, args.batch, args.out)


if __name__ == "__main__":
    main()
