#!/usr/bin/env python3
"""Run the recovered game's deterministic DOS runtime scenario matrix.

The matrix delegates runtime inspection to ``runtime_watchdog.py`` and
``capture_pterra_boundary.py``.  Each scenario receives a fresh copy of the
source ``cblood`` install directory and a stable, distinct X display.

Examples:

  python3 -P re/tools/runtime_scenario_matrix.py \
      --cd-dir output/recovered_dos_package/cd \
      --install-parent accuracy/cblood_install

  python3 -P re/tools/runtime_scenario_matrix.py \
      --cd-dir output/recovered_dos_package/cd \
      --install-parent accuracy/cblood_install \
      --scenario script1-bob-first-contact --scenario script2-radio

  python3 -P re/tools/runtime_scenario_matrix.py \
      --cd-dir output/recovered_dos_package/cd \
      --install-parent accuracy/cblood_install \
      --include-authentic-pterra
"""
from __future__ import annotations

import argparse
import concurrent.futures
import json
import re
import shlex
import shutil
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Sequence


ROOT = Path(__file__).resolve().parents[2]
WATCHDOG = Path(__file__).with_name("runtime_watchdog.py")
PTERRA_CAPTURE = Path(__file__).with_name("capture_pterra_boundary.py")
DEFAULT_OUTPUT_DIR = ROOT / "output" / "runtime-scenario-matrix"
DEFAULT_CONTACT_MANIFEST = ROOT / "re/vm/contact-manifest/contact-manifest.json"
AUTHENTIC_PTERRA = "authentic-pterra"
SCRIPT1_BOB_CHECKPOINTS = (
    (0x078E, "GOOD DAY COMMANDER. MY NAME IS BOB, BOB MORLOCK"),
    (0x07AE, "IF THE PHONE RINGS"),
    (0x07D4, "MY EARS ARE FRAGILE"),
    (0x07EA, "DO YOU WANT ME TO EXPLAIN YOUR MISSION"),
)
SCRIPT2_RADIO_CHECKPOINTS = (
    (0x2B05, "MESSAGE RADIO:"),
    (0x2BB5, "OKAY OKAY, WISE GUY!"),
    (0x2BC9, "YOU DO THE COUNTING"),
    (0x2BDB, "CRUIIIIK!"),
    (None, "REPORT FROM HONK"),
)


@dataclass(frozen=True)
class Scenario:
    name: str
    kind: str
    display_slot: int
    profile: int | None = None
    contact_selector: str | None = None


BASE_SCENARIOS = tuple(
    Scenario(f"teleport-{profile}", "teleport", profile, profile)
    for profile in range(5)
) + (
    Scenario("script2-radio", "radio", 5),
    Scenario("script1-bob-first-contact", "bob", 6),
    Scenario(AUTHENTIC_PTERRA, "pterra", 7),
)


def _load_contact_scenarios() -> tuple[Scenario, ...]:
    try:
        manifest = json.loads(DEFAULT_CONTACT_MANIFEST.read_text(encoding="utf-8"))
    except (OSError, UnicodeError, json.JSONDecodeError) as error:
        raise RuntimeError(
            f"cannot load contact scenarios from {DEFAULT_CONTACT_MANIFEST}: {error}"
        ) from error
    procedures = manifest.get("procedures") if isinstance(manifest, dict) else None
    if not isinstance(procedures, list) or len(procedures) != 65:
        raise RuntimeError("contact manifest must contain exactly 65 procedures")
    scenarios = []
    for index, procedure in enumerate(procedures):
        if not isinstance(procedure, dict):
            raise RuntimeError("contact manifest procedure is not an object")
        script = procedure.get("script")
        name = procedure.get("procedure")
        offset = procedure.get("procedure_offset")
        if not isinstance(script, str) or not isinstance(name, str) or not isinstance(offset, int):
            raise RuntimeError("contact manifest procedure identity is incomplete")
        slug = re.sub(r"[^a-z0-9]+", "-", name.lower()).strip("-")
        scenarios.append(
            Scenario(
                f"contact-{script.lower()}-{slug}-{offset:04x}",
                "contact",
                8 + index,
                int(script.removeprefix("SCRIPT")) - 1,
                f"{script}:{name}@{offset:04x}",
            )
        )
    return tuple(scenarios)


