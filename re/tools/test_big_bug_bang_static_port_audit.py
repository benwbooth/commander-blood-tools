#!/usr/bin/env python3
"""Regression guards for the exact, relocation-limited static port audit."""

from __future__ import annotations

import importlib.util
import json
import os
from pathlib import Path
import sys

HERE = Path(__file__).resolve().parent
sys.path[:] = [path for path in sys.path if Path(path or os.curdir).resolve() != HERE]
import unittest

spec = importlib.util.spec_from_file_location(
    "bbb_static_audit", HERE / "big_bug_bang_static_port_audit.py"
)
assert spec is not None and spec.loader is not None
audit = importlib.util.module_from_spec(spec)
spec.loader.exec_module(audit)


class StaticPortAuditTests(unittest.TestCase):
    def test_checked_report_is_current(self):
        report = audit.build_report()
        self.assertEqual(report, json.loads(audit.OUTPUT.read_text()))
        self.assertEqual(len(report["routines"]), 40)

    def test_unreviewed_changes_fail_closed(self):
        old = audit.COMMANDER.read_bytes()[0x4536:0x46B6]
        sequel = audit.BBB.read_bytes()[0x49B3:0x4B33]
        # Check every encoded byte, including branch displacements, flags,
        # remap addresses, widths, and all return paths, not just shape counts.
        for offset in range(len(sequel)):
            changed = bytearray(sequel)
            changed[offset] ^= 1
            with self.subTest(offset=offset), self.assertRaises(ValueError):
                audit.compare_body(old, bytes(changed), 0x4536, 0x49B3, "sprite")

    def test_hover_suffix_is_complete(self):
        old = audit.COMMANDER.read_bytes()[0x78D1:0x792D]
        sequel = audit.BBB.read_bytes()[0x892B:0x8987]
        instructions, patches = audit.compare_body(old, sequel, 0x78D1, 0x892B, "hover")
        self.assertEqual(sum(item.size for item in instructions), len(sequel))
        self.assertEqual(instructions[-1].mnemonic, "ret")
        self.assertTrue(patches)

    def test_palette_guard_has_complete_equivalent_suffix(self):
        original = audit.COMMANDER.read_bytes()[0x813B:0x817E]
        sequel = audit.BBB.read_bytes()[0x928D:0x92D0]
        instructions, _ = audit.compare_body(original, sequel, 0x813B, 0x928D, "bridge")
        self.assertEqual(sum(item.size for item in instructions), len(sequel))
        self.assertEqual(instructions[-1].mnemonic, "ret")

    def test_changed_actor_reviews_cover_every_instruction(self):
        bbb = audit.BBB.read_bytes()
        for entry, end, _, _, _, blocks in (
            *audit.ACTOR_REVIEWS,
            *audit.CONTROL_REVIEWS,
        ):
            boundaries = {
                item.address for item in audit.decode(bbb[entry:end], entry)
            } | {end}
            self.assertEqual(blocks[0][0], entry)
            self.assertEqual(blocks[-1][1], end)
            for start, stop, _ in blocks:
                self.assertIn(start, boundaries)
                self.assertIn(stop, boundaries)
            self.assertTrue(all(a[1] == b[0] for a, b in zip(blocks, blocks[1:])))


if __name__ == "__main__":
    unittest.main()
