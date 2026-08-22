#!/usr/bin/env python3

from __future__ import annotations

import io
import importlib.util
import struct
import sys
import unittest
from pathlib import Path


CAPTURE_PATH = Path(__file__).with_name("capture_pterra_boundary.py")
SPEC = importlib.util.spec_from_file_location(
    "capture_pterra_boundary", CAPTURE_PATH)
assert SPEC is not None and SPEC.loader is not None
capture = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = capture
SPEC.loader.exec_module(capture)
stable_scene_release_is_safe = capture.stable_scene_release_is_safe


class StableSceneReleaseTests(unittest.TestCase):
    def state(self, *, cs: int, ip: int) -> dict[str, int]:
        return {"cs": cs, "ip": ip, "ss": 0x1000, "sp": 0x0200}

    def memory_with_stack(self, *words: int) -> io.BytesIO:
        memory = bytearray(0x20000)
        stack = 0x1000 * 16 + 0x0200
        memory[stack:stack + len(words) * 2] = struct.pack(
            f"<{len(words)}H", *words)
        return io.BytesIO(memory)

    def test_rejects_timer_interrupt_over_resource_switch(self) -> None:
        game_segment = 0x187E
        code_segment = game_segment - 0x105B
        memory = self.memory_with_stack(0xC9F0)

        self.assertFalse(stable_scene_release_is_safe(
            memory, 0,
            self.state(cs=code_segment, ip=0x02A4),
            game_segment, "BPRG_RE.EXE"))

    def test_accepts_idle_game_stack(self) -> None:
        game_segment = 0x187E
        code_segment = game_segment - 0x105B
        memory = self.memory_with_stack(0x1234, 0x5678)

        self.assertTrue(stable_scene_release_is_safe(
            memory, 0,
            self.state(cs=code_segment, ip=0x1000),
            game_segment, "BPRG_RE.EXE"))

    def test_original_dos_call_is_not_releaseable(self) -> None:
        memory = self.memory_with_stack()

        self.assertFalse(stable_scene_release_is_safe(
            memory, 0,
            self.state(cs=0xF000, ip=0x1234),
            0x187E, "BLOODPRG.EXE"))


if __name__ == "__main__":
    unittest.main()
