#!/usr/bin/env python3
"""Compare original and recovered runtime scenario matrices semantically."""

from __future__ import annotations

import argparse
from collections import Counter
import json
from pathlib import Path
from typing import Any


PRESENTATION_FIELDS = (
    "list_state",
    "list_read_wrap_index",
    "list_wrap_count",
    "list_read_wrap_limit",
    "list_secondary_wrap_limit",
    "resource_source_offset",
    "resource_source_remaining",
    "list_head_offset",
    "list_tail_offset",
    "list_active_offset",
    "list_buffer_end",
    "list_queued_bytes",
    "list_iteration_count",
    "list_rollover_state",
    "list_entry_metric",
    "c2_presentation_gate",
    "ui_state",
    "presentation_mode",
    "presentation_box_mode",
    "presentation_box_phase",
    "word_choice_active",
    "text_display_active",
    "text_reveal_phase",
    "active_line",
    "displayed_line",
    "presentation_owner_offset",
    "presentation_request_flags",
    "presentation_active",
    "presentation_defer",
    "presentation_start_lock",
    "presentation_text_wait",
    "dialogue_hold_complete",
    "presentation_hold_ready",
)

AUDIO_FIELDS = (
    "voc_playback_enabled",
    "game_mode",
    "timer_hook_active",
    "clip_playback_state",
    "bank_clip_count",
    "bank_dialogue_delay_base",
    "bank_dialogue_delay_limit",
    "last_clip",
    "streamed_clip_count",
    "dialogue_seed",
    "text_mode_seed",
    "text_mode_play",
    "text_voice_trigger",
)

DIFFERENCE_CLASSIFICATIONS = {
    "divergent-pass-report",
    "divergent-pass",
    "candidate-regression",
    "candidate-only-pass",
    "divergent-failure",
}


def _selected_fields(value: Any, fields: tuple[str, ...]) -> dict[str, Any]:
    if not isinstance(value, dict):
        return {}
    return {field: value.get(field) for field in fields}


def _issue_kind(issue: Any) -> str:
    return str(issue).split("=", 1)[0].split(":start", 1)[0]


def _observation_signature(observation: Any) -> dict[str, Any]:
    if not isinstance(observation, dict):
        return {}
    return {
        "presentation": _selected_fields(
            observation.get("presentation_flow"), PRESENTATION_FIELDS
        ),
        "audio": _selected_fields(observation.get("audio_flow"), AUDIO_FIELDS),
    }


def semantic_report_signature(report: Any) -> dict[str, Any] | None:
    if not isinstance(report, dict):
        return None
    contact = report.get("contact_probe")
    if not isinstance(contact, dict):
        contact = {}
    checkpoints = []
    for checkpoint in contact.get("checkpoints", []):
        if isinstance(checkpoint, dict):
            checkpoints.append(
                {
                    "menu_words_offset": checkpoint.get("menu_words_offset"),
                    "subtitle": checkpoint.get("subtitle"),
                    "expected_subtitle": checkpoint.get("expected_subtitle"),
                }
            )

    anomalies = report.get("anomalies")
    anomaly_signatures = []
    if isinstance(anomalies, list):
        for anomaly in anomalies:
            if not isinstance(anomaly, dict):
                continue
            issues = anomaly.get("issues", [])
            anomaly_signatures.append(
                {
                    "issues": sorted(
                        _issue_kind(issue)
                        for issue in issues
                        if isinstance(issue, str)
                    ),
                    **_observation_signature(anomaly),
                }
            )

    line_states = contact.get("line_states", [])
    final_line = line_states[-1] if isinstance(line_states, list) and line_states else None
    if anomaly_signatures:
        final_line = None
    return {
        "verdict": report.get("verdict"),
        "contact": {
            "phase": contact.get("phase"),
            "completion_reason": contact.get("completion_reason"),
            "checkpoints": checkpoints,
            "final_line": _observation_signature(final_line),
        },
        "anomalies": anomaly_signatures,
    }


