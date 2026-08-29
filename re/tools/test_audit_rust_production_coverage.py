#!/usr/bin/env python3
from __future__ import annotations

import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))

import audit_rust_production_coverage as audit


class RustProductionCoverageAuditTests(unittest.TestCase):
    def test_correlates_qualified_symbols_only_inside_their_ledger_file(self) -> None:
        workspace = Path("/workspace")
        lcov = """\
SF:/workspace/crates/game/src/native/example.rs
FNDA:7,_RNvExampleType4step
FNDA:3,_RNvOtherType4step
FNDA:11,_RNv11free_helper
end_of_record
SF:/workspace/crates/game/src/native/other.rs
FNDA:29,_RNvExampleType4step
end_of_record
"""
        rows = [
            {
                "component": "bloodprg",
                "entry": "0x000001",
                "function": "native_method",
                "rust_path": "crates/game/src/native/example.rs",
                "rust_symbol": "ExampleType::step",
            },
            {
                "component": "bloodprg",
                "entry": "0x000002",
                "function": "native_free",
                "rust_path": "crates/game/src/native/example.rs",
                "rust_symbol": "free_helper",
            },
        ]

        result = audit.audit_rows(rows, audit.load_lcov(lcov, workspace))

        self.assertEqual(result[0]["execution_count"], 7)
        self.assertEqual(result[0]["instrumented_instances"], 1)
        self.assertEqual(result[1]["execution_count"], 11)

    def test_retains_zero_count_instrumented_functions_as_uncovered(self) -> None:
        workspace = Path("/workspace")
        lcov = """\
SF:/workspace/native.rs
FNDA:0,_RNv12unused_entry
end_of_record
"""
        rows = [
            {
                "component": "xdb_manu3",
                "entry": "0x000150",
                "function": "native_unused",
                "rust_path": "native.rs",
                "rust_symbol": "unused_entry",
            }
        ]

        result = audit.audit_rows(rows, audit.load_lcov(lcov, workspace))

        self.assertEqual(result[0]["execution_count"], 0)
        self.assertEqual(result[0]["instrumented_instances"], 1)


if __name__ == "__main__":
    unittest.main()
