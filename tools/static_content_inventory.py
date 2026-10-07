#!/usr/bin/env python3
# /// script
# requires-python = ">=3.11"
# dependencies = []
# ///
"""Freeze the authored anthology inventory without executing either game.

Source sites stay distinct, even when their text matches. Request fingerprints
identify possible rendering reuse, not equivalent scenes or reachable routes.
"""

import argparse
from collections import Counter
import hashlib
import json
from pathlib import Path

from native_sequence_anthology import read_json, require
from video_anthology import asset_path, digest, inventory as asset_inventory, resource_name, save_json

GAMES = {"cb": "commander_blood", "bbb": "big_bug_bang"}
CONTEXT_FIELDS = {"offset", "block", "procedure", "selector_node", "object"}


def fingerprint(value):
    return hashlib.sha256(json.dumps(value, sort_keys=True, ensure_ascii=True,
                                    separators=(",", ":")).encode()).hexdigest()


def site_entry(graph, kind, site, descriptions):
    identity = f"{graph['game']}.{graph['profile'].lower()}.{kind}.{site['offset']:08x}"
    symbols = sorted({word["offset"] for section in site["sections"] for word in section
                      if word["kind"] == "state_number"})
    inventory = any(word["kind"] == "inventory_choices"
                    for section in site["sections"] for word in section)
    content = ("inventory_menu" if inventory else "symbolic_text" if symbols
               else "text_and_choices" if site["choice_operands"] and site["spoken_operands"]
               else "choices" if site["choice_operands"] else "text"
               if site["spoken_operands"] else "control_only")
    request = {key: value for key, value in site.items() if key not in CONTEXT_FIELDS}
    if "display" in request:
        request["display"] = {key: value for key, value in request["display"].items()
                              if key != "catalog_id"}
    return dict(id=identity, game=graph["game"], profile=graph["profile"], kind=kind,
                offset=site["offset"], source_hashes=graph["resources"], authored=site,
                content_kind=content,
                direct_description_candidates=descriptions.get(site.get("record_name"), []),
                state_number_operands=symbols, generated_inventory_choices=inventory,
                authored_request_fingerprint=fingerprint(dict(
                    game=graph["game"], profile=graph["profile"], kind=kind,
                    source_hashes=graph["resources"], request=request)),
                render_status="not_rendered", gameplay_reachability="not_assessed")


def validate_description_commands(catalog, image):
    require(len(image) >= 2, "truncated DESCRIPT directory")
    count = int.from_bytes(image[:2], "little")
    require(count == len(catalog["records"]), "DESCRIPT directory count differs")
    kinds = {1: "Location", 2: "Character", 4: "Sequence", 15: "Object"}
    for index, record in enumerate(catalog["records"]):
        directory = image[2 + 18 * index:2 + 18 * (index + 1)]
        require(len(directory) == 18 and 0 in directory[:16], "truncated DESCRIPT directory entry")
        require(bytes(record["name_bytes"]) == directory[:16].split(b"\0", 1)[0],
                "DESCRIPT directory name bytes differ")
        offset = int.from_bytes(directory[16:], "little")
        require(0 < offset < len(image) - 1 and record["kind"] == kinds.get(image[offset - 1]),
                "DESCRIPT record kind differs")
        end = offset + int.from_bytes(image[offset:offset + 2], "little")
        cursor = offset + 2
        for command in record["commands"]:
            raw = bytes(command["source_bytes"])
            require(raw and command["source_offset"] == cursor
                    and command["source_byte_count"] == len(raw)
                    and image[cursor:cursor + len(raw)] == raw,
                    "DESCRIPT command bytes or authored order differ")
            cursor += len(raw)
        require(cursor == end - 1 and cursor < len(image)
                and image[cursor] in (0, 1, 2, 4, 15, 255),
                "DESCRIPT command stream is incomplete")


