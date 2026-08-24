#!/usr/bin/env python3
"""Verify DOS guest liveness across host focus loss and recapture."""
from __future__ import annotations

import argparse
import json
import os
import signal
import subprocess
import sys
import time
from pathlib import Path
from typing import Callable


ROOT = Path(__file__).resolve().parents[2]
WATCHDOG = Path(__file__).with_name("runtime_watchdog.py")
WATCHDOG_SUCCESS = "TIMEOUT-NO-ANOMALY"
PROBE_SUCCESS = "FOCUS-RECAPTURE-COMPLETE"


class FocusProbeError(RuntimeError):
    """The focus transition or its runtime evidence was invalid."""


def write_json(path: Path, value: object) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = path.with_name(path.name + ".tmp")
    temporary.write_text(
        json.dumps(value, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )
    temporary.replace(path)


def read_json(path: Path) -> dict[str, object] | None:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, UnicodeError, json.JSONDecodeError):
        return None
    return value if isinstance(value, dict) else None


def latest_runtime_point(report: dict[str, object]) -> dict[str, int]:
    samples = report.get("runtime_samples")
    if not isinstance(samples, list) or not samples:
        raise FocusProbeError("watchdog has no guarded runtime sample")
    sample = samples[-1]
    if not isinstance(sample, dict):
        raise FocusProbeError("watchdog runtime sample is not an object")
    audio = sample.get("audio_flow")
    presentation = sample.get("presentation_flow")
    if not isinstance(audio, dict) or not isinstance(presentation, dict):
        raise FocusProbeError("watchdog runtime sample has no flow state")

    fields = {
        "sample": sample.get("sample"),
        "timer_tick": audio.get("timer_tick"),
        "timer_hook_active": audio.get("timer_hook_active"),
        "game_mode": audio.get("game_mode"),
        "mouse_x": presentation.get("mouse_x"),
        "mouse_y": presentation.get("mouse_y"),
    }
    if not all(isinstance(value, int) for value in fields.values()):
        raise FocusProbeError("watchdog runtime point contains a non-integer")
    return {name: int(value) for name, value in fields.items()}


def timer_delta(before: int, after: int) -> int:
    return (after - before) & 0xFFFF


def validate_runtime_points(
    before: dict[str, int],
    unfocused: dict[str, int],
    restored: dict[str, int],
) -> list[str]:
    errors = []
    for label, point in (
        ("before", before),
        ("unfocused", unfocused),
        ("restored", restored),
    ):
        if point["timer_hook_active"] & 1 == 0:
            errors.append(f"{label} sample has no active timer hook")
        if point["game_mode"] & 1 != 0:
            errors.append(f"{label} sample is not in interrupt-driven game mode")
        if not 0 <= point["mouse_x"] < 320 or not 0 <= point["mouse_y"] < 200:
            errors.append(f"{label} sample has an invalid guest mouse position")
    if unfocused["sample"] <= before["sample"]:
        errors.append("watchdog sampling stopped while the game was unfocused")
    if restored["sample"] <= unfocused["sample"]:
        errors.append("watchdog sampling stopped after game focus was restored")
    if timer_delta(before["timer_tick"], unfocused["timer_tick"]) == 0:
        errors.append("guest timer stopped while the game was unfocused")
    if timer_delta(unfocused["timer_tick"], restored["timer_tick"]) == 0:
        errors.append("guest timer stopped after game focus was restored")
    return errors


def run_xdotool(
    display: str,
    *arguments: str,
    capture_output: bool = False,
) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        ["xdotool", *arguments],
        env=dict(os.environ, DISPLAY=display),
        text=True,
        stdout=subprocess.PIPE if capture_output else subprocess.DEVNULL,
        stderr=subprocess.PIPE,
        check=False,
    )


def find_window(display: str, title: str, timeout: float) -> str:
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        result = run_xdotool(display, "search", "--name", title, capture_output=True)
        windows = [line.strip() for line in result.stdout.splitlines() if line.strip()]
        if result.returncode == 0 and windows:
            return windows[0]
        time.sleep(0.1)
    raise FocusProbeError(f"could not locate X window named {title!r}")


def focused_window(display: str) -> str:
    result = run_xdotool(display, "getwindowfocus", capture_output=True)
    window = result.stdout.strip()
    if result.returncode != 0 or not window:
        raise FocusProbeError("could not read the focused X window")
    return window


def focus_window(display: str, window: str) -> None:
    result = run_xdotool(display, "windowfocus", "--sync", window)
    if result.returncode != 0:
        raise FocusProbeError(f"could not focus X window {window}")
    if focused_window(display) != window:
        raise FocusProbeError(f"X focus did not move to window {window}")


