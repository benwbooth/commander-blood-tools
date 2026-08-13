#!/usr/bin/env python3
"""Build and run recovered XDB routines as real-mode DOS programs."""

from __future__ import annotations

from pathlib import Path
import sys

# Running a script from re/tools puts that directory first on sys.path. Remove
# it before standard-library imports so re/tools/dis.py cannot shadow dis.
if sys.path and Path(sys.path[0]).resolve() == Path(__file__).resolve().parent:
    del sys.path[0]

import argparse
from dataclasses import dataclass
import hashlib
import os
import shutil
import subprocess


ROOT = Path(__file__).resolve().parents[2]
INTEGRATION_DIR = ROOT / "re" / "integration" / "dos"
OUT_ROOT = INTEGRATION_DIR / "out"
INCLUDE_DIR = ROOT / "re" / "source" / "xdb" / "candidates" / "include"
MANU3_RECOVERED_SOURCES = (
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
ALIEN_FACE_ACTIVATE_SOURCE = (
    ROOT
    / "re"
    / "source"
    / "xdb"
    / "candidates"
    / "croolis"
    / "func_002bdd_face_activate.c"
)
ALIEN_STARFIELD_SOURCE = (
    ROOT
    / "re"
    / "source"
    / "xdb"
    / "candidates"
    / "croolis"
    / "func_000775_render_starfield.c"
)
ALIEN_MAIN_SOURCE = (
    ROOT
    / "re"
    / "source"
    / "xdb"
    / "candidates"
    / "croolis"
    / "func_0000a3_main.c"
)
ALIEN_ENTRY_SOURCE = (
    ROOT
    / "re"
    / "source"
    / "xdb"
    / "candidates"
    / "croolis"
    / "func_000000_api_entry.c"
)


@dataclass(frozen=True)
class IntegrationCase:
    name: str
    source: Path
    executable_name: str
    expected_result: str
    recovered_sources: tuple[Path, ...]
    artifact_name: str | None = None
    artifact_size: int | None = None
    artifact_sha256: str | None = None


CASES = (
    IntegrationCase(
        name="manu3_renderer_empty",
        source=INTEGRATION_DIR / "manu3_renderer_empty.c",
        executable_name="MANU3E.EXE",
        expected_result="PASS manu3 renderer empty",
        recovered_sources=MANU3_RECOVERED_SOURCES,
    ),
    IntegrationCase(
        name="manu3_renderer_active",
        source=INTEGRATION_DIR / "manu3_renderer_active.c",
        executable_name="MANU3A.EXE",
        expected_result="PASS manu3 renderer active",
        recovered_sources=MANU3_RECOVERED_SOURCES,
        artifact_name="FRAME.BIN",
        artifact_size=320 * 200,
        artifact_sha256=(
            "4b19be7490c8f380df23c9b9f34af5cf"
            "96f7894ee22e4352ea547ea4f5dc2a98"
        ),
    ),
    IntegrationCase(
        name="manu3_face_activate",
        source=INTEGRATION_DIR / "manu3_face_activate.c",
        executable_name="MANU3F.EXE",
        expected_result="PASS manu3 face activate",
        recovered_sources=(MANU3_RECOVERED_SOURCES[1],),
        artifact_name="RECORD.BIN",
        artifact_size=0x5A,
        artifact_sha256=(
            "b52d511a27343c7992d24cdf5029dde8"
            "7c7324e7a5d219d2a3f143db400af6b1"
        ),
    ),
    IntegrationCase(
        name="alien_face_activate",
        source=INTEGRATION_DIR / "alien_face_activate.c",
        executable_name="ALIENF.EXE",
        expected_result="PASS alien face activate",
        recovered_sources=(ALIEN_FACE_ACTIVATE_SOURCE,),
        artifact_name="RECORD.BIN",
        artifact_size=0x5A,
        artifact_sha256=(
            "1955a6685562a0b8aaf5e65b4b1f551e"
            "cda83eab796dc87d1029b31e470a399f"
        ),
    ),
    IntegrationCase(
        name="alien_starfield",
        source=INTEGRATION_DIR / "alien_starfield.c",
        executable_name="ALIENS.EXE",
        expected_result="PASS alien starfield",
        recovered_sources=(ALIEN_STARFIELD_SOURCE,),
        artifact_name="STATE.BIN",
        artifact_size=0x20000,
        artifact_sha256=(
            "62aed86cf9626342b8a60bce2beee0fc"
            "350167c81cd188109bc5a9c93a22649c"
        ),
    ),
    IntegrationCase(
        name="alien_main",
        source=INTEGRATION_DIR / "alien_main.c",
        executable_name="ALIENM.EXE",
        expected_result="PASS alien main",
        recovered_sources=(ALIEN_MAIN_SOURCE,),
    ),
    IntegrationCase(
        name="alien_entry",
        source=INTEGRATION_DIR / "alien_entry.c",
        executable_name="ALIENE.EXE",
        expected_result="PASS alien entry",
        recovered_sources=(ALIEN_ENTRY_SOURCE,),
    ),
)


def resolve_executable(value: str) -> str:
    resolved = shutil.which(value)
    if resolved is None:
        raise SystemExit(f"executable not found: {value}")
    return resolved


def run_checked(
    command: list[str],
    *,
    cwd: Path,
    env: dict[str, str] | None = None,
    timeout: int | None = None,
) -> None:
    try:
        proc = subprocess.run(
            command,
            cwd=cwd,
            env=env,
            check=False,
            capture_output=True,
            text=True,
            timeout=timeout,
        )
    except subprocess.TimeoutExpired as error:
        raise SystemExit(
            f"command timed out after {timeout} seconds: {' '.join(command)}"
        ) from error
    if proc.returncode != 0:
        output = "\n".join(part for part in (proc.stdout, proc.stderr) if part)
        raise SystemExit(
            f"command exited {proc.returncode}: {' '.join(command)}\n{output}"
        )


def build(wcl: str, case: IntegrationCase) -> tuple[Path, Path]:
    out_dir = OUT_ROOT / case.name
    executable = out_dir / case.executable_name
    if out_dir.exists():
        shutil.rmtree(out_dir)
    out_dir.mkdir(parents=True)
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
        f"-fm={out_dir / 'MANU3.MAP'}",
        str(case.source),
        *(str(source) for source in case.recovered_sources),
    ]
    run_checked(command, cwd=out_dir)
    if not executable.is_file():
        raise SystemExit(f"compiler did not create {executable}")
    return out_dir, executable