CONTACT_SCENARIOS = _load_contact_scenarios()
SCENARIOS = BASE_SCENARIOS + CONTACT_SCENARIOS
SCENARIO_BY_NAME = {scenario.name: scenario for scenario in SCENARIOS}
DEFAULT_SCENARIOS = tuple(
    scenario.name for scenario in BASE_SCENARIOS if scenario.kind != "pterra"
)


def build_parser() -> argparse.ArgumentParser:
    names = ", ".join(scenario.name for scenario in SCENARIOS)
    parser = argparse.ArgumentParser(
        description=(
            "Run isolated teleport and conversation watchdog scenarios and "
            "aggregate their JSON reports."
        ),
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog=(
            f"Scenarios: {names}\n"
            "With no --scenario, the five teleports, SCRIPT2 radio, and "
            "SCRIPT1 Bob probes run. Authentic-save Pterra is opt-in."
        ),
    )
    parser.add_argument(
        "--cd-dir",
        type=Path,
        help="recovered package CD directory containing the executable",
    )
    parser.add_argument(
        "--install-parent",
        type=Path,
        help="source C-drive directory containing cblood/",
    )
    parser.add_argument("--executable", default="BPRG_RE.EXE")
    parser.add_argument(
        "--dosbox",
        default="dosbox-x",
        help="DOSBox-X or DOSBox Staging executable (default: dosbox-x)",
    )
    parser.add_argument(
        "--link-map",
        type=Path,
        help="link map passed to runtime_watchdog.py (default: its package path)",
    )
    parser.add_argument(
        "--output-dir",
        type=Path,
        default=DEFAULT_OUTPUT_DIR,
        help=f"matrix artifacts (default: {DEFAULT_OUTPUT_DIR})",
    )
    parser.add_argument(
        "--scenario",
        action="append",
        choices=tuple(SCENARIO_BY_NAME),
        help="run only this scenario; repeat for a focused subset",
    )
    parser.add_argument(
        "--include-authentic-pterra",
        action="store_true",
        help="add the authentic-save Pterra route to the default or selected set",
    )
    parser.add_argument(
        "--all-contacts",
        action="store_true",
        help="add all 65 binary-derived contact procedures to the selected matrix",
    )
    parser.add_argument(
        "--list-scenarios",
        action="store_true",
        help="list scenario names and exit",
    )
    parser.add_argument(
        "--display-base",
        type=int,
        default=90,
        help="first X display number; stable scenario slots use base through base+72",
    )
    parser.add_argument(
        "--jobs",
        type=int,
        default=1,
        help="isolated scenarios to run concurrently (default: 1)",
    )
    parser.add_argument(
        "--teleport-seconds",
        type=float,
        default=120.0,
        help="watchdog duration for each teleport (default: 120)",
    )
    parser.add_argument(
        "--radio-seconds",
        type=float,
        default=240.0,
        help="watchdog duration for SCRIPT2 radio (default: 240)",
    )
    parser.add_argument(
        "--bob-seconds",
        type=float,
        default=240.0,
        help="watchdog duration for SCRIPT1 Bob first contact (default: 240)",
    )
    parser.add_argument(
        "--contact-seconds",
        type=float,
        default=240.0,
        help="watchdog duration for each generated contact probe (default: 240)",
    )
    parser.add_argument(
        "--contact-manifest",
        type=Path,
        default=DEFAULT_CONTACT_MANIFEST,
        help=f"binary-derived contact manifest (default: {DEFAULT_CONTACT_MANIFEST})",
    )
    parser.add_argument(
        "--pterra-timeout",
        type=float,
        default=1800.0,
        help="authentic-save Pterra capture timeout (default: 1800)",
    )
    parser.add_argument(
        "--calibration-timeout",
        type=float,
        default=30.0,
        help="watchdog calibration timeout (default: 30)",
    )
    parser.add_argument(
        "--post-teleport-samples",
        type=int,
        default=4,
        help="guarded samples required after a teleport (default: 4)",
    )
    parser.add_argument(
        "--input-liveness-samples",
        type=int,
        default=12,
        help="SCRIPT2 radio input-stall limit (default: 12)",
    )
    parser.add_argument(
        "--active-liveness-samples",
        type=int,
        default=120,
        help="SCRIPT2 radio presentation-stall limit (default: 120)",
    )
    parser.add_argument(
        "--hang-samples",
        type=int,
        default=120,
        help="non-main execution hot-loop limit (default: 120)",
    )
    parser.add_argument(
        "--subprocess-grace-seconds",
        type=float,
        default=30.0,
        help="extra time for tool startup and cleanup (default: 30)",
    )
    parser.add_argument(
        "--python",
        default=sys.executable,
        help="Python interpreter used for the existing runtime tools",
    )
    return parser


