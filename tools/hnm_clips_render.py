#!/usr/bin/env python3
# /// script
# requires-python = ">=3.11"
# dependencies = []
# ///
"""Render planned character clips with the native exporter and check each result.

A clip passes when the exporter's lossless-verified master exists, its report says the
clip played, the decoded video is the planned resource, and (nearly) every frame decoded.
Failures are kept in coverage.json; interrupted runs resume.
"""

import argparse
import os
from pathlib import Path
import subprocess

from native_sequence_anthology import read_json, require
from video_anthology import digest, resource_name, run_game, save_json


def verify(path, entry, exporter_hash):
    report = read_json(path / "report.json")
    runner = report["runner"]
    require(report["target"] == "static_text_site" and report["exporter_sha256"] == exporter_hash,
            "report is not from this exporter")
    require(runner["completion"] == "isolated_clip_played" and report["decoded_master_verified"]
            and report["video_timestamps_verified"], "clip did not complete with verified media")
    clip = runner["clip"]
    require(resource_name(clip["resource"]) == entry["clip"], f"played {clip['resource']}, not {entry['clip']}")
    require(clip["decoded_frames"] + 1 >= entry["frames"], "clip did not decode every frame")
    return dict(clip=entry["clip"], actor=entry["actor"], line=entry["line"], path=str(path.resolve()),
                master_sha256=digest(path / "master.mkv"), duration_ns=runner["duration_ns"],
                frames=runner["presented_frames"], audio_samples=runner["audio_samples"])


def render(args):
    planning = read_json(args.plans / "planning.json")
    exporter_hash = digest(args.exporter)
    entries = planning["clips"][args.shard::args.shards]
    args.out.mkdir(parents=True, exist_ok=True)
    completed, failures = [], []
    for index, entry in enumerate(entries):
        name = entry["clip"].replace("/", "_")
        path = args.out / name
        print(f"[{index + 1}/{len(entries)}] {entry['clip']}", flush=True)
        try:
            require(digest(args.plans / entry["plan"]) == entry["plan_sha256"], "plan changed")
            if not path.exists():
                with (args.out / (name + ".log")).open("w") as log:
                    run_game([args.exporter.resolve(), args.assets, "static-text:" + str(args.plans / entry["plan"]),
                              path.resolve(), 6000], os.environ.copy(), log, 600)
            completed.append(verify(path, entry, exporter_hash))
        except (ValueError, RuntimeError, OSError, KeyError, subprocess.SubprocessError) as error:
            failures.append(dict(clip=entry["clip"], error=str(error)))
            print(f"  failed: {error}", flush=True)
        save_json(args.out / "coverage.json", dict(schema=1, exporter_sha256=exporter_hash,
                  plan_set_sha256=digest(args.plans / "planning.json"), completed=completed, failures=failures))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--assets", type=Path, required=True)
    parser.add_argument("--plans", type=Path, required=True)
    parser.add_argument("--out", type=Path, required=True)
    parser.add_argument("--exporter", type=Path, required=True)
    parser.add_argument("--shard", type=int, default=0)
    parser.add_argument("--shards", type=int, default=1)
    render(parser.parse_args())


if __name__ == "__main__":
    main()
