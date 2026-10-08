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
GAME_KEYS = {"cb": "cb", "bbb": "bbb"}
# Built-in records queued by the bridge console's immediate choices; the native
# binder checks the owner against the profile's named built-ins (DS:0x6754).
CONSOLE_RECORDS = {"Honk": "horn", "menu": "radio"}



def placement_records(site, locations, placements, location_names):
    """DESCRIPT Location records (planet, place) where the actor stands.

    CB: the source-default HOLDER_OR_LOCATION chain actor -> place -> planet (the field
    native travel staging reads). BBB actors all start in Trashlando and are moved at
    run time (D5 settlement), so only placements taken from verified native chapters apply.
    """
    actor = site["authored"].get("record_name")
    chain = []
    if site["game"] == "bbb":
        chain = list(placements.get(actor, ())) if placements else []
        if len(chain) == 2 and chain[0] == chain[1]:
            chain = chain[:1]
    elif locations:
        rows = {row["name"]: row for row in locations[site["game"]][site["profile"]]}
        row = rows.get(actor)
        place = row["holder"] if row and row["kind"] == "Actor" else None
        while place in rows and len(chain) < 3:
            chain.insert(0, place)
            if rows[place]["kind"] == "CelestialBody":
                break
            place = rows[place]["holder"]
    return [name for name in chain if (site["game"], name) in location_names] if chain else []


def candidate(site, descriptions, placement=()):
    authored = site["authored"]
    flags = authored["flags_b4"]
    # vm_op_a6_text (0x660C): b4&0x40 gates the line on the concept history matching
    # the next section; b4&0x10 publishes the following section as reply choices.
    # Section 0 is always the displayed text.
    history, resume = bool(flags & 0x40), bool(flags & 0x10)
    sections = 1 + history + resume
    if site["content_kind"] not in ("text", "text_and_choices", "symbolic_text") or (
            site["content_kind"] == "text_and_choices" and sections == 1):
        return None, site["content_kind"]
    # Record-field conditions stay deferred. The b4&0x02 random gate is prepared by
    # the exporter discarding PRNG draws until rand(5) == 0 (reported). b4&0x08 only
    # arms the rejection skip (vm_skip_count DS:0x67AB), which a presented line
    # discards. The native binder independently checks the typed instruction.
    if flags & 0x04:
        return None, "conditional_or_continuation_control"
    if site["content_kind"] in ("text", "symbolic_text") and (history or resume):
        return None, "non_plain_word_list"
    if not authored["flags_b5"] & 0x80:
        return None, "inactive_source_request"
    selector = authored["presentation_selector"]
    # The graph retains the serialized byte; the native decoder reads signed i8.
    if selector != 255 and not -1 <= selector <= 31:
        return None, "non_character_presentation_selector"
    # state_number words read the source-default VAR (the verifier checks they display
    # as numbers); generated inventory choices stay deferred.
    if len(authored["sections"]) != sections or any(
            word["kind"] not in ("dictionary", "state_number")
            for section in authored["sections"] for word in section):
        return None, "non_plain_word_list"
    console = CONSOLE_RECORDS.get(authored.get("record_name"))
    if console is None and not site["direct_description_candidates"]:
        # A contact transition resolves the owner's DESCRIPT record; an owner with
        # none can only reach the bridge as an answered radio call
        # (nav_actor_handler_4 0x81FB). The binder rejects non-actor owners.
        console = "radio_call"
    if resume and console is None:
        # The reply rows need the word-choice interface, which only advances on
        # presented frames; a contact scene held at its deferred-actor boundary
        # keeps the C2 presentation gate set, so it reports none.
        return None, "reply_choice_menu"
    matches = [descriptions[identity] for identity in site["direct_description_candidates"]]
    if console:
        record = dict(name=authored.get("record_name", ""))
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
                                     descript_records=[*placement, record["name"]]))
    if placement and not console:
        plan["title"] = plan["title"].replace("[prepared source-default", f"[prepared at {'/'.join(placement)}")
    if console:
        plan["title"] = plan["title"].replace(
            "[prepared source-default]",
            "[prepared radio call]" if console == "radio_call" else "[prepared bridge console]")
    if history:
        # Prepared state: exactly the authored candidates, oldest first.
        candidates = [word["offset"] for word in authored["sections"][1]]
        required = authored["flags_b5"] & 0x07  # detail & 7 required matches
        plan["context"]["history_concepts"] = candidates * max(1, -(-required // len(candidates)))
        plan["title"] = (plan["title"][:-1] + ", concept history]" if console
                         else plan["title"].replace("[prepared source-default]", "[prepared concept history]"))
    if site["kind"] == "bas":
        require(hashes.get("bas_sha256"), "BAS candidate is missing its source hash")
        plan["bas_sha256"] = hashes["bas_sha256"]
    if flags & 0x02:
        plan["title"] = plan["title"][:-1] + ", random draw]"
    if site["content_kind"] == "symbolic_text":
        plan["title"] = plan["title"][:-1] + ", source-default numbers]"
    return plan, None


def build(inventory_path, output, game=None, selected_sites=(), locations_path=None, placements_path=None):
    require(not output.exists(), f"output directory already exists: {output}")
    inventory_hash = digest(inventory_path)
    inventory = read_json(inventory_path)
    require(inventory["schema"] == 1 and inventory["static_inventory_complete"],
            "a complete static inventory is required")
    descriptions = {record["id"]: record for record in inventory["descriptions"]}
    sites = [site for site in inventory["sites"] if game is None or site["game"] == game]
    selected = set(selected_sites)
    require(selected <= {site["id"] for site in sites}, "selected site is absent from the chosen inventory/game")
    locations = read_json(locations_path) if locations_path else None
    placements = read_json(placements_path)["placements"] if placements_path else None
    location_names = {(GAME_KEYS[record["game"]] if record["game"] in GAME_KEYS else record["game"], record["authored"]["name"])
                      for record in inventory["descriptions"] if record["authored"]["kind"] == "Location"}
    plans, ledger = [], []
    for site in sites:
        plan, reason = candidate(site, descriptions, placement_records(site, locations, placements, location_names))
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
    parser.add_argument("--locations", type=Path, help="source-default holder chains (dump_source_default_object_locations)")
    parser.add_argument("--placements", type=Path, help="BBB actor placements taken from verified native chapters")
    args = parser.parse_args()
    build(args.inventory, args.out, args.game, args.site, args.locations, args.placements)


if __name__ == "__main__":
    main()
