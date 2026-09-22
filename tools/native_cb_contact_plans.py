#!/usr/bin/env python3
"""Plan CB BAS topic captures for every authored contact procedure in source order.

These are candidates, not verified chapters or proof of reachable game states.
"""

import argparse
from pathlib import Path

from native_bas_plans import plan_topics
from native_sequence_anthology import read_json, require
from video_anthology import ROOT, digest, save_json


def build(catalog, manifest_path, out):
    index = read_json(catalog / "catalog.json")
    profiles = {}
    for row in index["profiles"]:
        if row["game"] != "cb":
            continue
        path = catalog / row["directory"] / "graph.json"
        relative = row["directory"] + "/graph.json"
        require(digest(path) == index["artifacts"][relative], "static graph changed")
        profiles[row["profile"].upper()] = (read_json(path), digest(path))
    manifest = read_json(manifest_path)
    procedures = sorted(manifest["procedures"],
                        key=lambda row: (int(row["script"][6:]), row["procedure_offset"]))
    require(len({(row["script"], row["procedure_offset"]) for row in procedures}) == len(procedures),
            "duplicate authored contact procedure")
    planned, inventory = [], []
    for contact in procedures:
        script = contact["script"]
        graph, graph_hash = profiles[script]
        actor = contact["contact_object"]
        anchor = contact["procedure_offset"]
        procedure = next(row["procedure"] for row in graph["cod"]["instructions"]
                         if row["offset"] == anchor)
        cod_sites = [site for site in graph["cod"]["text_sites"]
                     if site["procedure"] == procedure]
        prerequisites = dict(cod_site_offsets=[site["offset"] for site in cod_sites],
                             cod_choice_offsets=[site["offset"] for site in cod_sites
                                                 if site["choice_operands"]],
                             state_resolution="required; a BAS menu match does not prove "
                             "the native entry reaches that menu")
        owner = [row for row in graph["bas"]["control_flow"]["lists"]
                 if row["entrypoint"]["object_name"] == actor]
        if len(owner) != 1:
            inventory.append(dict(script=script, procedure_offset=contact["procedure_offset"],
                                  actor=actor, entry_class=contact["entry_class"],
                                  status="no_unique_bas_list", plan_count=0, **prerequisites))
            continue
        plans, report = plan_topics(graph, contact)
        files = []
        for ordinal, (menu, word, plan) in enumerate(plans):
            plan["title"] += f" [{script} COD {contact['procedure_offset']:04x}]"
            filename = (f"{script.lower()}-{contact['procedure_offset']:05d}-"
                        f"{ordinal:03d}-{menu:04x}-{word:04x}.plan.json")
            planned.append((filename, plan))
            files.append(filename)
        inventory.append(dict(script=script, procedure_offset=contact["procedure_offset"],
                              actor=actor, entry_class=contact["entry_class"],
                              graph_sha256=graph_hash, status="candidate_plans",
                              plan_count=len(files), plans=files, **prerequisites, **report))
    require(not out.exists(), "output directory already exists")
    out.mkdir(parents=True)
    for filename, plan in planned:
        save_json(out / filename, plan)
    save_json(out / "planning.json", dict(schema=1, scope="source-ordered CB contact BAS candidates; "
              "not native capture, state feasibility, or complete dialogue coverage",
              catalog_sha256=digest(catalog / "catalog.json"),
              contact_manifest_sha256=digest(manifest_path),
              plans=[name for name, _ in planned], contacts=inventory))
    print(f"{len(procedures)} contacts, {len(planned)} candidate chapters; "
          f"{sum(row['status'] != 'candidate_plans' for row in inventory)} without a unique BAS list")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--catalog", type=Path, required=True)
    parser.add_argument("--contact-manifest", type=Path,
                        default=ROOT / "re/vm/contact-manifest/contact-manifest.json")
    parser.add_argument("--out", type=Path, required=True)
    args = parser.parse_args()
    build(args.catalog, args.contact_manifest, args.out)


if __name__ == "__main__":
    main()
