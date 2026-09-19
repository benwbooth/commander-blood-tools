#!/usr/bin/env python3
"""Build the explicit Big Bug Bang native-entrypoint disposition ledger."""

from __future__ import annotations

import argparse
from collections import Counter
import csv
import hashlib
import json
from pathlib import Path
import re
from typing import Any


ROOT = Path(__file__).resolve().parents[2]
COVERAGE = ROOT / "re/big_bug_bang_oracle_coverage.json"
COMPARISON = ROOT / "re/big_bug_bang_expanded_function_comparison.json"
FIELD_AUDIT = ROOT / "re/big_bug_bang_field_display_audit.json"
STATIC_PORT_AUDIT = ROOT / "re/big_bug_bang_static_port_audit.json"
UNREFERENCED_LIBRARY_AUDIT = (
    ROOT / "re/big_bug_bang_unreferenced_library_audit.json"
)
PORTED = ROOT / "re/rust-port/ported.tsv"
ELIMINATED = ROOT / "re/rust-port/eliminated.tsv"
EXECUTABLE = ROOT / "output/big-bug-bang/disc/BLOOD2PG.EXE"

AUTHORED_NO_OPERATIONS = {
    0x24B1: "c3",
    0x24B2: "c3",
    0x24B3: "c3",
    0x24B4: "c3",
    0x24B5: "c3",
    0x24B6: "1e5606575f075e1fc3",
    0x24BF: "1e5606575f075e1fc3",
    0x5517: "c3",
    0x5518: "c3",
    0x5519: "c3",
}

AUDIO_STREAM = "crates/commander-blood-game/src/native/bloodprg/audio_stream.rs"
AUDIO_PLAYBACK = "crates/commander-blood-game/src/native/bloodprg/audio_playback.rs"
AUDIO_BANK = "crates/commander-blood-game/src/native/bloodprg/audio_bank.rs"
RUNTIME_AUDIO = "crates/commander-blood-game/src/runtime/audio.rs"
TIMER = "crates/commander-blood-game/src/native/bloodprg/timer.rs"
RESOURCE_CACHE = "crates/commander-blood-game/src/native/bloodprg/resource_cache.rs"
ASSETS = "crates/commander-blood-game/src/assets.rs"
APP = "crates/commander-blood-game/src/app.rs"
RENDER = "crates/commander-blood-game/src/render.rs"
SEQUEL_INPUT_FIXTURE = "re/tools/oracle_vectors/big_bug_bang_input_handlers.json"

DIRECT_RUST_OWNER_OVERRIDES = {
    0x23A2: {
        "path": "crates/commander-blood-game/src/native/bloodprg/input_dispatch.rs",
        "symbol": "sequel_dispatch_pause_and_latch_match_original_vectors",
    },
    0x23D4: {
        "path": "crates/commander-blood-game/src/native/bloodprg/input_selection.rs",
        "symbol": "sequel_selection_handlers_match_original_vectors",
    },
    0x2421: {
        "path": "crates/commander-blood-game/src/native/bloodprg/input_selection.rs",
        "symbol": "sequel_selection_handlers_match_original_vectors",
    },
    0x2514: {
        "path": "crates/commander-blood-game/src/native/bloodprg/input_selection.rs",
        "symbol": "sequel_selection_handlers_match_original_vectors",
    },
    0x253D: {
        "path": "crates/commander-blood-game/src/native/bloodprg/input_cancel.rs",
        "symbol": "sequel_cancellation_matches_original_vectors",
    },
    0x25A7: {
        "path": "crates/commander-blood-game/src/native/bloodprg/input_dispatch.rs",
        "symbol": "sequel_dispatch_pause_and_latch_match_original_vectors",
    },
    0x25C5: {
        "path": "crates/commander-blood-game/src/native/bloodprg/input_dispatch.rs",
        "symbol": "sequel_dispatch_pause_and_latch_match_original_vectors",
    },
}

