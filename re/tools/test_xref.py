#!/usr/bin/env python3
from __future__ import annotations

import importlib.util
from pathlib import Path
import sys
import unittest


TOOL = Path(__file__).with_name("xref.py")
SPEC = importlib.util.spec_from_file_location("xref", TOOL)
assert SPEC is not None and SPEC.loader is not None
xref = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = xref
SPEC.loader.exec_module(xref)


class FakeMz:
    def __init__(self, data: bytes, segment_base: int) -> None:
        self.data = data
        self.segment_base = segment_base

    def segoff_to_file(self, _segment: int, offset: int) -> int:
        return self.segment_base + offset


class RelativeBranchTests(unittest.TestCase):
    def test_finds_every_supported_relative_transfer_encoding(self) -> None:
        segment_base = 2
        segment = bytearray(40)
        segment[0:3] = bytes.fromhex("e81d00")
        segment[3:6] = bytes.fromhex("e91a00")
        segment[6:8] = bytes.fromhex("7518")
        segment[8:10] = bytes.fromhex("eb16")
        segment[10:12] = bytes.fromhex("e214")
        segment[12:16] = bytes.fromhex("0f851000")
        mz = FakeMz(b"\xaa\xbb" + bytes(segment), segment_base)

        self.assertEqual(
            xref.relative_branches(mz, 0x1234, 0x20),
            [
                (2, "call near"),
                (5, "jmp near"),
                (8, "short branch"),
                (10, "short branch"),
                (12, "short branch"),
                (14, "conditional near"),
            ],
        )

    def test_rejects_scan_end_outside_the_image(self) -> None:
        mz = FakeMz(bytes(16), 4)
        with self.assertRaises(ValueError):
            xref.relative_branches(mz, 0x1234, 0, end_file=3)
        with self.assertRaises(ValueError):
            xref.relative_branches(mz, 0x1234, 0, end_file=17)


if __name__ == "__main__":
    unittest.main()
