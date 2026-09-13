#!/usr/bin/env python3

from __future__ import annotations

import importlib.util
from pathlib import Path
import sys
import unittest


TOOL_PATH = Path(__file__).with_name("big_bug_bang_indirect_dispatch_atlas.py")
SPEC = importlib.util.spec_from_file_location(
    "big_bug_bang_indirect_dispatch_atlas", TOOL_PATH
)
assert SPEC is not None and SPEC.loader is not None
ATLAS = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = ATLAS
SPEC.loader.exec_module(ATLAS)


class BigBugBangIndirectDispatchAtlasTests(unittest.TestCase):
    def test_every_boundary_category_is_distinct(self):
        records = [
            ["0x23cb", "call", "word ptr cs:[bx + 0x1412]"],
            ["0x10fa", "lcall", "0, 0x709"],
            ["0xce9", "lcall", "gs:[0xc42]"],
            ["0x185c", "lcall", "[0xc8e]"],
            ["0xcf66", "lcall", "[0xf21]"],
        ]
        self.assertEqual(
            [
                "static_internal_dispatch_table",
                "direct_far_segment0_runtime_call",
                "external_xms_driver_vector",
                "dynamic_presentation_callback_vector",
                "external_sound_driver_vector",
            ],
            [row["category"] for row in ATLAS.classify_indirect(records)],
        )

    def test_all_static_dispatch_sites_have_one_table_owner(self):
        sites = [site for table in ATLAS.TABLES for site in table["dispatch_sites"]]
        self.assertEqual(len(sites), len(set(sites)))
        self.assertEqual(set(sites), set(ATLAS.STATIC_SITE_TO_TABLE))


if __name__ == "__main__":
    unittest.main()
