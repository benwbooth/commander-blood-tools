#!/usr/bin/env python3
# /// script
# requires-python = ">=3.11"
# dependencies = []
# ///
"""Derive a static play order for every text site and HNM video of one game.

The order comes from the recovered BloodScript of each profile (the decompiled
SCRIPTn COD/BAS that recompiles byte-exact), not from captures:

* Profiles are visited from SCRIPT1 along their `run profile` statements, in the
  order those statements first become reachable.
* Inside a profile, a value-set abstract interpretation runs the logic in rounds.
  Every variable holds the set of values the script can have given it so far:
  the state block and `global` declarations seed it, assignments, `+=`/`-=`,
  `bits(...) =` and `transfer` grow it. Engine-owned fields the script never
  assigns (current location, encounter counts, timers) and player input
  (`choice`, `presentation`, `during`) can take any value. A proc fires in the
  first round where it is enabled and every guard is satisfiable; its body runs
  in source order with its own effects visible, and a `when` whose guard needs
  effects of a later round is reached in that later round (a later visit).
  Effects of a round are visible from the next one.
* A site's key is (profile rank, round, proc source index, statement index).
  Selector (BAS reply-menu) lines follow the first proc that opens a contact
  with their actor.

Videos are placed where the script first makes them visible: DESCRIPT sequence
records at `sequence_slots[n] = "record"` (the engine's initial slot holds
`present`, crates/commander-blood-game/src/native/bloodprg/script_sequence_slots.rs
INITIAL_SEQUENCE_NAME, so it opens the game), `request sequence` resources at
their statement, `say ... presentation=file.hnm` clips at their line, location
arrival videos before the first line spoken by an actor located there, object
videos where the item is first transferred aboard. Anything the scripts never
name is listed as unplaced, never guessed into the order.
"""

import argparse
import json
from pathlib import Path
import re

from native_sequence_anthology import read_json, require
from video_anthology import digest, resource_name, save_json

INITIAL_SEQUENCE = "present"
ANY = object()
MAX_VALUES = 48
# Fields the engine and the player change outside the script: travel moves the ship and
# whoever is aboard, the inventory screen trades items, navigation discovers planets,
# every contact counts an encounter.
ENGINE_FIELDS = (".current_location", ".holder", ".known", ".encounter_count")


# ---------------------------------------------------------------- parsing

class Node:
    def __init__(self, kind, line, text, **fields):
        self.kind, self.line, self.text = kind, line, text
        self.__dict__.update(fields)


def block(lines, i):
    """Parse statements until the closing brace; returns (nodes, index of the closing line)."""
    nodes = []
    while i < len(lines):
        raw = lines[i].strip()
        if raw.startswith("}"):
            return nodes, i
        if m := re.match(r'proc (\S+)(?: as "([^"]+)")?(?: (enabled|disabled))? \{$', raw):
            guards, i = block(lines, i + 1)
            if lines[i].strip() == "} then {":
                body, i = block(lines, i + 1)
            else:  # a proc without guards is written as its body alone
                guards, body = [], guards
            nodes.append(Node("proc", i, raw, name=m.group(1), alias=m.group(2) or m.group(1),
                              enabled=m.group(3) != "disabled",
                              guards=guards, body=body))
        elif raw == "when {":
            start = i
            guards, i = block(lines, i + 1)
            require(lines[i].strip() == "} then {", f"when without then at line {i + 1}")
            body, i = block(lines, i + 1)
            other = []
            if lines[i].strip() == "} else {":
                other, i = block(lines, i + 1)
            nodes.append(Node("when", start, raw, guards=guards, body=body, other=other))
        elif m := re.match(r"selector (\S+) \{$", raw):
            cases, i = block(lines, i + 1)
            nodes.append(Node("selector", i, raw, name=m.group(1), cases=cases))
        elif m := re.match(r'case "([^"]*)".*\{$', raw):
            body, i = block(lines, i + 1)
            nodes.append(Node("case", i, raw, word=m.group(1), body=body))
        elif raw.endswith("{"):
            body, i = block(lines, i + 1)
            nodes.append(Node("block", i, raw, body=body))
        elif raw:
            nodes.append(Node("stmt", i, raw))
        i += 1
    return nodes, i


