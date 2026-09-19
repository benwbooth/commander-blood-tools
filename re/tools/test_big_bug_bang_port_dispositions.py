#!/usr/bin/env python3

from __future__ import annotations

from collections import Counter
import csv
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import sys

_HERE = os.path.dirname(os.path.abspath(__file__))
sys.path[:] = [path for path in sys.path if os.path.abspath(path or os.curdir) != _HERE]

import unittest  # noqa: E402


ROOT = Path(__file__).resolve().parents[2]
LEDGER_PATH = ROOT / "re/big_bug_bang_port_dispositions.json"
COVERAGE_PATH = ROOT / "re/big_bug_bang_oracle_coverage.json"
UNREFERENCED_LIBRARY_AUDIT_PATH = (
    ROOT / "re/big_bug_bang_unreferenced_library_audit.json"
)
BUILDER_PATH = ROOT / "re/tools/build_big_bug_bang_port_dispositions.py"
KNOWN_ENTRYPOINT_COUNT = 383
CLASSIFIED_ENTRYPOINT_COUNT = 377
PENDING_GAME_SEMANTICS_COUNT = 6
EXPECTED_STATUS_COUNTS = {
    "eliminated_authored_no_operation": 10,
    "eliminated_dormant_diagnostic": 18,
    "eliminated_host_adapter": 26,
    "eliminated_static_host_adapter": 2,
    "eliminated_unreferenced_library": 3,
    "inherited_exact_eliminated": 7,
    "inherited_exact_typed": 6,
    "pending_game_semantics": PENDING_GAME_SEMANTICS_COUNT,
    "verified_direct_typed": 262,
    "verified_static_typed": 43,
}

spec = importlib.util.spec_from_file_location("bbb_disposition_builder", BUILDER_PATH)
assert spec is not None and spec.loader is not None
builder = importlib.util.module_from_spec(spec)
spec.loader.exec_module(builder)


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def commander_rows(path: Path) -> dict[int, dict[str, str]]:
    with path.open(newline="") as stream:
        return {
            int(row["entry"], 16): row
            for row in csv.DictReader(stream, delimiter="\t")
            if row["component"] == "bloodprg"
        }


