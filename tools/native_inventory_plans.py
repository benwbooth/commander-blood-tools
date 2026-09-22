#!/usr/bin/env python3
"""Plan BBB item-giving chapters from source-bound inventory reaction guards.

Plans are candidates, not coverage. Nested/state-dependent reactions remain
listed for separate preparation and native verification.
"""

import argparse
import copy
from pathlib import Path
import re

from native_sequence_anthology import read_json, require
from video_anthology import ROOT, digest, save_json


def travel_template(graph, menu_offset, planet, destination):
    """Prepare only the simple outer gift guard; native capture validates all bindings."""
    profile = re.fullmatch(r"SCRIPT([1-9]|1[0-7])", graph["profile"].upper())
    require(graph["game"] == "bbb" and profile, "not a BBB profile")
    menu = next((site for site in graph["cod"]["text_sites"] if site["offset"] == menu_offset), None)
    require(menu and menu["record_name"]
            and menu["choice_operands"] == [dict(kind="inventory_choices")],
            "not a named actor inventory menu")
    symbols = {row["name"]: row["offset"] for row in graph["symbols"] if row["kind"] == 1}
    require(planet in symbols and destination in symbols, "unknown prepared travel destination")
    require(symbols.get(menu["record_name"]) == menu["record_offset"] and "blood" in symbols,
            "inventory actor or player is not bound to the symbol table")
    rows = [row for row in graph["cod"]["instructions"] if row["procedure"] == menu["procedure"]]
    require(rows and "ConditionalBlock" in rows[0]["instruction"], "inventory procedure has no entry")
    guard = []
    for row in rows[1:]:
        if "GuardPop" in row["instruction"]:
            break
        guard.append(row["instruction"])
    else:
        raise ValueError("inventory procedure has no closed entry guard")
    activities = [row["FlagBranch"] for row in guard if "FlagBranch" in row]
    actors = [row["Actor"] for row in guard if "Actor" in row]
    require(len(guard) == 2 and len(activities) == len(actors) == 1
            and activities[0]["opcode"] == 0xD0 and not actors[0]["inverted"]
            # ACTION is byte 58 of the native actor record (vm.rs FIELD_OFFSETS).
            and actors[0]["record_offset"] == menu["record_offset"] + 58
            and actors[0]["related_record_offset"] == symbols["blood"],
            "inventory entry needs additional preparation or selects another actor")
    return dict(schema=1, game="big_bug_bang", title=menu["record_name"] + ": prepared gift visit",
        initial_profile=int(profile[1]) - 1, cod_sha256=graph["resources"]["cod_sha256"],
        dic_sha256=graph["resources"]["dic_sha256"], target=menu["record_name"], entry="travel",
        travel_setup=dict(planet=planet, destination=destination,
                          procedure_offset=rows[0]["offset"], stage_actor_at_destination=True),
        choices=[], required_cod_sites=[], required_frame_boundary_cod_sites=[],
        end=dict(kind="presentation_finished"))


