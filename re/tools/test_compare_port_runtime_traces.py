#!/usr/bin/env python3
from __future__ import annotations

import copy
import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))

import compare_port_runtime_traces as compare_port


def record(
    action_index: int, frame: int = 45, game_frame_sequence: int | None = None
) -> dict:
    return {
        "schema": 1,
        "action_index": action_index,
        "action": "wait 10" if action_index else None,
        "clock": {
            "game_frame_sequence": (
                action_index * 10
                if game_frame_sequence is None
                else game_frame_sequence
            )
        },
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
            "random": {
                "seed": 10023,
                "mix_low": action_index,
                "mix_high": action_index + 1,
                "counter": action_index + 2,
            },
            "name_area_effect": {
                "active": True,
                "restart_requested": False,
                "sequence_index": 7,
                "frame_index": action_index,
                "operation": 2,
                "frames_remaining": 9,
            },
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

    def test_normalizes_unloaded_resource_profiles_across_memory_models(self) -> None:
        original = [record(0)]
        original[0]["semantic"]["vm"]["resource_handles"] = [0, 0, 0, 0, 0]
        modern = copy.deepcopy(original)
        modern[0]["semantic"]["vm"]["resource_profile"] = None
        modern[0]["semantic"]["vm"]["resource_handles"] = []

        report = compare_port.compare_port_records(
            original, modern, start_action=0, bridge_frame_tolerance=2
        )

        self.assertEqual(report["status"], "equivalent")

    def test_minimum_indexed_rgb_comparisons_rejects_empty_coverage(self) -> None:
        original = [record(0)]
        modern = copy.deepcopy(original)
        modern[0]["semantic"]["video"]["indexed_frame_authoritative"] = False

        report = compare_port.compare_port_records(
            original,
            modern,
            start_action=0,
            bridge_frame_tolerance=2,
            require_indexed_rgb=True,
            minimum_indexed_rgb_comparisons=1,
        )

        self.assertEqual(report["status"], "diverged")
        self.assertEqual(report["indexed_rgb_comparisons"], 0)
        self.assertEqual(
            report["first_divergence"]["differences"][0]["path"],
            "coverage.indexed_rgb_comparisons",
        )

    def test_minimum_indexed_rgb_comparisons_accepts_real_coverage(self) -> None:
        original = [record(0)]
        modern = copy.deepcopy(original)

        report = compare_port.compare_port_records(
            original,
            modern,
            start_action=0,
            bridge_frame_tolerance=2,
            require_indexed_rgb=True,
            minimum_indexed_rgb_comparisons=1,
        )

        self.assertEqual(report["status"], "equivalent")
        self.assertEqual(report["indexed_rgb_comparisons"], 1)

    def test_hidden_bridge_frame_is_ignored_during_full_screen_opening(self) -> None:
        original = [record(0, frame=90)]
        modern = copy.deepcopy(original)
        modern[0]["semantic"]["presentation"]["bridge_frame"] = 179
        original[0]["semantic"]["vm"]["active_line"] = 0
        modern[0]["semantic"]["vm"]["active_line"] = 0

        report = compare_port.compare_port_records(
            original, modern, start_action=0, bridge_frame_tolerance=2
        )

        self.assertEqual(report["status"], "equivalent")
        self.assertEqual(report["hidden_bridge_frame_records"], 1)

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

    def test_reports_temporal_divergence_when_game_frame_deltas_align(self) -> None:
        original = [record(0), record(1)]
        modern = copy.deepcopy(original)
        modern[1]["semantic"]["random"]["counter"] = 99

        report = compare_port.compare_port_records(
            original,
            modern,
            start_action=0,
            bridge_frame_tolerance=2,
            require_game_frame_clock=True,
        )

        self.assertEqual(report["status"], "diverged")
        self.assertEqual(report["game_frame_comparisons"], 2)
        self.assertEqual(
            report["first_divergence"]["differences"][0]["path"],
            "semantic.random.counter",
        )

    def test_skips_temporal_state_when_game_frame_deltas_are_unaligned(self) -> None:
        original = [record(0), record(1)]
        modern = copy.deepcopy(original)
        modern[1]["clock"]["game_frame_sequence"] = 11
        modern[1]["semantic"]["random"]["mix_low"] = 91
        modern[1]["semantic"]["name_area_effect"]["frame_index"] = 4

        report = compare_port.compare_port_records(
            original,
            modern,
            start_action=0,
            bridge_frame_tolerance=2,
            require_game_frame_clock=False,
        )

        self.assertEqual(report["status"], "equivalent")
        self.assertEqual(report["game_frame_comparisons"], 2)
        self.assertEqual(report["game_frame_unaligned_records"], 1)
        self.assertEqual(
            report["game_frame_deltas"],
            [
                {"action_index": 0, "original": 0, "modern": 0, "status": "aligned"},
                {
                    "action_index": 1,
                    "original": 10,
                    "modern": 11,
                    "status": "unaligned",
                },
            ],
        )

    def test_non_temporal_state_still_compares_when_game_frames_are_unaligned(self) -> None:
        original = [record(0), record(1)]
        modern = copy.deepcopy(original)
        modern[1]["clock"]["game_frame_sequence"] = 11
        modern[1]["semantic"]["name_area_effect"]["active"] = False

        report = compare_port.compare_port_records(
            original,
            modern,
            start_action=0,
            bridge_frame_tolerance=2,
            require_game_frame_clock=False,
        )

        self.assertEqual(report["status"], "diverged")
        self.assertEqual(report["game_frame_unaligned_records"], 1)
        self.assertEqual(
            report["first_divergence"]["differences"][0]["path"],
            "semantic.name_area_effect.active",
        )

    def test_required_game_frame_clock_rejects_an_old_trace(self) -> None:
        original = [record(0), record(1)]
        modern = copy.deepcopy(original)
        del modern[1]["clock"]

        report = compare_port.compare_port_records(
            original,
            modern,
            start_action=0,
            bridge_frame_tolerance=2,
            require_game_frame_clock=True,
        )

        self.assertEqual(report["status"], "diverged")
        self.assertEqual(report["game_frame_missing_records"], 1)
        self.assertEqual(report["game_frame_deltas"][1]["status"], "missing")
        self.assertEqual(
            report["first_divergence"]["differences"][0]["reason"],
            "action delta lacks an exact game-frame clock",
        )

    def test_required_game_frame_clock_rejects_timing_drift(self) -> None:
        original = [record(0), record(1)]
        modern = copy.deepcopy(original)
        modern[1]["clock"]["game_frame_sequence"] = 11
        modern[1]["semantic"]["random"]["counter"] = 999

        report = compare_port.compare_port_records(
            original, modern, start_action=0, bridge_frame_tolerance=2,
            require_game_frame_clock=True,
        )

        self.assertEqual(report["status"], "diverged")
        difference = report["first_divergence"]["differences"][0]
        self.assertEqual(difference["path"], "clock.game_frame_sequence")
        self.assertEqual((difference["original"], difference["modern"]), (10, 11))

    def test_required_clock_cannot_hide_missing_temporal_state(self) -> None:
        for path in sorted(compare_port.OPTIONAL_EXACT_PATHS):
            for side in ("original", "modern"):
                with self.subTest(path=path, side=side):
                    original, modern = [record(0)], [record(0)]
                    target = original if side == "original" else modern
                    owner, field = path.split(".")
                    del target[0]["semantic"][owner][field]
                    report = compare_port.compare_port_records(
                        original, modern, start_action=0, bridge_frame_tolerance=2,
                        require_game_frame_clock=True,
                    )
                    self.assertEqual(report["status"], "diverged")
                    self.assertEqual(
                        report["first_divergence"]["differences"][0]["path"],
                        f"semantic.{path}",
                    )

    def test_empty_comparison_is_not_equivalence(self) -> None:
        for records, start_action in (([], 0), ([record(0)], 99)):
            with self.subTest(start_action=start_action):
                report = compare_port.compare_port_records(
                    records, records, start_action=start_action, bridge_frame_tolerance=2,
                )
                self.assertEqual(report["status"], "diverged")
                self.assertEqual(report["compared_records"], 0)

    def test_identically_truncated_traces_fail_required_record_count(self) -> None:
        report = compare_port.compare_port_records(
            [record(0)], [record(0)], start_action=0, bridge_frame_tolerance=2,
            minimum_compared_records=2,
        )
        self.assertEqual(report["status"], "diverged")
        self.assertEqual(
            report["first_divergence"]["differences"][0]["path"],
            "coverage.compared_records",
        )

    def test_comparison_rejects_duplicate_or_reordered_action_indices(self) -> None:
        for actions in ([0, 1, 1], [1, 0], [-1], [True], ["0"]):
            for side in ("original", "modern"):
                with self.subTest(actions=actions, side=side):
                    malformed = []
                    for index in actions:
                        item = record(0)
                        item["action_index"] = index
                        malformed.append(item)
                    original = malformed if side == "original" else [record(0)]
                    modern = malformed if side == "modern" else [record(0)]
                    with self.assertRaisesRegex(ValueError, "action_index"):
                        compare_port.compare_port_records(
                            original, modern, start_action=0, bridge_frame_tolerance=2,
                        )

    def test_comparison_rejects_backwards_frame_clocks(self) -> None:
        backwards = [record(0, game_frame_sequence=10), record(1, game_frame_sequence=9)]
        with self.assertRaisesRegex(ValueError, "game_frame_sequence"):
            compare_port.compare_port_records(
                backwards, backwards, start_action=0, bridge_frame_tolerance=2,
                require_game_frame_clock=True,
            )


if __name__ == "__main__":
    unittest.main()