class BigBugBangPortDispositionTests(unittest.TestCase):
    def setUp(self) -> None:
        self.ledger = json.loads(LEDGER_PATH.read_text())
        self.coverage = json.loads(COVERAGE_PATH.read_text())
        self.coverage_rows = {row["entry"]: row for row in self.coverage["entrypoints"]}
        self.oracle_rows = {row["oracle"]: row for row in self.coverage["oracles"]}
        self.unreferenced_library = {
            row["entry"]
            for row in json.loads(UNREFERENCED_LIBRARY_AUDIT_PATH.read_text())[
                "routines"
            ]
        }

    def test_checked_in_ledger_is_current(self) -> None:
        self.assertEqual(self.ledger, builder.build_report())
        for path, digest in self.ledger["inputs"].items():
            self.assertEqual(digest, sha256(ROOT / path), path)

    def test_input_only_fixture_reuse_is_not_native_output_evidence(self) -> None:
        for row in self.ledger["entrypoints"]:
            if row["entry"] in {"0x706e", "0x728b"}:
                self.assertEqual(row["rust_owner"]["symbol"],
                                 "sequel_growth_matches_complete_native_handler_and_selection_helper")

    def test_ledger_partitions_every_known_entrypoint_once(self) -> None:
        rows = self.ledger["entrypoints"]
        entries = [row["entry"] for row in rows]
        self.assertEqual(entries, list(self.coverage_rows))
        self.assertEqual(len(entries), len(set(entries)))

        counts = Counter(row["status"] for row in rows)
        summary = self.ledger["summary"]
        self.assertEqual(len(rows), KNOWN_ENTRYPOINT_COUNT)
        self.assertEqual(dict(sorted(counts.items())), EXPECTED_STATUS_COUNTS)
        self.assertEqual(
            summary["classified_entrypoint_count"], CLASSIFIED_ENTRYPOINT_COUNT
        )
        self.assertEqual(
            summary["pending_game_semantics_count"], PENDING_GAME_SEMANTICS_COUNT
        )
        self.assertEqual(summary["known_entrypoint_count"], len(rows))
        self.assertEqual(summary["status_counts"], dict(sorted(counts.items())))
        self.assertEqual(
            summary["pending_game_semantics_count"],
            counts["pending_game_semantics"],
        )
        self.assertEqual(
            summary["classified_entrypoint_count"],
            len(rows) - counts["pending_game_semantics"],
        )

        for row in rows:
            coverage = self.coverage_rows[row["entry"]]
            for key in ("origin", "entered", "comparison"):
                self.assertEqual(row[key], coverage[key], (row["entry"], key))
            self.assertTrue(row["evidence"])
            self.assertTrue(row["rationale"].strip())

    def test_every_nonpending_disposition_has_its_required_proof(self) -> None:
        ported = commander_rows(ROOT / "re/rust-port/ported.tsv")
        eliminated = commander_rows(ROOT / "re/rust-port/eliminated.tsv")
        executable = (ROOT / "output/big-bug-bang/disc/BLOOD2PG.EXE").read_bytes()

        for row in self.ledger["entrypoints"]:
            entry = int(row["entry"], 16)
            status = row["status"]
            if status == "pending_game_semantics":
                self.assertIsNone(row["rust_owner"])
            elif status == "verified_direct_typed":
                self.assertTrue(row["entered"])
                oracle_path, fixture = row["evidence"]
                self.assertIn(oracle_path, self.coverage_rows[row["entry"]]["oracles"])
                self.assertEqual(self.oracle_rows[oracle_path]["fixture"], fixture)
                self.assert_rust_owner_consumes(row["rust_owner"], fixture)
            elif status in {"verified_static_typed", "eliminated_static_host_adapter"}:
                audit_path, fixture = row["evidence"]
                audit = json.loads((ROOT / audit_path).read_text())
                static = next(item for item in audit["routines"] if item["entry"] == row["entry"])
                self.assertEqual(static["rust_owner"], row["rust_owner"])
                self.assertEqual(static["inherited_fixture"] or static["rust_owner"]["path"], fixture)
                self.assertEqual(static["body_sha256"], hashlib.sha256(
                    executable[entry:int(static["end"], 16)]).hexdigest())
                self.assert_rust_owner_exists(row["rust_owner"])
            elif status in {"inherited_exact_typed", "inherited_exact_eliminated"}:
                self.assertEqual(row["comparison"], "exact_body")
                commander_entry = int(row["commander_entry"], 16)
                source = ported if status == "inherited_exact_typed" else eliminated
                commander = source[commander_entry]
                self.assertEqual(row["rust_owner"]["path"], commander["rust_path"])
                self.assertEqual(row["rust_owner"]["symbol"], commander["rust_symbol"])
                self.assert_rust_owner_exists(row["rust_owner"])
            elif status == "eliminated_host_adapter":
                self.assertTrue(row["entered"])
                self.assertIn(
                    row["evidence"][0], self.coverage_rows[row["entry"]]["oracles"]
                )
                self.assert_rust_owner_exists(row["rust_owner"])
            elif status == "eliminated_dormant_diagnostic":
                self.assertEqual(
                    row["evidence"][0], "re/big_bug_bang_field_display_audit.json"
                )
                self.assertIsNone(row["rust_owner"])
            elif status == "eliminated_unreferenced_library":
                self.assertEqual(
                    row["evidence"],
                    [str(UNREFERENCED_LIBRARY_AUDIT_PATH.relative_to(ROOT))],
                )
                self.assertIn(row["entry"], self.unreferenced_library)
                self.assertIsNone(row["rust_owner"])
            elif status == "eliminated_authored_no_operation":
                body = bytes.fromhex(row["body_hex"])
                self.assertEqual(executable[entry : entry + len(body)], body)
                self.assertIsNone(row["rust_owner"])
            else:
                self.fail(f"unsupported disposition {status!r} at {entry:#x}")

    def assert_rust_owner_exists(self, owner: dict[str, str]) -> None:
        source = (ROOT / owner["path"]).read_text()
        function = owner["symbol"].rsplit("::", 1)[-1]
        self.assertIn(f"fn {function}", source)

    def assert_rust_owner_consumes(self, owner: dict[str, str], fixture: str) -> None:
        source = (ROOT / owner["path"]).read_text()
        function = owner["symbol"].rsplit("::", 1)[-1]
        self.assertIn(f"fn {function}", source)
        self.assertIn(Path(fixture).name, source)


if __name__ == "__main__":
    unittest.main()
