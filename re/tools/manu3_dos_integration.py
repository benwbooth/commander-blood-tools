#!/usr/bin/env python3
"""Build and run the recovered MANU3 renderer as a real-mode DOS program."""

from __future__ import annotations

from pathlib import Path
import sys

# Running a script from re/tools puts that directory first on sys.path. Remove
# it before standard-library imports so re/tools/dis.py cannot shadow dis.
if sys.path and Path(sys.path[0]).resolve() == Path(__file__).resolve().parent:
    del sys.path[0]

import argparse
import os
import shutil
import subprocess


ROOT = Path(__file__).resolve().parents[2]
INTEGRATION_DIR = ROOT / "re" / "integration" / "dos"
OUT_DIR = INTEGRATION_DIR / "out" / "manu3_renderer_empty"
INCLUDE_DIR = ROOT / "re" / "source" / "xdb" / "candidates" / "include"
SOURCES = (
    INTEGRATION_DIR / "manu3_renderer_empty.c",
    ROOT
    / "re"
    / "source"
    / "xdb"
    / "candidates"
    / "manu3"
    / "func_000700_face_bucket_sort.c",
    ROOT
    / "re"
    / "source"
    / "xdb"
    / "candidates"
    / "manu3"
    / "func_000d7d_face_activate.c",
)
EXE = OUT_DIR / "MANU3T.EXE"
RESULT = OUT_DIR / "RESULT.TXT"
EXPECTED_RESULT = "PASS manu3 renderer empty"


def resolve_executable(value: str) -> str:
    resolved = shutil.which(value)
    if resolved is None:
        raise SystemExit(f"executable not found: {value}")
    return resolved


def run_checked(command: list[str], *, env: dict[str, str] | None = None) -> None:
    proc = subprocess.run(
        command,
        cwd=OUT_DIR,
        env=env,
        check=False,
        capture_output=True,
        text=True,
    )
    if proc.returncode != 0:
        output = "\n".join(part for part in (proc.stdout, proc.stderr) if part)
        raise SystemExit(
            f"command exited {proc.returncode}: {' '.join(command)}\n{output}"
        )


def build(wcl: str) -> None:
    if OUT_DIR.exists():
        shutil.rmtree(OUT_DIR)
    OUT_DIR.mkdir(parents=True)
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
        f"-fe={EXE}",
        f"-fm={OUT_DIR / 'MANU3T.MAP'}",
        *(str(source) for source in SOURCES),
    ]
    run_checked(command)
    if not EXE.is_file():
        raise SystemExit(f"compiler did not create {EXE}")


def run_dosbox(dosbox: str) -> None:
    env = os.environ.copy()
    env["SDL_AUDIODRIVER"] = "dummy"
    env["SDL_VIDEODRIVER"] = "offscreen"
    command = [
        dosbox,
        "--noprimaryconf",
        "--nolocalconf",
        "--exit",
        "-set",
        "sdl fullscreen=false",
        "-set",
        "sdl output=texture",
        "-c",
        f'mount c "{OUT_DIR}"',
        "-c",
        "c:",
        "-c",
        "MANU3T.EXE > CONSOLE.TXT",
    ]
    run_checked(command, env=env)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--wcl", default="wcl", help="Open Watcom wcl executable")
    parser.add_argument(
        "--dosbox", default="dosbox-x", help="DOSBox-X executable"
    )
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    wcl = resolve_executable(args.wcl)
    dosbox = resolve_executable(args.dosbox)
    build(wcl)
    run_dosbox(dosbox)
    if not RESULT.is_file():
        raise SystemExit(f"DOS executable did not create {RESULT}")
    actual = RESULT.read_text(encoding="ascii").strip()
    if actual != EXPECTED_RESULT:
        console = OUT_DIR / "CONSOLE.TXT"
        detail = console.read_text(encoding="ascii", errors="replace") \
            if console.is_file() else ""
        raise SystemExit(f"integration failure: {actual!r}\n{detail}")
    print(f"{EXPECTED_RESULT}: {EXE.stat().st_size} byte DOS executable")
    return 0


if __name__ == "__main__":
    sys.exit(main())
