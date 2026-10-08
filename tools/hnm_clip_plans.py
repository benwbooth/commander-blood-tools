#!/usr/bin/env python3
# /// script
# requires-python = ">=3.11"
# dependencies = []
# ///
"""Plan native renders of every character clip (PE/*.HNM) over its authored background.

A character clip is drawn by the presentation line that DESCRIPT assigns it (idle line 8,
talk clips 9.., right/left scene videos 39/40, with talk entries 31/32 overwriting 39/40 as
in the original catalog). The plan reuses a static text plan of the same actor for the
scene context (source-default profile, placed location records) and adds clip_line and
clip_frames; the exporter then presents that line until the video has decoded.
"""

import argparse
import json
from pathlib import Path

from native_sequence_anthology import read_json, require
from video_anthology import digest, resource_name, save_json

IDLE, FIRST_TALK, RIGHT, LEFT = 8, 9, 39, 40


def clip_lines(record):
    """Final presentation-line -> clip name for a Character record, in catalog order."""
    names = {}
    for command in record["authored"]["commands"]:
        if command["kind"] == "character_right_video":
            names[RIGHT] = command["name"]
        elif command["kind"] == "character_left_video":
            names[LEFT] = command["name"]
    talk = 0
    for command in record["authored"]["commands"]:
        if command["kind"] == "idle_clip":
            names[IDLE] = command["name"]
        elif command["kind"] == "talk_clip":
            names[FIRST_TALK + talk] = command["name"]
            talk += 1
    return {line: resource_name("PE/" + name + ("" if name.upper().endswith(".HNM") else ".HNM"))
            for line, name in names.items()}


def build(inventory_path, plans_dir, video_manifest, game, out):
    require(not out.exists(), f"output already exists: {out}")
    inventory = read_json(inventory_path)
    frames = {resource_name(v["source_resource_name"]): v["frame_count"]
              for v in read_json(video_manifest)["videos"]}
    base = {}
    for path in sorted(plans_dir.glob("*.plan.json")):
        plan = read_json(path)
        context = plan["context"]
        if context["kind"] != "source_default" or not path.name.startswith(game + "."):
            continue
        actor = context["descript_records"][-1]
        rank = (-len(context["descript_records"]), plan["initial_profile"], path.name)
        if actor not in base or rank < base[actor][0]:
            base[actor] = (rank, plan, path.name)
    out.mkdir(parents=True)
    chosen, missing = {}, []
    for record in inventory["descriptions"]:
        if record["game"] != game or record["authored"]["kind"] != "Character":
            continue
        for line, clip in clip_lines(record).items():
            chosen.setdefault(clip, (record["authored"]["name"], line))
    entries = []
    for clip, (actor, line) in sorted(chosen.items()):
        if clip not in frames:
            missing.append(dict(clip=clip, reason="no decoded derivative"))
            continue
        extra = []
        if actor in base:
            _, plan, source = base[actor]
        else:
            # The actor has no script text: borrow the game's first scene and apply the
            # character's own DESCRIPT record on top, with no location placement.
            _, plan, source = min(base.values(), key=lambda item: (len(item[1]["context"]["descript_records"]), item[0]))
            extra = [actor]
        plan = dict(plan, clip_line=line, clip_frames=frames[clip], clip_records=extra,
                    title=f"{plan['title']} clip {clip} line {line}")
        if extra:
            plan["context"] = dict(plan["context"], descript_records=plan["context"]["descript_records"][-1:])
            plan["title"] += f" (character {actor}, unplaced)"
        name = clip.replace("/", "_") + ".plan.json"
        save_json(out / name, plan)
        entries.append(dict(clip=clip, actor=actor, line=line, frames=frames[clip], plan=name,
                            base_plan=source, plan_sha256=digest(out / name)))
    save_json(out / "planning.json", dict(schema=1, game=game, inventory_sha256=digest(inventory_path),
                                          planner_sha256=digest(__file__), clips=entries, unplanned=missing))
    print(f"{len(entries)} clip plans, {len(missing)} unplanned")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--inventory", type=Path, required=True)
    parser.add_argument("--plans", type=Path, required=True)
    parser.add_argument("--video-manifest", type=Path, required=True)
    parser.add_argument("--game", choices=("cb", "bbb"), required=True)
    parser.add_argument("--out", type=Path, required=True)
    args = parser.parse_args()
    build(args.inventory, args.plans, args.video_manifest, args.game, args.out)


if __name__ == "__main__":
    main()