HOST_ADAPTER_OWNERS = {
    0x09A2: (TIMER, "GameTimerState::start", "DOS timer-vector and PIT startup"),
    0x09F0: (TIMER, "GameTimerState::stop", "DOS timer-vector and PIT shutdown"),
    0x0C94: (
        RESOURCE_CACHE,
        "OriginalResourceCache::new",
        "EMS and XMS pool release",
    ),
    0x0D3D: (RENDER, "Renderer::render", "VGA retrace-phase calibration"),
    0x0DD2: (RENDER, "Renderer::render", "VGA retrace-phase polling"),
    0x0DFA: (APP, "run", "DOS Ctrl-Break and critical-error vector installation"),
    0x0E21: (RENDER, "Renderer::new", "BIOS and VGA Mode X initialization"),
    0x0EBB: (APP, "run", "BIOS video-mode restoration"),
    0x1971: (RENDER, "Renderer::render", "VGA page-offset and CRTC selection"),
    0x2B69: (
        ASSETS,
        "OriginalResourceStore::load",
        "DOS drive and current-directory restoration",
    ),
    0x2C86: (
        ASSETS,
        "OriginalResourceStore::load",
        "DOS and XMS resource transfer",
    ),
    0x2D77: (
        ASSETS,
        "OriginalResourceStore::load",
        "DOS and EMS mapped resource transfer",
    ),
    0xCF40: (AUDIO_STREAM, "start_audio_stream", "loaded DOS sound-driver ABI"),
    0xD9F3: (AUDIO_STREAM, "start_audio_stream", "Gravis stream startup protocol"),
    0xDA5F: (AUDIO_STREAM, "refill_audio_stream", "Gravis stream service protocol"),
    0xDB08: (AUDIO_STREAM, "start_audio_stream", "Gravis descriptor submission"),
    0xDBD4: (AUDIO_STREAM, "refill_audio_stream", "Gravis page transfer"),
    0xDC92: (AUDIO_BANK, "load_sound_bank", "Gravis streamed-bank transfer"),
    0xDCE5: (AUDIO_PLAYBACK, "update_audio_playback", "Gravis clip submission"),
    0xDDD2: (AUDIO_PLAYBACK, "update_audio_playback", "Gravis voice stop"),
    0xDDF8: (AUDIO_STREAM, "refill_audio_stream", "Gravis DRAM upload"),
    0xDE5B: (RUNTIME_AUDIO, "stop_all", "Gravis interrupt and voice shutdown"),
    0xDEC3: (RUNTIME_AUDIO, "open", "Gravis environment, vector, and PIC setup"),
    0xE05E: (RUNTIME_AUDIO, "open", "Gravis port detection"),
    0xE0DE: (RUNTIME_AUDIO, "open", "Gravis I/O delay"),
    0xE0ED: (AUDIO_STREAM, "refill_audio_stream", "Gravis interrupt service"),
}

FUNCTION_RE = re.compile(
    r"(?m)^\s*(?:pub(?:\([^)]*\))?\s+)?(?:async\s+)?fn\s+([A-Za-z0-9_]+)"
)

# These regressions borrow only initial records, never the native result rows.
INPUT_ONLY_FIXTURE_CONSUMERS = {
    ("big_bug_bang_growth.jsonl", "sequel_post_scan_bounds_follow_host_writes_and_skip_disabled_passes"),
    ("big_bug_bang_growth.jsonl", "sequel_post_scan_bounds_only_participating_actor_fields"),
}


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def read_tsv(path: Path) -> list[dict[str, str]]:
    with path.open(newline="") as stream:
        return list(csv.DictReader(stream, delimiter="\t"))


def commander_rows(path: Path) -> dict[int, dict[str, str]]:
    return {
        int(row["entry"], 16): row
        for row in read_tsv(path)
        if row["component"] == "bloodprg"
    }


