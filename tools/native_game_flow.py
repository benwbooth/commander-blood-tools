#!/usr/bin/env python3
# /// script
# requires-python = ">=3.11"
# dependencies = []
# ///
"""Record normal-input native route evidence without writing video or raw traces.

A recorded route is a witness, not a completeness proof or a DOS-parity claim.
Unknown branches stay uncovered; loaded media are distinct from played media.
"""

import argparse
import hashlib
import json
import os
from pathlib import Path
import selectors
import signal
import shutil
import struct
import subprocess
import time

from video_anthology import GAMES, ROOT, digest, display_environment, inventory, save_json

NORMAL_ACTIONS = {"move", "motion", "click", "sclick", "frameclick", "key", "wait",
                  "frames", "park", "choose", "alien-drive", "await-alien"}


def normal_actions(source):
    actions = []
    for number, line in enumerate(source.splitlines(), 1):
        line = line.strip()
        if not line or line.startswith("#"):
            continue
        if line.split()[0] not in NORMAL_ACTIONS:
            raise ValueError(f"line {number}: state injection or unsupported action: {line}")
        actions.append(line)
    if not actions:
        raise ValueError("empty route")
    return actions


def route_source(paths):
    parts = [path.read_text() for path in paths]
    provenance = [dict(path=str(path.resolve()), sha256=hashlib.sha256(part.encode()).hexdigest())
                  for path, part in zip(paths, parts)]
    return "\n".join(parts), provenance


def text_state(s):
    p = s["presentation"]
    subtitle = bool(p["text_display_active"])
    raster = s["subtitle_raster"] if subtitle else s["inline_menu_raster"]
    if raster and raster["expected_pixel_count"]:
        if raster["matching_pixel_count"] != raster["expected_pixel_count"]:
            raise ValueError("native dialogue glyph raster mismatch")
    if subtitle:
        content = s["subtitle_bytes"]
        cursor = p["text_state"]["subtitle_reveal_cursor"]
        complete = bool(content) and cursor is not None and cursor >= len(content)
    else:
        menu = p["inline_menu"]
        content = menu["display_words"]
        complete = bool(content) and menu["reveal_count"] >= len(content)
    return dict(kind="subtitle_bytes" if subtitle else "inline_words", content=content,
                complete=bool(complete and raster and raster["expected_pixel_count"])), raster


def scene_state(s):
    vm, p, v = s["vm"], s["presentation"], s["video"]
    # Whole authored text is stable while individual words are being revealed.
    text, _ = text_state(s)
    overlay = s.get("alien_overlay") or {}
    return dict(profile=vm["resource_profile"], cod_site=s["published_cod_text_site"],
                bas_site=s["published_bas_text_site"], text=text,
                actor=p["active_actor_presentation"], choices=p["rendered_word_choices"],
                choice_rows=[{key: row[key] for key in ("kind", "item_index", "position")}
                             for row in p["retained_word_choice"].get("rows", [])],
                pending_call=p.get("pending_presentation_owner"),
                waiting_for_input=p["waiting_for_input"],
                lifecycle=dict(vm_enabled=vm["execution_enabled"], active=p["active"],
                               screen_active=p["screen_active"],
                               choice_phase=p["retained_word_choice"]["phase"],
                               ship_dispatch_blocked=p["ship_scene"]["dispatch_blocked"]),
                text_controls={key: p["text_state"].get(key) for key in
                               ("start_locked", "hold_ready", "dialogue_hold_complete",
                                "word_buffer_nonempty", "text_menu_pending", "sequence_active")},
                active_video=v["active_resource"], video_open=v["source_open_or_draining"],
                displayed_line=vm["displayed_line"], descript=s["descript"],
                sequence_caption=s.get("sequence_caption"),
                save_load=s.get("save_load"),
                navigation=s["navigation"].get("target"),
                navigation_detail=s["navigation"],
                ui={key: p.get(key) for key in
                    ("ui_flags", "ship_ui_state", "mode", "request_flags", "defer",
                     "text_display_active", "screen_phase")},
                bridge_console=s.get("bridge_console"),
                travel_enabled=vm.get("sequel_travel_enabled"),
                nav_actor_blockers=p.get("nav_actor_blockers"),
                navigation_music=s["audio"]["loaded_navigation_music"],
                ending=p.get("sequel_control"),
                alien_overlay={key: overlay.get(key) for key in
                               ("completed_overlays", "invocations", "loaded_scene_resource")})


