#!/usr/bin/env python3
# /// script
# requires-python = ">=3.11"
# dependencies = []
# ///
"""Plan isolated native text presentations from the frozen authored inventory.

The initial source-default mode handles plain actor-owned text only. Every other
site remains in the planning ledger with a reason; no story guards are solved.
"""

import argparse
from collections import Counter
from pathlib import Path

from native_sequence_anthology import read_json, require
from video_anthology import digest, save_json

GAMES = {"cb": "commander_blood", "bbb": "big_bug_bang"}
# Built-in records queued by the bridge console's immediate choices; the native
# binder checks the owner against the profile's named built-ins (DS:0x6754).
CONSOLE_RECORDS = {"Honk": "horn", "menu": "radio"}


def candidate(site, descriptions):
    authored = site["authored"]
    # A6 b4&0x40 lines (vm_op_a6_text 0x660C) are gated on the concept history
    # matching section 1; they present section 0 when the player has said it.
    history = (site["content_kind"] == "text_and_choices" and authored["flags_b4"] & 0x56 == 0x40
               and len(authored["sections"]) == 2)
    if site["content_kind"] != "text" and not history:
        return None, site["content_kind"]
    # ScriptTextControl's random/record/resume/history bits. b4&0x08 only arms the
    # rejection skip (vm_skip_count DS:0x67AB), which a presented line discards, so
    # it does not gate display. The native binder independently checks the typed
    # source instruction rather than trusting this classification.
    if authored["flags_b4"] & (0x16 if history else 0x56):
        return None, "conditional_or_continuation_control"
    if not authored["flags_b5"] & 0x80:
        return None, "inactive_source_request"
    selector = authored["presentation_selector"]
    # The graph retains the serialized byte; the native decoder reads signed i8.
    if selector != 255 and not -1 <= selector <= 31:
        return None, "non_character_presentation_selector"
    if len(authored["sections"]) != (2 if history else 1) or any(
            word["kind"] != "dictionary" for section in authored["sections"] for word in section):
        return None, "non_plain_word_list"
    console = CONSOLE_RECORDS.get(authored.get("record_name"))
    matches = [descriptions[identity] for identity in site["direct_description_candidates"]]
    if console:
        record = dict(name=authored["record_name"])
    elif not matches or matches[0]["authored"]["kind"] != "Character":
        return None, "no_direct_character_description"
    else:
        record = matches[0]["authored"]
        if record["name_bytes"] != list(record["name"].encode("ascii", errors="replace")):
            return None, "non_ascii_description_name"
    hashes = site["source_hashes"]
    profile = site["profile"]
    require(profile.startswith("SCRIPT") and profile[6:].isdecimal(), "invalid script profile")
    plan = dict(schema=1, game=GAMES[site["game"]],
                title=f"{site['game'].upper()} {profile} {site['kind'].upper()} {site['offset']:04x}: "
                      f"{record['name']} [prepared source-default]",
                initial_profile=int(profile[6:]) - 1,
                cod_sha256=hashes["cod_sha256"], dic_sha256=hashes["dic_sha256"],
                source=site["kind"], text_site=site["offset"],
                context=dict(kind="bridge_console", actor_offset=authored["record_offset"], choice=console)
                if console else dict(kind="source_default", actor_offset=authored["record_offset"],
                                     descript_records=[record["name"]]))
    if console:
        plan["title"] = plan["title"].replace("[prepared source-default]", "[prepared bridge console]")
    if history:
        # Prepared state: exactly the authored candidates, oldest first.
        plan["context"]["history_concepts"] = [word["offset"] for word in authored["sections"][1]]
        plan["title"] = plan["title"].replace("[prepared source-default]", "[prepared concept history]")
    if site["kind"] == "bas":
        require(hashes.get("bas_sha256"), "BAS candidate is missing its source hash")
        plan["bas_sha256"] = hashes["bas_sha256"]
    return plan, None


def build(inventory_path, output, game=None, selected_sites=()):
    require(not output.exists(), f"output directory already exists: {output}")
    inventory_hash = digest(inventory_path)
    inventory = read_json(inventory_path)
    require(inventory["schema"] == 1 and inventory["static_inventory_complete"],
            "a complete static inventory is required")
    descriptions = {record["id"]: record for record in inventory["descriptions"]}
    sites = [site for site in inventory["sites"] if game is None or site["game"] == game]
    selected = set(selected_sites)
    require(selected <= {site["id"] for site in sites}, "selected site is absent from the chosen inventory/game")
    plans, ledger = [], []
    for site in sites:
        plan, reason = candidate(site, descriptions)
        status = "deferred" if reason else "eligible_not_selected" if selected and site["id"] not in selected else "planned"
        entry = dict(id=site["id"], status=status, reason=reason)
        if status == "planned":
            filename = site["id"] + ".plan.json"
            plans.append((filename, plan))
            entry["plan"] = filename
        ledger.append(entry)
    require(digest(inventory_path) == inventory_hash, "inventory changed during planning")
    output.mkdir(parents=True)
    for filename, plan in plans:
        save_json(output / filename, plan)
    report = dict(schema=1, mode="static_text", scope="isolated source-default native presentation candidates; "
                  "not route reachability or canonical scene context", inventory=str(inventory_path.resolve()),
                  inventory_sha256=inventory_hash, planner_sha256=digest(__file__),
                  context_policy="native source defaults plus explicit actor description; "
                  "no invented location or story state; exact preparation is reported by the renderer",
                  plans=[filename for filename, _ in plans],
                  plan_sha256={filename: digest(output / filename) for filename, _ in plans},
                  counts=dict(sorted(Counter(entry["status"] for entry in ledger).items())),
                  deferred_reasons=dict(sorted(Counter(entry["reason"] for entry in ledger if entry["reason"]).items())),
                  sites=ledger, complete_render=False, full_game_complete=False)
    save_json(output / "planning.json", report)
    print(f"{len(plans)} prepared text plans; {report['counts']}; deferred: {report['deferred_reasons']}")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--inventory", type=Path, required=True)
    parser.add_argument("--out", type=Path, required=True)
    parser.add_argument("--game", choices=tuple(GAMES))
    parser.add_argument("--site", action="append", default=[])
    args = parser.parse_args()
    build(args.inventory, args.out, args.game, args.site)


if __name__ == "__main__":
    main()