def fixture_consumers(fixtures: set[str]) -> dict[str, list[dict[str, str]]]:
    consumers: dict[str, list[dict[str, str]]] = {}
    for path in sorted((ROOT / "crates").rglob("*.rs")):
        source = path.read_text()
        functions = list(FUNCTION_RE.finditer(source))
        for fixture in fixtures:
            start = 0
            while (offset := source.find(Path(fixture).name, start)) >= 0:
                owners = [match for match in functions if match.start() < offset]
                if not owners:
                    raise RuntimeError(
                        f"{path.relative_to(ROOT)} references {fixture} outside a function"
                    )
                if (Path(fixture).name, owners[-1].group(1)) in INPUT_ONLY_FIXTURE_CONSUMERS:
                    start = offset + 1
                    continue
                consumers.setdefault(fixture, []).append(
                    {
                        "path": str(path.relative_to(ROOT)),
                        "symbol": owners[-1].group(1),
                    }
                )
                start = offset + 1
    return consumers


def diagnostic_entries(audit: dict[str, Any]) -> set[int]:
    owner = audit["owner"]
    entries = {
        int(owner["entry"], 16),
        int(owner["directory_list_entry"], 16),
        int(owner["selected_record_entry"], 16),
        int(owner["empty_selection_entry"], 16),
        0x2495,
        0x24A3,
    }
    entries.update(
        int(row["handler_file_offset"], 16) for row in audit["display"]["selectors"]
    )
    entries.update(
        int(row["entry"], 16) for row in audit["display"]["exclusive_helpers"]
    )
    return entries


def choose_direct_evidence(
    entry: int,
    coverage_row: dict[str, Any],
    oracle_rows: dict[str, dict[str, Any]],
    consumers: dict[str, list[dict[str, str]]],
) -> tuple[dict[str, Any], dict[str, str]] | None:
    candidates = []
    for oracle_path in coverage_row["oracles"]:
        oracle = oracle_rows[oracle_path]
        fixture = oracle["fixture"]
        if fixture is not None and consumers.get(fixture):
            candidates.append(
                (
                    len(oracle["entered_entrypoints"]),
                    oracle_path,
                    oracle,
                    sorted(
                        consumers[fixture], key=lambda row: (row["path"], row["symbol"])
                    )[0],
                )
            )
    if not candidates:
        return None
    _, _, oracle, owner = min(candidates, key=lambda row: (row[0], row[1]))
    if entry in DIRECT_RUST_OWNER_OVERRIDES:
        if oracle["fixture"] != SEQUEL_INPUT_FIXTURE:
            raise RuntimeError(
                f"direct owner override at {entry:#x} selected unexpected fixture "
                f"{oracle['fixture']}"
            )
        owner = DIRECT_RUST_OWNER_OVERRIDES[entry]
        if owner not in consumers[SEQUEL_INPUT_FIXTURE]:
            raise RuntimeError(
                f"direct owner override at {entry:#x} does not consume "
                f"{SEQUEL_INPUT_FIXTURE}"
            )
    return oracle, owner


def direct_evidence(oracle: dict[str, Any]) -> list[str]:
    evidence = [oracle["oracle"]]
    if oracle["fixture"] is not None:
        evidence.append(oracle["fixture"])
    return evidence


def base_row(coverage_row: dict[str, Any]) -> dict[str, Any]:
    row = {
        "entry": coverage_row["entry"],
        "origin": coverage_row["origin"],
        "entered": coverage_row["entered"],
        "comparison": coverage_row["comparison"],
    }
    for key in ("commander_entry", "commander_candidates"):
        if key in coverage_row:
            row[key] = coverage_row[key]
    return row