def value(token):
    token = token.strip()
    if token.startswith('"') and token.endswith('"'):
        return token[1:-1]
    if re.fullmatch(r"-?\d+", token):
        return int(token)
    if re.fullmatch(r"0x[0-9A-Fa-f]+", token):
        return int(token, 16)
    return {"true": True, "false": False}.get(token, token)


def parse_state(lines):
    """object name -> {field: value} from the profile's state block."""
    objects, current = {}, None
    for raw in (line.strip() for line in lines):
        if m := re.match(r'(\w+) "([^"]+)" \{$', raw):
            current = objects.setdefault(m.group(2), {"kind": m.group(1)})
        elif raw == "}":
            current = None
        elif current is not None:
            if m := re.match(r"(\w+) = (.+)$", raw):
                current[m.group(1)] = value(m.group(2))
            elif re.fullmatch(r"\w+", raw):
                current[raw] = True
    return objects


def parse_profile(path):
    lines = path.read_text().splitlines()
    start = lines.index("state {")
    logic = lines.index("logic {")
    state = parse_state(lines[start + 1:logic])
    nodes, _ = block(lines, logic + 1)
    return state, nodes


# ---------------------------------------------------------------- interpretation

def var_name(expr):
    expr = expr.strip()
    if m := re.fullmatch(r"bits\((\S+?),\s*(0x[0-9A-Fa-f]+|\d+)\)", expr):
        return f"{m.group(1)}&{value(m.group(2))}"
    return expr


ASSIGN = re.compile(r"^(bits\([^)]*\)|[\w.\[\]\"]+) (=|\+=|-=) (.+)$")
COND = re.compile(r"^(!?)(bits\([^)]*\)|[\w.\[\]\"]+)(?: (==|!=|>=|<=|>|<) (.+))?$")


class Values:
    def __init__(self, state, nodes, carried=None, external=frozenset()):
        self.sets = {}
        self.assigned = set()
        raised, compared = set(), set()
        for kind, stmt in walk_statements(nodes):
            if m := re.match(r"require (\S+) (>|>=) ", stmt):
                compared.add(var_name(m.group(1)))
            if m := ASSIGN.match(stmt):
                self.assigned.add(var_name(m.group(1)))
                rhs = value(m.group(3))
                if m.group(2) == "+=" or (m.group(2) == "=" and not isinstance(rhs, bool) and isinstance(rhs, int) and rhs > 0):
                    raised.add(var_name(m.group(1)))
            elif m := re.match(r"transfer (\S+) from \S+ to \S+", stmt):
                self.assigned.add(f"{m.group(1)}.holder")
            elif m := re.match(r"global (\w+) = (.+)$", stmt):
                self.sets[f"globals.{m.group(1)}"] = {value(m.group(2))}
        for name, fields in state.items():
            for field, v in fields.items():
                self.sets.setdefault(f"{name}.{field}", {v})
                if field == "location":
                    self.sets.setdefault(f"{name}.current_location", {v})
        for proc in (n for n in nodes if n.kind == "proc"):
            self.sets[f"{proc.name}.enabled"] = {proc.enabled}
        # A counter the script compares upward but never raises is fed by the engine
        # (bionium from the cyberspace game, for example).
        self.engine_fed = (compared - raised) | external
        for name, values in (carried or {}).items():
            self.sets.setdefault(name, set()).update(values)
            self.assigned.add(name)

    def get(self, name):
        if name not in self.assigned or name.endswith(ENGINE_FIELDS) or name in self.engine_fed:
            return ANY
        return self.sets.get(name, {0, False} if "&" in name or name.endswith((".known", ".active")) else {0})

    def satisfiable(self, guard):
        g = guard.strip()
        if g.startswith("during ") or g.startswith("require choice") or g.startswith("require presentation"):
            return True
        if not g.startswith("require "):
            return True
        m = COND.match(g.removeprefix("require ").strip())
        if not m or "[" in m.group(2):
            return True  # table lookups and unparsed forms: never block an order
        negate, name, op, rhs = m.groups()
        values = self.get(var_name(name))
        if values is ANY:
            return True
        if op is None:
            return any(bool(v) != bool(negate) for v in values)
        target = value(rhs)
        def holds(v):
            try:
                return {"==": v == target, "!=": v != target, ">": v > target, "<": v < target,
                        ">=": v >= target, "<=": v <= target}[op]
            except TypeError:
                return op == "!="
        return any(holds(v) != bool(negate) for v in values)

    def effect(self, stmt, pending):
        if m := ASSIGN.match(stmt):
            name, op, rhs = var_name(m.group(1)), m.group(2), value(m.group(3))
            current = self.get(name)
            current = {0} if current is ANY else current
            if op == "=":
                new = {rhs}
            else:
                step = rhs if isinstance(rhs, int) else 1
                new = {v + step if op == "+=" else v - step for v in current if isinstance(v, int)}
            pending.setdefault(name, set()).update(new)
        elif m := re.match(r"transfer (\S+) from \S+ to (\S+)", stmt):
            pending.setdefault(f"{m.group(1)}.holder", set()).add(m.group(2))

    def commit(self, pending, local=None):
        changed = False
        for name, new in pending.items():
            target = self.sets.setdefault(name, set(self.get(name)) if self.get(name) is not ANY else set())
            for v in new:
                if v not in target and len(target) < MAX_VALUES:
                    target.add(v)
                    changed = True
        return changed