def revealed_text_sites(state):
    # Navigation text may retain the previous COD offset after its actor closes.
    if (state["profile"] is None or not state["actor"]
            or not state["lifecycle"]["active"] or not state["text"]["complete"]):
        return set()
    return {(state["profile"], domain, state[f"{domain}_site"])
            for domain in ("cod", "bas") if state[f"{domain}_site"] is not None}


class FlowRecorder:
    def __init__(self, output, checkpoint=None, writable=None):
        self.output = output
        self.previous = None
        self.objects = {}
        self.globals = {}
        self.globals_available = False
        self.frames = 0
        self.events = 0
        self.decoded = 0
        self.last_video = None
        self.played = set()
        self.published_sites = set()
        self.revealed_sites = set()
        self.profiles = set()
        self.last_frame = -1
        self.checkpoint = checkpoint
        self.writable = writable
        self.loaded_checkpoint = False
        self.completed_saves = 0
        self.saved_slots = {}

    def record(self, record):
        if record.get("schema") != 1 or record.get("executable") != "modern-rust":
            raise ValueError("unsupported native trace")
        if record["frame"] != self.last_frame + 1:
            raise ValueError("native frame trace is not contiguous from startup")
        self.last_frame = record["frame"]
        s = record["semantic"]
        state = scene_state(s)
        shown_text = (bytes(state["text"]["content"]).decode("latin1")
                      if state["text"]["kind"] == "subtitle_bytes"
                      else " ".join(state["text"]["content"]))
        if "CHEAT MODE" in shown_text.upper():
            raise ValueError("route entered the script cheat menu")
        operations = s.get("save_load") or {}
        loads = operations.get("completed_loads", 0)
        if loads not in (0, 1) or (self.loaded_checkpoint and loads != 1):
            raise ValueError("route loaded a save without a validated predecessor")
        if loads and not self.loaded_checkpoint:
            if self.checkpoint is None or operations["active_slot"] != self.checkpoint["slot"]:
                raise ValueError("route loaded a save without a validated predecessor")
            if digest(self.writable / self.checkpoint["filename"]) != self.checkpoint["sha256"]:
                raise ValueError("checkpoint changed before its normal UI load completed")
            self.loaded_checkpoint = True
        saves = operations.get("completed_saves", 0)
        if saves != self.completed_saves:
            if saves != self.completed_saves + 1:
                raise ValueError("native save-operation counter is not contiguous")
            slot = operations["active_slot"]
            if not isinstance(slot, int) or not 0 <= slot < 10:
                raise ValueError("native save published an invalid slot")
            self.saved_slots[slot] = record["frame"]
            self.completed_saves = saves
        profile = state["profile"]
        if profile is not None:
            self.profiles.add(profile)
        for domain in ("cod", "bas"):
            site = state[f"{domain}_site"]
            if profile is not None and site is not None:
                key = (profile, domain, site)
                self.published_sites.add(key)
        self.revealed_sites.update(revealed_text_sites(state))
        video = state["active_video"]
        decoded = s["video"]["decoded_frame_count"]
        # A retained/loaded name is not evidence that a source frame was decoded.
        restart = video is not None and (video != self.last_video or decoded < self.decoded)
        advanced = video is not None and decoded > (0 if restart else self.decoded)
        if advanced:
            self.played.add(video)
        current_objects = {str(o["record"]): o for o in s["persistent"]["object_locations"]}
        object_changes = {key: value for key, value in current_objects.items()
                          if self.objects.get(key) != value}
        removed_objects = sorted(set(self.objects) - set(current_objects))
        self.globals_available = s["persistent"].get("script_globals") is not None
        current_globals = s["persistent"].get("script_globals") or {}
        global_changes = {key: value for key, value in current_globals.items()
                          if self.globals.get(key) != value}
        removed_globals = sorted(set(self.globals) - set(current_globals))
        changed = state != self.previous
        if changed or restart or object_changes or removed_objects or global_changes or removed_globals:
            event = dict(event=self.events, frame=record["frame"], boundary=record["boundary"],
                         observed_elapsed_ns=record["elapsed_ns"],
                         state=state if changed else None, video_restart=restart,
                         text_raster=text_state(s)[1],
                         decoded_frame_count=decoded,
                         object_changes=object_changes, removed_objects=removed_objects,
                         global_changes=global_changes, removed_globals=removed_globals,
                         state_array_hash=s["persistent"]["state_array_hash"],
                         character_slots_hash=s["persistent"]["character_slots_hash"])
            self.output.write(json.dumps(event, separators=(",", ":")) + "\n")
            self.output.flush()
            self.events += 1
        self.frames += 1
        self.previous, self.objects = state, current_objects
        self.globals = current_globals
        self.decoded, self.last_video = decoded, video

    def summary(self):
        def sites(values):
            return [dict(profile=p + 1, domain=d, offset=o) for p, d, o in sorted(values)]
        return dict(observed_frames=self.frames, events=self.events,
                    profiles=[p + 1 for p in sorted(self.profiles)],
                    decoded_video_resources=sorted(self.played),
                    published_sites=sites(self.published_sites),
                    fully_revealed_sites=sites(self.revealed_sites),
                    text_site_attribution="fully rasterized text with an active actor presentation",
                    completed_saves=self.completed_saves,
                    loaded_checkpoint=self.loaded_checkpoint,
                    final_state=self.previous, final_objects=list(self.objects.values()),
                    final_globals=self.globals if self.globals_available else None)