def selected_scenarios(
    requested: Sequence[str] | None,
    include_authentic_pterra: bool,
    all_contacts: bool = False,
) -> list[Scenario]:
    if requested:
        names = set(requested)
    elif all_contacts:
        names = set()
    else:
        names = set(DEFAULT_SCENARIOS)
    if include_authentic_pterra:
        names.add(AUTHENTIC_PTERRA)
    if all_contacts:
        names.update(scenario.name for scenario in CONTACT_SCENARIOS)
    return [scenario for scenario in SCENARIOS if scenario.name in names]


def _write_json(path: Path, value: object) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = path.with_name(path.name + ".tmp")
    temporary.write_text(
        json.dumps(value, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )
    temporary.replace(path)


def _read_report(path: Path) -> tuple[dict[str, object] | None, str | None]:
    if not path.is_file():
        return None, f"tool did not create report: {path}"
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, UnicodeError, json.JSONDecodeError) as error:
        return None, f"cannot read tool report: {type(error).__name__}: {error}"
    if not isinstance(value, dict):
        return None, "tool report root is not a JSON object"
    return value, None


def _validate_teleport(
    report: dict[str, object], profile: int
) -> list[str]:
    errors: list[str] = []
    if report.get("verdict") != "TELEPORTS-COMPLETE":
        errors.append("verdict is not TELEPORTS-COMPLETE")
    if report.get("anomalies") != []:
        errors.append("anomalies are present or missing")
    teleports = report.get("teleports")
    if not isinstance(teleports, list) or len(teleports) != 1:
        errors.append("report does not contain exactly one teleport")
        return errors
    teleport = teleports[0]
    if not isinstance(teleport, dict):
        errors.append("teleport record is not an object")
        return errors
    if teleport.get("target") != profile:
        errors.append(f"teleport target is not profile {profile}")
    if not isinstance(teleport.get("completed_sample"), int):
        errors.append("teleport completion sample is missing")
    state = teleport.get("completed_state")
    if not isinstance(state, dict):
        errors.append("teleport completed state is missing")
        return errors
    if state.get("profile") != profile:
        errors.append(f"completed profile is not {profile}")
    if state.get("request") != -1:
        errors.append("completed request was not cleared")
    if state.get("execution_enabled") != 1:
        errors.append("VM execution is not enabled")
    handles = state.get("handles")
    if not isinstance(handles, list) or handles != state.get("expected_handles"):
        errors.append("loaded handles do not match the selected profile")
    images = state.get("images")
    if (
        not isinstance(images, list)
        or len(images) != 5
        or any(
            not isinstance(image, str) or image.startswith("0000:")
            for image in images
        )
    ):
        errors.append("profile image pointers are incomplete")
    blockers = state.get("blockers")
    if (
        not isinstance(blockers, dict)
        or not blockers
        or blockers.get("vm_ui", 0) not in (0, 4)
        or any(
            value != 0
            for name, value in blockers.items()
            if name != "vm_ui"
        )
    ):
        errors.append("profile handoff blockers contain unexpected state")
    return errors


def _validate_radio(report: dict[str, object]) -> list[str]:
    errors: list[str] = []
    if report.get("verdict") != "RADIO-PROBE-COMPLETE":
        errors.append("verdict is not RADIO-PROBE-COMPLETE")
    if report.get("anomalies") != []:
        errors.append("anomalies are present or missing")
    probe = report.get("radio_probe")
    if not isinstance(probe, dict):
        errors.append("radio probe is missing")
        return errors
    if not isinstance(probe.get("completed_sample"), int):
        errors.append("radio completion sample is missing")
    errors.extend(
        _validate_dialogue_checkpoints(
            probe.get("checkpoints"), SCRIPT2_RADIO_CHECKPOINTS, "radio"
        )
    )
    return errors


