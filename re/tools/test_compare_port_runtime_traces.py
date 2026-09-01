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
                "logical_display_hash": f"screen-{action_index}",
                "indexed_rgb_hash": f"rgb-{action_index}",
                "logical_indexed_rgb_hash": f"rgb-{action_index}",
                "indexed_frame_authoritative": True,
                "queue_metrics": {
                    "sequence_index": action_index,
                    "read_wrap_index": action_index,
                },
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

    def test_reports_split_object_and_actor_layers_without_stale_schema_failures(self) -> None:
        modern = [record(0), record(1)]
        for action_index, item in enumerate(modern):
            item["semantic"]["video"]["bridge_layers"] = {
                "object_sprite_hash": f"object-{action_index}",
                "actor_sprite_hash": f"actor-{action_index}",
            }

        observations = compare_port.render_observations(modern, start_action=0)

        self.assertEqual(observations["modern_distinct_bridge_sprite_hashes"], 2)
        self.assertEqual(
            observations["modern_distinct_bridge_object_sprite_hashes"], 2
        )
        self.assertEqual(observations["modern_distinct_bridge_actor_sprite_hashes"], 2)

    def test_reports_authoritative_logical_indexed_rgb_divergence(self) -> None:
        original = [record(0), record(1)]
        modern = copy.deepcopy(original)
        modern[1]["semantic"]["video"]["logical_indexed_rgb_hash"] = "wrong-rgb"

        report = compare_port.compare_port_records(
            original,
            modern,
            start_action=0,
            bridge_frame_tolerance=2,
            require_indexed_rgb=True,
        )

        self.assertEqual(report["status"], "diverged")
        self.assertEqual(report["indexed_rgb_comparisons"], 2)
        self.assertEqual(
            report["first_divergence"]["differences"][0]["path"],
            "semantic.video.logical_indexed_rgb_hash",
        )

    def test_unaligned_queue_sequences_do_not_compare_different_frames(self) -> None:
        original = [record(0), record(1)]
        modern = copy.deepcopy(original)
        modern[1]["semantic"]["video"]["queue_metrics"]["sequence_index"] = 17
        modern[1]["semantic"]["video"]["logical_indexed_rgb_hash"] = "later-frame"
        modern[1]["semantic"]["video"]["indexed_rgb_hash"] = "later-display"

        report = compare_port.compare_port_records(
            original,
            modern,
            start_action=0,
            bridge_frame_tolerance=2,
            require_indexed_rgb=True,
        )

        self.assertEqual(report["status"], "equivalent")
        self.assertEqual(report["indexed_rgb_comparisons"], 1)
        self.assertEqual(report["indexed_rgb_unaligned_records"], 1)

    def test_reports_stable_indexed_display_rgb_divergence(self) -> None:
        original = [record(0)]
        modern = copy.deepcopy(original)
        modern[0]["semantic"]["video"]["indexed_rgb_hash"] = "wrong-display"

        report = compare_port.compare_port_records(
            original,
            modern,
            start_action=0,
            bridge_frame_tolerance=2,
            require_indexed_rgb=True,
        )

        self.assertEqual(report["status"], "diverged")
        self.assertEqual(report["indexed_display_rgb_comparisons"], 1)
        self.assertEqual(
            report["first_divergence"]["differences"][0]["path"],
            "semantic.video.indexed_rgb_hash",
        )

    def test_skips_in_flight_dos_display_page_but_compares_logical_rgb(self) -> None:
        original = [record(0)]
        original[0]["semantic"]["video"]["screen_hash"] = "previous-page"
        original[0]["semantic"]["video"]["indexed_rgb_hash"] = "previous-rgb"
        modern = [record(0)]

        report = compare_port.compare_port_records(
            original,
            modern,
            start_action=0,
            bridge_frame_tolerance=2,
            require_indexed_rgb=True,
        )

        self.assertEqual(report["status"], "equivalent")
        self.assertEqual(report["indexed_rgb_comparisons"], 1)
        self.assertEqual(report["indexed_display_rgb_comparisons"], 0)
        self.assertEqual(report["indexed_display_in_flight_records"], 1)

    def test_ignores_indexed_hash_for_true_color_bridge_composition(self) -> None:
        original = [record(0)]
        modern = copy.deepcopy(original)
        modern[0]["semantic"]["video"]["indexed_frame_authoritative"] = False
        modern[0]["semantic"]["video"]["indexed_rgb_hash"] = "bridge-layer-rgb"
        modern[0]["semantic"]["video"]["logical_indexed_rgb_hash"] = (
            "bridge-layer-rgb"
        )

        report = compare_port.compare_port_records(
            original,
            modern,
            start_action=0,
            bridge_frame_tolerance=2,
            require_indexed_rgb=True,
        )

        self.assertEqual(report["status"], "equivalent")
        self.assertEqual(report["indexed_rgb_comparisons"], 0)

    def test_required_indexed_hash_rejects_an_old_original_trace(self) -> None:
        original = [record(0)]
        del original[0]["semantic"]["video"]["logical_indexed_rgb_hash"]
        modern = [record(0)]

        report = compare_port.compare_port_records(
            original,
            modern,
            start_action=0,
            bridge_frame_tolerance=2,
            require_indexed_rgb=True,
        )

        self.assertEqual(report["status"], "diverged")
        self.assertEqual(report["indexed_rgb_missing_records"], 1)
        self.assertEqual(
            report["first_divergence"]["differences"][0]["reason"],
            "authoritative indexed frame lacks a logical RGB oracle hash",
        )


if __name__ == "__main__":
    unittest.main()
