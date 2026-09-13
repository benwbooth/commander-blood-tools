#!/usr/bin/env python3

from __future__ import annotations

import importlib.util
from pathlib import Path
import sys
import unittest


TOOL_PATH = Path(__file__).with_name("compare_big_bug_bang_expanded_graphs.py")
SPEC = importlib.util.spec_from_file_location(
    "compare_big_bug_bang_expanded_graphs", TOOL_PATH
)
assert SPEC is not None and SPEC.loader is not None
EXPANDED = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = EXPANDED
SPEC.loader.exec_module(EXPANDED)


class ExpandedGraphComparisonTests(unittest.TestCase):
    def test_near_table_roots_preserve_the_code_segment(self):
        class SyntheticMZ:
            header_size = 0x20
            data = bytes.fromhex("00" * 0x40 + "04 00 08 00")

        self.assertEqual(
            {0x64: 4, 0x68: 4},
            EXPANDED.near_table_roots(SyntheticMZ(), 0x40, 2, 0x60),
        )


if __name__ == "__main__":
    unittest.main()
