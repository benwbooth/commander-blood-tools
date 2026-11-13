#!/usr/bin/env python3
# /// script
# requires-python = ">=3.11"
# dependencies = []
# ///
"""Plan where and how the game itself plays every HNM video and every text line.

Every text line and clip is rendered in the scene the engine shows it in, taken from
the script proc that speaks it (its `during` guard) and from the code path that
presents the video, never from a borrowed scene:

* `during contact` (D1, scene gate DS:0x274F): the contact scene transition. Its image
  is the fixed FRIGO.FD (DS:0x00F3), the ship's freezer; SQ/CRYOGEL opens it (line 41,
  scene row 0), the actor enters with its right clip (line 39, which dims scene colours
  128..191 by 40), leaves with its left clip (line 40), and SQ/CRYORAD closes it
  (line 42, row 0). Navigation music is retained.
* `during bridge` (CE): the bridge itself; the actor's presentation is deferred to C4
  the way an answered radio call is (nav_actor_handler_4 0x81FB), no transition image.
* `during travel` (D0): the commander's travel to the actor's place
  (begin_chapter_travel: planet then destination DESCRIPT records with their
  backgrounds and navigation music, the place's PL film on line 3); the actor
  presents over the place's background.

Videos follow the code path that presents them (presentation_catalog line map):
DESCRIPT Sequence records play in the sequence panel (line 2); Location films on
line 3 when the commander arrives; character idle/talk clips (lines 8..38) in the
scenes their actor speaks in; right/left clips (39/40) in the contact transition;
object clips (line 43) where a script moves the object aboard (C2); script
`request sequence` films (A8, line 7) in the requesting proc's scene; fixed
executable films (asset path table BLOODPRG 0x0F48B): mind (line 0, opening reel),
the_star (line 1, credits), ejectorx/ejection (lines 4/5, ship presentation cycle),
hyper_00..07 (line 6, hyperspace), cryogel/cryorad (lines 41/42), pollup (line 44,
black-hole travel). A video no record, script or executable path of this game names
is never played by it and is planned as a plain decode.
"""

import argparse
from collections import Counter, defaultdict
from pathlib import Path
import re

from native_sequence_anthology import read_json, require
from play_order import parse_profile
from video_anthology import digest, resource_name, save_json

GAMES = {"cb": "commander_blood", "bbb": "big_bug_bang"}
FIXED_FILMS = {  # name -> (line, trigger, scene)
    "SQ/MIND.HNM": (0, "opening logo reel run by the lifecycle before the first frame (OPENING line 0)", "opening"),
    "SQ/THE_STAR.HNM": (1, "credits presentation (line 1)", "credits"),
    "SQ/EJECTORX.HNM": (4, "ship presentation dialogue cycle, first line (ship_presentation DIALOGUE_FIRST_LINE 4)", "bridge"),
    "SQ/EJECTION.HNM": (5, "ship presentation dialogue cycle, second line (lines 4..5)", "bridge"),
    "SQ/CRYOGEL.HNM": (41, "contact transition description lookup (scene_transition DESCRIPTION_LOOKUP_LINE 41), before FRIGO.FD loads", "contact"),
    "SQ/CRYORAD.HNM": (42, "contact transition completion (scene_transition COMPLETE_LINE 42), at cleanup", "contact"),
    "SQ/POLLUP.HNM": (44, "black-hole travel between its camera and relation phases (script_action TravelReady line 44)", "bridge"),
}
HYPERSPACE = re.compile(r"SQ/HYPER_0[0-7]\.HNM")
ROW_ZERO_LINES = {41, 42}
SHIP_CYCLE_NOTE = ("played by the ship presentation during travel (cycle lines 4..6, hyperjump, TravelReady 44); "
                   "the static exporter cannot yet establish that ship state, so the film is decoded full screen "
                   "with no scene rather than shown in a wrong one")  # presented while the transition image row is 0


