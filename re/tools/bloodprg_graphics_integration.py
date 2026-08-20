#!/usr/bin/env python3
"""Build and run recovered BLOODPRG graphics routines in real-mode DOS."""

from __future__ import annotations

from pathlib import Path
import os
import shutil
import subprocess
import sys


ROOT = Path(__file__).resolve().parents[1]
INTEGRATION_DIR = ROOT / "integration" / "dos"
INCLUDE_DIR = ROOT / "source" / "bloodprg" / "candidates" / "include"
OUT_ROOT = INTEGRATION_DIR / "out" / "bloodprg_graphics"


GATES = (
    {
        "name": "chunky_planar",
        "executable": "BCHUNK.EXE",
        "harness": "bloodprg_chunky_planar.c",
        "sources": (
            "seg_0299/func_003ece_chunky_to_planar_framebuffer.c",
        ),
        "expected": "PASS bloodprg chunky-to-planar VGA bytes",
    },
    {
        "name": "palette_transition",
        "executable": "BPALETTE.EXE",
        "harness": "bloodprg_palette_transition.c",
        "sources": (
            "seg_008b/func_001f78_palette_transition_step.c",
            "seg_01ce/func_0023c5_palette_range_interpolate.c",
        ),
        "expected": "PASS bloodprg palette transition ABI",
    },
)


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


def build_gate(wcl: str, gate: dict[str, object]) -> Path:
    out_dir = OUT_ROOT / str(gate["name"])
    out_dir.mkdir(parents=True)
    executable = out_dir / str(gate["executable"])
    candidate_dir = ROOT / "source" / "bloodprg" / "candidates"
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
        str(INTEGRATION_DIR / str(gate["harness"])),
        *(str(candidate_dir / source) for source in gate["sources"]),
    ]
    run_checked(command, cwd=out_dir)
    if not executable.is_file():
        raise SystemExit(f"compiler did not create {executable}")
    return executable


def run_gate(dosbox: str, gate: dict[str, object], executable: Path) -> None:
    out_dir = executable.parent
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
            f'mount c "{out_dir}"',
            "-c",
            "c:",
            "-c",
            executable.name,
        ],
        cwd=out_dir,
        timeout=30,
    )
    result = out_dir / "RESULT.TXT"
    actual = result.read_text(encoding="ascii").strip() if result.is_file() else ""
    if actual != gate["expected"]:
        raise SystemExit(
            f"{gate['name']} integration failure: {actual!r}, "
            f"expected {gate['expected']!r}"
        )
    print(f"{actual}: {executable.stat().st_size} byte DOS executable")


def main() -> int:
    wcl = resolve_executable("wcl")
    dosbox = resolve_executable("dosbox-x")
    if OUT_ROOT.exists():
        shutil.rmtree(OUT_ROOT)
    for gate in GATES:
        executable = build_gate(wcl, gate)
        run_gate(dosbox, gate, executable)
    return 0


if __name__ == "__main__":
    sys.exit(main())
