#!/usr/bin/env python3
from __future__ import annotations

import copy
import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))

import verify_startup_phone_trace as verify_phone


def record(
    action_index: int,
    action: str | None,
    *,
    implementation: str,
    profile: int = verify_phone.INITIAL_PROFILE,
    active: bool = False,
    defer: bool = False,
    selector: int = verify_phone.NEUTRAL_SELECTOR,
    waiting: bool = False,
    choice_active: bool = False,
    actor_hash: str = "empty",
) -> dict:
    presentation = {
        "active": int(active),
        "defer": int(defer),
        "waiting_for_input": waiting,
        "word_choice_active": int(choice_active),
        "ui_flags": verify_phone.UI_ENABLED_FLAG,
    }
    if implementation == "original":
        presentation["manu3_selector_current"] = selector
    else:
        presentation.update(
            {
                "manu3_current": selector,
                "rendered_word_choices": [],
                "portrait_entity": {
                    "flags": 0,
                    "draw_position": [0, 0],
                    "extent": [0, 0],
                    "source": None,
                },
            }
        )
    return {
        "schema": 1,
        "action_index": action_index,
        "action": action,
        "guest_end": None,
        "liveness": "progress",
        "semantic": {
            "assets": [],
            "vm": {"resource_profile": profile, "active_line": None},
            "presentation": presentation,
            "video": {"bridge_layers": {"actor_sprite_hash": actor_hash}},
        },
    }


def valid_trace(implementation: str) -> list[dict]:
    trace = [record(0, None, implementation=implementation)]
    trace.append(record(1, "park 316 45", implementation=implementation))
    trace.append(
        record(
            2,
            verify_phone.PHONE_CLICK,
            implementation=implementation,
            selector=verify_phone.ANSWER_SELECTOR,
            actor_hash="hand",
        )
    )
    active = record(
        3,
        "wait 10",
        implementation=implementation,
        active=True,
        defer=True,
        actor_hash="izwalito-a",
    )
    if implementation == "original":
        active["semantic"]["assets"] = ["descript.des", "izwalito.spr"]
        active["semantic"]["vm"]["active_line"] = 0xFFFF
    else:
        active["semantic"]["presentation"]["portrait_entity"] = {
            "flags": verify_phone.ACTIVE_ENTITY_FLAG,
            "draw_position": verify_phone.PORTRAIT_POSITION,
            "extent": verify_phone.PORTRAIT_EXTENT,
            "source": {"kind": "cached", "resource": verify_phone.PORTRAIT_RESOURCE},
        }
    trace.append(active)
    animated = copy.deepcopy(active)
    animated["action_index"] = 4
    animated["semantic"]["video"]["bridge_layers"]["actor_sprite_hash"] = "izwalito-b"
    trace.append(animated)
    waiting = copy.deepcopy(animated)
    waiting["action_index"] = 5
    waiting["liveness"] = "waiting_for_input"
    waiting["semantic"]["presentation"]["waiting_for_input"] = True
    waiting["semantic"]["presentation"]["word_choice_active"] = 1
    if implementation == "modern":
        waiting["semantic"]["presentation"]["rendered_word_choices"] = [
            "explanations",
            "game",
        ]
    trace.append(waiting)
    trace.append(
        record(
            6,
            verify_phone.GAME_CHOICE_CLICK,
            implementation=implementation,
            active=True,
            defer=True,
            selector=verify_phone.CHOICE_SELECTOR,
            actor_hash="choice",
        )
    )
    trace.append(
        record(
            7,
            "wait 20",
            implementation=implementation,
            profile=verify_phone.POST_CALL_PROFILE,
            selector=verify_phone.NEUTRAL_SELECTOR,
        )
    )
    return trace


class StartupPhoneTraceTests(unittest.TestCase):
    def test_accepts_complete_original_and_modern_phone_sequences(self) -> None:
        report = verify_phone.verify_pair(valid_trace("original"), valid_trace("modern"))

        self.assertEqual(report["status"], "equivalent")
        self.assertEqual(report["original_errors"], [])
        self.assertEqual(report["modern_errors"], [])

    def test_rejects_missing_answer_flick_and_stale_portrait(self) -> None:
        modern = valid_trace("modern")
        modern[2]["semantic"]["presentation"]["manu3_current"] = verify_phone.NEUTRAL_SELECTOR
        modern[-1]["semantic"]["presentation"]["portrait_entity"]["flags"] = (
            verify_phone.ACTIVE_ENTITY_FLAG
        )

        errors = verify_phone.verify_trace(modern, "modern")

        self.assertTrue(any("phone answer selected MANU3" in error for error in errors))
        self.assertTrue(any("portrait remained active" in error for error in errors))

    def test_rejects_hnm_on_text_only_original_phone_call(self) -> None:
        original = valid_trace("original")
        original[3]["semantic"]["assets"].append("aaisw.hnm")

        errors = verify_phone.verify_trace(original, "original")

        self.assertTrue(any("unexpectedly loaded an HNM" in error for error in errors))


if __name__ == "__main__":
    unittest.main()