def walk_statements(nodes):
    for node in nodes:
        if node.kind == "stmt":
            yield "stmt", node.text
        for child in ("guards", "body", "other", "cases"):
            yield from walk_statements(getattr(node, child, []) or [])


def interpret(state, nodes, carried=None, external=frozenset(), max_rounds=64):
    """Returns ({line: round} for every reached statement, procs in source order, rounds used,
    final value sets of the globals)."""
    values = Values(state, nodes, carried, external)
    procs = [n for n in nodes if n.kind == "proc"]
    reached = {}

    def run(body, rnd, local, pending):
        for node in body:
            if node.kind == "stmt":
                reached.setdefault(node.line, rnd)
                values.effect(node.text, local)
                values.effect(node.text, pending)
            elif node.kind == "when":
                saved = dict(values.sets)
                overlay(local)
                ok = all(values.satisfiable(g.text) for g in node.guards if g.kind == "stmt")
                values.sets = saved
                if ok:
                    run(node.body, rnd, local, pending)
                run(node.other, rnd, local, pending)  # else: reachable whenever the guard can fail
            elif node.kind == "block":
                run(node.body, rnd, local, pending)

    def overlay(local):
        for name, new in local.items():
            base = values.get(name)
            values.sets[name] = (set() if base is ANY else set(base)) | new
            values.assigned.add(name)

    rounds = 0
    for rnd in range(max_rounds):
        pending = {}
        before = len(reached)
        for proc in procs:
            enabled = values.get(f"{proc.name}.enabled")
            if enabled is not ANY and True not in enabled:
                continue
            if all(values.satisfiable(g.text) for g in proc.guards if g.kind == "stmt"):
                reached.setdefault(proc.line, rnd)
                run(proc.body, rnd, {}, pending)
        rounds = rnd + 1
        if not values.commit(pending) and len(reached) == before:
            break
    finals = {k: set(v) for k, v in values.sets.items() if k.startswith("globals.")}
    return reached, procs, rounds, finals


# ---------------------------------------------------------------- ordering

def say_nodes(nodes, owner=None):
    """(owner proc, statement node) for every say, in source order."""
    for node in nodes:
        if node.kind == "proc":
            yield from say_nodes(node.guards + node.body, node)
        elif node.kind == "stmt" and node.text.startswith(("say ", "text_tokens ")):
            yield owner, node
        else:
            for child in ("body", "other", "guards", "cases"):
                yield from say_nodes(getattr(node, child, []) or [], owner)


def all_stmts(nodes, owner=None):
    for node in nodes:
        if node.kind == "proc":
            yield from all_stmts(node.guards + node.body, node)
        elif node.kind == "stmt":
            yield owner, node
        else:
            for child in ("guards", "body", "other", "cases"):
                yield from all_stmts(getattr(node, child, []) or [], owner)


QUIET = re.compile(r"^(say |text_tokens |dictionary |end presentation|choice = none|yield|halt|menu |\w+:$)")


def drives(proc):
    for owner, node in all_stmts(proc.body):
        text = node.text
        # Enabling or disabling procs only sequences them; the rounds already order that.
        if QUIET.match(text) or re.fullmatch(r"\w+\.enabled = (true|false)", text) or text.startswith("require "):
            continue
        return True
    return False


