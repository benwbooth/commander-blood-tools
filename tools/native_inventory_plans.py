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


def plan_inventory(graph, template, menu_offset, labels):
    require(graph["game"] == "bbb" and template["game"] == "big_bug_bang"
            and graph["profile"].lower() == f"script{template['initial_profile'] + 1}",
            "inventory template and graph belong to different profiles")
    require(all(template[key] == graph["resources"][key] for key in ("cod_sha256", "dic_sha256")),
            "inventory template source hash differs")
    require(template.get("entry") == "travel" and not template["choices"]
            and not template.get("max_exit_retries")
            and not template["travel_setup"].get("stage_aboard_inventory"),
            "inventory planning requires an unstocked travel template without choices")
    sites = graph["cod"]["text_sites"]
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
        plan["choices"] = [dict(source="inventory", text_site=menu_offset, inventory_item=item)]
        required = {menu_offset, *(site["offset"] for site in reactions)}
        for key in ("required_cod_sites", "required_frame_boundary_cod_sites"):
            plan[key] = sorted(set(plan[key]) | required)
        plans.append((item, plan))
    require(len({item for item, _ in plans}) == len(plans), "ambiguous multiple item reactions")
    return plans, dict(scope="static flat item-reaction candidates; not all branches or verified captures",
        inventory_menu=menu_offset, reaction_procedure=rows[0]["offset"],
        deferred=deferred, items_without_simple_flag_guard=sorted(set(items) - considered))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--catalog", type=Path, required=True)
    parser.add_argument("--template", type=Path, required=True)
    parser.add_argument("--inventory-menu", type=int, required=True)
    parser.add_argument("--out", type=Path, required=True)
    args = parser.parse_args()
    template = read_json(args.template)
    index = read_json(args.catalog / "catalog.json")
    tag = f"script{template['initial_profile'] + 1}"
    profiles = [row for row in index["profiles"] if row["game"] == "bbb" and row["profile"].lower() == tag]
    require(len(profiles) == 1, "no unique BBB profile in catalog")
    relative = profiles[0]["directory"] + "/graph.json"
    graph_path = args.catalog / relative
    require(digest(graph_path) == index["artifacts"][relative], "static graph changed")
    labels_path = ROOT / "localization/big-bug-bang/en/inventory.json"
    labels = {bytes(row["source"]).decode("cp437"): row["english"] for row in read_json(labels_path)["entries"]}
    plans, report = plan_inventory(read_json(graph_path), template, args.inventory_menu, labels)
    require(plans, "no simple item reactions for this template")
    args.out.mkdir(parents=True, exist_ok=False)
    actor = re.sub(r"[^a-z0-9]+", "-", template["target"].lower()).strip("-")
    paths = []
    for item, plan in plans:
        filename = f"bbb-{tag}-{actor}-give-{item:04x}.plan.json"
        save_json(args.out / filename, plan)
        paths.append(filename)
    save_json(args.out / "planning.json", dict(schema=1, graph_sha256=digest(graph_path),
        template_sha256=digest(args.template), inventory_labels_sha256=digest(labels_path), plans=paths, **report))
    print(f"Planned {len(plans)} inventory chapters; {len(report['deferred'])} guarded branches deferred.")


if __name__ == "__main__":
    main()
