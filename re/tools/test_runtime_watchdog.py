#!/usr/bin/env python3
"""Deterministic tests for the DOS runtime watchdog's structural checks."""
from __future__ import annotations

import importlib.util
import struct
import sys
import tempfile
import unittest
from pathlib import Path


WATCHDOG_PATH = Path(__file__).with_name("runtime_watchdog.py")
SPEC = importlib.util.spec_from_file_location("runtime_watchdog", WATCHDOG_PATH)
assert SPEC is not None and SPEC.loader is not None
watchdog = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = watchdog
SPEC.loader.exec_module(watchdog)


MEMORY_SIZE = 0x100000
CHAIN_START = 0x0100
PROGRAM_MCB = 0x0200
PSP = PROGRAM_MCB + 1
FINAL_MCB = 0x0301


def put_mcb(
    memory: bytearray,
    segment: int,
    kind: str,
    owner: int,
    paragraphs: int,
    name: bytes = b"",
) -> None:
    address = segment * 16
    memory[address] = ord(kind)
    struct.pack_into("<HH", memory, address + 1, owner, paragraphs)
    memory[address + 8 : address + 16] = name.ljust(8, b"\0")


def valid_memory() -> bytearray:
    memory = bytearray(MEMORY_SIZE)
    put_mcb(memory, CHAIN_START, "M", 8, 0x00FF, b"DOS")
    put_mcb(memory, PROGRAM_MCB, "M", PSP, 0x0100, b"BPRG_RE")
    put_mcb(memory, FINAL_MCB, "Z", 0, 0x0010)
    return memory


class McbChainTests(unittest.TestCase):
    def test_discovers_complete_chain_and_game_owned_segment(self) -> None:
        blocks = watchdog.discover_mcb_chain(valid_memory(), PROGRAM_MCB, PSP)
        self.assertEqual([block.segment for block in blocks], [0x0100, 0x0200, 0x0301])
        owner = watchdog.program_owned_block(blocks, 0x0250, PSP)
        self.assertIsNotNone(owner)
        self.assertEqual(owner.segment, PROGRAM_MCB)

    def test_rejects_corrupt_successor_signature(self) -> None:
        memory = valid_memory()
        memory[FINAL_MCB * 16] = 0
        with self.assertRaisesRegex(watchdog.McbError, "invalid MCB type"):
            watchdog.parse_mcb_chain(memory, CHAIN_START, PROGRAM_MCB)

    def test_rejects_wrong_program_owner(self) -> None:
        memory = valid_memory()
        struct.pack_into("<H", memory, PROGRAM_MCB * 16 + 1, 0x9999)
        with self.assertRaisesRegex(watchdog.McbError, "is not owned by PSP"):
            watchdog.discover_mcb_chain(memory, PROGRAM_MCB, PSP)

    def test_rejects_chain_that_omits_program_header(self) -> None:
        with self.assertRaisesRegex(watchdog.McbError, "omits required header"):
            watchdog.parse_mcb_chain(valid_memory(), FINAL_MCB, PROGRAM_MCB)


class LinkLayoutTests(unittest.TestCase):
    def test_reads_zero_based_runtime_segments(self) -> None:
        text = (
            "GAME_DATA FAR_DATA DGROUP 105b:0000 00007c78\n"
            "FS_DATA FAR_DATA AUTO 1823:0000 00001230\n"
        )
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "link.map"
            path.write_text(text, encoding="ascii")
            self.assertEqual(
                watchdog.parse_segment_layout(path),
                watchdog.SegmentLayout(game_data=0x105B, fs_data=0x1823),
            )

    def test_rejects_nonzero_segment_base(self) -> None:
        text = (
            "GAME_DATA FAR_DATA DGROUP 105b:0000 00007c78\n"
            "FS_DATA FAR_DATA AUTO 1823:0008 00001230\n"
        )
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "link.map"
            path.write_text(text, encoding="ascii")
            with self.assertRaisesRegex(watchdog.WatchdogError, "not zero"):
                watchdog.parse_segment_layout(path)


class InterruptVectorTests(unittest.TestCase):
    def test_reports_only_changed_vectors(self) -> None:
        before = bytearray(0x400)
        after = bytearray(before)
        struct.pack_into("<HH", before, 0x08 * 4, 0x1234, 0x5678)
        struct.pack_into("<HH", after, 0x08 * 4, 0x1111, 0x2222)
        self.assertEqual(
            watchdog.changed_interrupt_vectors(before, after),
            [
                {
                    "vector": "0x08",
                    "before": "5678:1234",
                    "after": "2222:1111",
                }
            ],
        )


if __name__ == "__main__":
    unittest.main()