def classify_result_pair(
    candidate_status: str,
    reference_status: str,
    candidate_signature: dict[str, Any] | None,
    reference_signature: dict[str, Any] | None,
) -> str:
    candidate_pass = candidate_status == "PASS"
    reference_pass = reference_status == "PASS"
    if candidate_pass and reference_pass:
        if candidate_signature == reference_signature:
            return "verified-match"
        if candidate_signature is None or reference_signature is None:
            return "divergent-pass-report"
        return "divergent-pass"
    if not candidate_pass and reference_pass:
        return "candidate-regression"
    if candidate_pass and not reference_pass:
        return "candidate-only-pass"
    if candidate_signature == reference_signature and candidate_signature is not None:
        return "shared-inconclusive"
    return "divergent-failure"


def _read_json(path: Path) -> Any:
    with path.open(encoding="utf-8") as source:
        return json.load(source)


def _result_map(matrix: dict[str, Any]) -> dict[str, dict[str, Any]]:
    results = matrix.get("results")
    if not isinstance(results, list):
        raise ValueError("matrix has no results list")
    mapped: dict[str, dict[str, Any]] = {}
    for result in results:
        if not isinstance(result, dict) or not isinstance(result.get("name"), str):
            raise ValueError("matrix contains a malformed result")
        name = result["name"]
        if name in mapped:
            raise ValueError(f"matrix contains duplicate scenario {name}")
        mapped[name] = result
    return mapped


def _load_result_report(matrix_path: Path, result: dict[str, Any]) -> Any:
    report_value = result.get("raw_report")
    if not isinstance(report_value, str):
        return None
    report_path = Path(report_value)
    if not report_path.is_absolute():
        report_path = matrix_path.parent / report_path
    if not report_path.is_file():
        return None
    return _read_json(report_path)


def compare_matrices(candidate_path: Path, reference_path: Path) -> dict[str, Any]:
    candidate_matrix = _read_json(candidate_path)
    reference_matrix = _read_json(reference_path)
    if not isinstance(candidate_matrix, dict) or not isinstance(reference_matrix, dict):
        raise ValueError("matrix root must be an object")
    candidate_results = _result_map(candidate_matrix)
    reference_results = _result_map(reference_matrix)
    if candidate_results.keys() != reference_results.keys():
        missing_candidate = sorted(reference_results.keys() - candidate_results.keys())
        missing_reference = sorted(candidate_results.keys() - reference_results.keys())
        raise ValueError(
            "scenario sets differ: "
            f"missing candidate={missing_candidate}, missing reference={missing_reference}"
        )

    rows = []
    counts: Counter[str] = Counter()
    for name in candidate_results:
        candidate = candidate_results[name]
        reference = reference_results[name]
        candidate_signature = semantic_report_signature(
            _load_result_report(candidate_path, candidate)
        )
        reference_signature = semantic_report_signature(
            _load_result_report(reference_path, reference)
        )
        classification = classify_result_pair(
            str(candidate.get("status")),
            str(reference.get("status")),
            candidate_signature,
            reference_signature,
        )
        counts[classification] += 1
        rows.append(
            {
                "name": name,
                "classification": classification,
                "candidate_status": candidate.get("status"),
                "reference_status": reference.get("status"),
                "candidate_signature": candidate_signature,
                "reference_signature": reference_signature,
            }
        )

    behavior_status = (
        "DIFFERENT"
        if any(counts[name] for name in DIFFERENCE_CLASSIFICATIONS)
        else "MATCH"
    )
    coverage_status = (
        "INCONCLUSIVE" if counts["shared-inconclusive"] else "COMPLETE"
    )
    return {
        "format_version": 1,
        "candidate_matrix": str(candidate_path),
        "reference_matrix": str(reference_path),
        "scenario_count": len(rows),
        "behavior_status": behavior_status,
        "coverage_status": coverage_status,
        "classification_counts": dict(sorted(counts.items())),
        "results": rows,
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--candidate", type=Path, required=True)
    parser.add_argument("--reference", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    try:
        comparison = compare_matrices(args.candidate, args.reference)
    except (OSError, ValueError, json.JSONDecodeError) as error:
        parser.error(str(error))
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(comparison, indent=2) + "\n", encoding="utf-8")
    print(
        f"behavior={comparison['behavior_status']} "
        f"coverage={comparison['coverage_status']} "
        f"scenarios={comparison['scenario_count']}"
    )
    for name, count in comparison["classification_counts"].items():
        print(f"{name}: {count}")
    return 0 if comparison["behavior_status"] == "MATCH" else 1


if __name__ == "__main__":
    raise SystemExit(main())
