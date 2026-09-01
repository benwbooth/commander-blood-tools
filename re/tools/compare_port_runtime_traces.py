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
    "name_area_effect.active",
    "name_area_effect.restart_requested",
    "subtitle",
)
TEMPORAL_EXACT_PATHS = (
    "random.mix_low",
    "random.mix_high",
    "random.counter",
    "name_area_effect.sequence_index",
    "name_area_effect.frame_index",
    "name_area_effect.operation",
    "name_area_effect.frames_remaining",
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
UNLOADED_RESOURCE_PROFILE = "unloaded"
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


def normalized_value(record: dict[str, Any], path: str) -> Any:
    value = value_at(record, path)
    if path == "vm.resource_profile":
        handles = record.get("semantic", {}).get("vm", {}).get("resource_handles")
        if isinstance(handles, list) and all(handle == 0 for handle in handles):
            return UNLOADED_RESOURCE_PROFILE
    return normalize(path, value)


def circular_distance(left: int, right: int, modulus: int) -> int:
    distance = abs(left - right) % modulus
    return min(distance, modulus - distance)


def presentation_sequence(video: dict[str, Any]) -> int | None:
    metrics = video.get("queue_metrics")
    if not isinstance(metrics, dict):
        return None
    sequence = metrics.get("sequence_index")
    return int(sequence) if sequence is not None else None


def game_frame_sequence(record: dict[str, Any]) -> int | None:
    clock = record.get("clock")
    if not isinstance(clock, dict):
        return None
    sequence = clock.get("game_frame_sequence")
    if not isinstance(sequence, int) or isinstance(sequence, bool):
        return None
    return sequence


def game_frame_delta(
    records_by_action: dict[int, dict[str, Any]], action_index: int
) -> int | None:
    current = game_frame_sequence(records_by_action[action_index])
    if current is None:
        return None
    previous_indices = [index for index in records_by_action if index < action_index]
    if not previous_indices:
        return current
    previous = game_frame_sequence(records_by_action[max(previous_indices)])
    if previous is None:
        return None
    return current - previous


def opening_page_owns_display(record: dict[str, Any]) -> bool:
    return (
        normalize("vm.active_line", value_at(record, "vm.active_line")) == 0
        and value_at(record, "video").get("indexed_frame_authoritative") is True
    )


def compare_port_records(
    original: list[dict[str, Any]],
    modern: list[dict[str, Any]],
    *,
    start_action: int,
    bridge_frame_tolerance: int,
    require_indexed_rgb: bool = False,
    minimum_indexed_rgb_comparisons: int = 0,
    require_game_frame_clock: bool = False,
) -> dict[str, Any]:
    if minimum_indexed_rgb_comparisons < 0:
        raise ValueError("minimum indexed RGB comparisons must be nonnegative")
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
        "minimum_indexed_rgb_comparisons": minimum_indexed_rgb_comparisons,
        "indexed_rgb_missing_records": 0,
        "indexed_rgb_unaligned_records": 0,
        "indexed_display_rgb_comparisons": 0,
        "indexed_display_rgb_missing_records": 0,
        "indexed_display_in_flight_records": 0,
        "game_frame_comparisons": 0,
        "game_frame_missing_records": 0,
        "game_frame_unaligned_records": 0,
        "game_frame_deltas": [],
        "hidden_bridge_frame_records": 0,
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
            original_game_frame_delta = game_frame_delta(
                original_by_action, action_index
            )
            modern_game_frame_delta = game_frame_delta(modern_by_action, action_index)
            temporal_paths = TEMPORAL_EXACT_PATHS
            if (
                original_game_frame_delta is None
                or modern_game_frame_delta is None
            ):
                report["game_frame_missing_records"] += 1
                report["game_frame_deltas"].append(
                    {
                        "action_index": action_index,
                        "original": original_game_frame_delta,
                        "modern": modern_game_frame_delta,
                        "status": "missing",
                    }
                )
                if require_game_frame_clock:
                    differences.append(
                        {
                            "path": "clock.game_frame_sequence",
                            "original": original_game_frame_delta,
                            "modern": modern_game_frame_delta,
                            "reason": "action delta lacks an exact game-frame clock",
                        }
                    )
                    temporal_paths = ()
            else:
                report["game_frame_comparisons"] += 1
                game_frames_aligned = (
                    original_game_frame_delta == modern_game_frame_delta
                )
                report["game_frame_deltas"].append(
                    {
                        "action_index": action_index,
                        "original": original_game_frame_delta,
                        "modern": modern_game_frame_delta,
                        "status": "aligned" if game_frames_aligned else "unaligned",
                    }
                )
                if not game_frames_aligned:
                    report["game_frame_unaligned_records"] += 1
                    temporal_paths = ()
            for path in (*EXACT_PATHS, *temporal_paths):
                if path in OPTIONAL_EXACT_PATHS and (
                    not has_value(expected, path) or not has_value(actual, path)
                ):
                    continue
                expected_value = normalized_value(expected, path)
                actual_value = normalized_value(actual, path)
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
            if opening_page_owns_display(expected) and opening_page_owns_display(actual):
                report["hidden_bridge_frame_records"] += 1
            else:
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
                original_rgb_hash = original_video.get("logical_indexed_rgb_hash")
                modern_rgb_hash = modern_video.get("logical_indexed_rgb_hash")
                original_sequence = presentation_sequence(original_video)
                modern_sequence = presentation_sequence(modern_video)
                if original_rgb_hash is None or modern_rgb_hash is None:
                    report["indexed_rgb_missing_records"] += 1
                    if require_indexed_rgb:
                        differences.append(
                            {
                                "path": "semantic.video.logical_indexed_rgb_hash",
                                "original": original_rgb_hash,
                                "modern": modern_rgb_hash,
                                "reason": "authoritative indexed frame lacks a logical RGB oracle hash",
                            }
                        )
                elif (
                    original_sequence is None
                    or modern_sequence is None
                    or original_sequence != modern_sequence
                ):
                    report["indexed_rgb_unaligned_records"] += 1
                else:
                    report["indexed_rgb_comparisons"] += 1
                    if original_rgb_hash != modern_rgb_hash:
                        differences.append(
                            {
                                "path": "semantic.video.logical_indexed_rgb_hash",
                                "original": original_rgb_hash,
                                "modern": modern_rgb_hash,
                            }
                        )

                    original_screen_hash = original_video.get("screen_hash")
                    original_logical_hash = original_video.get("logical_display_hash")
                    if (
                        original_screen_hash is not None
                        and original_screen_hash == original_logical_hash
                    ):
                        original_display_rgb_hash = original_video.get("indexed_rgb_hash")
                        modern_display_rgb_hash = modern_video.get("indexed_rgb_hash")
                        if (
                            original_display_rgb_hash is None
                            or modern_display_rgb_hash is None
                        ):
                            report["indexed_display_rgb_missing_records"] += 1
                            if require_indexed_rgb:
                                differences.append(
                                    {
                                        "path": "semantic.video.indexed_rgb_hash",
                                        "original": original_display_rgb_hash,
                                        "modern": modern_display_rgb_hash,
                                        "reason": "stable indexed display lacks an RGB oracle hash",
                                    }
                                )
                        else:
                            report["indexed_display_rgb_comparisons"] += 1
                            if original_display_rgb_hash != modern_display_rgb_hash:
                                differences.append(
                                    {
                                        "path": "semantic.video.indexed_rgb_hash",
                                        "original": original_display_rgb_hash,
                                        "modern": modern_display_rgb_hash,
                                    }
                                )
                    else:
                        report["indexed_display_in_flight_records"] += 1

        report["compared_records"] += 1
        if differences:
            report["status"] = "diverged"
            report["first_divergence"] = {
                "action_index": action_index,
                "action": expected.get("action") if expected else actual.get("action"),
                "differences": differences,
            }
            break
    if (
        report["status"] == "equivalent"
        and report["indexed_rgb_comparisons"]
        < report["minimum_indexed_rgb_comparisons"]
    ):
        report["status"] = "diverged"
        report["first_divergence"] = {
            "action_index": None,
            "action": None,
            "differences": [
                {
                    "path": "coverage.indexed_rgb_comparisons",
                    "original": f">={minimum_indexed_rgb_comparisons}",
                    "modern": report["indexed_rgb_comparisons"],
                    "reason": (
                        "runtime trace produced fewer aligned indexed RGB "
                        "comparisons than required"
                    ),
                }
            ],
        }
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
            "Authoritative logical indexed pages are compared after exact RGB expansion "
            "at aligned HNM queue sequences; DOS display pages are compared only after "
            "a completed page flip."
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
        help="fail when an aligned indexed frame lacks a logical or stable display RGB hash",
    )
    parser.add_argument(
        "--minimum-indexed-rgb-comparisons",
        type=int,
        default=0,
        help="fail unless at least this many aligned indexed RGB frames are compared",
    )
    parser.add_argument(
        "--require-game-frame-clock",
        action="store_true",
        help="fail when an action delta lacks an exact native or Rust game-frame clock",
    )
    parser.add_argument("--output", type=Path)
    parser.add_argument("--report-only", action="store_true")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    if args.minimum_indexed_rgb_comparisons < 0:
        raise SystemExit("--minimum-indexed-rgb-comparisons must be nonnegative")
    report = compare_port_records(
        load_trace(args.original),
        load_trace(args.modern),
        start_action=args.start_action,
        bridge_frame_tolerance=args.bridge_frame_tolerance,
        require_indexed_rgb=args.require_indexed_rgb,
        minimum_indexed_rgb_comparisons=args.minimum_indexed_rgb_comparisons,
        require_game_frame_clock=args.require_game_frame_clock,
    )
    rendered = json.dumps(report, indent=2) + "\n"
    if args.output:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(rendered, encoding="utf-8")
    print(rendered, end="")
    return 0 if report["status"] == "equivalent" or args.report_only else 1


if __name__ == "__main__":
    raise SystemExit(main())