def plan_inventory(graph, template, menu_offset, labels, excluded_items=()):
    require(graph["game"] == "bbb" and template["game"] == "big_bug_bang"
            and graph["profile"].lower() == f"script{template['initial_profile'] + 1}",
            "inventory template and graph belong to different profiles")
    require(all(template[key] == graph["resources"][key] for key in ("cod_sha256", "dic_sha256")),
            "inventory template source hash differs")
    require(template.get("entry") == "travel"
            and not template.get("max_exit_retries")
            and not template["travel_setup"].get("stage_aboard_inventory"),
            "inventory planning requires an unstocked travel template without exit retries")
    sites = graph["cod"]["text_sites"]
    for choice in template["choices"]:
        site = next((site for site in sites if site["offset"] == choice["text_site"]), None)
        require(choice.get("source", "cod") == "cod" and "inventory_item" not in choice
                and site and site["record_name"] == template["target"]
                and any(word["kind"] == "dictionary" and word["offset"] == choice.get("word_offset")
                        for word in site["choice_operands"]),
                "inventory prerequisite must name this actor's authored COD choice")
    menu = next((site for site in sites if site["offset"] == menu_offset), None)
    require(menu and menu["choice_operands"] == [dict(kind="inventory_choices")]
            and menu["record_name"] == template["target"], "not this actor's inventory menu")
    instructions = graph["cod"]["instructions"]
    rows = [row for row in instructions if row["procedure"] == menu["procedure"]]
    require(rows and "ConditionalBlock" in rows[0]["instruction"]
            and rows[0]["offset"] in [template["travel_setup"]["procedure_offset"],
                *template["travel_setup"].get("supporting_procedures", [])],
            "inventory procedure is not enabled by the template")
    symbols = {row["offset"]: row["name"] for row in graph["symbols"] if row["kind"] == 1}
    items = {offset: name for offset, name in symbols.items() if name in labels}
    guards = [(row["offset"], row["instruction"]["GuardPush"]["target"])
              for row in rows if "GuardPush" in row["instruction"]]
    plans, deferred, considered = [], [], set()
    for index, row in enumerate(rows):
        test = row["instruction"].get("SharedBitState", {})
        item = test.get("field_offset", -1) - 2
        if (test.get("opcode") != 0xAE or test.get("mask") != 0x40
                or test.get("inverted") or item not in items):
            continue
        # A single item flag must be the complete guard, not an assignment or
        # one predicate borrowed from a larger condition.
        previous = rows[index - 1]["instruction"] if index else {}
        following = rows[index + 1]["instruction"] if index + 1 < len(rows) else {}
        if "GuardPush" not in previous or "GuardPop" not in following:
            deferred.append(dict(item=item, guard=row["offset"], reason="compound item guard"))
            continue
        start, end = rows[index - 1]["offset"], previous["GuardPush"]["target"]
        body = [entry for entry in rows[index + 2:] if entry["offset"] < end]
        reactions = [site for site in sites if rows[index + 1]["offset"] < site["offset"] < end
                     and site["procedure"] == menu["procedure"]]
        considered.add(item)
        nested = any("GuardPush" in entry["instruction"] for entry in body)
        enclosed = any(begin < start < finish for begin, finish in guards)
        clears = any(entry["instruction"].get("SharedBitState", {}).get("field_offset") == item + 2
                     and entry["instruction"]["SharedBitState"].get("mask") == 0x40
                     and entry["instruction"]["SharedBitState"].get("inverted") for entry in body)
        simple = all(not site["flags_b4"] & 0x56 and not site["choice_operands"]
                     and not site["dynamic"] and site["record_name"] == template["target"]
                     for site in reactions)
        if nested or enclosed or not clears or not reactions or not simple:
            deferred.append(dict(item=item, guard=row["offset"], sites=[site["offset"] for site in reactions],
                reason="nested, state-dependent, dynamic, or unconsumed item reaction"))
            continue
        plan = copy.deepcopy(template)
        plan["title"] = template["target"].replace("_", " ") + ": give " + labels[items[item]]
        plan["travel_setup"]["stage_aboard_inventory"] = [item]
        plan["choices"].append(dict(source="inventory", text_site=menu_offset, inventory_item=item))
        # AF writes an inventory holder at byte 20 back to the aboard sentinel.
        # Its native chooser reopens with the returned/new item still available.
        if any(entry["instruction"].get("RecordWildcard", {}).get("opcode") == 0xAF
               and entry["instruction"]["RecordWildcard"].get("record_offset") in
                   {offset + 20 for offset in items}
               and entry["instruction"]["RecordWildcard"].get("value") == 0xFFFF
               and not entry["instruction"]["RecordWildcard"].get("inverted") for entry in body):
            plan["choices"].append(dict(source="inventory_cancel", text_site=menu_offset))
        required = {menu_offset, *(site["offset"] for site in reactions)}
        for key in ("required_cod_sites", "required_frame_boundary_cod_sites"):
            plan[key] = sorted(set(plan[key]) | required)
        plans.append((item, plan))
    require(len({item for item, _ in plans}) == len(plans), "ambiguous multiple item reactions")
    excluded = set(excluded_items)
    require(excluded <= {item for item, _ in plans}, "excluded item is not a flat candidate")
    plans = [(item, plan) for item, plan in plans if item not in excluded]
    return plans, dict(scope="static flat item-reaction candidates; not all branches or verified captures",
        inventory_menu=menu_offset, reaction_procedure=rows[0]["offset"],
        deferred=deferred, items_without_simple_flag_guard=sorted(set(items) - considered),
        **(dict(excluded_items=sorted(excluded)) if excluded else {}))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--catalog", type=Path, required=True)
    source = parser.add_mutually_exclusive_group(required=True)
    source.add_argument("--template", type=Path)
    source.add_argument("--profile", help="BBB SCRIPTn; derive a simple gift-entry template from the graph")
    parser.add_argument("--planet", help="explicit prepared planet; only with --profile")
    parser.add_argument("--destination", help="explicit prepared location; only with --profile")
    parser.add_argument("--inventory-menu", type=int, required=True)
    parser.add_argument("--exclude-item", type=int, action="append", default=[],
                        help="exclude a flat candidate by VAR offset, retaining the exclusion in the report")
    parser.add_argument("--out", type=Path, required=True)
    args = parser.parse_args()
    if args.template:
        require(args.planet is None and args.destination is None,
                "travel destination is owned by the supplied template")
        template = read_json(args.template)
        tag = f"script{template['initial_profile'] + 1}"
    else:
        require(args.planet and args.destination, "graph-derived entry requires --planet and --destination")
        tag = args.profile.lower()
    index = read_json(args.catalog / "catalog.json")
    profiles = [row for row in index["profiles"] if row["game"] == "bbb" and row["profile"].lower() == tag]
    require(len(profiles) == 1, "no unique BBB profile in catalog")
    relative = profiles[0]["directory"] + "/graph.json"
    graph_path = args.catalog / relative
    require(digest(graph_path) == index["artifacts"][relative], "static graph changed")
    graph = read_json(graph_path)
    if not args.template:
        template = travel_template(graph, args.inventory_menu, args.planet, args.destination)
    labels_path = ROOT / "localization/big-bug-bang/en/inventory.json"
    labels = {bytes(row["source"]).decode("cp437"): row["english"] for row in read_json(labels_path)["entries"]}
    plans, report = plan_inventory(graph, template, args.inventory_menu, labels, args.exclude_item)
    require(plans, "no simple item reactions for this template")
    args.out.mkdir(parents=True, exist_ok=False)
    actor = re.sub(r"[^a-z0-9]+", "-", template["target"].lower()).strip("-")
    paths = []
    for item, plan in plans:
        filename = f"bbb-{tag}-{actor}-give-{item:04x}.plan.json"
        save_json(args.out / filename, plan)
        paths.append(filename)
    template_source = (dict(template_sha256=digest(args.template)) if args.template else
        dict(template_source="simple_authored_travel_gift_guard", derived_template=template))
    save_json(args.out / "planning.json", dict(schema=1, graph_sha256=digest(graph_path),
        **template_source, inventory_labels_sha256=digest(labels_path), plans=paths, **report))
    print(f"Planned {len(plans)} inventory chapters; {len(report['deferred'])} guarded branches deferred.")


if __name__ == "__main__":
    main()
