#!/usr/bin/env python3
"""Build and run the recovered BLOODPRG startup parser slice in DOSBox-X."""

from __future__ import annotations

from pathlib import Path
import os
import shutil
import subprocess
import sys


ROOT = Path(__file__).resolve().parents[1]
INTEGRATION_DIR = ROOT / "integration" / "dos"
OUT_DIR = INTEGRATION_DIR / "out" / "bloodprg_startup_options"
INCLUDE_DIR = (
    ROOT / "source" / "bloodprg" / "candidates" / "include"
)
SOURCE = INTEGRATION_DIR / "bloodprg_startup_options.c"
RECOVERED_SOURCES = (
    ROOT
    / "source"
    / "bloodprg"
    / "candidates"
    / "seg_0000"
    / "func_0006f1_startup_command_line_parse.c",
    ROOT
    / "source"
    / "bloodprg"
    / "candidates"
    / "seg_0000"
    / "func_000726_startup_option_apply.c",
    ROOT
    / "source"
    / "bloodprg"
    / "candidates"
    / "seg_01ce"
    / "func_002612_ascii_digit_parse.c",
)
EXECUTABLE_NAME = "BSTART.EXE"


def resolve_executable(value: str) -> str:
    resolved = shutil.which(value)
    if resolved is None:
        raise SystemExit(f"executable not found: {value}")
    return resolved


def run_checked(command: list[str], *, cwd: Path, timeout: int | None = None) -> None:
    try:
        process = subprocess.run(
            command,
            cwd=cwd,
            env=os.environ.copy(),
            capture_output=True,
            text=True,
            timeout=timeout,
            check=False,
        )
    except subprocess.TimeoutExpired as error:
        raise SystemExit(f"command timed out: {' '.join(command)}") from error
    if process.returncode != 0:
        output = "\n".join(
            part for part in (process.stdout, process.stderr) if part
        )
        raise SystemExit(
            f"command exited {process.returncode}: {' '.join(command)}\n{output}"
        )


def build(wcl: str) -> Path:
    if OUT_DIR.exists():
        shutil.rmtree(OUT_DIR)
    OUT_DIR.mkdir(parents=True)
    executable = OUT_DIR / EXECUTABLE_NAME
    command = [
        wcl,
        "-q",
        "-3",
        "-ox",
        "-mm",
        "-zdp",
        "-we",
        "-lr",
        f"-i={INCLUDE_DIR}",
        f"-fe={executable}",
        str(SOURCE),
        *(str(source) for source in RECOVERED_SOURCES),
    ]
    run_checked(command, cwd=OUT_DIR)
    if not executable.is_file():
        raise SystemExit(f"compiler did not create {executable}")
    return executable


def run_dosbox(dosbox: str, executable: Path) -> None:
    env = os.environ.copy()
    env["SDL_AUDIODRIVER"] = "dummy"
    env["SDL_VIDEODRIVER"] = "offscreen"
    run_checked(
        [
            dosbox,
            "--noprimaryconf",
            "--nolocalconf",
            "--exit",
            "-silent",
            "-set",
            "sdl fullscreen=false",
            "-set",
            "sdl output=surface",
            "-c",
            f'mount c "{OUT_DIR}"',
            "-c",
            "c:",
            "-c",
            f"{executable.name} > CONSOLE.TXT",
        ],
        cwd=OUT_DIR,
        timeout=30,
    )


def verify() -> None:
    result = OUT_DIR / "RESULT.TXT"
    if not result.is_file():
        raise SystemExit(f"DOS executable did not create {result}")
    actual = result.read_text(encoding="ascii").strip()
    expected = "PASS bloodprg startup options"
    if actual != expected:
        console = OUT_DIR / "CONSOLE.TXT"
        detail = (
            console.read_text(encoding="ascii", errors="replace")
            if console.is_file()
            else ""
        )
        raise SystemExit(f"integration failure: {actual!r}\n{detail}")
    print(f"{actual}: {(OUT_DIR / EXECUTABLE_NAME).stat().st_size} byte DOS executable")


def main() -> int:
    wcl = resolve_executable("wcl")
    dosbox = resolve_executable("dosbox-x")
    executable = build(wcl)
    run_dosbox(dosbox, executable)
    verify()
    return 0


if __name__ == "__main__":
    sys.exit(main())
