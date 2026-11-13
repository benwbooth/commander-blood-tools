#!/usr/bin/env python3
# /// script
# requires-python = ">=3.11"
# dependencies = []
# ///
"""Plan and render conversation runs as single continuous native captures.

A conversation run (play-order `conversation_runs`) is one proc firing: its lines share
the profile, the actor and the scene. Rendering each line on its own restarts the scene
and its music at every join, so a run is rendered as one capture: the first line's plan
sets the scene up once, and the rest of the run follows in `run_lines`. The exporter
publishes each line when the previous one reaches its native hold, exactly as a single
prepared line ends, and reports every line's start and end inside the run.

`plan` writes one run plan per run (split where the scene context changes) from the
per-line plans of a video-play-plan text plan set. `render` renders a shard of them and
writes a sidecar (`run.json`) beside each verified `master.mkv`.
"""

import argparse
import json
import os
from pathlib import Path
import subprocess

from native_sequence_anthology import read_json, require
from video_anthology import digest, run_game, save_json


def plan(args):
    out = args.out
    require(not out.exists(), f"output already exists: {out}")
    out.mkdir(parents=True)
    runs = read_json(args.play_order)["conversation_runs"]
    planning = read_json(args.text_plans / "planning.json")
    plans = {row["id"]: row for row in planning["sites"] if row["status"] == "planned"}
    entries, missing = [], []
    for index, run in enumerate(runs):
        group, context = [], None
        pieces = []
        for site in run["sites"]:
            if site not in plans:
                missing.append(dict(run=index, site=site))
                continue
            line = read_json(args.text_plans / plans[site]["plan"])
            key = json.dumps(dict(line["context"], history_concepts=None), sort_keys=True)
            if group and key != context:
                pieces.append(group)
                group = []
            context = key
            group.append((site, line, plans[site]["reason"]))
        if group:
            pieces.append(group)
        for part, group in enumerate(pieces):
            first = dict(group[0][1], run_lines=[line for _, line, _ in group[1:]])
            name = f"run{index:04d}-{part}"
            save_json(out / f"{name}.plan.json", first)
            entries.append(dict(name=name, run=index, part=part, actor=run.get("actor"), scene=run.get("scene"),
                                sites=[site for site, _, _ in group], placement=[why for _, _, why in group],
                                plan=f"{name}.plan.json", plan_sha256=digest(out / f"{name}.plan.json")))
    save_json(out / "planning.json", dict(schema=1, mode="static_run", play_order_sha256=digest(args.play_order),
                                          text_plans=str(args.text_plans.resolve()), runs=entries, missing=missing))
    print(f"{len(entries)} run captures from {len(runs)} runs; {sum(len(e['sites']) for e in entries)} lines; "
          f"{len(missing)} lines without a verified plan")


def render(args):
    planning = read_json(args.plans / "planning.json")
    exporter_hash = digest(args.exporter)
    entries = planning["runs"][args.shard::args.shards]
    args.out.mkdir(parents=True, exist_ok=True)
    completed, failures = [], []
    for index, entry in enumerate(entries):
        path = args.out / entry["name"]
        print(f"[{index + 1}/{len(entries)}] {entry['name']} ({len(entry['sites'])} lines)", flush=True)
        try:
            require(digest(args.plans / entry["plan"]) == entry["plan_sha256"], "plan changed")
            if not path.exists():
                with (args.out / (entry["name"] + ".log")).open("w") as log:
                    run_game([args.exporter.resolve(), args.assets, "static-text:" + str(args.plans / entry["plan"]),
                              path.resolve(), args.max_frames], os.environ.copy(), log, args.timeout)
            report = read_json(path / "report.json")
            runner = report["runner"]
            require(report["exporter_sha256"] == exporter_hash, "report is not from this exporter")
            require(runner["completion"] == "isolated_text_hold_completed" and report["decoded_master_verified"]
                    and report["video_timestamps_verified"], "run did not complete with verified media")
            lines = runner.get("run_lines") or [dict(site=entry["sites"][0], start_ns=0, end_ns=runner["duration_ns"])]
            require([line["site"] for line in lines] == entry["sites"], "run presented different lines")
            sidecar = dict(schema=1, run=entry["run"], part=entry["part"], actor=entry["actor"], scene=entry["scene"],
                           duration_ns=runner["duration_ns"],
                           lines=[dict(site=line["site"], start_ns=line["start_ns"], end_ns=line["end_ns"],
                                       placement=why) for line, why in zip(lines, entry["placement"])])
            save_json(path / "run.json", sidecar)
            completed.append(dict(name=entry["name"], path=str(path.resolve()), sites=entry["sites"],
                                  master_sha256=digest(path / "master.mkv"), duration_ns=runner["duration_ns"]))
        except (ValueError, RuntimeError, OSError, KeyError, subprocess.SubprocessError) as error:
            failures.append(dict(name=entry["name"], sites=entry["sites"], error=str(error)))
            print(f"  failed: {error}", flush=True)
        save_json(args.out / "coverage.json", dict(schema=1, exporter_sha256=exporter_hash,
                  plan_set_sha256=digest(args.plans / "planning.json"), completed=completed, failures=failures))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="command", required=True)
    p = sub.add_parser("plan")
    p.add_argument("--play-order", type=Path, required=True)
    p.add_argument("--text-plans", type=Path, required=True)
    p.add_argument("--out", type=Path, required=True)
    r = sub.add_parser("render")
    r.add_argument("--assets", type=Path, required=True)
    r.add_argument("--plans", type=Path, required=True)
    r.add_argument("--out", type=Path, required=True)
    r.add_argument("--exporter", type=Path, required=True)
    r.add_argument("--shard", type=int, default=0)
    r.add_argument("--shards", type=int, default=1)
    r.add_argument("--max-frames", type=int, default=60000)
    r.add_argument("--timeout", type=int, default=3600)
    args = parser.parse_args()
    plan(args) if args.command == "plan" else render(args)


if __name__ == "__main__":
    main()