def saved_checkpoints(writable, saved_slots):
    if not saved_slots:
        return []
    directory = writable / "BLOOD.SAV"
    if directory.is_symlink():
        raise ValueError("save directory is a symlink")
    data = directory.read_bytes()
    if len(data) != 320:
        raise ValueError("native save directory is not ten 32-byte records")
    slots = list(struct.iter_unpack("<16s16s", data))
    checkpoints = []
    for slot, frame in sorted(saved_slots.items()):
        label, filename = slots[slot]
        filename = filename.split(b"\0", 1)[0].decode("ascii")
        if (not filename or "/" in filename or "\\" in filename
                or Path(filename).name != filename or not filename.upper().endswith(".SAV")):
            raise ValueError("native save directory contains an unsafe filename")
        matches = [path for path in writable.iterdir() if path.name.casefold() == filename.casefold()]
        if len(matches) != 1:
            raise ValueError("completed native save has no unique DOS filename")
        path = matches[0]
        if path.is_symlink() or not path.is_file():
            raise ValueError("completed native save has no regular save file")
        checkpoints.append(dict(slot=slot, saved_at_frame=frame, filename=path.name,
                                label=label.split(b"\0", 1)[0].decode("latin1"),
                                sha256=digest(path), directory_sha256=digest(directory)))
    return checkpoints


