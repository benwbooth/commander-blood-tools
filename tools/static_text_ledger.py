#!/usr/bin/env python3
# /// script
# requires-python = ">=3.11"
# dependencies = []
# ///
"""Merge every prepared-text batch into one ledger covering all inventory sites.

Each site gets exactly one status: verified_prepared_scene (with the batch that
verified it), failed (every retained error), or deferred (planner reason). A
verified scene is never gameplay coverage. Batches are read, never modified.
"""

import argparse
from collections import Counter
from pathlib import Path

from native_sequence_anthology import read_json, require
from video_anthology import digest, save_json


def build(inventory_path, plan_set, batches, out):
    require(not out.exists(), f"output already exists: {out}")
    inventory = read_json(inventory_path)
    planning = read_json(plan_set)
    require(planning["inventory_sha256"] == digest(inventory_path), "plan set belongs to another inventory")
    reasons = {row["id"]: row for row in planning["sites"]}
    sites = {site["id"]: site for site in inventory["sites"]}
    require(set(reasons) == set(sites), "plan set does not cover the inventory exactly")
    verified, failed = {}, {}
    for batch in batches:
        coverage = read_json(batch / "coverage.json")
        for entry in coverage["completed"]:
            require(entry["status"] == "verified_prepared_scene", "unexpected completed status")
            verified[entry["id"]] = dict(batch=batch.name, path=entry["path"], master_sha256=entry["master_sha256"],
                                         duration_ns=entry["duration_ns"], audio_samples=entry["audio_samples"],
                                         record=entry["record"])
        for entry in coverage["failures"]:
            failed.setdefault(entry["id"], []).append(dict(batch=batch.name, error=entry["error"]))
    rows, counts = [], Counter()
    for identity in sorted(sites):
        if identity in verified:
            status, detail = "verified_prepared_scene", verified[identity]
        elif identity in failed:
            status, detail = "failed", failed[identity]
        elif reasons[identity]["status"] == "deferred":
            status, detail = "deferred", dict(reason=reasons[identity]["reason"])
        else:
            status, detail = "planned_not_rendered", {}
        counts[(sites[identity]["game"], status)] += 1
        rows.append(dict(id=identity, status=status, gameplay_reachability="not_assessed", **{"detail": detail}))
    ledger = dict(schema=1, scope="prepared static text scenes; not gameplay coverage",
                  inventory_sha256=digest(inventory_path), plan_set_sha256=digest(plan_set),
                  counts={f"{game}.{status}": n for (game, status), n in sorted(counts.items())}, sites=rows)
    save_json(out, ledger)
    print("\n".join(f"{key}: {n}" for key, n in ledger["counts"].items()))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--inventory", type=Path, required=True)
    parser.add_argument("--plan-set", type=Path, required=True)
    parser.add_argument("--batch", type=Path, action="append", required=True)
    parser.add_argument("--out", type=Path, required=True)
    args = parser.parse_args()
    build(args.inventory, args.plan_set, args.batch, args.out)


if __name__ == "__main__":
    main()