def validate_media_manifest(catalog, manifest):
    expected = {}
    for row in manifest["resources"]:
        name = resource_name(row["resource_name"])
        require(name not in expected, "duplicate imported resource name")
        expected[name] = row
    videos = {name: row for name, row in expected.items() if name.endswith(".HNM")}
    require(len(catalog["videos"]) == len(videos)
            and {row["name"] for row in catalog["videos"]} == videos.keys(),
            "video catalog differs from the complete imported HNM census")
    for row in catalog["videos"]:
        require(all(row.get(key) == value for key, value in videos[row["name"]].items()),
                "video resource metadata differs from imported manifest")
    require(catalog["game"] == manifest.get("game", "commander_blood"), "media manifest game differs")
    description = expected.get("DESCRIPT.DES")
    require(description is not None and all(catalog["descript"][key] == description[key]
                                           for key in ("sha256", "byte_count", "path")),
            "DESCRIPT metadata differs from imported manifest")
    return expected


def validate_profile_sources(graph, resources):
    for kind in ("cod", "dic", "bas"):
        expected = graph["resources"].get(kind + "_sha256")
        if expected is None and kind == "bas":
            continue
        resource = resources.get(graph["profile"].upper() + "." + kind.upper())
        require(expected and resource and resource["sha256"] == expected,
                f"{graph['game']} {graph['profile']} {kind.upper()} differs from imported media source")


def join_inventory(graphs, media):
    records, sites, videos, profiles = [], [], [], []
    seen_profiles, seen_sites = set(), set()
    for game, catalog in media.items():
        require(catalog["game"] == GAMES[game], "media game differs")
        require(catalog.get("descript", {}).get("ordered_commands_exported") is True,
                "full ordered DESCRIPT commands are required; regenerate the media catalog")
        for index, record in enumerate(catalog["records"]):
            require("commands" in record and "name_bytes" in record,
                    "description is missing its lossless ordered commands or name")
            records.append(dict(id=f"{game}.descript.{index:04d}", game=game,
                                source_sha256=catalog["descript"]["sha256"],
                                directory_index=index, authored=record,
                                render_status="not_rendered", gameplay_reachability="not_assessed"))
        names = set()
        for video in catalog["videos"]:
            require(video["name"] not in names, "duplicate native video resource")
            names.add(video["name"])
            videos.append(dict(game=game, resource=video["name"], authored=video,
                               render_status="not_rendered"))
    for graph in graphs:
        key = (graph["game"], graph["profile"])
        require(key not in seen_profiles, "duplicate dialogue profile")
        require(graph["game"] in media, "dialogue game has no media catalog")
        seen_profiles.add(key)
        descriptions = {}
        for record in records:
            if record["game"] == graph["game"]:
                descriptions.setdefault(record["authored"]["name"], []).append(record["id"])
        profiles.append(dict(game=graph["game"], profile=graph["profile"],
                             source_hashes=graph["resources"],
                             localization=graph.get("localization"),
                             conditions="retained in the hash-bound graph; not solved for reachability"))
        for kind in ("cod", "bas"):
            if graph[kind] is None:
                continue
            for site in graph[kind]["text_sites"]:
                entry = site_entry(graph, kind, site, descriptions)
                require(entry["id"] not in seen_sites, "duplicate authored site identity")
                seen_sites.add(entry["id"])
                sites.append(entry)
    counts = {}
    for game in media:
        subset = [site for site in sites if site["game"] == game]
        counts[game] = dict(profiles=sum(profile["game"] == game for profile in profiles),
                           text_sites=len(subset),
                           content_kinds=dict(sorted(Counter(site["content_kind"] for site in subset).items())),
                           authored_request_groups=len({site["authored_request_fingerprint"] for site in subset}),
                           description_records=sum(record["game"] == game for record in records),
                           description_commands=sum(len(record["authored"]["commands"])
                                                    for record in records if record["game"] == game),
                           native_video_resources=sum(video["game"] == game for video in videos),
                           rendered_sites=0)
    return dict(schema=1, scope="static authored content inventory for a prepared-state native anthology",
                static_inventory_complete=True, render_complete=False, full_game_complete=False,
                reachability_policy="not a prerequisite for a prepared-state render; never inferred from extraction",
                reuse_policy="fingerprints are candidate request matches, not proof of matching presentation state",
                description_binding="exact record-name candidates only; runtime may select another description",
                dynamic_policy="retain symbolic operands; every rendered value must have explicit state provenance",
                counts=counts, profiles=profiles, descriptions=records, sites=sites, videos=videos)