def _validate_bob(report: dict[str, object]) -> list[str]:
    errors: list[str] = []
    if report.get("verdict") != "BOB-PROBE-COMPLETE":
        errors.append("verdict is not BOB-PROBE-COMPLETE")
    if report.get("anomalies") != []:
        errors.append("anomalies are present or missing")
    probe = report.get("bob_probe")
    if not isinstance(probe, dict):
        errors.append("Bob probe is missing")
        return errors
    if not isinstance(probe.get("completed_sample"), int):
        errors.append("Bob completion sample is missing")
    errors.extend(
        _validate_dialogue_checkpoints(
            probe.get("checkpoints"), SCRIPT1_BOB_CHECKPOINTS, "Bob"
        )
    )
    return errors


def _validate_contact(
    report: dict[str, object], scenario: Scenario
) -> list[str]:
    errors: list[str] = []
    if report.get("verdict") != "CONTACT-PROBE-COMPLETE":
        errors.append("verdict is not CONTACT-PROBE-COMPLETE")
    if report.get("anomalies") != []:
        errors.append("anomalies are present or missing")
    probe = report.get("contact_probe")
    if not isinstance(probe, dict):
        errors.append("contact probe is missing")
        return errors
    if probe.get("selector") != scenario.contact_selector:
        errors.append("contact selector does not match the scenario")
    if not isinstance(probe.get("completed_sample"), int):
        errors.append("contact completion sample is missing")
    if probe.get("completion_reason") not in (
        "line-target",
        "word-choice",
        "presentation-complete",
    ):
        errors.append("contact completion reason is invalid")
    checkpoints = probe.get("checkpoints")
    if not isinstance(checkpoints, list) or not checkpoints:
        errors.append("contact checkpoints are missing")
    elif any(
        not isinstance(checkpoint, dict)
        or not isinstance(checkpoint.get("menu_words_offset"), int)
        or not isinstance(checkpoint.get("subtitle"), str)
        for checkpoint in checkpoints
    ):
        errors.append("contact checkpoint data is malformed")
    setup = probe.get("setup")
    if not isinstance(setup, dict) or setup.get("selected_object") != probe.get(
        "contact_object_offset"
    ):
        errors.append("contact predicate setup is missing or selected the wrong object")
    return errors


def _validate_dialogue_checkpoints(
    actual: object,
    expected: Sequence[tuple[int | None, str]],
    label: str,
) -> list[str]:
    if not isinstance(actual, list):
        return [f"{label} checkpoints are missing"]
    if len(actual) != len(expected):
        return [
            f"{label} checkpoint count is {len(actual)}, expected {len(expected)}"
        ]
    errors: list[str] = []
    for index, ((expected_offset, expected_text), checkpoint) in enumerate(
        zip(expected, actual, strict=True), start=1
    ):
        if not isinstance(checkpoint, dict):
            errors.append(f"{label} checkpoint {index} is not an object")
            continue
        if (
            expected_offset is not None
            and checkpoint.get("menu_words_offset") != expected_offset
        ):
            errors.append(
                f"{label} checkpoint {index} word-list offset is not "
                f"{expected_offset:#06x}"
            )
        subtitle = checkpoint.get("subtitle")
        if not isinstance(subtitle, str) or expected_text not in subtitle.upper():
            errors.append(
                f"{label} checkpoint {index} subtitle does not contain "
                f"{expected_text!r}"
            )
    return errors


