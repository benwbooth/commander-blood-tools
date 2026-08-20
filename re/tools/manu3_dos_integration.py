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


def alien_slot3_sources(module: str, entries: tuple[str, ...]) -> tuple[Path, ...]:
    base = ROOT / "re" / "source" / "xdb" / "candidates" / module
    return (INTEGRATION_DIR / "alien_slot3_globals.c",) + tuple(
        base / f"func_{entry}.c" for entry in entries
    )


@dataclass(frozen=True)
class IntegrationCase:
    name: str
    source: Path
    executable_name: str
    expected_result: str
    recovered_sources: tuple[Path, ...]
    defines: tuple[str, ...] = ()
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
    IntegrationCase(
        name="amer_slot1_callbacks",
        source=INTEGRATION_DIR / "alien_amer_slot1_callbacks.c",
        executable_name="AMER1C.EXE",
        expected_result="PASS amer slot1 callbacks",
        recovered_sources=alien_slot3_sources(
            "amer",
            (
                "000bea_slot1_state_update",
                "000cac_slot1_motion_continuation",
            ),
        ),
        defines=("TEST_AMER",),
    ),
    IntegrationCase(
        name="amer_slot2_callbacks",
        source=INTEGRATION_DIR / "alien_amer_slot2_callbacks.c",
        executable_name="AMER2C.EXE",
        expected_result="PASS amer slot2 callbacks",
        recovered_sources=alien_slot3_sources(
            "amer",
            (
                "001688_slot2_restart",
                "001692_slot2_update",
                "00171d_slot2_common_update",
                "0018d3_slot2_return_update",
                "00193e_slot2_selection_wait",
                "001948_slot2_selection_update",
                "0019cb_slot2_selection_late_update",
                "001a2b_slot2_reset",
                "001a5c_slot2_steer_update",
                "001aa0_slot2_finish_update",
            ),
        ),
        defines=("TEST_AMER",),
    ),
    IntegrationCase(
        name="amer_slot3_callbacks",
        source=INTEGRATION_DIR / "alien_slot3_callbacks.c",
        executable_name="AMER3C.EXE",
        expected_result="PASS amer slot3 callbacks",
        recovered_sources=alien_slot3_sources(
            "amer",
            (
                "001414_slot3_update",
                "001558_slot3_restart_initial_update",
                "00158a_slot3_resume_callback",
                "0015db_slot3_capture_resume_state",
                "001614_slot3_ring_zero_callback",
                "001c03_resume_apply_object_delta",
                "001c34_resume_1c34",
                "001c7d_resume_stage_pair",
                "001cbf_resume_stage_timeout",
                "001ccf_resume_stage_final",
                "001cfa_resume_pair_outside",
            ),
        ),
        defines=("TEST_AMER",),
    ),
    IntegrationCase(
        name="croolis_slot2_callbacks",
        source=INTEGRATION_DIR / "alien_croolis_slot2_callbacks.c",
        executable_name="CROOL2C.EXE",
        expected_result="PASS croolis slot2 callbacks",
        recovered_sources=alien_slot3_sources(
            "croolis",
            (
                "00171d_slot2_restart",
                "001727_slot2_update",
                "00178e_slot2_common_dispatch",
                "001794_slot2_motion_update",
                "0017e4_slot2_begin_fade",
                "0017f2_slot2_fade_update",
                "001815_slot2_selection_init",
                "001828_slot2_selection_update",
                "001960_slot2_reset_or_camera",
                "001a86_unreferenced_steering_update",
            ),
        ),
        defines=("TEST_CROOLIS",),
    ),
    IntegrationCase(
        name="croolis_slot3_callbacks",
        source=INTEGRATION_DIR / "alien_slot3_callbacks.c",
        executable_name="CROOL3C.EXE",
        expected_result="PASS croolis slot3 callbacks",
        recovered_sources=alien_slot3_sources(
            "croolis",
            (
                "00146c_slot3_update",
                "0015b0_slot3_restart_initial_update",
                "0015e2_slot3_resume_callback",
                "001633_slot3_capture_resume_state",
                "00166c_slot3_ring_zero_callback",
                "001b5f_resume_apply_object_delta",
                "001b85_resume_1b85",
                "001bc9_resume_stage_pair",
                "001c0b_resume_stage_timeout",
                "001c1b_resume_stage_final",
                "001c46_resume_pair_outside",
            ),
        ),
        defines=("TEST_CROOLIS",),
    ),
    IntegrationCase(
        name="scrut_slot3_callbacks",
        source=INTEGRATION_DIR / "alien_slot3_callbacks.c",
        executable_name="SCRUT3C.EXE",
        expected_result="PASS scrut slot3 callbacks",
        recovered_sources=alien_slot3_sources(
            "scrut",
            (
                "00145a_slot3_update",
                "00159e_slot3_restart_initial_update",
                "0015d0_slot3_resume_callback",
                "001621_slot3_capture_resume_state",
                "00165a_slot3_ring_zero_callback",
                "001c14_resume_apply_object_delta",
                "001c45_resume_1c45",
                "001c89_resume_stage_pair",
                "001ccb_resume_stage_timeout",
                "001cdb_resume_stage_final",
                "001d06_resume_pair_outside",
            ),
        ),
        defines=("TEST_SCRUT",),
    ),
    IntegrationCase(
        name="scrut_slot2_callbacks",
        source=INTEGRATION_DIR / "alien_scrut_slot2_callbacks.c",
        executable_name="SCRUT2C.EXE",
        expected_result="PASS scrut slot2 callbacks",
        recovered_sources=alien_slot3_sources(
            "scrut",
            (
                "001711_slot2_restart",
                "00171b_slot2_update",
                "001781_slot2_common_dispatch",
                "001787_slot2_motion_update",
                "0017e1_slot2_begin_fade",
                "0017e6_slot2_fade_update",
                "001802_slot2_selection_init",
                "001810_slot2_selection_restart",
                "00181b_slot2_selection_begin",
                "001858_slot2_selection_damp",
                "001868_slot2_selection_approach",
                "0018d9_slot2_steering_helper",
                "001952_slot2_finish_setup",
                "001957_slot2_finish_update",
                "0019cf_slot2_selection_reset_restart",
                "001a03_slot2_active_reset_setup",
                "001a11_slot2_reset_or_camera",
                "001b3b_unreferenced_steering_update",
            ),
        ),
        defines=("TEST_SCRUT",),
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
        *(f"-d{symbol}" for symbol in case.defines),
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
    parser.add_argument(
        "--case",
        action="append",
        choices=tuple(case.name for case in CASES),
        help="run only the named integration case; repeatable",
    )
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    wcl = resolve_executable(args.wcl)
    dosbox = resolve_executable(args.dosbox)
    selected = CASES
    if args.case:
        selected = tuple(case for case in CASES if case.name in args.case)
    for case in selected:
        out_dir, executable = build(wcl, case)
        run_dosbox(dosbox, out_dir, executable)
        verify(case, out_dir, executable)
    return 0


if __name__ == "__main__":
    sys.exit(main())
