#!/usr/bin/env python3
from __future__ import annotations

import importlib.util
import sys
import unittest
from collections import Counter
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location(
    "compare_segment_roles", ROOT / "re/tools/compare_segment_roles.py"
)
MODULE = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = MODULE
SPEC.loader.exec_module(MODULE)


class SegmentRoleComparisonTests(unittest.TestCase):
    def test_missing_role_is_reported(self):
        access = MODULE.Access("memseg:GAME_DATA:6726", "r", 2, "based", 4)
        rows = MODULE.compare("routine", Counter({access: 2}), Counter())
        self.assertEqual(len(rows), 1)
        self.assertEqual(rows[0].status, "missing_role")
        self.assertEqual(rows[0].original_count, 2)

    def test_register_allocation_does_not_enter_shape(self):
        left = MODULE.Access("argument", "r", 1, "based", 2)
        right = MODULE.Access("argument", "r", 1, "based", 2)
        rows = MODULE.compare("routine", Counter({left: 1}), Counter({right: 1}))
        self.assertEqual(rows[0].status, "exact")

    def test_shape_difference_is_advisory(self):
        before = MODULE.Access("constant:a000", "w", 1, "based", 0)
        after = MODULE.Access("constant:a000", "w", 2, "based", 0)
        rows = MODULE.compare(
            "routine", Counter({before: 1}), Counter({after: 1})
        )
        self.assertEqual(rows[0].status, "shape_difference")
        self.assertIn("w1", rows[0].missing_shapes)
        self.assertIn("w2", rows[0].extra_shapes)


if __name__ == "__main__":
    unittest.main()