def _validate_pterra(report: dict[str, object]) -> list[str]:
    errors: list[str] = []
    if report.get("mode") != "authentic-save-pterra":
        errors.append("capture mode is not authentic-save-pterra")
    if report.get("errors") != []:
        errors.append("capture errors are present or missing")
    title_evidence = report.get("title_transition_evidence")
    if report.get("title_transition_confirmed") is not True:
        errors.append("native title transition was not confirmed")
    if not isinstance(title_evidence, list) or not title_evidence \
            or not all(item in {
                "startup-presentation-line",
                "native-gameplay-load-boundary",
                "authentic-save-loaded",
            } for item in title_evidence):
        errors.append("native title transition evidence is invalid")
    if report.get("authentic_save_loaded") is not True:
        errors.append("authentic GAME1.SAV load was not observed")
    for key in (
        "fault_detected",
        "dos_read_overflow_detected",
        "integrity_fault_detected",
        "hang_detected",
    ):
        if report.get(key) is not False:
            errors.append(f"{key} is true or missing")
    for key, label in (
        ("pterra_unlock_requested", "SCRIPT2 Pterra unlock was not requested"),
        ("pterra_unlock_completed", "SCRIPT2 Pterra unlock did not complete"),
        ("pterra_nav_chart_started", "native nav chart was not started"),
        ("pterra_nav_chart_active", "native nav chart did not become active"),
        ("pterra_nav_chart_selected", "native nav chart did not select Pterra"),
        ("pterra_nav_panel_close_confirmed",
         "native Pterra location panel did not complete"),
        ("pterra_map_command_generated",
         "native map did not generate the Pterra C1 command"),
        ("pterra_map_command_consumed",
         "native VM did not consume the map Pterra C1 command"),
        ("pterra_map_destination_committed",
         "native map travel did not commit Pterra"),
        ("pterra_ship_navigation_activated",
         "native current-location interaction did not activate ship navigation"),
        ("pterra_travel_command_generated",
         "native ship HUD did not generate the Orxx Pterra C1 command"),
        ("pterra_travel_command_consumed",
         "native VM did not consume the Orxx Pterra C1 command"),
    ):
        if report.get(key) is not True:
            errors.append(label)
    map_setup = report.get("pterra_map_setup")
    if not isinstance(map_setup, dict):
        errors.append("native Pterra map setup is missing")
    else:
        chart_offsets = map_setup.get("chart_object_offsets")
        if not isinstance(chart_offsets, list) or 0x0DA0 not in chart_offsets:
            errors.append("native nav chart setup does not contain Pterra")
        if map_setup.get("pterra_marker") != [201, 93]:
            errors.append("native Pterra chart marker is not exact")
        if map_setup.get("generated_via") not in (
                "arche-action", "deferred-record"):
            errors.append("native Pterra map C1 provenance is invalid")
        if map_setup.get("panel_close_confirmed") is not True:
            errors.append("native Pterra panel close evidence is missing")
    travel_setup = report.get("pterra_travel_setup")
    target_row = report.get("pterra_target_row")
    if not isinstance(travel_setup, dict):
        errors.append("native Pterra ship-HUD setup is missing")
    else:
        if travel_setup.get("entry") != "native-current-location-entity" \
                or travel_setup.get("entity_index") != 31 \
                or not isinstance(travel_setup.get("entity_rect"), list) \
                or int(travel_setup.get("entity_click_count", 0)) <= 0:
            errors.append(
                "native ship navigation lacks current-location input evidence")
        if travel_setup.get("pterra_access_count_before") == 0 \
                and travel_setup.get("intro_hold_dismissed") is not True:
            errors.append(
                "first-visit ship intro was not dismissed before HUD entry")
        target_offsets = travel_setup.get("target_name_offsets")
        setup_row = travel_setup.get("pterra_target_row")
        if not isinstance(target_offsets, list) \
                or not isinstance(setup_row, int) \
                or setup_row < 0 \
                or setup_row >= len(target_offsets) \
                or target_offsets[setup_row] != 0x0DA4:
            errors.append("native ship target row does not identify Pterra")
        if target_row != setup_row:
            errors.append("reported Pterra target row is inconsistent")
        click_evidence = travel_setup.get("target_click_evidence")
        click_point = (click_evidence.get("point")
                       if isinstance(click_evidence, dict) else None)
        if not isinstance(click_evidence, dict) \
                or click_evidence.get("adapter") not in (
                    "host-primary-edge", "guest-primary-edge") \
                or not isinstance(click_point, list) \
                or len(click_point) != 2 \
                or not all(isinstance(value, int) for value in click_point):
            errors.append("native Pterra target click evidence is invalid")
    if report.get("destination_committed") is not True:
        errors.append("Pterra destination was not committed")
    if report.get("scruter_scene_requested") is not True:
        errors.append("native Scruter_Jo descriptor transition was not requested")
    if report.get("scruter_scene_active_seen") is not True:
        errors.append("native Scruter_Jo descriptor transition never became active")
    if report.get("scruter_scene_completed") is not True:
        errors.append("native Scruter_Jo Pterra lifecycle did not complete")
    if report.get("scruter_sound_bank_loaded") is not True:
        errors.append("native Scruter_Jo streamed sound bank was not loaded")
    clip_count_before = report.get("scruter_streamed_clip_count_before")
    clip_count = report.get("scruter_streamed_clip_count")
    if not isinstance(clip_count_before, int) or clip_count_before < 0:
        errors.append("Scruter_Jo initial streamed clip count is invalid")
    if not isinstance(clip_count, int) or clip_count <= 0:
        errors.append("Scruter_Jo streamed clip count is not positive")
    elif isinstance(clip_count_before, int) and clip_count == clip_count_before:
        errors.append("Scruter_Jo streamed clip count did not change")
    if report.get("pter_reached") is not True or report.get("pter") is None:
        errors.append("Pterra procedure was not reached")
    if report.get("pter_completed") is not True:
        errors.append("Pterra procedure was not completed")
    choices = report.get("pter_choice_results")
    if choices != [0x0171, 0x02A8]:
        errors.append("Pterra scripted choices are not exxos then teleport")
    if report.get("pter_sustained") is not True or report.get("post_pter") is None:
        errors.append("Pterra post-encounter liveness was not sustained")
    if report.get("marker") is None:
        errors.append("Pterra boundary marker is missing")
    return errors


