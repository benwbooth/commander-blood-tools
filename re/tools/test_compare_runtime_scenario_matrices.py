#!/usr/bin/env python3

from __future__ import annotations

import importlib.util
from pathlib import Path
import unittest


TOOL_PATH = Path(__file__).with_name("compare_runtime_scenario_matrices.py")
SPEC = importlib.util.spec_from_file_location("compare_runtime_scenario_matrices", TOOL_PATH)
assert SPEC is not None and SPEC.loader is not None
comparison = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(comparison)


def report(
    verdict: str,
    *,
    words_offset: int = 0x1234,
    subtitle: str = "Hello",
    anomaly: bool = False,
    sample: int = 10,
    segment: int = 0x2000,
    timer_tick: int = 100,
) -> dict[str, object]:
    presentation = {
        "list_state": 1 if anomaly else 0,
        "list_head_offset": 0x4567,
        "list_head_segment": segment,
        "active_line": 4,
        "displayed_line": 3 if anomaly else 4,
        "presentation_active": 1,
    }
    audio = {
        "voc_playback_enabled": 1,
        "timer_tick": timer_tick,
        "frame_delay": timer_tick % 9,
        "dialogue_hold_countdown": timer_tick % 3,
        "streamed_clip_count": 5,
        "last_clip": 3,
    }
    result: dict[str, object] = {
        "verdict": verdict,
        "contact_probe": {
            "phase": "wait-contact",
            "completion_reason": None if anomaly else "line-target",
            "checkpoints": [
                {
                    "sample": sample,
                    "menu_words_offset": words_offset,
                    "subtitle": subtitle,
                    "expected_subtitle": subtitle,
                }
            ],
            "line_states": [
                {
                    "sample": sample,
                    "presentation_flow": presentation,
                    "audio_flow": audio,
                }
            ],
        },
        "anomalies": [],
    }
    if anomaly:
        result["anomalies"] = [
            {
                "sample": sample,
                "issues": [
                    f"active-presentation-stalled=start:{sample - 5},current:{sample}"
                ],
                "presentation_flow": presentation,
                "audio_flow": audio,
            }
        ]
    return result


class SemanticSignatureTests(unittest.TestCase):
    def test_matching_passes_are_verified(self) -> None:
        candidate = comparison.semantic_report_signature(
            report("CONTACT-PROBE-COMPLETE", segment=0x2000, timer_tick=100)
        )
        reference = comparison.semantic_report_signature(
            report("CONTACT-PROBE-COMPLETE", segment=0x3000, timer_tick=200)
        )
        self.assertEqual(candidate, reference)
        self.assertEqual(
            comparison.classify_result_pair("PASS", "PASS", candidate, reference),
            "verified-match",
        )

    def test_shared_stall_ignores_host_sample_timer_and_allocated_segment(self) -> None:
        candidate = comparison.semantic_report_signature(
            report(
                "ANOMALY",
                anomaly=True,
                sample=120,
                segment=0x2000,
                timer_tick=100,
            )
        )
        reference = comparison.semantic_report_signature(
            report(
                "ANOMALY",
                anomaly=True,
                sample=240,
                segment=0x3000,
                timer_tick=200,
            )
        )
        self.assertEqual(candidate, reference)
        self.assertEqual(
            comparison.classify_result_pair("FAIL", "FAIL", candidate, reference),
            "shared-inconclusive",
        )

    def test_original_pass_and_candidate_failure_is_regression(self) -> None:
        failed = comparison.semantic_report_signature(report("ANOMALY", anomaly=True))
        passed = comparison.semantic_report_signature(report("CONTACT-PROBE-COMPLETE"))
        self.assertEqual(
            comparison.classify_result_pair("FAIL", "PASS", failed, passed),
            "candidate-regression",
        )

    def test_distinct_failures_are_divergent(self) -> None:
        candidate = comparison.semantic_report_signature(
            report("ANOMALY", anomaly=True, words_offset=0x1234)
        )
        reference = comparison.semantic_report_signature(
            report("ANOMALY", anomaly=True, words_offset=0x5678)
        )
        self.assertEqual(
            comparison.classify_result_pair("FAIL", "FAIL", candidate, reference),
            "divergent-failure",
        )

    def test_matching_retry_pass_overrides_a_failed_base_attempt(self) -> None:
        signature = comparison.semantic_report_signature(
            report("CONTACT-PROBE-COMPLETE")
        )
        failed = comparison.semantic_report_signature(report("ANOMALY", anomaly=True))
        self.assertEqual(
            comparison.classify_attempts(
                [{"status": "PASS", "signature": signature}],
                [
                    {"status": "FAIL", "signature": failed},
                    {"status": "PASS", "signature": signature},
                ],
            ),
            "verified-match",
        )


if __name__ == "__main__":
    unittest.main()