def build(catalog, media_paths, output):
    require(not output.exists(), f"output already exists: {output}")
    index = read_json(catalog / "catalog.json")
    inputs = {str((catalog / "catalog.json").resolve()): digest(catalog / "catalog.json")}
    for relative, expected in index["artifacts"].items():
        path = asset_path(catalog, relative)
        require(digest(path) == expected, f"static catalog artifact changed: {relative}")
        inputs[str(path)] = expected
    graphs = []
    expected_profiles = {(game, f"SCRIPT{number}") for game, count in (("cb", 5), ("bbb", 17))
                         for number in range(1, count + 1)}
    require({(row["game"], row["profile"]) for row in index["profiles"]} == expected_profiles,
            "catalog must contain all 22 active profiles")
    for row in index["profiles"]:
        relative = row["directory"] + "/graph.json"
        require(relative in index["artifacts"], "profile graph is not source-bound")
        graph = read_json(catalog / relative)
        require((graph["game"], graph["profile"]) == (row["game"], row["profile"]),
                "profile identity changed")
        graphs.append(graph)
    media, resources = {}, {}
    for game, path in media_paths.items():
        inputs[str(path.resolve())] = digest(path)
        item = read_json(path)
        assets = Path(item["assets"])
        manifest = assets / "manifest.json"
        imported, manifest_hash = asset_inventory(assets)
        require(manifest_hash == item["manifest_sha256"], "media asset manifest changed")
        resources[game] = validate_media_manifest(item, imported)
        description = asset_path(assets, item["descript"]["path"])
        require(digest(description) == item["descript"]["sha256"]
                and description.stat().st_size == item["descript"]["byte_count"],
                "original DESCRIPT resource changed")
        validate_description_commands(item, description.read_bytes())
        inputs[str(manifest.resolve())] = item["manifest_sha256"]
        inputs[str(description)] = item["descript"]["sha256"]
        media[game] = item
    for graph in graphs:
        validate_profile_sources(graph, resources[graph["game"]])
    result = join_inventory(graphs, media)
    for profile, row in zip(result["profiles"], index["profiles"]):
        relative = row["directory"] + "/graph.json"
        profile["graph_path"] = str((catalog / relative).resolve())
        profile["graph_sha256"] = index["artifacts"][relative]
    for game, totals in index["totals"].items():
        require(result["counts"][game]["text_sites"] == totals["text_sites"], "static site census differs")
    inputs[str(Path(__file__).resolve())] = digest(__file__)
    result["provenance"] = dict(inputs=inputs, input_fingerprint=fingerprint(inputs))
    # Recheck every input before publishing a single immutable inventory file.
    require(all(digest(Path(path)) == sha for path, sha in inputs.items()), "input changed during inventory")
    output.parent.mkdir(parents=True, exist_ok=True)
    require(not output.exists(), "output appeared during inventory")
    save_json(output, result)
    print(json.dumps(result["counts"], indent=2))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--catalog", type=Path, required=True)
    parser.add_argument("--cb-media", type=Path, required=True)
    parser.add_argument("--bbb-media", type=Path, required=True)
    parser.add_argument("--out", type=Path, required=True)
    args = parser.parse_args()
    build(args.catalog, {"cb": args.cb_media, "bbb": args.bbb_media}, args.out)


if __name__ == "__main__":
    main()