def validate_report(
    scenario: Scenario, report: dict[str, object]
) -> list[str]:
    if scenario.kind == "teleport":
        assert scenario.profile is not None
        return _validate_teleport(report, scenario.profile)
    if scenario.kind == "radio":
        return _validate_radio(report)
    if scenario.kind == "bob":
        return _validate_bob(report)
    if scenario.kind == "contact":
        return _validate_contact(report, scenario)
    if scenario.kind == "pterra":
        return _validate_pterra(report)
    return [f"unknown scenario kind: {scenario.kind}"]


def _build_command(
    args: argparse.Namespace,
    scenario: Scenario,
    install_parent: Path,
    raw_report: Path,
    display: str,
    artifact_dir: Path,
) -> tuple[list[str], float]:
    common = [
        args.python,
        "-P",
    ]
    if scenario.kind in ("teleport", "radio", "bob", "contact"):
        command = common + [
            str(WATCHDOG),
            "--cd-dir",
            str(args.cd_dir),
            "--install-parent",
            str(install_parent),
            "--executable",
            args.executable,
            "--dosbox",
            args.dosbox,
            "--display",
            display,
            "--calibration-timeout",
            str(args.calibration_timeout),
            "--report",
            str(raw_report),
            "--xvfb",
        ]
        if args.link_map is not None:
            command += ["--link-map", str(args.link_map)]
        if scenario.kind == "teleport":
            assert scenario.profile is not None
            command += [
                "--seconds",
                str(args.teleport_seconds),
                "--teleport-profile",
                str(scenario.profile),
                "--post-teleport-samples",
                str(args.post_teleport_samples),
            ]
            return command, args.teleport_seconds + args.subprocess_grace_seconds
        if scenario.kind == "radio":
            command += [
                "--seconds",
                str(args.radio_seconds),
                "--script2-radio-probe",
            ]
            duration = args.radio_seconds
        elif scenario.kind == "bob":
            command += [
                "--seconds",
                str(args.bob_seconds),
                "--script1-bob-probe",
            ]
            duration = args.bob_seconds
        else:
            assert scenario.contact_selector is not None
            command += [
                "--seconds",
                str(args.contact_seconds),
                "--contact-manifest",
                str(args.contact_manifest),
                "--contact-probe",
                scenario.contact_selector,
            ]
            # The watchdog gives setup and post-selection dialogue their own
            # bounded windows so a slow native save load cannot consume the
            # contact observation period.
            duration = args.contact_seconds * 2
        command += [
            "--input-liveness-samples",
            str(args.input_liveness_samples),
            "--active-liveness-samples",
            str(args.active_liveness_samples),
            "--hang-samples",
            str(args.hang_samples),
        ]
        return command, duration + args.subprocess_grace_seconds

    command = common + [
        str(PTERRA_CAPTURE),
        "--cd-dir",
        str(args.cd_dir),
        "--install-parent",
        str(install_parent),
        "--executable",
        args.executable,
        "--dosbox",
        args.dosbox,
        "--output",
        str(raw_report),
        "--display",
        display,
        "--timeout",
        str(args.pterra_timeout),
        "--manual-pterra",
        "--open-load-menu",
        "--trigger-pterra-after-load",
        "--drive-authentic-save",
        "--guest-snapshot",
        str(artifact_dir / "guest-data.bin"),
        "--dosbox-log",
        str(artifact_dir / "dosbox.log"),
    ]
    return command, args.pterra_timeout + args.subprocess_grace_seconds