def analyse_profile(path, carried=None, external=frozenset()):
    state, nodes = parse_profile(path)
    reached, procs, rounds, finals = interpret(state, nodes, carried, external)
    index = {p.name: i for i, p in enumerate(procs)}
    # Within a round, procs that change the game (state, profiles, queued scenes) play
    # before procs that only show text: a console menu the player may open at any time
    # does not precede the tutorial that the boot timer queues.
    passive = {p.name: 0 if drives(p) else 1 for p in procs}
    # Statement keys: (round, passive, proc index, line). Unreached statements stay unkeyed.
    keys = {}
    for owner, node in all_stmts(nodes):
        if owner is None or node.line not in reached:
            continue
        keys[node.line] = (reached[node.line], passive[owner.name], index[owner.name], node.line)
    # Contact openers: earliest key of a proc that requires presentation == actor.
    openers = {}
    for proc in procs:
        for guard in proc.guards:
            if m := re.match(r"require presentation == (\S+)", guard.text):
                if proc.line in reached:
                    key = (reached[proc.line], passive[proc.name], index[proc.name], proc.line)
                    if m.group(1) not in openers or key < openers[m.group(1)]:
                        openers[m.group(1)] = key
    contact = {p.name for p in procs if any(g.text == "during travel" for g in p.guards)}
    return dict(state=state, nodes=nodes, procs=procs, reached=reached, keys=keys, openers=openers,
                contact_lines={node.line for owner, node in all_stmts(nodes) if owner and owner.name in contact},
                during={node.line: next((g.text.split()[1] for g in owner.guards if g.text.startswith("during ")), None)
                        for owner, node in all_stmts(nodes) if owner},
                rounds=rounds, finals=finals, path=path)


def externally_set(paths):
    """Variables some guard tests for a value no profile's script can produce.

    The sequel engine writes globals itself (inventory gifts, quiz results); a guard
    waiting for a value that no assignment, increment or declaration in any profile
    yields is waiting on the engine, so that variable may take any value.
    """
    produced, increased, tests = {}, set(), []
    # Item and actor flag bits the script only clears are set by the engine's gift screen.
    for path in paths:
        state, nodes = parse_profile(path)
        for kind, stmt in walk_statements(nodes):
            if m := ASSIGN.match(stmt):
                name = var_name(m.group(1))
                if m.group(2) == "=":
                    produced.setdefault(name, set()).add(value(m.group(3)))
                else:
                    increased.add(name)
            elif m := re.match(r"global (\w+) = (.+)$", stmt):
                produced.setdefault(f"globals.{m.group(1)}", set()).add(value(m.group(2)))
            if m := re.match(r"require (!?)(\S+) (==|>|>=) (\S+)$", stmt):
                tests.append((var_name(m.group(2)), m.group(3), value(m.group(4))))
            elif m := re.match(r"require (bits\([^)]*\))$", stmt):
                tests.append((var_name(m.group(1)), "==", True))
    external = set()
    for name, op, target in tests:
        if name in increased or not (name.startswith("globals.") or "&" in name):
            continue
        values = produced.get(name, set())
        if op == "==" and target not in values:
            external.add(name)
        elif op != "==" and not any(isinstance(v, int) and not isinstance(v, bool) and v > target for v in values):
            external.add(name)
    return frozenset(external)


def profile_order(sources):
    """Profiles from SCRIPT1 in the order their run statements become reachable."""
    order, queue = [], ["SCRIPT1"]
    while queue:
        name = queue.pop(0)
        if name in order or name not in sources:
            continue
        order.append(name)
        analysis = sources[name]
        runs = sorted((analysis["keys"][node.line], m.group(1))
                      for owner, node in all_stmts(analysis["nodes"])
                      if (m := re.match(r"run profile (\S+)", node.text)) and node.line in analysis["keys"])
        queue[:0] = [target for _, target in runs if target not in order]
    return order + sorted((p for p in sources if p not in order), key=lambda p: int(p.removeprefix("SCRIPT")))