def build_report() -> dict[str, Any]:
    coverage = json.loads(COVERAGE.read_text())
    audit = json.loads(FIELD_AUDIT.read_text())
    static_audit = json.loads(STATIC_PORT_AUDIT.read_text())
    for path, digest in static_audit["inputs"].items():
        if sha256(ROOT / path) != digest:
            raise RuntimeError(f"stale static port audit input: {path}")
    static_rows = {int(item["entry"], 16): item for item in static_audit["routines"]}
    unreferenced_audit = json.loads(UNREFERENCED_LIBRARY_AUDIT.read_text())
    ported = commander_rows(PORTED)
    eliminated = commander_rows(ELIMINATED)
    oracle_rows = {row["oracle"]: row for row in coverage["oracles"]}
    fixtures = {
        row["fixture"] for row in coverage["oracles"] if row["fixture"] is not None
    }
    consumers = fixture_consumers(fixtures)
    diagnostics = diagnostic_entries(audit)
    unreferenced_library = {
        int(item["entry"], 16) for item in unreferenced_audit["routines"]
    }
    executable = EXECUTABLE.read_bytes()

    rows = []
    for coverage_row in coverage["entrypoints"]:
        entry = int(coverage_row["entry"], 16)
        row = base_row(coverage_row)

        if entry in HOST_ADAPTER_OWNERS:
            path, symbol, adapter = HOST_ADAPTER_OWNERS[entry]
            if path == TIMER:
                boundary = "modern host"
            elif path == RESOURCE_CACHE:
                boundary = "Rust memory"
            elif path == ASSETS:
                boundary = "rooted asset-store"
            elif path in {APP, RENDER}:
                boundary = "SDL/wgpu"
            else:
                boundary = "SDL audio"
            oracle = min(
                (oracle_rows[path] for path in coverage_row["oracles"]),
                key=lambda item: (len(item["entered_entrypoints"]), item["oracle"]),
            )
            row.update(
                status="eliminated_host_adapter",
                evidence=direct_evidence(oracle),
                rust_owner={"path": path, "symbol": symbol},
                rationale=(
                    f"The {adapter} is executable-verified but eliminated at the "
                    f"owned {boundary} boundary; no DOS device, interrupt, or far-call "
                    "ABI is exposed by the modern runtime."
                ),
            )
        elif entry in diagnostics:
            evidence = [str(FIELD_AUDIT.relative_to(ROOT))]
            evidence.extend(coverage_row["oracles"])
            row.update(
                status="eliminated_dormant_diagnostic",
                evidence=evidence,
                rust_owner=None,
                rationale=(
                    "This entry belongs only to the dormant French field inspector; "
                    "the pinned static audit finds a zero initializer and no direct "
                    "writer for its mode, so inspector rendering is outside playable "
                    "game semantics."
                ),
            )
        elif entry in unreferenced_library:
            row.update(
                status="eliminated_unreferenced_library",
                evidence=[str(UNREFERENCED_LIBRARY_AUDIT.relative_to(ROOT))],
                rust_owner=None,
                rationale=(
                    "This isolated compiler-library formatter follows a far return, "
                    "has no direct caller in the expanded graph, and has no encoded "
                    "offset materialization in the executable. No recovered shipped "
                    "path references it, so no production API is introduced."
                ),
            )
        elif entry in AUTHORED_NO_OPERATIONS:
            body_hex = AUTHORED_NO_OPERATIONS[entry]
            body = bytes.fromhex(body_hex)
            if executable[entry : entry + len(body)] != body:
                raise RuntimeError(f"authored no-op body changed at {entry:#x}")
            row.update(
                status="eliminated_authored_no_operation",
                evidence=[str(EXECUTABLE.relative_to(ROOT))],
                body_hex=body_hex,
                rust_owner=None,
                rationale=(
                    "The original dispatch-table target is an authored return-only or "
                    "register-preserving no-op and requires no production operation."
                ),
            )
        elif entry in static_rows:
            static = static_rows[entry]
            row.update(
                status=("eliminated_static_host_adapter"
                        if static["kind"] == "reviewed_host_adapter"
                        else "verified_static_typed"),
                evidence=[str(STATIC_PORT_AUDIT.relative_to(ROOT)),
                          static["inherited_fixture"] or static["rust_owner"]["path"]],
                rust_owner=static["rust_owner"],
                rationale=" ".join(static["notes"]),
            )
        elif (
            coverage_row["comparison"] == "exact_body"
            and int(coverage_row["commander_entry"], 16) in eliminated
        ):
            commander = eliminated[int(coverage_row["commander_entry"], 16)]
            row.update(
                status="inherited_exact_eliminated",
                evidence=[str(COMPARISON.relative_to(ROOT)), commander["evidence"]],
                rust_owner={
                    "path": commander["rust_path"],
                    "symbol": commander["rust_symbol"],
                },
                rationale=(
                    "The complete BBB body is byte-identical to the audited Commander "
                    f"entry {commander['entry']}, whose {commander['disposition']} "
                    "disposition and typed replacement therefore apply unchanged."
                ),
            )
        elif (
            coverage_row["entered"]
            and (
                selected := choose_direct_evidence(
                    entry, coverage_row, oracle_rows, consumers
                )
            )
            is not None
        ):
            oracle, owner = selected
            row.update(
                status="verified_direct_typed",
                evidence=direct_evidence(oracle),
                rust_owner=owner,
                rationale=(
                    "Unchanged BBB execution enters this routine in the named oracle, "
                    "and that oracle's checked fixture rows are consumed by "
                    "the named Rust test owner."
                ),
            )
        elif (
            coverage_row["comparison"] == "exact_body"
            and int(coverage_row["commander_entry"], 16) in ported
        ):
            commander = ported[int(coverage_row["commander_entry"], 16)]
            row.update(
                status="inherited_exact_typed",
                evidence=[str(COMPARISON.relative_to(ROOT)), commander["evidence"]],
                rust_owner={
                    "path": commander["rust_path"],
                    "symbol": commander["rust_symbol"],
                },
                rationale=(
                    "The complete BBB body is byte-identical to the audited Commander "
                    f"entry {commander['entry']} and inherits its verified typed owner."
                ),
            )
        else:
            comparison = coverage_row["comparison"].replace("_", " ")
            row.update(
                status="pending_game_semantics",
                evidence=[str(COVERAGE.relative_to(ROOT))],
                rust_owner=None,
                rationale=(
                    f"The current evidence is {comparison} with no qualifying direct "
                    "BBB fixture-to-Rust ownership proof; semantic disposition remains "
                    "open."
                ),
            )
        rows.append(row)

    status_counts = Counter(row["status"] for row in rows)
    pending = status_counts["pending_game_semantics"]
    return {
        "format": "big_bug_bang_port_dispositions_v1",
        "scope": (
            "One disposition for every known BBB native entrypoint. Direct execution "
            "requires oracle output checked against a fixture consumed by Rust. "
            "Reviewed static equivalence is separately identified; fuzzy structural "
            "similarity alone remains pending."
        ),
        "inputs": {
            str(COVERAGE.relative_to(ROOT)): sha256(COVERAGE),
            str(COMPARISON.relative_to(ROOT)): sha256(COMPARISON),
            str(FIELD_AUDIT.relative_to(ROOT)): sha256(FIELD_AUDIT),
            str(STATIC_PORT_AUDIT.relative_to(ROOT)): sha256(STATIC_PORT_AUDIT),
            str(UNREFERENCED_LIBRARY_AUDIT.relative_to(ROOT)): sha256(
                UNREFERENCED_LIBRARY_AUDIT
            ),
            str(PORTED.relative_to(ROOT)): sha256(PORTED),
            str(ELIMINATED.relative_to(ROOT)): sha256(ELIMINATED),
            str(EXECUTABLE.relative_to(ROOT)): sha256(EXECUTABLE),
        },
        "summary": {
            "known_entrypoint_count": len(rows),
            "classified_entrypoint_count": len(rows) - pending,
            "pending_game_semantics_count": pending,
            "status_counts": dict(sorted(status_counts.items())),
        },
        "entrypoints": rows,
    }


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("output", type=Path)
    return parser.parse_args()


def main() -> None:
    args = parse_args()
    result = build_report()
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(result, indent=2) + "\n")
    summary = result["summary"]
    print(
        f"classified {summary['classified_entrypoint_count']}/"
        f"{summary['known_entrypoint_count']} BBB entrypoints; "
        f"{summary['pending_game_semantics_count']} pending"
    )


if __name__ == "__main__":
    main()