def wait_for_report(
    path: Path,
    process: subprocess.Popen[str],
    timeout: float,
    predicate: Callable[[dict[str, object]], bool],
    description: str,
) -> dict[str, object]:
    deadline = time.monotonic() + timeout
    latest = None
    while time.monotonic() < deadline:
        latest = read_json(path)
        if latest is not None and predicate(latest):
            return latest
        if process.poll() is not None:
            raise FocusProbeError(
                f"watchdog exited before {description} with status "
                f"{process.returncode}"
            )
        time.sleep(0.1)
    suffix = "" if latest is None else f"; last verdict={latest.get('verdict')}"
    raise FocusProbeError(f"timed out waiting for {description}{suffix}")


def wait_for_new_runtime_point(
    path: Path,
    process: subprocess.Popen[str],
    timeout: float,
    after_sample: int,
) -> tuple[dict[str, object], dict[str, int]]:
    report = wait_for_report(
        path,
        process,
        timeout,
        lambda value: (
            isinstance(value.get("runtime_samples"), list)
            and bool(value["runtime_samples"])
            and isinstance(value["runtime_samples"][-1], dict)
            and isinstance(value["runtime_samples"][-1].get("sample"), int)
            and value["runtime_samples"][-1]["sample"] > after_sample
        ),
        "a newer guarded runtime sample",
    )
    return report, latest_runtime_point(report)


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        description="Prove DOS guest liveness across host focus loss and recapture."
    )
    parser.add_argument("--cd-dir", type=Path, required=True)
    parser.add_argument("--install-parent", type=Path, required=True)
    parser.add_argument("--executable", default="BPRG_RE.EXE")
    parser.add_argument("--link-map", type=Path)
    parser.add_argument("--dosbox", default="dosbox-x")
    parser.add_argument("--display", default=":163")
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--artifact-dir", type=Path)
    parser.add_argument("--calibration-timeout", type=float, default=30.0)
    parser.add_argument("--pre-focus-seconds", type=float, default=2.0)
    parser.add_argument("--unfocused-seconds", type=float, default=12.0)
    parser.add_argument("--post-focus-seconds", type=float, default=5.0)
    parser.add_argument("--hang-samples", type=int, default=120)
    return parser


