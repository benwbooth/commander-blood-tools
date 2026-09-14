#!/usr/bin/env python3

from __future__ import annotations

import importlib.util
import json
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
TOOL_PATH = Path(__file__).with_name("big_bug_bang_unreferenced_library_audit.py")
REPORT_PATH = ROOT / "re/big_bug_bang_unreferenced_library_audit.json"

spec = importlib.util.spec_from_file_location(
    "bbb_unreferenced_library_audit", TOOL_PATH
)
assert spec is not None and spec.loader is not None
audit = importlib.util.module_from_spec(spec)
spec.loader.exec_module(audit)


class BigBugBangUnreferencedLibraryAuditTests(unittest.TestCase):
    def setUp(self) -> None:
        self.report = json.loads(REPORT_PATH.read_text())

    def test_checked_in_report_is_current(self) -> None:
        self.assertEqual(self.report, audit.build_report())

    def test_report_pins_only_isolated_unreferenced_formatters(self) -> None:
        self.assertEqual(
            [row["entry"] for row in self.report["routines"]],
            ["0x28b2", "0x28cc", "0x28e9"],
        )
        for row in self.report["routines"]:
            self.assertEqual(row["predecessor_opcode"], "0xcb")
            self.assertEqual(row["direct_callers"], [])
            self.assertEqual(row["direct_callees"], [])
            self.assertEqual(row["encoded_offset_hits"], [])


if __name__ == "__main__":
    unittest.main()
