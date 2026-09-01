#!/usr/bin/env python3
"""Compare action-aligned DOS and flat Rust runtime semantics."""
from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any

EXACT_PATHS = (
    "vm.resource_profile",
    "vm.profile_request",
    "vm.execution_enabled",
    "vm.active_line",
    "presentation.mode",
    "presentation.box_mode",
    "presentation.word_choice_active",
    "presentation.nav_target_selection",
    "presentation.active",
    "presentation.defer",
    "presentation.text_display_active",
    "presentation.waiting_for_input",
    "random.seed",
    "random.mix_low",
    "random.mix_high",
    "random.counter",
    "name_area_effect.active",
    "name_area_effect.restart_requested",
    "name_area_effect.sequence_index",
    "name_area_effect.frame_index",
    "name_area_effect.operation",
    "name_area_effect.frames_remaining",
    "subtitle",
)
OPTIONAL_EXACT_PATHS = frozenset(
    (
        "random.seed",
        "random.mix_low",
        "random.mix_high",
        "random.counter",
        "name_area_effect.active",
        "name_area_effect.restart_requested",
        "name_area_effect.sequence_index",
        "name_area_effect.frame_index",
        "name_area_effect.operation",
        "name_area_effect.frames_remaining",
    )
)
OPTIONAL_SENTINEL_PATHS = frozenset(("vm.resource_profile", "vm.active_line"))
BRIDGE_FRAME_COUNT = 180
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


def has_value(record: dict[str, Any], path: str) -> bool:
    value: Any = record.get("semantic", {})
    for component in path.split("."):
        if not isinstance(value, dict) or component not in value:
            return False
        value = value[component]
    return True


def compact(value: Any, limit: int = 240) -> Any:
    rendered = json.dumps(value, sort_keys=True)
    if len(rendered) <= limit:
        return value
    return rendered[: limit - 3] + "..."


def normalize(path: str, value: Any) -> Any:
    if path in OPTIONAL_SENTINEL_PATHS and value is None:
        return 0xFFFF
    if isinstance(value, bool):
        return int(value)
    return value


def circular_distance(left: int, right: int, modulus: int) -> int:
    distance = abs(left - right) % modulus
    return min(distance, modulus - distance)


def compare_port_records(
    original: list[dict[str, Any]],
    modern: list[dict[str, Any]],
    *,
    start_action: int,
    bridge_frame_tolerance: int,
    require_indexed_rgb: bool = False,
) -> dict[str, Any]:
    original_by_action = {record["action_index"]: record for record in original}
    modern_by_action = {record["action_index"]: record for record in modern}
    action_indices = sorted(
        set(original_by_action) | set(modern_by_action)
    )
    action_indices = [index for index in action_indices if index >= start_action]
    report: dict[str, Any] = {
        "status": "equivalent",
        "start_action": start_action,
        "bridge_frame_tolerance": bridge_frame_tolerance,
        "compared_records": 0,
        "indexed_rgb_comparisons": 0,
        "indexed_rgb_missing_records": 0,
        "first_divergence": None,
        "render_observations": render_observations(modern, start_action),
    }

    for action_index in action_indices:
        expected = original_by_action.get(action_index)
        actual = modern_by_action.get(action_index)
        differences: list[dict[str, Any]] = []
        if expected is None or actual is None:
            differences.append(
                {
                    "path": "record",
                    "original": expected is not None,
                    "modern": actual is not None,
                }
            )
        else:
            if expected.get("action") != actual.get("action"):
                differences.append(
                    {
                        "path": "action",
                        "original": expected.get("action"),
                        "modern": actual.get("action"),
                    }
                )
            for side, record in (("original", expected), ("modern", actual)):
                if record.get("guest_end") is not None:
                    differences.append(
                        {
                            "path": f"{side}.guest_end",
                            "original": compact(expected.get("guest_end")),
                            "modern": compact(actual.get("guest_end")),
                        }
                    )
                if record.get("liveness") in FATAL_LIVENESS:
                    differences.append(
                        {
                            "path": f"{side}.liveness",
                            "original": expected.get("liveness"),
                            "modern": actual.get("liveness"),
                        }
                    )
            for path in EXACT_PATHS:
                if path in OPTIONAL_EXACT_PATHS and (
                    not has_value(expected, path) or not has_value(actual, path)
                ):
                    continue
                expected_value = normalize(path, value_at(expected, path))
                actual_value = normalize(path, value_at(actual, path))
                if expected_value != actual_value:
                    differences.append(
                        {
                            "path": f"semantic.{path}",
                            "original": compact(expected_value),
                            "modern": compact(actual_value),
                        }
                    )
            original_ui = int(value_at(expected, "presentation.ui_flags")) & 0xF
            modern_ui = int(value_at(actual, "presentation.ui_flags")) & 0xF
            if original_ui != modern_ui:
                differences.append(
                    {
                        "path": "semantic.presentation.ui_flags.low_nibble",
                        "original": original_ui,
                        "modern": modern_ui,
                    }
                )
            original_frame = int(value_at(expected, "presentation.bridge_frame"))
            modern_frame = int(value_at(actual, "presentation.bridge_frame"))
            frame_distance = circular_distance(
                original_frame, modern_frame, BRIDGE_FRAME_COUNT
            )
            if frame_distance > bridge_frame_tolerance:
                differences.append(
                    {
                        "path": "semantic.presentation.bridge_frame",
                        "original": original_frame,
                        "modern": modern_frame,
                        "circular_distance": frame_distance,
                    }
                )
            modern_video = actual["semantic"]["video"]
            original_video = expected["semantic"]["video"]
            if modern_video.get("indexed_frame_authoritative") is True:
                original_rgb_hash = original_video.get("indexed_rgb_hash")
                modern_rgb_hash = modern_video.get("indexed_rgb_hash")
                if original_rgb_hash is None or modern_rgb_hash is None:
                    report["indexed_rgb_missing_records"] += 1
                    if require_indexed_rgb:
                        differences.append(
                            {
                                "path": "semantic.video.indexed_rgb_hash",
                                "original": original_rgb_hash,
                                "modern": modern_rgb_hash,
                                "reason": "authoritative indexed frame lacks an RGB oracle hash",
                            }
                        )
                else:
                    report["indexed_rgb_comparisons"] += 1
                    if original_rgb_hash != modern_rgb_hash:
                        differences.append(
                            {
                                "path": "semantic.video.indexed_rgb_hash",
                                "original": original_rgb_hash,
                                "modern": modern_rgb_hash,
                            }
                        )

        report["compared_records"] += 1
        if differences:
            report["status"] = "diverged"
            report["first_divergence"] = {
                "action_index": action_index,
                "action": expected.get("action") if expected else actual.get("action"),
                "differences": differences,
            }
            break
    return report


