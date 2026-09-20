#!/usr/bin/env python3
"""Plan native CB contact topic chapters from the source-bound BAS menu graph.

This is static planning, not runtime exploration. Random/record/history variants,
unoffered topics, and menu cycles remain explicit gaps rather than invented paths.
"""

import argparse
from collections import Counter, deque
from pathlib import Path
import re

from native_sequence_anthology import read_json, require
from video_anthology import ROOT, digest, save_json


def choice(menu, word):
    return dict(source="bas_menu", text_site=menu["menu_offset"], word_offset=word)


def travel_procedure(graph, offset):
    instructions = graph["cod"]["instructions"]
    entry = next((row for row in instructions if row["offset"] == offset), None)
    require(entry is not None and "ConditionalBlock" in entry["instruction"],
            "travel procedure is not an authored procedure entry")
    rows = [row for row in instructions if row["procedure"] == entry["procedure"]]
    guard = []
    for row in rows[1:]:
        if "GuardPop" in row["instruction"]:
            break
        guard.append(row["instruction"])
    else:
        raise ValueError("travel procedure has no closed entry guard")
    require(any(row.get("FlagBranch", {}).get("opcode") == 0xD0 for row in guard),
            "procedure has no authored travel guard")
    sites = [site for site in graph["cod"]["text_sites"] if site["procedure"] == entry["procedure"]]
    require(sites and sites[0]["record_name"], "travel procedure has no named first text owner")
    return dict(script=graph["profile"].upper(), procedure_offset=offset,
                contact_object=sites[0]["record_name"], texts=[dict(opcode_offset=sites[0]["offset"])])


def plan_topics(graph, contact, travel_setup=None, entry_menu=None):
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
    if entry_menu is not None:
        entries = [node["offset"] for node in nodes.values() if node["menu_offset"] == entry_menu]
        require(len(entries) == 1, "entry menu is not an authored menu in this actor list")
        root = entries[0]
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
            if travel_setup is not None:
                require(travel_setup["procedure_offset"] == contact["procedure_offset"],
                        "travel setup names a different procedure")
                plan["entry"] = "travel"
                plan["travel_setup"] = travel_setup
                plan["max_exit_retries"] = 1
                del plan["contact_procedure"]
            plans.append((current, word["offset"], plan))
    titles = Counter(plan["title"] for _, _, plan in plans)
    words = {(node["menu_offset"], word["offset"]): word["text"].replace("_", " ")
             for node in nodes.values() for word in node["menu_choices"]}
    for current, word, plan in plans:
        if titles[plan["title"]] > 1:
            labels = [words[(step["text_site"], step["word_offset"])] for step in paths[current]]
            labels.append(words[(nodes[current]["menu_offset"], word)])
            plan["title"] = contact["contact_object"].replace("_", " ") + ": " + " > ".join(labels)
    titles = Counter(plan["title"] for _, _, plan in plans)
    for current, word, plan in plans:
        if titles[plan["title"]] > 1:
            plan["title"] += f" [BAS {current:04x}, word {word:04x}]"
    targeted = {site for _, _, plan in plans for site in plan["required_bas_sites"]}
    return plans, dict(scope="finite menu paths and simple topic responses; not all dialogue variants or verified captures",
                       entry_bas_menu=nodes[root]["menu_offset"],
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
    parser.add_argument("--travel-planet", help="authored planet used for prepared travel entry")
    parser.add_argument("--travel-destination", help="authored local navigation destination")
    parser.add_argument("--entry-menu", type=lambda value: int(value, 0),
                        help="authored BAS menu body selected by the COD entry; planning only, not a runtime override")
    args = parser.parse_args()
    require(bool(args.travel_planet) == bool(args.travel_destination),
            "travel entry requires both planet and destination")
    tag = f"SCRIPT{args.profile}"
    index = read_json(args.catalog / "catalog.json")
    profile = next(row for row in index["profiles"] if row["game"] == "cb" and row["profile"].upper() == tag)
    relative = profile["directory"] + "/graph.json"
    graph_path = args.catalog / relative
    require(digest(graph_path) == index["artifacts"][relative], "static graph changed")
    graph = read_json(graph_path)
    travel_setup = None
    manifest_hash = None
    if args.travel_planet:
        contact = travel_procedure(graph, args.procedure)
        travel_setup = dict(planet=args.travel_planet, destination=args.travel_destination,
                            procedure_offset=args.procedure)
    else:
        manifest_path = ROOT / "re/vm/contact-manifest/contact-manifest.json"
        manifest = read_json(manifest_path)
        contacts = [row for row in manifest["procedures"] if row["script"] == tag and row["procedure_offset"] == args.procedure]
        require(len(contacts) == 1, "no unique contact procedure")
        contact = contacts[0]
        manifest_hash = digest(manifest_path)
    plans, report = plan_topics(graph, contact, travel_setup, args.entry_menu)
    require(plans, "no simple BAS topic plans for this contact")
    args.out.mkdir(parents=True, exist_ok=False)
    actor = re.sub(r"[^a-z0-9]+", "-", contact["contact_object"].lower()).strip("-")
    paths = []
    for menu, word, plan in plans:
        filename = f"cb-script{args.profile}-{actor}-{menu:04x}-{word:04x}.plan.json"
        save_json(args.out / filename, plan)
        paths.append(filename)
    save_json(args.out / "planning.json", dict(schema=1, graph_sha256=digest(graph_path),
              contact_manifest_sha256=manifest_hash, travel_setup=travel_setup, plans=paths, **report))
    print(f"Planned {len(plans)} chapters targeting {len(report['targeted_bas_sites'])} BAS sites; "
          f"{len(report['not_targeted_bas_sites'])} actor-list sites remain unplanned.")


if __name__ == "__main__":
    main()