def _text(value: str | bytes | None) -> str:
    if value is None:
        return ""
    if isinstance(value, bytes):
        return value.decode("utf-8", errors="replace")
    return value


def _run_one(
    args: argparse.Namespace,
    scenario: Scenario,
    output_dir: Path,
    source_cblood: Path,
) -> dict[str, object]:
    display = f":{args.display_base + scenario.display_slot}"
    work_dir = output_dir / "work" / scenario.name
    install_parent = work_dir / "install"
    raw_report = output_dir / "reports" / f"{scenario.name}.json"
    result_path = output_dir / "results" / f"{scenario.name}.json"
    log_path = output_dir / "logs" / f"{scenario.name}.log"
    artifact_dir = output_dir / "artifacts" / scenario.name
    result: dict[str, object] = {
        "display": display,
        "install_parent": str(install_parent),
        "name": scenario.name,
        "raw_report": str(raw_report),
        "status": "FAIL",
        "validation_errors": [],
    }
    errors = result["validation_errors"]
    assert isinstance(errors, list)

    try:
        if work_dir.exists():
            shutil.rmtree(work_dir)
        if raw_report.exists():
            raw_report.unlink()
        artifact_dir.mkdir(parents=True, exist_ok=True)
        copied_cblood = install_parent / "cblood"
        shutil.copytree(source_cblood, copied_cblood, symlinks=True)
        if scenario.kind == "pterra":
            removed = []
            for stale in sorted(copied_cblood.glob("PTERRA1[DFG].LBM")):
                removed.append(stale.name)
                stale.unlink()
            result["removed_stale_artifacts"] = removed
        command, timeout = _build_command(
            args,
            scenario,
            install_parent,
            raw_report,
            display,
            artifact_dir,
        )
        result["command"] = command
        result["timeout_seconds"] = timeout
        try:
            process = subprocess.run(
                command,
                cwd=ROOT,
                text=True,
                capture_output=True,
                timeout=timeout,
                check=False,
            )
            stdout = process.stdout
            stderr = process.stderr
            returncode: int | None = process.returncode
            timed_out = False
        except subprocess.TimeoutExpired as error:
            stdout = _text(error.stdout)
            stderr = _text(error.stderr)
            returncode = None
            timed_out = True
            errors.append(f"subprocess exceeded {timeout:g} seconds")
        except OSError as error:
            stdout = ""
            stderr = ""
            returncode = None
            timed_out = False
            errors.append(f"cannot run subprocess: {type(error).__name__}: {error}")

        result["process"] = {
            "returncode": returncode,
            "timed_out": timed_out,
        }
        log_path.parent.mkdir(parents=True, exist_ok=True)
        log_path.write_text(
            "$ "
            + shlex.join(command)
            + "\n\n[stdout]\n"
            + stdout
            + "\n[stderr]\n"
            + stderr,
            encoding="utf-8",
        )
        result["log"] = str(log_path)
        if returncode not in (0, None):
            errors.append(f"subprocess exited with status {returncode}")

        report, report_error = _read_report(raw_report)
        if report_error is not None:
            errors.append(report_error)
        elif report is not None:
            result["tool_verdict"] = report.get("verdict", report.get("mode"))
            errors.extend(validate_report(scenario, report))
    except (OSError, shutil.Error) as error:
        errors.append(f"scenario setup failed: {type(error).__name__}: {error}")

    if not errors:
        result["status"] = "PASS"
    result["result"] = str(result_path)
    _write_json(result_path, result)
    return result