def main() -> int:
    parser = build_parser()
    args = parser.parse_args()
    for name in (
        "calibration_timeout",
        "pre_focus_seconds",
        "unfocused_seconds",
        "post_focus_seconds",
    ):
        if getattr(args, name) <= 0:
            parser.error("--" + name.replace("_", "-") + " must be positive")
    if args.hang_samples < 0:
        parser.error("--hang-samples cannot be negative")

    output = args.output.resolve()
    artifact_dir = (
        args.artifact_dir.resolve()
        if args.artifact_dir is not None
        else output.with_name(output.stem + "-artifacts")
    )
    artifact_dir.mkdir(parents=True, exist_ok=True)
    watchdog_report = artifact_dir / "watchdog.json"
    watchdog_log = artifact_dir / "watchdog.log"
    dosbox_log = artifact_dir / "dosbox.log"
    focus_token = f"CBLOOD_FOCUS_PROBE_{os.getpid()}"
    watchdog_seconds = (
        args.calibration_timeout
        + args.pre_focus_seconds
        + args.unfocused_seconds
        + args.post_focus_seconds
        + 5.0
    )
    command = [
        sys.executable,
        "-P",
        str(WATCHDOG),
        "--cd-dir",
        str(args.cd_dir.resolve()),
        "--install-parent",
        str(args.install_parent.resolve()),
        "--executable",
        args.executable,
        "--dosbox",
        args.dosbox,
        "--display",
        args.display,
        "--seconds",
        str(watchdog_seconds),
        "--calibration-timeout",
        str(args.calibration_timeout),
        "--hang-samples",
        str(args.hang_samples),
        "--dosbox-log",
        str(dosbox_log),
        "--report",
        str(watchdog_report),
        "--xvfb",
    ]
    if args.link_map is not None:
        command += ["--link-map", str(args.link_map.resolve())]

    report: dict[str, object] = {
        "verdict": "INCOMPLETE",
        "errors": [],
        "display": args.display,
        "watchdog_command": command,
        "watchdog_report": str(watchdog_report),
        "watchdog_log": str(watchdog_log),
        "dosbox_log": str(dosbox_log),
        "focus_events": [],
    }
    errors = report["errors"]
    events = report["focus_events"]
    assert isinstance(errors, list) and isinstance(events, list)
    watchdog = None
    sink = None
    started = time.monotonic()
    try:
        log_stream = watchdog_log.open("w", encoding="utf-8")
        watchdog = subprocess.Popen(
            command,
            cwd=ROOT,
            text=True,
            stdout=log_stream,
            stderr=subprocess.STDOUT,
        )
        initial = wait_for_report(
            watchdog_report,
            watchdog,
            args.calibration_timeout + 8.0,
            lambda value: (
                value.get("calibrated") is not None
                and isinstance(value.get("runtime_samples"), list)
                and bool(value["runtime_samples"])
            ),
            "watchdog calibration and its first guarded sample",
        )
        game_window = find_window(args.display, Path(args.executable).stem, 5.0)
        sink = subprocess.Popen(
            [
                "xterm",
                "-display",
                args.display,
                "-title",
                focus_token,
                "-name",
                focus_token,
                "-geometry",
                "20x4+0+0",
                "-e",
                "sleep",
                str(watchdog_seconds + 30.0),
            ],
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        )
        sink_window = find_window(args.display, focus_token, 5.0)
        focus_window(args.display, game_window)
        events.append(
            {
                "phase": "game-focused",
                "elapsed_seconds": round(time.monotonic() - started, 3),
                "window": game_window,
                "focused_window": focused_window(args.display),
            }
        )

        initial_point = latest_runtime_point(initial)
        time.sleep(args.pre_focus_seconds)
        _, before = wait_for_new_runtime_point(
            watchdog_report,
            watchdog,
            3.0,
            initial_point["sample"],
        )
        focus_window(args.display, sink_window)
        events.append(
            {
                "phase": "game-unfocused",
                "elapsed_seconds": round(time.monotonic() - started, 3),
                "window": sink_window,
                "focused_window": focused_window(args.display),
            }
        )
        time.sleep(args.unfocused_seconds)
        _, unfocused = wait_for_new_runtime_point(
            watchdog_report,
            watchdog,
            3.0,
            before["sample"],
        )

        focus_window(args.display, game_window)
        events.append(
            {
                "phase": "game-refocused",
                "elapsed_seconds": round(time.monotonic() - started, 3),
                "window": game_window,
                "focused_window": focused_window(args.display),
            }
        )
        time.sleep(args.post_focus_seconds)
        _, restored = wait_for_new_runtime_point(
            watchdog_report,
            watchdog,
            3.0,
            unfocused["sample"],
        )
        report["timer_evidence"] = {
            "before": before,
            "unfocused": unfocused,
            "restored": restored,
            "unfocused_delta": timer_delta(
                before["timer_tick"], unfocused["timer_tick"]
            ),
            "restored_delta": timer_delta(
                unfocused["timer_tick"], restored["timer_tick"]
            ),
        }
        errors.extend(validate_runtime_points(before, unfocused, restored))

        watchdog.wait(timeout=watchdog_seconds + 10.0)
        final_watchdog = read_json(watchdog_report)
        if final_watchdog is None:
            errors.append("watchdog did not produce a readable final report")
        else:
            report["watchdog"] = {
                "verdict": final_watchdog.get("verdict"),
                "samples": final_watchdog.get("samples"),
                "guarded_samples": final_watchdog.get("guarded_samples"),
                "anomalies": final_watchdog.get("anomalies"),
            }
            if final_watchdog.get("verdict") != WATCHDOG_SUCCESS:
                errors.append(
                    "watchdog verdict is not TIMEOUT-NO-ANOMALY: "
                    + str(final_watchdog.get("verdict"))
                )
            if final_watchdog.get("anomalies") != []:
                errors.append("watchdog reported one or more runtime anomalies")
        if watchdog.returncode != 0:
            errors.append(f"watchdog exited with status {watchdog.returncode}")
    except (FocusProbeError, OSError, subprocess.SubprocessError) as error:
        errors.append(f"{type(error).__name__}: {error}")
    finally:
        if sink is not None and sink.poll() is None:
            sink.terminate()
            try:
                sink.wait(timeout=3.0)
            except subprocess.TimeoutExpired:
                sink.kill()
                sink.wait()
        if watchdog is not None and watchdog.poll() is None:
            # SIGINT lets runtime_watchdog execute its cleanup block and reap
            # the DOSBox and Xvfb children before this wrapper falls back to
            # terminating it.
            watchdog.send_signal(signal.SIGINT)
            try:
                watchdog.wait(timeout=5.0)
            except subprocess.TimeoutExpired:
                watchdog.terminate()
                try:
                    watchdog.wait(timeout=3.0)
                except subprocess.TimeoutExpired:
                    watchdog.kill()
                    watchdog.wait()
        if "log_stream" in locals():
            log_stream.close()

    report["elapsed_seconds"] = round(time.monotonic() - started, 3)
    if not errors:
        report["verdict"] = PROBE_SUCCESS
    write_json(output, report)
    print(json.dumps({"verdict": report["verdict"], "errors": errors}))
    return 0 if report["verdict"] == PROBE_SUCCESS else 1


if __name__ == "__main__":
    raise SystemExit(main())
