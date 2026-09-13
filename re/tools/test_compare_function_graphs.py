#!/usr/bin/env python3

from __future__ import annotations

import importlib.util
import os
from pathlib import Path
import sys
import unittest


_HERE = Path(__file__).resolve().parent
sys.path[:] = [
    path
    for path in sys.path
    if Path(os.path.abspath(path or os.curdir)) != _HERE
]

import capstone  # noqa: E402


TOOL_PATH = _HERE / "compare_function_graphs.py"
SPEC = importlib.util.spec_from_file_location("compare_function_graphs", TOOL_PATH)
assert SPEC is not None and SPEC.loader is not None
COMPARE = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = COMPARE
SPEC.loader.exec_module(COMPARE)


def decode(encoded: str, address: int):
    disassembler = capstone.Cs(capstone.CS_ARCH_X86, capstone.CS_MODE_16)
    disassembler.detail = True
    return next(disassembler.disasm(bytes.fromhex(encoded), address, count=1))


class FunctionGraphComparisonTests(unittest.TestCase):
    def test_local_branch_shape_is_relocation_independent(self):
        left = COMPARE.instruction_shape(decode("74 02", 0x100), 0x100)
        right = COMPARE.instruction_shape(decode("74 02", 0x500), 0x500)
        self.assertEqual(left, right)

    def test_call_destination_is_normalized(self):
        left = COMPARE.instruction_shape(decode("e8 34 12", 0x100), 0x100)
        right = COMPARE.instruction_shape(decode("e8 78 56", 0x100), 0x100)
        self.assertEqual(left, right)

    def test_address_like_immediate_is_normalized(self):
        left = COMPARE.instruction_shape(decode("b8 34 12", 0x100), 0x100)
        right = COMPARE.instruction_shape(decode("b8 78 56", 0x100), 0x100)
        self.assertEqual(left, right)

    def test_small_behavioral_immediate_is_preserved(self):
        left = COMPARE.instruction_shape(decode("b8 01 00", 0x100), 0x100)
        right = COMPARE.instruction_shape(decode("b8 02 00", 0x100), 0x100)
        self.assertNotEqual(left, right)

    def test_absolute_memory_address_is_normalized(self):
        left = COMPARE.instruction_shape(decode("a1 34 12", 0x100), 0x100)
        right = COMPARE.instruction_shape(decode("a1 78 56", 0x100), 0x100)
        self.assertEqual(left, right)


if __name__ == "__main__":
    unittest.main()