def run_dosbox(dosbox: str, out_dir: Path, executable: Path) -> None:
    env = os.environ.copy()
    env["SDL_AUDIODRIVER"] = "dummy"
    env["SDL_VIDEODRIVER"] = "offscreen"
    command = [
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
        f"{executable.name} > CONSOLE.TXT",
    ]
    run_checked(command, cwd=out_dir, env=env, timeout=30)


def verify(case: IntegrationCase, out_dir: Path, executable: Path) -> None:
    result = out_dir / "RESULT.TXT"
    if not result.is_file():
        raise SystemExit(f"DOS executable did not create {result}")
    actual = result.read_text(encoding="ascii").strip()
    if actual != case.expected_result:
        console = out_dir / "CONSOLE.TXT"
        detail = (
            console.read_text(encoding="ascii", errors="replace")
            if console.is_file()
            else ""
        )
        raise SystemExit(f"integration failure: {actual!r}\n{detail}")
    if case.artifact_sha256 is not None:
        assert case.artifact_name is not None
        assert case.artifact_size is not None
        artifact = out_dir / case.artifact_name
        if not artifact.is_file():
            raise SystemExit(f"DOS executable did not create {artifact}")
        data = artifact.read_bytes()
        if len(data) != case.artifact_size:
            raise SystemExit(
                f"{case.artifact_name} length {len(data)} does not equal "
                f"{case.artifact_size}"
            )
        actual_hash = hashlib.sha256(data).hexdigest()
        if actual_hash != case.artifact_sha256:
            raise SystemExit(
                f"{case.artifact_name} sha256 {actual_hash} does not match "
                f"raw-overlay oracle {case.artifact_sha256}"
            )
    print(f"{case.expected_result}: {executable.stat().st_size} byte DOS executable")


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
    for case in CASES:
        out_dir, executable = build(wcl, case)
        run_dosbox(dosbox, out_dir, executable)
        verify(case, out_dir, executable)
    return 0


if __name__ == "__main__":
    sys.exit(main())
