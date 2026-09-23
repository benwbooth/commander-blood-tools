#!/usr/bin/env python3
"""Recapture a verified anthology's travel chapters with current native audio."""

import argparse
from concurrent.futures import ThreadPoolExecutor
import json
from pathlib import Path
import subprocess
import sys

from native_sequence_anthology import read_json, require
from video_anthology import ROOT, digest, save_json


def travel_plans(manifest):
    require([entry["record"] for entry in manifest["sources"]] ==
            [chapter["title"] for chapter in manifest["chapters"]],
            "manifest chapter/source order differs")
    plans = []
    for index, entry in enumerate(manifest["sources"]):
        if "report_sha256" not in entry:
            continue
        report_path = Path(entry["path"]) / "report.json"
        require(digest(report_path) == entry["report_sha256"], "source report changed")
        report = read_json(report_path)
        plan = report["runner"]["chapter"]
        require(plan["title"] == entry["record"] and plan["game"] == manifest["game"],
                "source plan differs from anthology entry")
        if plan.get("entry") == "travel":
            plans.append((index, entry, plan))
    require(plans, "anthology has no travel chapters")
    require(len({plan["title"] for _, _, plan in plans}) == len(plans),
            "travel chapter titles are not unique")
    return plans


def assign_workers(plans, jobs):
    groups = [[] for _ in range(jobs)]
    durations = [0] * jobs
    for item in sorted(plans, key=lambda row: row[1]["duration_ns"], reverse=True):
        worker = min(range(jobs), key=lambda index: durations[index])
        groups[worker].append(item)
        durations[worker] += item[1]["duration_ns"]
    return groups


def prepare(manifest, out, jobs):
    plans = travel_plans(manifest)
    (out / "plans").mkdir(parents=True, exist_ok=True)
    groups = assign_workers(plans, jobs)
    files = []
    for group in groups:
        paths = []
        for index, entry, plan in group:
            path = out / "plans" / f"{index:03d}-{Path(entry['path']).name}.json"
            if path.exists():
                require(read_json(path) == plan, "generated travel plan changed")
            else:
                save_json(path, plan)
            paths.append(path)
        files.append(paths)
    return files


def render_worker(index, plans, assets, exporter, out, max_frames):
    batch = out / f"capture-{index:02d}"
    command = [sys.executable, str(ROOT / "tools/native_dialogue_anthology.py"), "render",
               "--assets", str(assets), "--exporter", str(exporter),
               "--out", str(batch), "--max-frames", str(max_frames)]
    for path in plans:
        command.extend(["--plan", str(path)])
    with (out / f"capture-{index:02d}.log").open("w") as log:
        result = subprocess.run(command, stdout=log, stderr=subprocess.STDOUT, check=False)
    coverage_path = batch / "coverage.json"
    coverage = read_json(coverage_path) if coverage_path.exists() else None
    return dict(worker=index, batch=str(batch), exit_code=result.returncode,
                completed=len(coverage["completed"]) if coverage else 0,
                failures=coverage["failures"] if coverage else [dict(error="no coverage report")])


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--base-manifest", type=Path, required=True)
    parser.add_argument("--assets", type=Path, required=True)
    parser.add_argument("--out", type=Path, required=True)
    parser.add_argument("--exporter", type=Path, default=ROOT / "target/release/offline-presentation")
    parser.add_argument("--jobs", type=int, default=4)
    parser.add_argument("--max-frames", type=int, default=100_000)
    parser.add_argument("--prepare-only", action="store_true")
    args = parser.parse_args()
    require(1 <= args.jobs <= 8, "jobs must be between 1 and 8")
    base = read_json(args.base_manifest)
    args.out.mkdir(parents=True, exist_ok=True)
    groups = prepare(base, args.out, args.jobs)
    print(f"Prepared {sum(map(len, groups))} travel chapters in {len(groups)} batches", flush=True)
    if args.prepare_only:
        return
    require(args.exporter.is_file() and args.assets.is_dir(), "missing exporter or asset directory")
    with ThreadPoolExecutor(max_workers=args.jobs) as pool:
        pending = [pool.submit(render_worker, index, group, args.assets.resolve(),
                               args.exporter.resolve(), args.out.resolve(), args.max_frames)
                   for index, group in enumerate(groups) if group]
        results = [future.result() for future in pending]
    summary = dict(schema=1, base_manifest=str(args.base_manifest.resolve()),
                   exporter_sha256=digest(args.exporter), requested=sum(map(len, groups)),
                   completed=sum(result["completed"] for result in results), workers=results)
    save_json(args.out / "summary.json", summary)
    print(json.dumps(dict(requested=summary["requested"], completed=summary["completed"],
                          failures=sum(len(result["failures"]) for result in results))), flush=True)
    require(all(result["exit_code"] == 0 for result in results), "travel recapture incomplete")


if __name__ == "__main__":
    main()
