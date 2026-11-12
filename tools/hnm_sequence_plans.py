#!/usr/bin/env python3
# /// script
# requires-python = ">=3.11"
# dependencies = []
# ///
"""Plan in-engine renders of the script-requested sequence videos no DESCRIPT record names.

The scripts request these (A8, presentation line 7) while an actor speaks; the engine
supplies the picture frame, hand cursor, cadence and whatever music the scene is
running. Where the decompiled script shows the requesting actor the plan reuses that
actor's scene (placed location, profile); otherwise it borrows the game's first scene.

Sequences no script requests and whose picture is taller than the 130-row sequence band
are the full-screen films the executable presents itself: the boot reel (sq\\mind.HNM,
sq\\the_star.HNM), the ejection film (sq\\ejection.HNM), the cryobox films (sq\\cryogel.hnm, sq\\cryorad.hnm), all fixed literals
in the asset path table at BLOODPRG.EXE 0x0F48B, and the CRYO card sq\\microfol.hnm. The
game presents them outside any DESCRIPT-placed location scene, i.e. from scene row 0; the
borrowed scene's location row (e.g. 78) would push their authored rectangles past the
200-row screen (RectangleOutOfBounds in presentation_present). The extent is the last row
any frame owns in the decoder's mask derivative.
"""

import argparse
from pathlib import Path
import re
import subprocess

from native_sequence_anthology import read_json, require
from video_anthology import digest, resource_name, save_json

SCRIPT_SEQUENCE_LINE = 7
UNPLACED_SCENE_ROW = 0
SEQUENCE_BAND_ROWS = 130
SCREEN_WIDTH, SCREEN_HEIGHT = 320, 200


def owned_rows(mask):
    """Number of screen rows down to the last one any frame of the mask derivative owns."""
    raw = subprocess.run(["ffmpeg", "-v", "error", "-i", str(mask), "-f", "rawvideo", "-pix_fmt", "gray", "-"],
                         check=True, capture_output=True).stdout
    frame = SCREEN_WIDTH * SCREEN_HEIGHT
    require(raw and len(raw) % frame == 0, f"mask is not 320x200: {mask}")
    bottom = 0
    for start in range(0, len(raw), frame):
        for row in range(SCREEN_HEIGHT - 1, bottom - 1, -1):
            offset = start + row * SCREEN_WIDTH
            if raw[offset:offset + SCREEN_WIDTH].strip(b"\0"):
                bottom = row + 1
                break
    return bottom


def request_sites(dialogue_catalog, game):
    sites = {}
    for path in sorted((dialogue_catalog / game).glob("script*/source.blood")):
        actor = None
        for line in path.read_text().splitlines():
            speaker = re.search(r"say (\w+)", line)
            actor = speaker.group(1) if speaker else actor
            request = re.search(r'request sequence "([^"]+)"', line)
            if request:
                sites.setdefault(resource_name("SQ/" + request.group(1)), (path.parent.name, actor))
    return sites


def build(catalog_path, plans_dir, video_manifest, dialogue_catalog, game, out):
    require(not out.exists(), f"output already exists: {out}")
    catalog = read_json(catalog_path)
    derivatives = {resource_name(v["source_resource_name"]): v for v in read_json(video_manifest)["videos"]}
    frames = {name: v["frame_count"] for name, v in derivatives.items()}
    sites = request_sites(dialogue_catalog, game)
    bases = {}
    for path in sorted(plans_dir.glob(f"{game}.*.plan.json")):
        plan = read_json(path)
        context = plan["context"]
        if context["kind"] == "source_default":
            actor = context["descript_records"][-1]
            rank = (-len(context["descript_records"]), plan["initial_profile"], path.name)
            if actor not in bases or rank < bases[actor][0]:
                bases[actor] = (rank, plan, path.name)
    # Unplaced scene: a location row from a placed context can push full-height clips off screen.
    default = min((item for item in bases.values() if len(item[1]["context"]["descript_records"]) == 1),
                  key=lambda item: (item[1]["initial_profile"], item[2]))
    out.mkdir(parents=True)
    entries = []
    for video in catalog["videos"]:
        if not video["name"].startswith("SQ/") or video["references"]:
            continue
        site = sites.get(video["name"])
        actor = site[1] if site else None
        _, plan, source = bases.get(actor, default)
        plan = dict(plan, clip_line=SCRIPT_SEQUENCE_LINE, clip_frames=frames[video["name"]],
                    clip_sequence=video["name"].split("/")[1].lower(),
                    title=f"{plan['title']} script sequence {video['name']}")
        if site is None and owned_rows(video_manifest.parent / derivatives[video["name"]]["mask_webm_path"]) > SEQUENCE_BAND_ROWS:
            plan["clip_vertical_offset"] = UNPLACED_SCENE_ROW
        name = video["name"].replace("/", "_") + ".plan.json"
        save_json(out / name, plan)
        entries.append(dict(clip=video["name"], actor=actor or "(game's first scene)", line=SCRIPT_SEQUENCE_LINE,
                            frames=frames[video["name"]], plan=name, base_plan=source,
                            request_site=site, vertical_offset=plan.get("clip_vertical_offset"),
                            plan_sha256=digest(out / name)))
    save_json(out / "planning.json", dict(schema=1, game=game, catalog_sha256=digest(catalog_path),
                                          planner_sha256=digest(__file__), clips=entries, unplanned=[]))
    print(f"{len(entries)} sequence plans ({sum(1 for e in entries if e['request_site'])} with a script request site)")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--catalog", type=Path, required=True)
    parser.add_argument("--plans", type=Path, required=True)
    parser.add_argument("--video-manifest", type=Path, required=True)
    parser.add_argument("--dialogue-catalog", type=Path, required=True)
    parser.add_argument("--game", choices=("cb", "bbb"), required=True)
    parser.add_argument("--out", type=Path, required=True)
    args = parser.parse_args()
    build(args.catalog, args.plans, args.video_manifest, args.dialogue_catalog, args.game, args.out)


if __name__ == "__main__":
    main()