def render_observations(
    modern: list[dict[str, Any]], start_action: int
) -> dict[str, Any]:
    sprite_layer_hashes = []
    object_sprite_hashes = []
    actor_sprite_hashes = []
    screen_hashes = []
    for record in modern:
        if record["action_index"] < start_action:
            continue
        screen_hashes.append(value_at(record, "video.screen_hash"))
        layers = value_at(record, "video").get("bridge_layers")
        if layers is not None:
            if "sprite_hash" in layers:
                sprite_layer_hashes.append((layers["sprite_hash"],))
                continue
            object_hash = layers["object_sprite_hash"]
            actor_hash = layers["actor_sprite_hash"]
            object_sprite_hashes.append(object_hash)
            actor_sprite_hashes.append(actor_hash)
            sprite_layer_hashes.append((object_hash, actor_hash))
    return {
        "modern_distinct_screen_hashes": len(set(screen_hashes)),
        "modern_distinct_bridge_sprite_hashes": len(set(sprite_layer_hashes)),
        "modern_distinct_bridge_object_sprite_hashes": len(
            set(object_sprite_hashes)
        ),
        "modern_distinct_bridge_actor_sprite_hashes": len(set(actor_sprite_hashes)),
        "note": (
            "Raw screen and split-layer hashes remain temporal-stasis observations. "
            "Authoritative indexed pages are compared separately after exact RGB expansion."
        ),
    }


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("original", type=Path)
    parser.add_argument("modern", type=Path)
    parser.add_argument("--start-action", type=int, default=0)
    parser.add_argument("--bridge-frame-tolerance", type=int, default=2)
    parser.add_argument(
        "--require-indexed-rgb",
        action="store_true",
        help="fail when an indexed Rust frame lacks an original RGB hash",
    )
    parser.add_argument("--output", type=Path)
    parser.add_argument("--report-only", action="store_true")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    report = compare_port_records(
        load_trace(args.original),
        load_trace(args.modern),
        start_action=args.start_action,
        bridge_frame_tolerance=args.bridge_frame_tolerance,
        require_indexed_rgb=args.require_indexed_rgb,
    )
    rendered = json.dumps(report, indent=2) + "\n"
    if args.output:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(rendered, encoding="utf-8")
    print(rendered, end="")
    return 0 if report["status"] == "equivalent" or args.report_only else 1


if __name__ == "__main__":
    raise SystemExit(main())
