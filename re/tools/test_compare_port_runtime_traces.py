#!/usr/bin/env python3
from __future__ import annotations

import copy
import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))

import compare_port_runtime_traces as compare_port


def record(action_index: int, frame: int = 45) -> dict:
    return {
        "schema": 1,
        "action_index": action_index,
        "action": "wait 10" if action_index else None,
        "guest_end": None,
        "liveness": "progress",
        "semantic": {
            "vm": {
                "resource_profile": 0,
                "profile_request": -1,
                "execution_enabled": 1,
                "active_line": 0xFFFF,
            },
            "presentation": {
                "ui_flags": 0x41,
                "bridge_frame": frame,
                "mode": 0,
                "box_mode": 0,
                "word_choice_active": 0,
                "nav_target_selection": 0,
                "active": 0,
                "defer": 0,
                "text_display_active": 0,
                "waiting_for_input": False,
            },
            "subtitle": "WAIT COMMANDER ...",
            "video": {
                "screen_hash": f"screen-{action_index}",
                "bridge_layers": {"sprite_hash": f"sprite-{action_index}"},
            },
        },
    }


class PortTraceComparisonTests(unittest.TestCase):
    def test_normalizes_flat_option_sentinels_ui_ownership_and_ring_tolerance(self) -> None:
        original = [record(0), record(1, 46)]
        modern = copy.deepcopy(original)
        modern[0]["semantic"]["vm"]["active_line"] = None
        modern[1]["semantic"]["presentation"]["ui_flags"] = 0x01
        modern[1]["semantic"]["presentation"]["bridge_frame"] = 44

        report = compare_port.compare_port_records(
            original, modern, start_action=0, bridge_frame_tolerance=2
        )

        self.assertEqual(report["status"], "equivalent")
        self.assertEqual(
            report["render_observations"]["modern_distinct_bridge_sprite_hashes"], 2
        )

    def test_reports_the_first_semantic_divergence(self) -> None:
        original = [record(0), record(1)]
        modern = copy.deepcopy(original)
        modern[1]["semantic"]["presentation"]["active"] = 1

        report = compare_port.compare_port_records(
            original, modern, start_action=0, bridge_frame_tolerance=2
        )

        self.assertEqual(report["status"], "diverged")
        self.assertEqual(report["first_divergence"]["action_index"], 1)
        self.assertEqual(
            report["first_divergence"]["differences"][0]["path"],
            "semantic.presentation.active",
        )


if __name__ == "__main__":
    unittest.main()