def verified_predecessor(manifest, slot, game, provenance, visited=None, runtime_update=None):
    if runtime_update is not None:
        reason = runtime_update.get("reason") if isinstance(runtime_update, dict) else runtime_update
        if not isinstance(reason, str) or not reason.strip():
            raise ValueError("runtime update needs a nonempty reason")
    manifest = manifest.resolve()
    visited = set() if visited is None else visited
    if manifest in visited or len(visited) >= 100:
        raise ValueError("checkpoint lineage is cyclic or too deep")
    visited.add(manifest)
    manifest_bytes = manifest.read_bytes()
    manifest_hash = hashlib.sha256(manifest_bytes).hexdigest()
    status = json.loads(manifest_bytes)
    if (status.get("schema") != 1 or status.get("game") != game
            or status.get("status") != "observed_route" or status.get("state_injection") is not False):
        raise ValueError("predecessor is not a completed normal-input route witness")
    for key in ("binary_sha256", "asset_manifest_sha256", "sources"):
        if status["provenance"][key] != provenance[key]:
            if key == "binary_sha256" and runtime_update:
                continue
            raise ValueError(f"predecessor uses different {key}")
    root = manifest.parent
    required = {"events.jsonl", "actions.jsonl", "scenario.tsv", "game.log", "flow.md"}
    if not required.issubset(status["files"]):
        raise ValueError("predecessor is missing required hashed evidence")
    for name, expected in status["files"].items():
        path = root / name
        if not path.resolve().is_relative_to(root) or path.is_symlink() or digest(path) != expected:
            raise ValueError(f"predecessor evidence changed: {name}")
    source = (root / "scenario.tsv").read_text()
    normal_actions(source)
    if hashlib.sha256(source.encode()).hexdigest() != status["provenance"]["scenario_sha256"]:
        raise ValueError("predecessor scenario changed")
    parent = status.get("predecessor")
    if parent is not None:
        if status["observations"].get("loaded_checkpoint") is not True:
            raise ValueError("predecessor did not witness its own checkpoint load")
        if digest(parent["manifest"]) != parent["manifest_sha256"]:
            raise ValueError("earlier lineage manifest changed")
        # Validate each historical continuation against the runtime it actually used.
        verified_predecessor(Path(parent["manifest"]), parent["slot"], game,
                             status["provenance"], visited, parent.get("runtime_update"))
    checkpoint = next((item for item in status.get("checkpoints", []) if item["slot"] == slot), None)
    if checkpoint is None:
        raise ValueError("predecessor has no witnessed save in the requested slot")
    actual = saved_checkpoints(root / "writable", {slot: checkpoint["saved_at_frame"]})
    if actual != [checkpoint]:
        raise ValueError("predecessor checkpoint or directory changed")
    witnessed = False
    with (root / "events.jsonl").open() as source:
        for line in source:
            event = json.loads(line)
            if event["frame"] == checkpoint["saved_at_frame"]:
                operations = (event.get("state") or {}).get("save_load") or {}
                witnessed = operations.get("completed_saves", 0) > 0 and operations.get("active_slot") == slot
    if not witnessed:
        raise ValueError("checkpoint has no matching native save event")
    if digest(manifest) != manifest_hash:
        raise ValueError("predecessor manifest changed during validation")
    reference = dict(manifest=str(manifest), manifest_sha256=manifest_hash, slot=slot)
    if status["provenance"]["binary_sha256"] != provenance["binary_sha256"]:
        reference["runtime_update"] = dict(
            reason=runtime_update["reason"] if isinstance(runtime_update, dict) else runtime_update,
            previous_binary_sha256=status["provenance"]["binary_sha256"],
            current_binary_sha256=provenance["binary_sha256"])
        if isinstance(runtime_update, dict) and reference["runtime_update"] != runtime_update:
            raise ValueError("historical runtime update hashes changed")
    elif isinstance(runtime_update, dict):
        raise ValueError("historical runtime update hashes changed")
    return checkpoint, reference


