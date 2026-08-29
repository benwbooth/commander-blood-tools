#!/usr/bin/env python3
"""Verify the recovered startup-phone sequence in DOS and modern traces."""
from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any, Callable

PHONE_CLICK = "click 125 118"
GAME_CHOICE_CLICK = "sclick 200 105"
ANSWER_SELECTOR = 4
CHOICE_SELECTOR = 7
NEUTRAL_SELECTOR = 1
INITIAL_PROFILE = 0
POST_CALL_PROFILE = 1
PORTRAIT_RESOURCE = 7
PORTRAIT_POSITION = [16, 74]
PORTRAIT_EXTENT = [104, 80]
ACTIVE_ENTITY_FLAG = 1
UI_ENABLED_FLAG = 1
FATAL_LIVENESS = frozenset(("guest_stopped", "input_not_consumed"))


def load_trace(path: Path) -> list[dict[str, Any]]:
    records = []
    for line_number, line in enumerate(
        path.read_text(encoding="utf-8").splitlines(), start=1
    ):
        if not line.strip():
            continue
        try:
            record = json.loads(line)
        except json.JSONDecodeError as error:
            raise ValueError(f"{path}:{line_number}: {error}") from error
        if record.get("schema") != 1:
            raise ValueError(
                f"{path}:{line_number}: unsupported trace schema "
                f"{record.get('schema')!r}"
            )
        records.append(record)
    if not records:
        raise ValueError(f"{path}: empty runtime trace")
    return records


def presentation(record: dict[str, Any]) -> dict[str, Any]:
    return record["semantic"]["presentation"]


def profile(record: dict[str, Any]) -> int | None:
    return record["semantic"]["vm"]["resource_profile"]


def selector(record: dict[str, Any]) -> int:
    state = presentation(record)
    return int(state.get("manu3_current", state.get("manu3_selector_current", 0)))


def first_record(
    records: list[dict[str, Any]],
    predicate: Callable[[dict[str, Any]], bool],
) -> dict[str, Any] | None:
    return next((record for record in records if predicate(record)), None)


def bridge_actor_hash(record: dict[str, Any]) -> str | None:
    layers = record["semantic"]["video"].get("bridge_layers")
    if not layers:
        return None
    return layers.get("actor_sprite_hash")


