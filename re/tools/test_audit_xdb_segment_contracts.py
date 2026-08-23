#!/usr/bin/env python3
from __future__ import annotations

import importlib.util
import sys
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location(
    "audit_xdb_segment_contracts",
    ROOT / "re/tools/audit_xdb_segment_contracts.py",
)
MODULE = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = MODULE
SPEC.loader.exec_module(MODULE)


class XdbSegmentContractTests(unittest.TestCase):
    def test_alien_api_starts_with_foreign_data_segments(self):
        state = MODULE.initial_segments("amer", "func_000000_api_entry")
        self.assertEqual(state["ds"], MODULE.CORE.UNKNOWN)
        self.assertEqual(state["fs"], MODULE.CORE.UNKNOWN)
        self.assertEqual(state["ss"], "STACK")

    def test_alien_callback_starts_with_installed_xdb_data(self):
        state = MODULE.initial_segments("scrut", "func_000b78_slot1_wave_update")
        self.assertEqual(state["ds"], "XDB_DATA")
        self.assertEqual(state["fs"], "XDB_DATA")
        self.assertEqual(state["es"], MODULE.CORE.UNKNOWN)

    def test_alien_main_must_prove_its_own_data_install(self):
        state = MODULE.initial_segments("croolis", "func_0000a3_main")
        self.assertEqual(state["ds"], MODULE.CORE.UNKNOWN)
        self.assertEqual(state["fs"], MODULE.CORE.UNKNOWN)

    def test_manu3_entry_inherits_data_from_loader_shim(self):
        state = MODULE.initial_segments("manu3", "func_000000_api_entry")
        self.assertEqual(state["ds"], "XDB_DATA")
        self.assertEqual(state["fs"], "XDB_DATA")


if __name__ == "__main__":
    unittest.main()