def flow_markdown(directory, status):
    summary = status["observations"]
    lines = [f"# {status['game'].upper()} Observed Normal-Input Route", "",
             "Partial route evidence from the native port, not a complete game flow or DOS-parity proof.",
             "No injected contacts, teleports, inventory, or guards.", "",
             f"{summary['observed_frames']} native frame boundaries; {summary['events']} compact events; "
             f"{len(summary['fully_revealed_sites'])} fully revealed text sites.", "",
             "The action file is the replay recipe. Frame numbers are native trace boundaries, not video PTS.",
             "Route-exit credits are not evidence that the story ending was reached.", "",
             "[Actions](scenario.tsv) | [Exact evidence](events.jsonl) | [Provenance](flow.json)", "",
             "## Presented Flow", ""]
    if status.get("predecessor"):
        lines[3:3] = ["Continues a hash-verified, normally saved predecessor through the game's load menu.", ""]
        update = status["predecessor"].get("runtime_update")
        if update:
            lines[3:3] = ["Runtime changed since the predecessor: " + json.dumps(update["reason"]), ""]
    if status.get("observed_ending"):
        ending = status["observed_ending"]
        lines[3:3] = [
            f"Observed the expected SCRIPT2 ending sequence assignment at 0x{ending['code_offset']:04X} "
            "and a clean native process exit. This does not classify the ending as a story success.", ""]
    previous = None
    with (directory / "events.jsonl").open() as source:
        for line in source:
            event = json.loads(line)
            state = event["state"]
            if state is None:
                continue
            label = {key: state[key] for key in ("profile", "cod_site", "bas_site",
                     "text", "actor", "choices", "active_video", "navigation_music")}
            if label == previous:
                continue
            previous = label
            profile = state["profile"]
            location = "bootstrap" if profile is None else f"SCRIPT{profile + 1}"
            for domain in ("cod", "bas"):
                if state[f"{domain}_site"] is not None:
                    location += f" {domain.upper()} {state[f'{domain}_site']:04X}"
            lines += [f"### Boundary {event['frame']}: {location}", "",
                      "```text", f"Video: {state['active_video'] or '(none)'}",
                      f"Navigation music: {state['navigation_music'] or '(none)'}"]
            if state["actor"]:
                lines.append(f"Actor: {state['actor'].get('name')}")
            if state["text"]["content"]:
                # JSON quoting keeps source dialogue from becoming Markdown markup.
                content = state["text"]["content"]
                text = (bytes(content).decode("utf-8", "replace")
                        if state["text"]["kind"] == "subtitle_bytes" else " ".join(content))
                lines.append("Text: " + json.dumps(text))
                lines.append("Fully shown: " + str(state["text"]["complete"]).lower())
            if state["choices"]:
                lines.append("Choices: " + json.dumps(state["choices"]))
            lines += ["```", ""]
    return "\n".join(lines)


def verify_route_completion(actions, completed, returncode, final_state, ending_offset=None):
    if returncode != 0:
        raise RuntimeError(f"native route exited {returncode}; see game.log")
    if ending_offset is None:
        if completed != actions:
            raise ValueError(f"route completed {len(completed)}/{len(actions)} expected actions")
        return None
    # A natural ending may interrupt only the final passive wait, never a choice.
    interrupted_wait = (bool(actions) and actions[-1].split()[0] in {"wait", "frames"}
                        and completed == actions[:-1])
    if completed != actions and not interrupted_wait:
        raise ValueError("ending interrupted actions other than the final passive wait")
    ending = (final_state or {}).get("ending") or {}
    assignment = ending.get("last_assignment") or {}
    if ((final_state or {}).get("profile") != 1 or not ending.get("ending_active")
            or assignment.get("code_offset") != ending_offset
            or assignment.get("query_mode") is not False):
        raise ValueError("native exit did not reach the expected SCRIPT2 ending sequence assignment")
    if (final_state.get("video_open") is not False or "active_video" not in final_state
            or final_state["active_video"] is not None):
        raise ValueError("native exit left ending video open")
    return dict(profile=2, code_offset=ending_offset, native_exit_code=returncode,
                completed_actions=len(completed), interrupted_final_wait=interrupted_wait)


