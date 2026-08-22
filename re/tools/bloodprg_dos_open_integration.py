#!/usr/bin/env python3
"""Build and run the BLOODPRG DOS open adapter contract in DOSBox-X."""

from __future__ import annotations

from pathlib import Path
import os
import shutil
import subprocess
import sys


ROOT = Path(__file__).resolve().parents[1]
INTEGRATION_DIR = ROOT / "integration" / "dos"
OUT_DIR = INTEGRATION_DIR / "out" / "bloodprg_dos_open"
SOURCE = INTEGRATION_DIR / "bloodprg_dos_open_probe.c"
EXECUTABLE_NAME = "BDOSOPEN.EXE"


def executable(name: str) -> str:
    resolved = shutil.which(name)
    if resolved is None:
        raise SystemExit(f"executable not found: {name}")
    return resolved


def run(command: list[str], *, timeout: int | None = None) -> None:
    process = subprocess.run(
        command,
        cwd=OUT_DIR,
        env=os.environ.copy(),
        capture_output=True,
        text=True,
        timeout=timeout,
        check=False,
    )
    if process.returncode != 0:
        output = "\n".join(
            part for part in (process.stdout, process.stderr) if part
        )
        raise SystemExit(
            f"command exited {process.returncode}: {' '.join(command)}\n{output}"
        )


def main() -> int:
    if OUT_DIR.exists():
        shutil.rmtree(OUT_DIR)
    OUT_DIR.mkdir(parents=True)
    run(
        [
            executable("wcl"),
            "-q",
            "-3",
            "-ox",
            "-mm",
            "-zdp",
            "-we",
            "-lr",
            f"-fe={OUT_DIR / EXECUTABLE_NAME}",
            str(SOURCE),
        ]
    )

    env = os.environ.copy()
    env["SDL_AUDIODRIVER"] = "dummy"
    env["SDL_VIDEODRIVER"] = "offscreen"
    process = subprocess.run(
        [
            executable("dosbox-x"),
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
            EXECUTABLE_NAME,
        ],
        cwd=OUT_DIR,
        env=env,
        capture_output=True,
        text=True,
        timeout=30,
        check=False,
    )
    if process.returncode != 0:
        raise SystemExit(
            f"DOSBox-X exited {process.returncode}: {process.stderr}"
        )

    result_path = OUT_DIR / "RESULT.TXT"
    actual = result_path.read_text(encoding="ascii").strip()
    if actual != "PASS bloodprg DOS open":
        raise SystemExit(actual)
    print(actual)
    return 0


if __name__ == "__main__":
    sys.exit(main())
