#!/usr/bin/env python3

from __future__ import annotations

import hashlib
import json
import os
from pathlib import Path
import sys

_HERE = os.path.dirname(os.path.abspath(__file__))
sys.path[:] = [path for path in sys.path if os.path.abspath(path or os.curdir) != _HERE]

import unittest  # noqa: E402


ROOT = Path(__file__).resolve().parents[2]
REPORT_PATH = ROOT / "re/big_bug_bang_oracle_coverage.json"
GRAPH_PATH = ROOT / "re/big_bug_bang_expanded_func_graph.json"
RUNNER_PATH = ROOT / "re/tools/run_big_bug_bang_oracle_with_coverage.py"
COLLECTOR_PATH = ROOT / "re/tools/collect_big_bug_bang_oracle_coverage.py"
DYNAMIC_ENTRYPOINTS = {0xE0ED}


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


class BigBugBangOracleCoverageTests(unittest.TestCase):
    def setUp(self) -> None:
        self.report = json.loads(REPORT_PATH.read_text())
        self.graph = json.loads(GRAPH_PATH.read_text())

    def test_report_partitions_every_known_entrypoint_once(self) -> None:
        static_entries = set(self.graph["funcs"])
        expected = static_entries | DYNAMIC_ENTRYPOINTS
        rows = self.report["entrypoints"]
        actual = [int(row["entry"], 16) for row in rows]

        self.assertEqual(actual, sorted(expected))
        self.assertEqual(len(actual), len(set(actual)))
        for row in rows:
            entry = int(row["entry"], 16)
            self.assertEqual(
                row["origin"],
                "static_graph" if entry in static_entries else "runtime_vector",
            )
            self.assertEqual(row["entered"], bool(row["oracles"]))

        summary = self.report["summary"]
        entered_static = {
            int(row["entry"], 16)
            for row in rows
            if row["origin"] == "static_graph" and row["entered"]
        }
        entered_dynamic = {
            int(row["entry"], 16)
            for row in rows
            if row["origin"] == "runtime_vector" and row["entered"]
        }
        self.assertEqual(summary["static_entrypoint_count"], len(static_entries))
        self.assertEqual(summary["known_entrypoint_count"], len(expected))
        self.assertEqual(summary["entered_static_entrypoints"], len(entered_static))
        self.assertEqual(
            summary["unentered_static_entrypoints"],
            len(static_entries - entered_static),
        )
        self.assertEqual(summary["entered_dynamic_entrypoints"], len(entered_dynamic))

    def test_report_tracks_every_oracle_and_its_current_source(self) -> None:
        expected = {
            str(path.relative_to(ROOT)): sha256(path)
            for path in (ROOT / "re/tools").glob("big_bug_bang_*_oracle.py")
        }
        actual = {row["oracle"]: row["oracle_sha256"] for row in self.report["oracles"]}
        self.assertEqual(actual, expected)
        self.assertEqual(self.report["summary"]["oracle_count"], len(expected))
        fixtures = 0
        for row in self.report["oracles"]:
            fixture = row["fixture"]
            if fixture is None:
                self.assertIsNone(row["fixture_sha256"])
                self.assertIsNone(row["fixture_relation"])
                continue
            fixtures += 1
            fixture_path = ROOT / fixture
            self.assertTrue(fixture_path.is_file())
            self.assertEqual(row["fixture_sha256"], sha256(fixture_path))
            self.assertIn(row["fixture_relation"], {"exact", "prefix"})
        self.assertEqual(self.report["summary"]["oracle_fixture_count"], fixtures)
        self.assertEqual(
            self.report["summary"]["fixture_relation_counts"],
            {"exact": 133, "prefix": 1},
        )
        self.assertEqual(self.report["inputs"]["runner"]["sha256"], sha256(RUNNER_PATH))
        self.assertEqual(
            self.report["inputs"]["collector"]["sha256"], sha256(COLLECTOR_PATH)
        )
        self.assertEqual(
            self.report["inputs"]["static_graph"]["sha256"], sha256(GRAPH_PATH)
        )


if __name__ == "__main__":
    unittest.main()