def consume_process(command, env, directory, recorder, actions, timeout, ending_offset=None):
    """Drain both native trace streams concurrently; never persist per-frame JSON."""
    selector = selectors.DefaultSelector()
    streams = {}
    process = None
    completed = []
    try:
        for kind in ("live", "actions"):
            path = directory / f"{kind}.fifo"
            os.mkfifo(path)
            fd = os.open(path, os.O_RDWR | os.O_NONBLOCK)
            streams[fd] = dict(kind=kind, buffer=b"")
            selector.register(fd, selectors.EVENT_READ)
        with (directory / "game.log").open("w") as log, (directory / "actions.jsonl").open("w") as output:
            process = subprocess.Popen(command, env=env, stdout=log, stderr=subprocess.STDOUT,
                                       start_new_session=True)
            deadline = time.monotonic() + timeout
            while True:
                ready = selector.select(0.1)
                for key, _ in ready:
                    stream = streams[key.fd]
                    stream["buffer"] += os.read(key.fd, 1 << 20)
                    lines = stream["buffer"].split(b"\n")
                    stream["buffer"] = lines.pop()
                    for line in lines:
                        item = json.loads(line)
                        if stream["kind"] == "live":
                            recorder.record(item)
                        else:
                            semantic = item.pop("semantic")
                            item["state_array_hash"] = semantic["persistent"]["state_array_hash"]
                            item["scene"] = scene_state(semantic)
                            item["bridge"] = {key: semantic["presentation"].get(key) for key in
                                              ("bridge_frame", "bridge_presentation_mode",
                                               "bridge_actor_slots")}
                            output.write(json.dumps(item, separators=(",", ":")) + "\n")
                            output.flush()
                            if item["phase"] == "after":
                                completed.append(item["action"])
                    if len(stream["buffer"]) > 10_000_000:
                        raise ValueError("oversized unterminated native trace record")
                if process.poll() is not None and not ready:
                    break
                if time.monotonic() >= deadline:
                    raise TimeoutError("native route exceeded its time limit")
        if any(stream["buffer"] for stream in streams.values()):
            raise ValueError("truncated native trace record")
        if not recorder.frames:
            raise ValueError("route produced no native trace")
        return verify_route_completion(actions, completed, process.returncode,
                                       recorder.previous, ending_offset)
    finally:
        if process is not None and process.poll() is None:
            os.killpg(process.pid, signal.SIGTERM)
            try:
                process.wait(timeout=5)
            except subprocess.TimeoutExpired:
                os.killpg(process.pid, signal.SIGKILL)
                process.wait()
        selector.close()
        for fd in streams:
            os.close(fd)
        for kind in ("live", "actions"):
            (directory / f"{kind}.fifo").unlink(missing_ok=True)


