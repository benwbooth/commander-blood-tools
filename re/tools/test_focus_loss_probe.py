#!/usr/bin/env python3
"""Unit tests for the DOS focus-loss runtime probe."""
from __future__ import annotations

import importlib.util
import sys
import unittest
from pathlib import Path


PROBE_PATH = Path(__file__).with_name("focus_loss_probe.py")
SPEC = importlib.util.spec_from_file_location("focus_loss_probe", PROBE_PATH)
assert SPEC is not None and SPEC.loader is not None
probe = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = probe
SPEC.loader.exec_module(probe)


def runtime_report(
    sample: int = 7,
    timer_tick: int = 100,
    mouse_x: int = 160,
    mouse_y: int = 100,
) -> dict[str, object]:
    return {
        "runtime_samples": [
            {
                "sample": sample,
                "audio_flow": {
                    "timer_tick": timer_tick,
                    "timer_hook_active": 1,
                    "game_mode": 0,
                },
                "presentation_flow": {
                    "mouse_x": mouse_x,
                    "mouse_y": mouse_y,
                },
            }
        ]
    }


class RuntimeEvidenceTests(unittest.TestCase):
    def test_extracts_guarded_runtime_point(self) -> None:
        self.assertEqual(
            probe.latest_runtime_point(runtime_report()),
            {
                "sample": 7,
                "timer_tick": 100,
                "timer_hook_active": 1,
                "game_mode": 0,
                "mouse_x": 160,
                "mouse_y": 100,
            },
        )

    def test_timer_delta_accepts_sixteen_bit_wrap(self) -> None:
        self.assertEqual(probe.timer_delta(0xFFFE, 3), 5)

    def test_accepts_progress_while_unfocused_and_after_restore(self) -> None:
        before = probe.latest_runtime_point(runtime_report(10, 0xFFFE))
        unfocused = probe.latest_runtime_point(runtime_report(20, 3))
        restored = probe.latest_runtime_point(runtime_report(30, 17))
        self.assertEqual(
            probe.validate_runtime_points(before, unfocused, restored), []
        )

    def test_rejects_stopped_timer_and_stopped_sampling(self) -> None:
        before = probe.latest_runtime_point(runtime_report(10, 100))
        unfocused = probe.latest_runtime_point(runtime_report(10, 100))
        restored = probe.latest_runtime_point(runtime_report(11, 100))
        errors = probe.validate_runtime_points(before, unfocused, restored)
        self.assertIn(
            "watchdog sampling stopped while the game was unfocused", errors
        )
        self.assertIn("guest timer stopped while the game was unfocused", errors)
        self.assertIn("guest timer stopped after game focus was restored", errors)

    def test_rejects_invalid_mouse_and_timer_service_state(self) -> None:
        before = probe.latest_runtime_point(runtime_report(10, 100, -1, 200))
        unfocused = probe.latest_runtime_point(runtime_report(20, 110))
        restored = probe.latest_runtime_point(runtime_report(30, 120))
        before["timer_hook_active"] = 0
        before["game_mode"] = 1
        errors = probe.validate_runtime_points(before, unfocused, restored)
        self.assertIn("before sample has no active timer hook", errors)
        self.assertIn(
            "before sample is not in interrupt-driven game mode", errors
        )
        self.assertIn("before sample has an invalid guest mouse position", errors)

    def test_rejects_incomplete_runtime_sample(self) -> None:
        with self.assertRaisesRegex(probe.FocusProbeError, "no flow state"):
            probe.latest_runtime_point(
                {"runtime_samples": [{"sample": 1}]}
            )


if __name__ == "__main__":
    unittest.main()