def chain_of(actor, rows):
    """Planet .. place holder chain of an actor in one profile's source-default VAR."""
    chain, place = [], rows.get(actor, {}).get("holder")
    while place in rows and len(chain) < 4 and place not in chain:
        chain.insert(0, place)
        if rows[place]["kind"] == "CelestialBody":
            break
        place = rows[place]["holder"]
    return [p for p in chain if rows[p]["kind"] in ("CelestialBody", "Location")]


def scene_context(plan, scene, rows, places, located=frozenset()):
    """Rewrite a prepared plan's context for the scene the line is shown in."""
    ctx = plan["context"]
    actor_offset, history = ctx["actor_offset"], ctx.get("history_concepts", [])
    extra = {"history_concepts": history} if history else {}
    if ctx["kind"] == "bridge_console":
        return dict(ctx), "bridge console (built-in record or answered radio call)"
    records = ctx.get("descript_records", [])
    character, placement = records[-1:], [r for r in records[:-1] if r in places]
    during = scene.get("during")
    if during == "bridge":
        return dict(kind="bridge_console", actor_offset=actor_offset, choice="radio_call", **extra), \
            "during bridge: actor presented on the bridge (deferred C4 presentation, no transition)"
    if during == "travel":
        actor = character[0] if character else None
        chain = placement or (chain_of(actor, rows) if actor else [])
        if not chain and scene.get("place"):
            chain = [p for p in reversed(scene["place"]) if p in places]
        if chain and not set(chain) <= located:
            return dict(kind="source_default", actor_offset=actor_offset, descript_records=character, **extra), \
                f"during travel at {'/'.join(chain)}, which has no DESCRIPT Location record to travel to " \
                "(BBB actors leave it by runtime D5 settlement): contact scene"
        if chain:
            return dict(kind="travel", actor_offset=actor_offset, planet=chain[0], destination=chain[-1],
                        descript_records=character, **extra), \
                f"during travel: native travel to {'/'.join(chain)}, actor over the place's background"
        return dict(kind="source_default", actor_offset=actor_offset, descript_records=character, **extra), \
            "during travel but the actor has no place in this profile: contact scene"
    return dict(kind="source_default", actor_offset=actor_offset, descript_records=character, **extra), \
        "during contact: contact transition (FRIGO.FD freezer)" if during == "contact" else \
        "no during guard (reply menu or unguarded proc): contact transition (FRIGO.FD freezer)"


def proc_facts(dialogue, game):
    """Per profile: procs with their during guard, speakers, presentations, requests and aboard moves."""
    facts = []
    for path in sorted((dialogue / game).glob("script*/source.blood")):
        profile = path.parent.name.upper()
        _, nodes = parse_profile(path)

        def walk(body):
            for node in body:
                if node.kind == "stmt":
                    yield node
                for attr in ("body", "other", "cases", "guards"):
                    yield from walk(getattr(node, attr, []) or [])

        for proc in (n for n in nodes if n.kind == "proc"):
            during = next((g.text.split()[1] for g in proc.guards if g.text.startswith("during ")), None)
            stmts = list(walk(proc.body))
            speakers = [m.group(1) for s in stmts if (m := re.match(r"say (\w+)", s.text))]
            facts.append(dict(
                profile=profile, proc=proc.name, line=proc.line + 1, during=during, speakers=speakers,
                presentations=[(m.group(1), m.group(2)) for s in stmts
                               if (m := re.match(r"say (\w+) presentation=(\S+\.hnm)", s.text, re.I))],
                requests=[(m.group(1), s.line + 1) for s in stmts
                          if (m := re.search(r'request sequence "([^"]+)"', s.text))],
                aboard=[(m.group(1) or m.group(2), s.line + 1) for s in stmts
                        if (m := re.match(r"(?:(\w+)\.current_location = aboard|transfer (\w+) from \w+ to aboard)$",
                                          s.text))]))
    return facts


