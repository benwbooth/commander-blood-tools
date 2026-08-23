#!/usr/bin/env python3
"""Compare action-aligned semantic traces from original and rebuilt BLOODPRG."""
from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any


SEMANTIC_PATHS = (
    "vm.resource_profile",
    "vm.profile_request",
    "vm.execution_enabled",
    "vm.active_line",
    "vm.displayed_line",
    "presentation.ui_flags",
    "presentation.actor_transition",
    "presentation.bridge_frame",
    "presentation.mode",
    "presentation.box_mode",
    "presentation.word_choice_active",
    "presentation.nav_target_selection",
    "presentation.active",
    "presentation.defer",
    "presentation.text_wait",
    "presentation.text_display_active",
    "presentation.waiting_for_input",
    "input.mouse_x",
    "input.mouse_y",
    "input.buttons",
    "input.previous_buttons",
    "input.primary_pressed",
    "input.press_pending",
    "audio.driver_pending",
    "audio.stream_mode",
    "audio.stream_channel",
    "audio.dialogue_delay",
    "audio.dialogue_hold",
    "audio.clip_playback_state",
    "audio.last_clip",
    "audio.streamed_clip_count",
    "audio.events",
    "subtitle",
    "persistent.state_array_hash",
    "persistent.character_slots_hash",
    "persistent.record_hash",
    "assets",
    "video.screen_hash",
    "video.palette_hash",
)
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


def value_at(record: dict[str, Any], path: str) -> Any:
    value: Any = record["semantic"]
    for component in path.split("."):
        value = value[component]
    return value


def compact(value: Any, limit: int = 240) -> Any:
    rendered = json.dumps(value, sort_keys=True)
    if len(rendered) <= limit:
        return value
    return rendered[: limit - 3] + "..."


def compare_records(
    original: list[dict[str, Any]], rebuilt: list[dict[str, Any]]
) -> dict[str, Any]:
    report: dict[str, Any] = {
        "status": "equivalent",
        "original_records": len(original),
        "rebuilt_records": len(rebuilt),
        "compared_records": 0,
        "first_divergence": None,
    }
    count = min(len(original), len(rebuilt))
    for index in range(count):
        expected = original[index]
        actual = rebuilt[index]
        differences = []
        for field in ("action_index", "phase", "action"):
            if expected.get(field) != actual.get(field):
                differences.append(
                    {
                        "path": field,
                        "original": compact(expected.get(field)),
                        "rebuilt": compact(actual.get(field)),
                    }
                )
        for side, record in (("original", expected), ("rebuilt", actual)):
            if record.get("guest_end") is not None:
                differences.append(
                    {
                        "path": f"{side}.guest_end",
                        "original": compact(expected.get("guest_end")),
                        "rebuilt": compact(actual.get("guest_end")),
                    }
                )
            if record.get("liveness") in FATAL_LIVENESS:
                differences.append(
                    {
                        "path": f"{side}.liveness",
                        "original": expected.get("liveness"),
                        "rebuilt": actual.get("liveness"),
                    }
                )
        for path in SEMANTIC_PATHS:
            expected_value = value_at(expected, path)
            actual_value = value_at(actual, path)
            if expected_value != actual_value:
                differences.append(
                    {
                        "path": f"semantic.{path}",
                        "original": compact(expected_value),
                        "rebuilt": compact(actual_value),
                    }
                )
        report["compared_records"] = index + 1
        if differences:
            report["status"] = "diverged"
            report["first_divergence"] = {
                "record": index,
                "action_index": expected.get("action_index"),
                "action": expected.get("action"),
                "original_steps": expected.get("steps"),
                "rebuilt_steps": actual.get("steps"),
                "differences": differences,
            }
            return report
    if len(original) != len(rebuilt):
        report["status"] = "diverged"
        report["first_divergence"] = {
            "record": count,
            "action_index": None,
            "action": None,
            "differences": [
                {
                    "path": "record_count",
                    "original": len(original),
                    "rebuilt": len(rebuilt),
                }
            ],
        }
    return report


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("original", type=Path)
    parser.add_argument("rebuilt", type=Path)
    parser.add_argument("--output", type=Path)
    parser.add_argument("--report-only", action="store_true")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    report = compare_records(load_trace(args.original), load_trace(args.rebuilt))
    rendered = json.dumps(report, indent=2) + "\n"
    if args.output:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(rendered, encoding="utf-8")
    print(rendered, end="")
    return 0 if report["status"] == "equivalent" or args.report_only else 1


if __name__ == "__main__":
    raise SystemExit(main())
