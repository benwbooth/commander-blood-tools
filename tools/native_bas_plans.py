#!/usr/bin/env python3
"""Plan native CB contact topic chapters from the source-bound BAS menu graph.

This is static planning, not runtime exploration. Random/record/history variants,
unoffered topics, and menu cycles remain explicit gaps rather than invented paths.
"""

import argparse
from collections import deque
from pathlib import Path
import re

from native_sequence_anthology import read_json, require
from video_anthology import ROOT, digest, save_json


def choice(menu, word):
    return dict(source="bas_menu", text_site=menu["menu_offset"], word_offset=word)


def plan_topics(graph, contact):
    require(graph["game"] == "cb" and graph["profile"].upper() == contact["script"],
            "contact and static graph belong to different profiles")
    require(graph["resources"].get("bas_sha256"), "static catalog has no BAS source hash")
    flow = graph["bas"]["control_flow"]
    owners = [entry for entry in flow["lists"]
              if entry["entrypoint"]["object_name"] == contact["contact_object"]]
    require(len(owners) == 1, "contact does not own one BAS selector list")
    owner = owners[0]
    nodes = {node["offset"]: node for node in flow["nodes"] if node["offset"] in owner["node_offsets"]}
    root = owner["entrypoint"]["root_node"]
    require(root in nodes and nodes[root]["menu_offset"] is not None, "contact has no root menu")
    sites = [site for site in graph["bas"]["text_sites"] if site["selector_node"] in nodes]
    cod_sites = {site["offset"] for site in graph["cod"]["text_sites"]}
    anchor = contact["texts"][0]["opcode_offset"]
    require(anchor in cod_sites, "contact anchor is not in this COD graph")
    edges = {(edge["from_node"], edge["choice"]["offset"]): edge
             for edge in graph["bas"]["choice_edges"] if edge["from_node"] in nodes}
    paths = {root: []}
    queue = deque([root])
    while queue:
        current = queue.popleft()
        node = nodes[current]
        for word in node["menu_choices"]:
            target = edges[(current, word["offset"])]["to_node"]
            if target is not None and target not in paths and nodes[target]["menu_offset"] is not None:
                paths[target] = paths[current] + [choice(node, word["offset"])]
                queue.append(target)
    plans, deferred = [], []
    for current, prefix in paths.items():
        node = nodes[current]
        exits = [word for word in node["menu_choices"] if word["text"].lower() == "bye_bye"]
        if len(exits) != 1:
            deferred.append(dict(menu=current, reason="no unique authored bye_bye row"))
            continue
        for word in node["menu_choices"]:
            if word == exits[0] or edges[(current, word["offset"])]["to_node"] is not None:
                continue
            matched = [site for site in sites if site["selector_node"] == current and
                       any(operand.get("kind") == "dictionary" and operand.get("offset") == word["offset"]
                           for operand in site["choice_operands"])]
            # A6 bits 1/2 add random/record gates, bit 4 adds resume-side choices.
            simple = all(site["flags_b4"] & 0x40 and not site["flags_b4"] & 0x16
                         and site["recent_choice_count"] == 0
                         and len(site["choice_operands"]) == 1 for site in matched)
            if not matched or not simple:
                deferred.append(dict(menu=current, word=word,
                                     sites=[site["offset"] for site in matched],
                                     reason="no simple response or additional random/record/history/resume conditions"))
                continue
            plan = dict(schema=1, game="commander_blood",
                        title=contact["contact_object"].replace("_", " ") + ": " + word["text"].replace("_", " "),
                        initial_profile=int(contact["script"][6:]) - 1,
                        **graph["resources"], target=contact["contact_object"], entry="contact",
                        contact_procedure=contact["procedure_offset"],
                        choices=prefix + [choice(node, word["offset"]) for _ in matched] + [choice(node, exits[0]["offset"])],
                        required_cod_sites=[anchor], required_frame_boundary_cod_sites=[anchor],
                        required_bas_sites=[site["offset"] for site in matched],
                        required_frame_boundary_bas_sites=[site["offset"] for site in matched],
                        end=dict(kind="presentation_finished"))
            plans.append((current, word["offset"], plan))
    targeted = {site for _, _, plan in plans for site in plan["required_bas_sites"]}
    return plans, dict(scope="finite menu paths and simple topic responses; not all dialogue variants or verified captures",
                       bas_sites_in_actor_list=[site["offset"] for site in sites],
                       targeted_bas_sites=sorted(targeted),
                       not_targeted_bas_sites=sorted(site["offset"] for site in sites if site["offset"] not in targeted),
                       deferred=deferred,
                       unvisited_menu_nodes=sorted(set(nodes) - paths.keys()))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--catalog", type=Path, required=True)
    parser.add_argument("--profile", type=int, required=True, choices=range(1, 6))
    parser.add_argument("--procedure", type=int, required=True, help="original COD procedure offset")
    parser.add_argument("--out", type=Path, required=True)
    args = parser.parse_args()
    manifest_path = ROOT / "re/vm/contact-manifest/contact-manifest.json"
    manifest = read_json(manifest_path)
    tag = f"SCRIPT{args.profile}"
    contacts = [row for row in manifest["procedures"] if row["script"] == tag and row["procedure_offset"] == args.procedure]
    require(len(contacts) == 1, "no unique contact procedure")
    index = read_json(args.catalog / "catalog.json")
    profile = next(row for row in index["profiles"] if row["game"] == "cb" and row["profile"].upper() == tag)
    relative = profile["directory"] + "/graph.json"
    graph_path = args.catalog / relative
    require(digest(graph_path) == index["artifacts"][relative], "static graph changed")
    plans, report = plan_topics(read_json(graph_path), contacts[0])
    require(plans, "no simple BAS topic plans for this contact")
    args.out.mkdir(parents=True, exist_ok=False)
    actor = re.sub(r"[^a-z0-9]+", "-", contacts[0]["contact_object"].lower()).strip("-")
    paths = []
    for menu, word, plan in plans:
        filename = f"cb-script{args.profile}-{actor}-{menu:04x}-{word:04x}.plan.json"
        save_json(args.out / filename, plan)
        paths.append(filename)
    save_json(args.out / "planning.json", dict(schema=1, graph_sha256=digest(graph_path),
              contact_manifest_sha256=digest(manifest_path), plans=paths, **report))
    print(f"Planned {len(plans)} chapters targeting {len(report['targeted_bas_sites'])} BAS sites; "
          f"{len(report['not_targeted_bas_sites'])} actor-list sites remain unplanned.")


if __name__ == "__main__":
    main()