def verify_trace(records: list[dict[str, Any]], implementation: str) -> list[str]:
    errors = []
    for record in records:
        if record.get("guest_end") is not None:
            errors.append(f"guest terminated at action {record.get('action_index')}")
        if record.get("liveness") in FATAL_LIVENESS:
            errors.append(
                f"fatal liveness {record.get('liveness')} at action "
                f"{record.get('action_index')}"
            )

    phone_index = next(
        (index for index, record in enumerate(records) if record.get("action") == PHONE_CLICK),
        None,
    )
    if phone_index is None:
        return errors + [f"missing authored phone action {PHONE_CLICK!r}"]
    answer = records[phone_index]
    if selector(answer) != ANSWER_SELECTOR:
        errors.append(
            f"phone answer selected MANU3 {selector(answer)}, expected {ANSWER_SELECTOR}"
        )

    after_answer = records[phone_index + 1 :]
    active = first_record(
        after_answer,
        lambda record: profile(record) == INITIAL_PROFILE
        and bool(presentation(record)["active"])
        and bool(presentation(record)["defer"]),
    )
    if active is None:
        errors.append("phone answer never acquired presentation and deferred-text ownership")
    elif implementation == "original":
        assets = [str(asset).lower() for asset in active["semantic"].get("assets", [])]
        if "izwalito.spr" not in assets:
            errors.append("DOS phone presentation did not load izwalito.spr")
        if any(asset.endswith(".hnm") for asset in assets):
            errors.append("DOS text-only phone presentation unexpectedly loaded an HNM")
    else:
        portrait = presentation(active).get("portrait_entity", {})
        source = portrait.get("source") or {}
        if source.get("kind") != "cached" or source.get("resource") != PORTRAIT_RESOURCE:
            errors.append("modern phone presentation did not use cached portrait resource 7")
        if portrait.get("draw_position") != PORTRAIT_POSITION:
            errors.append("modern Izwalito portrait is not drawn at the native inset position")
        if portrait.get("extent") != PORTRAIT_EXTENT:
            errors.append("modern Izwalito portrait does not have the native inset extent")
        if int(portrait.get("flags", 0)) & ACTIVE_ENTITY_FLAG == 0:
            errors.append("modern Izwalito portrait entity is not active")
        if active["semantic"]["vm"].get("active_line") is not None:
            errors.append("modern text-only phone presentation unexpectedly selected an HNM line")

        active_hashes = {
            actor_hash
            for record in after_answer
            if bool(presentation(record)["active"])
            for actor_hash in (bridge_actor_hash(record),)
            if actor_hash is not None
        }
        if len(active_hashes) < 2:
            errors.append("modern Izwalito inset did not animate across distinct actor frames")

    waiting = first_record(
        after_answer,
        lambda record: bool(presentation(record)["waiting_for_input"])
        and bool(presentation(record)["word_choice_active"]),
    )
    if waiting is None:
        errors.append("startup call never reached its authored word-choice gate")
    elif implementation == "modern":
        choices = presentation(waiting).get("rendered_word_choices")
        if choices != ["explanations", "game"]:
            errors.append(f"modern startup choices are {choices!r}, expected EXPLANATIONS/GAME")

    choice = first_record(
        after_answer,
        lambda record: record.get("action") == GAME_CHOICE_CLICK,
    )
    if choice is None:
        errors.append(f"missing authored GAME choice action {GAME_CHOICE_CLICK!r}")
    elif selector(choice) != CHOICE_SELECTOR:
        errors.append(
            f"GAME choice selected MANU3 {selector(choice)}, expected {CHOICE_SELECTOR}"
        )

    teardown = first_record(
        after_answer,
        lambda record: profile(record) == POST_CALL_PROFILE
        and not bool(presentation(record)["active"])
        and not bool(presentation(record)["defer"]),
    )
    if teardown is None:
        errors.append("startup call did not release ownership and enter SCRIPT2")
    else:
        if selector(teardown) != NEUTRAL_SELECTOR:
            errors.append(
                f"post-call MANU3 selector is {selector(teardown)}, expected {NEUTRAL_SELECTOR}"
            )
        if int(presentation(teardown)["ui_flags"]) & UI_ENABLED_FLAG == 0:
            errors.append("post-call bridge UI is not enabled")
        if implementation == "modern":
            portrait = presentation(teardown).get("portrait_entity", {})
            if int(portrait.get("flags", 0)) & ACTIVE_ENTITY_FLAG != 0:
                errors.append("modern Izwalito portrait remained active after the call")
            before_answer = records[phone_index - 1] if phone_index else records[0]
            if bridge_actor_hash(teardown) != bridge_actor_hash(before_answer):
                errors.append("modern bridge actor layer did not return to its pre-call state")

    return errors


def verify_pair(
    original: list[dict[str, Any]], modern: list[dict[str, Any]]
) -> dict[str, Any]:
    original_errors = verify_trace(original, "original")
    modern_errors = verify_trace(modern, "modern")
    return {
        "status": "equivalent" if not original_errors and not modern_errors else "diverged",
        "original_errors": original_errors,
        "modern_errors": modern_errors,
        "checks": {
            "answer_selector": ANSWER_SELECTOR,
            "portrait_resource": PORTRAIT_RESOURCE,
            "portrait_position": PORTRAIT_POSITION,
            "portrait_extent": PORTRAIT_EXTENT,
            "choice_selector": CHOICE_SELECTOR,
            "post_call_selector": NEUTRAL_SELECTOR,
            "post_call_profile": POST_CALL_PROFILE,
        },
    }


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("original", type=Path)
    parser.add_argument("modern", type=Path)
    parser.add_argument("--output", type=Path)
    parser.add_argument("--report-only", action="store_true")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    report = verify_pair(load_trace(args.original), load_trace(args.modern))
    rendered = json.dumps(report, indent=2) + "\n"
    if args.output:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(rendered, encoding="utf-8")
    print(rendered, end="")
    return 0 if report["status"] == "equivalent" or args.report_only else 1


if __name__ == "__main__":
    raise SystemExit(main())
