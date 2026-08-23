#!/usr/bin/env python3
from __future__ import annotations

import copy
import importlib.util
import sys
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location(
    "compare_runtime_traces", ROOT / "re/tools/compare_runtime_traces.py"
)
MODULE = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = MODULE
SPEC.loader.exec_module(MODULE)


def record() -> dict:
    semantic = {
        "vm": {
            "resource_profile": 1,
            "profile_request": -1,
            "execution_enabled": 1,
            "active_line": 4,
            "displayed_line": 4,
        },
        "presentation": {
            "ui_flags": 0,
            "actor_transition": 0,
            "bridge_frame": 45,
            "mode": 0,
            "box_mode": 0,
            "word_choice_active": 1,
            "nav_target_selection": 0,
            "active": 1,
            "defer": 0,
            "text_wait": 2,
            "text_display_active": 0,
            "waiting_for_input": True,
        },
        "input": {
            "mouse_x": 225,
            "mouse_y": 61,
            "buttons": 0,
            "previous_buttons": 0,
            "primary_pressed": 0,
            "press_pending": 0,
        },
        "audio": {
            "driver_pending": 2,
            "stream_mode": 0,
            "stream_channel": 1,
            "dialogue_delay": 0,
            "dialogue_hold": 0,
            "clip_playback_state": 0,
            "last_clip": 1,
            "streamed_clip_count": 4,
            "events": [[1024, 11025]],
        },
        "subtitle": "Okay Okay wise guy",
        "persistent": {
            "state_array_hash": "state",
            "character_slots_hash": "characters",
            "record_hash": "records",
        },
        "assets": ["D:\\SCRIPT2.COD"],
        "video": {"screen_hash": "screen", "palette_hash": "palette"},
    }
    return {
        "schema": 1,
        "action_index": 1,
        "phase": "after",
        "action": "click 225 61",
        "steps": 10,
        "guest_end": None,
        "liveness": "progress",
        "semantic": semantic,
    }


class RuntimeTraceComparisonTests(unittest.TestCase):
    def test_equivalent_semantics_ignore_instruction_count(self):
        original = record()
        rebuilt = copy.deepcopy(original)
        rebuilt["steps"] = 99
        self.assertEqual(
            MODULE.compare_records([original], [rebuilt])["status"],
            "equivalent",
        )

    def test_reports_first_semantic_divergence(self):
        original = record()
        rebuilt = copy.deepcopy(original)
        rebuilt["semantic"]["subtitle"] = "wrong line"
        report = MODULE.compare_records([original], [rebuilt])
        self.assertEqual(report["status"], "diverged")
        self.assertEqual(
            report["first_divergence"]["differences"][0]["path"],
            "semantic.subtitle",
        )

    def test_rejects_unconsumed_input_even_when_state_matches(self):
        original = record()
        rebuilt = copy.deepcopy(original)
        rebuilt["liveness"] = "input_not_consumed"
        report = MODULE.compare_records([original], [rebuilt])
        self.assertEqual(report["status"], "diverged")
        self.assertEqual(
            report["first_divergence"]["differences"][0]["path"],
            "rebuilt.liveness",
        )


if __name__ == "__main__":
    unittest.main()