def _validate_arguments(
    parser: argparse.ArgumentParser, args: argparse.Namespace
) -> None:
    if args.cd_dir is None:
        parser.error("--cd-dir is required unless --list-scenarios is used")
    if args.install_parent is None:
        parser.error("--install-parent is required unless --list-scenarios is used")
    positive = (
        ("--teleport-seconds", args.teleport_seconds),
        ("--radio-seconds", args.radio_seconds),
        ("--bob-seconds", args.bob_seconds),
        ("--contact-seconds", args.contact_seconds),
        ("--pterra-timeout", args.pterra_timeout),
        ("--calibration-timeout", args.calibration_timeout),
        ("--subprocess-grace-seconds", args.subprocess_grace_seconds),
        ("--post-teleport-samples", args.post_teleport_samples),
    )
    for name, value in positive:
        if value <= 0:
            parser.error(f"{name} must be positive")
    for name, value in (
        ("--input-liveness-samples", args.input_liveness_samples),
        ("--active-liveness-samples", args.active_liveness_samples),
        ("--hang-samples", args.hang_samples),
    ):
        if value < 0:
            parser.error(f"{name} cannot be negative")
    if args.display_base < 1:
        parser.error("--display-base must be at least 1")
    if args.jobs < 1:
        parser.error("--jobs must be positive")

    args.cd_dir = args.cd_dir.resolve()
    args.install_parent = args.install_parent.resolve()
    args.output_dir = args.output_dir.resolve()
    args.contact_manifest = args.contact_manifest.resolve()
    if args.link_map is not None:
        args.link_map = args.link_map.resolve()
    if not args.cd_dir.is_dir():
        parser.error(f"CD directory does not exist: {args.cd_dir}")
    if not (args.cd_dir / args.executable).is_file():
        parser.error(f"executable does not exist: {args.cd_dir / args.executable}")
    source_cblood = args.install_parent / "cblood"
    if not source_cblood.is_dir():
        parser.error(f"install tree does not exist: {source_cblood}")
    if args.output_dir == source_cblood or args.output_dir.is_relative_to(
        source_cblood
    ):
        parser.error("--output-dir cannot be inside the source cblood tree")
    if args.link_map is not None and not args.link_map.is_file():
        parser.error(f"link map does not exist: {args.link_map}")
    if not args.contact_manifest.is_file():
        parser.error(f"contact manifest does not exist: {args.contact_manifest}")


def run_matrix(args: argparse.Namespace) -> tuple[int, dict[str, object]]:
    scenarios = selected_scenarios(
        args.scenario,
        args.include_authentic_pterra,
        args.all_contacts,
    )
    output_dir = args.output_dir
    output_dir.mkdir(parents=True, exist_ok=True)
    source_cblood = args.install_parent / "cblood"
    if args.jobs == 1:
        results = [
            _run_one(args, scenario, output_dir, source_cblood)
            for scenario in scenarios
        ]
    else:
        with concurrent.futures.ThreadPoolExecutor(
            max_workers=args.jobs
        ) as executor:
            results = list(
                executor.map(
                    lambda scenario: _run_one(
                        args, scenario, output_dir, source_cblood
                    ),
                    scenarios,
                )
            )
    passed = sum(result["status"] == "PASS" for result in results)
    aggregate: dict[str, object] = {
        "cd_dir": str(args.cd_dir),
        "executable": args.executable,
        "install_source": str(args.install_parent),
        "passed": passed,
        "results": results,
        "scenario_count": len(results),
        "selected_scenarios": [scenario.name for scenario in scenarios],
        "status": "PASS" if passed == len(results) else "FAIL",
    }
    aggregate_path = output_dir / "matrix.json"
    _write_json(aggregate_path, aggregate)
    for result in results:
        print(f"{result['name']}: {result['status']} ({result['display']})")
    print(f"matrix: {aggregate['status']} ({passed}/{len(results)}) {aggregate_path}")
    return (0 if aggregate["status"] == "PASS" else 1), aggregate


def main(argv: Sequence[str] | None = None) -> int:
    parser = build_parser()
    args = parser.parse_args(argv)
    if args.list_scenarios:
        for scenario in SCENARIOS:
            default = " [default]" if scenario.name in DEFAULT_SCENARIOS else ""
            print(f"{scenario.name}{default}")
        return 0
    _validate_arguments(parser, args)
    return run_matrix(args)[0]


if __name__ == "__main__":
    raise SystemExit(main())