def build(args):
    game = args.game
    out = args.out
    require(not (out / f"{game}.json").exists(), f"plan already exists: {out / (game + '.json')}")
    catalog = read_json(args.catalog)
    inventory = read_json(args.inventory)
    ledger = read_json(args.ledger)
    order = read_json(args.play_order)["order"]
    locations = read_json(args.locations)[game]
    frames = {resource_name(v["source_resource_name"]): v["frame_count"]
              for v in read_json(args.video_manifest)["videos"]}
    places = {r["name"] for rows in locations.values() for r in rows if r["kind"] in ("CelestialBody", "Location")}
    rows_of = {profile: {r["name"]: r for r in rows} for profile, rows in locations.items()}
    scene_of = {e["id"]: e["scene"] for e in order if e["kind"] == "site"}
    actor_of = {e["id"]: e.get("actor") for e in order if e["kind"] == "site"}
    rank = {e["id"]: i for i, e in enumerate(order) if e["kind"] == "site"}
    descriptions = {r["authored"]["name"]: r["authored"] for r in inventory["descriptions"] if r["game"] == game}
    located = {name for name, record in descriptions.items() if record["kind"] == "Location"}

    # ---- text lines: every verified prepared render, moved into its real scene
    text_dir = out / f"text-plans-{game}"
    text_dir.mkdir(parents=True)  # refuses to overwrite an earlier plan set
    sites, plan_hash, scenes, site_plans = [], {}, Counter(), {}
    for row in ledger["sites"]:
        if not row["id"].startswith(game + ".") or row["status"] != "verified_prepared_scene":
            continue
        plan = read_json(Path(row["detail"]["path"]) / "report.json")["runner"]["plan"]
        profile = f"SCRIPT{plan['initial_profile'] + 1}"
        scene = scene_of.get(row["id"], {"during": None})
        context, why = scene_context(plan, scene, rows_of.get(profile, {}), places, located)
        plan = dict(plan, context=context)
        plan["title"] = re.sub(r"\[prepared [^\]]*\]", f"[{context['kind'].replace('_', ' ')} scene]", plan["title"])
        name = row["id"] + ".plan.json"
        save_json(text_dir / name, plan)
        plan_hash[name] = digest(text_dir / name)
        site_plans[row["id"]] = (plan, scene, why)
        scenes[context["kind"]] += 1
        sites.append(dict(id=row["id"], status="planned", reason=why, plan=name))
    planning = read_json(args.text_planning)
    save_json(text_dir / "planning.json", dict(
        schema=1, mode="static_text", scope="every verified text line in the scene its proc runs in",
        inventory=planning["inventory"], inventory_sha256=planning["inventory_sha256"],
        planner_sha256=digest(__file__), plans=[s["plan"] for s in sites], plan_sha256=plan_hash,
        counts=dict(scenes), sites=sites, complete_render=False, full_game_complete=False))

    # ---- which proc scene shows each video
    facts = proc_facts(args.dialogue, game)

    def base_for(profile, actor, during):
        """The earliest verified line of `actor` in `profile` shown in the `during` scene."""
        best = None
        for ident, (plan, scene, _) in site_plans.items():
            if f"script{plan['initial_profile'] + 1}." not in ident or actor_of.get(ident) != actor:
                continue
            if during and scene.get("during") != during:
                continue
            if best is None or rank.get(ident, 1 << 30) < rank.get(best, 1 << 30):
                best = ident
        return best

    def any_base(profile, during):
        for ident in sorted(site_plans, key=lambda i: rank.get(i, 1 << 30)):
            plan, scene, _ = site_plans[ident]
            if ident.split(".")[1] == profile.lower() and scene.get("during") == during:
                return ident
        return None

    record_videos = defaultdict(list)  # name -> [(record, kind, command kind, index)]
    for record in descriptions.values():
        talk = 0
        for command in record["commands"]:
            directory = {"sequence_video": "SQ", "location_video": "PL", "object_video": "OB", "talk_clip": "PE",
                         "idle_clip": "PE", "character_right_video": "PE", "character_left_video": "PE"}.get(command["kind"])
            if directory:
                name = resource_name(f"{directory}/{command.get('normalized_name') or command['name']}")
                line = {"idle_clip": 8, "character_right_video": 39, "character_left_video": 40,
                        "location_video": 3, "object_video": 43, "sequence_video": 2}.get(command["kind"])
                if command["kind"] == "talk_clip":
                    talk += 1
                    line = 8 + talk
                record_videos[name].append((record["name"], record["kind"], command["kind"], line))
    presented_by = defaultdict(list)  # PE name -> facts
    requested_by = defaultdict(list)
    aboard_by = defaultdict(list)
    for fact in facts:
        for actor, clip in fact["presentations"]:
            presented_by[resource_name("PE/" + clip)].append((fact, actor))
        for clip, line in fact["requests"]:
            requested_by[resource_name("SQ/" + clip)].append((fact, line))
        for obj, line in fact["aboard"]:
            aboard_by[obj].append((fact, line))

    clip_dir = out / f"clip-plans-{game}"
    clip_dir.mkdir()
    videos, clips = [], []

    def add_clip(entry, base_id, line, extra, scene_note):
        plan, scene, why = site_plans[base_id]
        plan = dict(plan, clip_line=line, clip_frames=frames[entry["name"]], **extra)
        plan["title"] = f"{plan['title']} clip {entry['name']} line {line}"
        name = entry["name"].replace("/", "_") + ".plan.json"
        save_json(clip_dir / name, plan)
        clips.append(dict(clip=entry["name"], actor=actor_of.get(base_id) or "", line=line,
                          frames=frames[entry["name"]], plan=name, base_plan=base_id,
                          plan_sha256=digest(clip_dir / name)))
        entry["render"] = dict(method="engine_clip", plan=f"clip-plans-{game}/{name}", base_line=base_id,
                               line=line, context=plan["context"])
        entry["scene"] = dict(entry.get("scene", {}), context=plan["context"]["kind"], note=scene_note or why)

    for video in sorted(catalog["videos"], key=lambda v: v["name"]):
        name = video["name"]
        entry = dict(name=name, frames=frames[name], references=record_videos.get(name, []))
        uses = record_videos.get(name, [])
        fixed = dict(FIXED_FILMS)
        if game == "bbb":  # BLOOD2PG's line 0 is sq\microfol.HNM, not sq\mind.HNM
            fixed.pop("SQ/MIND.HNM")
            fixed["SQ/MICROFOL.HNM"] = (0, "opening logo reel (BLOOD2PG asset table line 0)", "opening")
        if name in fixed:
            line, trigger, kind = fixed[name]
            entry.update(played=True, trigger=dict(path="executable fixed film", line=line, detail=trigger,
                                                   citation="BLOODPRG asset path table 0x0F48B"
                                                   if game == "cb" else "BLOOD2PG asset path table"))
            if kind in ("opening", "credits"):
                entry["scene"] = dict(context=kind, background="full screen, no bridge frame", music="native mixer")
                entry["render"] = dict(method=kind)
            elif kind == "contact":
                base = any_base("SCRIPT1", "contact") or next(iter(site_plans))
                add_clip(entry, base, line, {"clip_vertical_offset": 0} if line in ROW_ZERO_LINES else {},
                         "contact transition over the bridge before/after FRIGO.FD")
            else:
                entry["scene"] = dict(context="ship_travel", note="ship presentation during travel")
                entry["render"] = dict(method="plain_decode", note=SHIP_CYCLE_NOTE)
        elif HYPERSPACE.fullmatch(name):
            entry.update(played=True, trigger=dict(path="hyperspace", line=6,
                                                   detail="camera_approach HYPERSPACE_ACTIVE_LINE 6; the executable names hyper_00..07"))
            entry["scene"] = dict(context="ship_travel", note="hyperjump")
            entry["render"] = dict(method="plain_decode", note=SHIP_CYCLE_NOTE)
        elif name.startswith("SQ/") and any(kind == "Sequence" for _, kind, _, _ in uses):
            records = sorted({record for record, kind, _, _ in uses if kind == "Sequence"})
            entry.update(played=True, trigger=dict(path="DESCRIPT Sequence record (sequence panel, line 2)",
                                                   records=records))
            entry["scene"] = dict(context="sequence_panel", music="the record's music command")
            entry["render"] = dict(method="descript_sequence", record=records[0])
        elif name.startswith("SQ/") and name in requested_by:
            fact, line_no = requested_by[name][0]
            actor = fact["speakers"][0] if fact["speakers"] else None
            base = (base_for(fact["profile"], actor, fact["during"]) if actor else None) or \
                any_base(fact["profile"], fact["during"] or "contact")
            entry.update(played=True, trigger=dict(
                path="script request sequence (A8, line 7)", profile=fact["profile"], proc=fact["proc"],
                during=fact["during"], source_line=line_no,
                citation=f"static-dialogue {game}/{fact['profile'].lower()}/source.blood:{line_no}"))
            if base:
                add_clip(entry, base, 7, {"clip_sequence": name.split("/")[1].lower(),
                                          **({"clip_vertical_offset": 0} if frames[name] and name in args.full_height else {})},
                         f"{fact['during'] or 'contact'} scene of proc {fact['proc']}")
        elif name.startswith("PL/") and uses:
            records = [record for record, kind, _, _ in uses if kind == "Location"]
            entry.update(played=True, trigger=dict(path="Location record arrival film (line 3)", records=records))
            base = None
            for profile in sorted(rows_of, key=lambda p: int(p[6:])):
                rows = rows_of[profile]
                for record in records:
                    if record in rows:
                        chain = [record]
                        place = rows[record]["holder"]
                        while rows.get(record, {}).get("kind") != "CelestialBody" and place in rows and len(chain) < 4:
                            chain.insert(0, place)
                            if rows[place]["kind"] == "CelestialBody":
                                break
                            place = rows[place]["holder"]
                        cand = any_base(profile, "travel") or any_base(profile, "contact")
                        if cand:
                            base = (cand, chain)
                            break
                if base:
                    break
            if base:
                ident, chain = base
                plan = site_plans[ident][0]
                ctx = plan["context"]
                saved = site_plans[ident]
                site_plans[ident] = (dict(plan, context=dict(
                    kind="travel", actor_offset=ctx["actor_offset"], planet=chain[0], destination=chain[-1],
                    descript_records=ctx.get("descript_records", []),
                    **({"history_concepts": ctx["history_concepts"]} if ctx.get("history_concepts") else {}))),
                    saved[1], f"arrival at {'/'.join(chain)}")
                add_clip(entry, ident, 3, {}, f"native travel arrival at {'/'.join(chain)}")
                site_plans[ident] = saved
        elif name.startswith("PE/") and uses:
            character = next((record for record, kind, _, _ in uses if kind == "Character"), None)
            line = next(line for _, kind, _, line in uses if kind == "Character")
            entry.update(played=True, trigger=dict(path="Character record clip", record=character, line=line,
                                                   records=sorted({r for r, *_ in uses})))
            base = None
            forced = None
            if line in (39, 40):
                for profile in sorted(rows_of, key=lambda p: int(p[6:])):
                    base = base_for(profile, character, "contact")
                    if base:
                        break
                if not base:
                    # Any line of the actor, moved into its contact transition.
                    base = next((i for i in sorted(site_plans, key=lambda i: rank.get(i, 1 << 30))
                                 if actor_of.get(i) == character), None)
                    forced = "contact"
                note = "contact transition entry/exit (lines 39/40) over FRIGO.FD"
            else:
                said = presented_by.get(name, [])
                for fact, actor in said:
                    base = base_for(fact["profile"], actor, fact["during"])
                    if base:
                        note = f"{fact['during'] or 'contact'} scene of proc {fact['proc']} ({fact['profile']})"
                        break
                if not base:
                    best = None
                    for ident in sorted(site_plans, key=lambda i: rank.get(i, 1 << 30)):
                        if actor_of.get(ident) == character:
                            best = ident
                            break
                    base, note = best, "the scene of the actor's first line"
            if not base:
                # A character with no verified line of its own: its record is applied in
                # a contact scene (clip_records), the way the renderer plays record-only clips.
                base = next((any_base(p, "contact") for p in sorted(rows_of, key=lambda p: int(p[6:]))
                             if any_base(p, "contact")), None)
                forced, note = None, "character without verified lines: its record applied in a contact scene"
            if base and forced:
                plan, scene, why = site_plans[base]
                ctx = plan["context"]
                saved = site_plans[base]
                site_plans[base] = (dict(plan, context=dict(
                    kind="source_default", actor_offset=ctx["actor_offset"], descript_records=[character],
                    **({"history_concepts": ctx["history_concepts"]} if ctx.get("history_concepts") else {}))),
                    scene, note)
                add_clip(entry, base, line, {}, note)
                site_plans[base] = saved
            elif base:
                add_clip(entry, base, line, {"clip_records": [character]} if character else {}, note)
        elif name.startswith("OB/") and uses:
            obj = next((record for record, kind, _, _ in uses if kind == "Object"), uses[0][0])
            moves = [m for m in aboard_by.get(obj, []) if m[0]["proc"] != "init"]
            entry.update(played=bool(moves), trigger=dict(
                path="object moved aboard by C2 (line 43)", record=obj,
                sites=[f"{m[0]['profile']}:{m[0]['proc']}:{m[1]}" for m in moves]))
            if moves:
                fact = moves[0][0]
                actor = fact["speakers"][0] if fact["speakers"] else None
                base = (base_for(fact["profile"], actor, fact["during"]) if actor else None) or \
                    any_base(fact["profile"], fact["during"] or "contact")
                if base:
                    add_clip(entry, base, 43, {"clip_records": [obj]},
                             f"{fact['during'] or 'contact'} scene of proc {fact['proc']} ({fact['profile']})")
        if "played" not in entry:
            entry.update(played=False, trigger=dict(
                path="none", detail="no DESCRIPT record, script, dictionary or executable path of this game names it"))
        if "render" not in entry:
            entry["render"] = dict(method="plain_decode", note="never presented by this game's code: shown full "
                                   "screen over black, silent" if not entry["played"] else
                                   "played, but no verified line places it; decoded full screen")
        videos.append(entry)
    save_json(clip_dir / "planning.json", dict(schema=1, game=game, planner_sha256=digest(__file__),
                                               clips=clips, unplanned=[]))
    save_json(out / f"{game}.json", dict(
        schema=1, game=GAMES[game], planner_sha256=digest(__file__), catalog_sha256=digest(args.catalog),
        play_order_sha256=digest(args.play_order), ledger_sha256=digest(args.ledger),
        text_scene_counts=dict(scenes), render_methods=dict(Counter(v["render"]["method"] for v in videos)),
        videos=videos))
    print(game, dict(scenes), dict(Counter(v["render"]["method"] for v in videos)))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--game", choices=GAMES, required=True)
    parser.add_argument("--catalog", type=Path, required=True)
    parser.add_argument("--inventory", type=Path, required=True)
    parser.add_argument("--ledger", type=Path, required=True)
    parser.add_argument("--text-planning", type=Path, required=True)
    parser.add_argument("--play-order", type=Path, required=True)
    parser.add_argument("--locations", type=Path, required=True)
    parser.add_argument("--video-manifest", type=Path, required=True)
    parser.add_argument("--dialogue", type=Path, required=True)
    parser.add_argument("--full-height", nargs="*", default=[])
    parser.add_argument("--out", type=Path, required=True)
    build(parser.parse_args())


if __name__ == "__main__":
    main()