def build(dialogue, inventory_path, catalog_path, game, out):
    inventory = read_json(inventory_path)
    catalog = read_json(catalog_path)
    sources = {}
    paths = sorted((dialogue / game).glob("script*/source.blood"))
    external = externally_set(paths)
    for path in paths:
        sources[path.parent.name.upper()] = analyse_profile(path, None, external)
    profiles = profile_order(sources)
    # Globals persist across profiles (the hub profile runs the episodes): replay the
    # profiles in play order, each starting from what the earlier ones can have set.
    # The hub profile is re-entered after each episode, so repeat whole passes until the
    # carried globals stop growing; a statement first reached in pass n plays in visit n.
    carried, merged = {}, {}
    for visit in range(8):
        before = {k: len(v) for k, v in carried.items()}
        for profile in profiles:
            analysis = analyse_profile(sources[profile]["path"], carried, external)
            for name, values in analysis["finals"].items():
                carried.setdefault(name, set()).update(values)
            keys = {line: (visit,) + key for line, key in analysis["keys"].items()}
            openers = {a: (visit,) + key for a, key in analysis["openers"].items()}
            if profile in merged:
                keys.update(merged[profile]["keys"])
                for actor, key in merged[profile]["openers"].items():
                    openers[actor] = key
            analysis.update(keys=keys, openers=openers, visits=visit + 1)
            merged[profile] = analysis
        if {k: len(v) for k, v in carried.items()} == before:
            break
    sources = merged
    profiles = profile_order(sources)
    rank = {p: i for i, p in enumerate(profiles)}
    videos = {v["name"] for v in catalog["videos"]}
    by_file = {}
    for name in videos:
        by_file.setdefault(name.split("/")[1].lower(), []).append(name)
    records = {r["authored"]["name"]: r for r in inventory["descriptions"] if r["game"] == game and "name" in r["authored"]}
    directories = {"location_video": "PL", "sequence_video": "SQ", "object_video": "OB", "idle_clip": "PE",
                   "character_right_video": "PE", "character_left_video": "PE", "talk_clip": "PE"}

    def record_videos(record):
        out = []
        for command in record["authored"]["commands"]:
            directory = directories.get(command["kind"])
            if directory:
                name = resource_name(f"{directory}/{command.get('normalized_name') or command['name']}")
                if name in videos and name not in out:
                    out.append(name)
        return out

    items = []  # (key, kind, payload)
    mismatches, unresolved, on_planet, line_site, scene_of = [], [], set(), {}, {}
    # ---- text sites
    sites = [s for s in inventory["sites"] if s["game"] == game]
    for profile in profiles:
        analysis = sources[profile]
        says = {}
        for owner, node in say_nodes(analysis["nodes"]):
            if owner is not None:
                says.setdefault(owner.alias, []).append(node)
        cod = {}
        for site in sites:
            if site["profile"] == profile and site["kind"] == "cod":
                cod.setdefault(site["authored"]["procedure"].rsplit("_", 1)[0], []).append(site)
        for proc, entries in cod.items():
            entries.sort(key=lambda s: s["offset"])
            nodes = says.get(proc, [])
            if len(nodes) != len(entries):
                mismatches.append(dict(profile=profile, proc=proc, sites=len(entries), says=len(nodes)))
            for i, site in enumerate(entries):
                node = nodes[min(i, len(nodes) - 1)] if nodes else None
                if i < len(nodes):
                    line_site[(profile, node.line)] = site["id"]
                key = analysis["keys"].get(node.line) if node else None
                if key is None:
                    # A guard the analysis cannot satisfy: keep the line with its proc,
                    # after everything else in the profile's last visit.
                    key = (analysis["visits"] - 1, 10**6, 0, 0, node.line if node else 10**7)
                    unresolved.append(site["id"])
                items.append((key[:1] + (rank[profile],) + key[1:], "site", site["id"], site["authored"].get("record_name")))
                if node is not None and node.line in analysis["contact_lines"]:
                    on_planet.add(site["id"])
                if node is not None:
                    scene_of[site["id"]] = analysis["during"].get(node.line)
        for site in sorted((s for s in sites if s["profile"] == profile and s["kind"] == "bas"), key=lambda s: s["offset"]):
            actor = site["authored"].get("record_name")
            opener = analysis["openers"].get(actor)
            if opener is None:
                opener = (analysis["visits"] - 1, 10**6, 0, 0, 0)
                unresolved.append(site["id"])
            key = opener[:1] + (rank[profile],) + opener[1:-1] + (10**9 + site["offset"],)
            items.append((key, "site", site["id"], actor))
            scene_of[site["id"]] = "contact"  # reply menus open inside the actor's contact
    # ---- learn before visiting: the player can only fly to a place someone has told them
    # about, so an on-planet contact follows the first line (by another actor) that names
    # its planet or location. Chunks are procs (key[:5]); a moved chunk sorts right after
    # the chunk holding that line.
    actor_location, parents = {}, {}
    places = {}
    for profile in profiles:
        for name, fields in sources[profile]["state"].items():
            if "location" in fields:
                actor_location.setdefault(name, fields["location"])
            if "parent" in fields:
                parents.setdefault(name, fields["parent"])
            if fields.get("kind") in ("planet", "location", "black_hole"):
                places[name] = re.compile(r"\b" + re.escape(name.replace("_", " ")) + r"\b", re.I)

    def lineage(place_name):
        seen = []
        while place_name and place_name not in seen:
            seen.append(place_name)
            place_name = parents.get(place_name)
        return seen

    text = {s["id"]: (s["authored"].get("text") or "").replace("\n", " ") for s in sites}
    named = {ident: {p for p, pattern in places.items() if pattern.search(text[ident])}
             for key, kind, ident, actor in items}
    moved = {}

    def final(key):
        key = tuple(key)
        chunk = key[:5]
        return moved[chunk] + key[5:] if chunk in moved else key

    for _ in range(16):
        first = {}
        for key, kind, ident, actor in sorted(items, key=lambda item: final(item[0])):
            home = set(lineage(actor_location.get(actor)))
            for place_name in named[ident] - home:
                first.setdefault(place_name, final(key)[:-1])
        changed = False
        for key, kind, ident, actor in items:
            if ident not in on_planet or actor not in actor_location:
                continue
            chunk = tuple(key[:5])
            told = [first[p] for p in lineage(actor_location[actor]) if p in first]
            if told and min(told) > final(key)[:-1]:
                moved[chunk] = min(told) + (10**9,) + chunk
                changed = True
        if not changed:
            break
    items = [(final(key), kind, ident, actor) for key, kind, ident, actor in items]

    # ---- videos named by statements
    placed, sequences = {}, {}

    def place(key, name, why, site=None):
        if key is not None and (name not in placed or key < placed[name][0]):
            placed[name] = (key, why, site)

    def place_record(key, name, why):
        record = records.get(name)
        if record is None:
            return
        if record["authored"]["kind"] == "Sequence" and (name not in sequences or key < sequences[name][0]):
            sequences[name] = (key, why)
        for index, video in enumerate(record_videos(record)):
            place(key + (index,), video, why)  # the record's films play in their authored order

    def pick(file_name, preference):
        found = by_file.get(file_name.lower(), [])
        for directory in preference:
            for video in found:
                if video.startswith(directory + "/"):
                    return [video]
        return []

    mentions = {}
    for profile in profiles:
        analysis = sources[profile]
        for owner, node in all_stmts(analysis["nodes"]):
            key = analysis["keys"].get(node.line)
            if key is None:
                continue
            key = final(key[:1] + (rank[profile],) + key[1:])
            key = key[:-1] + (key[-1] - 0.5,)  # just before the statement
            text = node.text
            if m := re.match(r'sequence_slots\[\d+\] = "([^"]+)"', text):
                place_record(key, m.group(1), f"{profile} sequence slot {m.group(1)}")
            # The same file name can exist under several directories; a sequence request
            # loads from SQ, a character line from PE (DESCRIPT directory prefixes).
            if m := re.match(r'request sequence "([^"]+)"', text):
                for video in pick(m.group(1), ("SQ", "PL", "OB", "PE")):
                    place(key, video, f"{profile} request sequence")
            if m := re.search(r"presentation=(\S+\.hnm)", text, re.I):
                for video in pick(m.group(1), ("PE", "SQ", "PL", "OB")):
                    place(key, video, f"{profile} say presentation", line_site.get((profile, node.line)))
            if m := re.match(r"transfer (\S+) from \S+ to (aboard|blood)", text):
                place_record(key, m.group(1), f"{profile} transfer {m.group(1)}")
            if m := re.match(r"(\w+)\.holder = (aboard|blood)", text):
                place_record(key, m.group(1), f"{profile} {m.group(1)} aboard")
            # A location named by a statement (made known, active, or someone moved there).
            for word in re.findall(r"\b(\w+)\b", text.split(":")[0] if text.startswith("say ") else text):
                if word in records and records[word]["authored"]["kind"] == "Location":
                    if word not in mentions or key < mentions[word]:
                        mentions[word] = key
    if INITIAL_SEQUENCE in records:
        place_record((-1, 0, 0, 0, 0, 0), INITIAL_SEQUENCE, "initial sequence slot (engine)")
    # Location arrivals: before the first line spoken by an actor located there (or in a
    # place under it); a location nobody speaks in arrives where a statement first names it.
    first_line = {}
    for key, kind, ident, actor in items:
        if key is not None and actor in actor_location and ident in on_planet:
            for loc in lineage(actor_location[actor]):
                if loc not in first_line or key < first_line[loc]:
                    first_line[loc] = key
    for name, record in records.items():
        if record["authored"]["kind"] != "Location":
            continue
        if name in first_line:
            key, why = first_line[name][:-1] + (first_line[name][-1] - 0.75,), f"arrival at {name}"
        elif name in mentions:
            key, why = mentions[name], f"{name} named by the script"
        else:
            continue
        for video in record_videos(record):
            place(key, video, why)
    # Character records: clips no line names play at the actor's first contact.
    for name, record in records.items():
        if record["authored"]["kind"] == "Character":
            keys = [k for k, kind, ident, actor in items if actor == name and k is not None]
            for video in record_videos(record):
                if keys and video not in placed:
                    place(min(keys), video, f"contact with {name}")
    # Where each line is seen: the proc's `during` guard (bridge console, phone contact,
    # on-planet travel) and, on a planet, the speaker's place and its holders.
    def scene(ident, actor):
        during = scene_of.get(ident)
        out = {"during": during}
        if during == "travel" and actor in actor_location:
            out["place"] = lineage(actor_location[actor])
        return out

    entries = [dict(key=list(k), kind=kind, id=ident, actor=actor, scene=scene(ident, actor))
               for k, kind, ident, actor in items if k is not None]
    report_moves = len(moved)
    entries += [dict(key=list(k), kind="video", id=name, why=why, **({"shown_in_site": site} if site else {}))
                for name, (k, why, site) in placed.items()]
    entries += [dict(key=list(k), kind="sequence_record", id=name, why=why,
                     videos=record_videos(records[name])) for name, (k, why) in sequences.items()]
    entries.sort(key=lambda e: (tuple(e["key"]), e["kind"] == "site", e["kind"] == "video", e["id"]))
    unplaced_sites = sorted(ident for k, kind, ident, actor in items if k is None)
    unplaced_videos = sorted(videos - set(placed))
    report = dict(
        schema=1, game=game, method=__doc__.strip().splitlines()[0], analyser_sha256=digest(__file__),
        inventory_sha256=digest(inventory_path), catalog_sha256=digest(catalog_path),
        sources={p: digest(dialogue / game / p.lower() / "source.blood") for p in profiles},
        profiles=profiles, visits={p: sources[p]["visits"] for p in profiles}, engine_set_globals=sorted(external), rounds={p: sources[p]["rounds"] for p in profiles},
        contacts_after_first_mention=report_moves, order=entries, unresolved_guard_sites=sorted(unresolved), unplaced_sites=unplaced_sites, unplaced_videos=unplaced_videos,
        site_statement_mismatches=mismatches)
    save_json(out, report)
    print(f"{game}: {len(profiles)} profiles {profiles}; {sum(e['kind'] == 'site' for e in entries)} sites and "
          f"{sum(e['kind'] == 'video' for e in entries)} videos ordered; {len(unresolved)} sites past unresolved guards, {len(unplaced_sites)} sites and "
          f"{len(unplaced_videos)} videos unplaced; {len(mismatches)} proc count mismatches; "
          f"rounds {report['rounds']}")


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--dialogue", type=Path, required=True, help="static dialogue catalog with <game>/scriptN/source.blood")
    parser.add_argument("--inventory", type=Path, required=True)
    parser.add_argument("--catalog", type=Path, required=True)
    parser.add_argument("--game", choices=("cb", "bbb"), required=True)
    parser.add_argument("--out", type=Path, required=True)
    args = parser.parse_args()
    build(args.dialogue, args.inventory, args.catalog, args.game, args.out)


if __name__ == "__main__":
    main()