def record_route(args):
    if args.runtime_update is not None and (not args.resume_from or not args.runtime_update.strip()):
        raise ValueError("--runtime-update requires a predecessor and a nonempty reason")
    if args.expect_bbb_ending is not None and (args.game != "bbb"
                                             or not 0 <= args.expect_bbb_ending <= 0xFFFF):
        raise ValueError("--expect-bbb-ending requires BBB and a 16-bit SCRIPT2 code offset")
    source, scenario_sources = route_source([args.scenario, *args.then])
    actions = normal_actions(source)
    binary = (args.bin_dir / GAMES[args.game]).resolve()
    _, manifest_hash = inventory(args.assets)
    profile_root = ROOT / "re/vm" / ("profiles" if args.game == "cb" else "big-bug-bang-profiles")
    sources = sorted(profile_root.glob("script*.blood"))
    sources.append(ROOT / "re/descript" / ("DESCRIPT.descript" if args.game == "cb"
                                          else "big-bug-bang/DESCRIPT.descript"))
    provenance = dict(binary_sha256=digest(binary), asset_manifest_sha256=manifest_hash,
                      recorder_sha256=digest(__file__),
                      scenario_sha256=hashlib.sha256(source.encode()).hexdigest(),
                      scenario_sources=scenario_sources,
                      sources={str(path.relative_to(ROOT)): digest(path) for path in sources},
                      packed_second=args.packed_second)
    directory = args.out.resolve()
    checkpoint, predecessor = (None, None)
    if args.resume_from:
        checkpoint, predecessor = verified_predecessor(
            args.resume_from, args.slot, args.game, provenance, runtime_update=args.runtime_update)
    directory.mkdir(parents=True, exist_ok=False)
    writable = directory / "writable"
    writable.mkdir()
    if checkpoint:
        parent_writable = args.resume_from.resolve().parent / "writable"
        for name in ("BLOOD.SAV", checkpoint["filename"]):
            shutil.copyfile(parent_writable / name, writable / name)
        if saved_checkpoints(writable, {checkpoint["slot"]: checkpoint["saved_at_frame"]}) != [checkpoint]:
            raise ValueError("checkpoint changed while being copied")
    (directory / "scenario.tsv").write_text(source)
    status = dict(schema=1, game=args.game, status="running", provenance=provenance,
                  start=("normal UI load of witnessed predecessor" if checkpoint else
                         "new game; empty private writable directory"),
                  predecessor=predecessor, state_injection=False,
                  full_game_complete=False, all_normal_branches_complete=False,
                  time_basis="native frame boundaries; elapsed_ns is wall time, not video PTS")
    if args.expect_bbb_ending is not None:
        status["expected_ending"] = dict(profile=2, code_offset=args.expect_bbb_ending)
    save_json(directory / "flow.json", status)
    recorder = None
    try:
        with (directory / "events.jsonl").open("w") as output, display_environment(False) as env:
            env["CBLOOD_SCRIPT_SOURCE"] = str(ROOT / "re")
            env.pop("CBLOOD_ASSET_CACHE", None)
            command = [str(binary), "--data", str(args.assets.resolve()), "--write-data",
                       str(directory / "writable"), "--scenario", str(directory / "scenario.tsv"),
                       "--trace", str(directory / "actions.fifo"), "--live-trace",
                       str(directory / "live.fifo"), "--oracle-packed-second", str(args.packed_second)]
            recorder = FlowRecorder(output, checkpoint, writable)
            ending = consume_process(command, env, directory, recorder, actions, args.timeout,
                                     args.expect_bbb_ending)
            if checkpoint and not recorder.loaded_checkpoint:
                raise ValueError("route never completed its expected normal checkpoint load")
            status.update(status="observed_ending" if ending else "observed_route",
                          observations=recorder.summary())
            if ending:
                status["observed_ending"] = ending
            status["checkpoints"] = saved_checkpoints(writable, recorder.saved_slots)
        for path, expected in provenance["sources"].items():
            if digest(ROOT / path) != expected:
                raise ValueError(f"script source changed during recording: {path}")
        if digest(binary) != provenance["binary_sha256"]:
            raise ValueError("native binary changed during recording")
        (directory / "flow.md").write_text(flow_markdown(directory, status))
        status["files"] = {name: digest(directory / name)
                           for name in ("events.jsonl", "actions.jsonl", "scenario.tsv", "game.log", "flow.md")}
        for checkpoint in status["checkpoints"]:
            for name in ("BLOOD.SAV", checkpoint["filename"]):
                status["files"][f"writable/{name}"] = digest(writable / name)
    except Exception as error:
        status.update(status="failed", error=str(error))
        if recorder is not None:
            status["observations"] = recorder.summary()
            (directory / "flow.md").write_text(flow_markdown(directory, status))
        raise
    finally:
        save_json(directory / "flow.json", status)
    summary = status["observations"]
    print(json.dumps(dict(status=status["status"], frames=summary["observed_frames"],
                          events=summary["events"], profiles=summary["profiles"],
                          played_resources=len(summary["decoded_video_resources"]),
                          fully_revealed_sites=len(summary["fully_revealed_sites"]))))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--game", required=True, choices=GAMES)
    parser.add_argument("--scenario", required=True, type=Path)
    parser.add_argument("--then", action="append", type=Path, default=[],
                        help="append normal-input fragment; state is carried without loading a save")
    parser.add_argument("--resume-from", type=Path, help="flow.json with a witnessed normal save")
    parser.add_argument("--runtime-update", metavar="REASON",
                        help="explicitly record a binary update while retaining exact save/script/asset checks")
    parser.add_argument("--expect-bbb-ending", type=lambda value: int(value, 0), metavar="OFFSET",
                        help="require clean exit with this SCRIPT2 ending sequence assignment; allow the last wait to stop early")
    parser.add_argument("--slot", type=int, choices=range(10), default=0,
                        help="witnessed predecessor save slot; default 0")
    parser.add_argument("--assets", required=True, type=Path)
    parser.add_argument("--out", required=True, type=Path)
    parser.add_argument("--bin-dir", type=Path, default=ROOT / "target/release")
    parser.add_argument("--packed-second", type=int, default=39, choices=range(256), metavar="BYTE")
    parser.add_argument("--timeout", type=float, default=900)
    record_route(parser.parse_args())


if __name__ == "__main__":
    main()
