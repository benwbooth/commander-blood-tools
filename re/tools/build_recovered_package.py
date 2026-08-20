#!/usr/bin/env python3
"""Build a runnable package from recovered C and BloodScript sources.

The shipped BLOODPRG.EXE remains available as a fallback. An opt-in runtime
build links every recovered BLOODPRG C routine with the recovered entrypoint,
DOS adapters, and byte-backed data owners. The archive is rebuilt through its
real resource directory: generated scripts are byte-exact, and all four XDB
overlays are linked from the complete recovered C routine set.
"""

from __future__ import annotations

import argparse
import csv
import hashlib
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys


ROOT = Path(__file__).resolve().parents[2]
DEFAULT_CD_ROOT = ROOT / "output" / "_tmp_iso"
DEFAULT_XDB_DIR = ROOT / "output" / "_tmp_dat"
DEFAULT_SOURCE_DIR = ROOT / "re" / "vm" / "bloodscript"
DEFAULT_REFERENCE_DIR = ROOT / "accuracy" / "cblood_install" / "cblood"
DEFAULT_OUTPUT_DIR = ROOT / "output" / "recovered_dos_package"
XDB_MANIFEST = ROOT / "re" / "source" / "xdb" / "candidates" / "manifest.tsv"

XDB_MODULES = ("amer", "croolis", "manu3", "scrut")

# These handlers have both semantic/oracle coverage and an exact WCL machine
# byte result at their original BLOODPRG file offsets.  Keep this list
# deliberately small: an accepted C routine with a different ABI is not a
# production patch merely because its standalone harness passes.
BLOODPRG_FIXED_PATCH_FUNCTIONS = (
    ("vm_token_special", 0x006293, 0x10),
    ("sprite_blitter_noop_5", 0x00509A, 1),
    ("sprite_blitter_noop_6", 0x00509B, 1),
    ("sprite_blitter_noop_7", 0x00509C, 1),
    ("byte_parser_op_01_mark_b16", 0x007542, 7),
    ("byte_parser_op_02_mark_b16", 0x007549, 7),
    ("byte_parser_op_0f_mark_b16", 0x007550, 7),
    ("byte_parser_op_04_mark_b16", 0x007557, 7),
)

# This routine has one accepted compiler-encoding difference.  Its zero
# bytes are linker placeholders for original data offsets; only the nonzero
# XOR opcode is allowed to come from the generated object.
BLOODPRG_RELOCATION_MASKED_PATCH_FUNCTIONS = (
    ("list_d8c_init", 0x00A757, 0x21, ((9, 0x33, 0x31),)),
    ("queue_d8c_enqueue", 0x00A734, 0x0A, ((8, 0xF8, 0xC3),)),
)

# These natural C replacements have reviewed semantic differences from the
# original bodies.  Each generated body is fixed-size, and any changed MZ
# relocation positions are listed explicitly.  Keep both byte strings here so
# a compiler/toolchain change cannot silently widen either exception.
BLOODPRG_SEMANTIC_PATCH_FUNCTIONS = (
    (
        "vm_special_slot_remove",
        0x005FD8,
        bytes.fromhex(
            "53 BB 00 00 3B 07 74 0D 83 C3 02 81 FB 20 00 "
            "75 F3 31 C0 5B C3 C7 07 00 00 B8 01 00 5B C3"
        ),
        bytes.fromhex(
            "53 BB 3E 6D 3B 07 74 0D 83 C3 02 81 FB 5E 6D "
            "75 F3 31 C0 5B C3 C7 07 00 00 B8 01 00 5B C3"
        ),
        (),
    ),
    (
        "lookup_table_1fb5",
        0x009F80,
        bytes.fromhex("89 C3 C1 E3 02 8B 9F 00 00 C3"),
        bytes.fromhex("89 C3 C1 E3 02 8B 9F B5 1F C3"),
        (),
    ),
    (
        "matrix_table_clear_2a1b",
        0x00963F,
        bytes.fromhex("53 BB 00 00 C7 07 00 00 83 C3 18 81 FB 90 00 75 F3 5B CB"),
        bytes.fromhex("53 BB 1B 2A C7 07 00 00 83 C3 18 81 FB AB 2A 75 F3 5B CB"),
        (),
    ),
    (
        "presentation_queue_finish",
        0x00A2DD,
        bytes.fromhex(
            "80 0E 00 00 01 83 3E 00 00 00 74 01 C3 "
            "80 0E 00 00 02 E9 00 00"
        ),
        bytes.fromhex(
            "80 0E 5F 0D 01 83 3E 9A 0D 00 74 01 C3 "
            "80 0E 5F 0D 02 E9 4F FE"
        ),
        (),
    ),
    (
        "presentation_mode_dispatch",
        0x0078D0,
        bytes.fromhex(
            "53 F6 06 00 00 50 74 1B BB 0C 00 F6 06 00 00 40 "
            "74 03 83 C3 30 A1 00 00 3B 07 7D 09 F6 06 00 00 01 "
            "75 2D 5B C3 2B 47 04 3B 07 7F F0 A1 00 00 3B 47 02 "
            "7C E8 2B 47 06 3B 47 02 7F E0 F6 06 00 00 01 75 E0 "
            "C6 06 00 00 01 C7 06 00 00 09 00 5B C3 C6 06 00 00 "
            "00 A1 00 00 A3 00 00 5B C3"
        ),
        bytes.fromhex(
            "53 F6 06 93 27 50 74 1B BB 27 2A F6 06 93 27 40 "
            "74 03 83 C3 30 A1 2A 0A 3B 07 7D 09 F6 06 EA 27 01 "
            "75 2D 5B C3 2B 47 04 3B 07 7F F0 A1 2C 0A 3B 47 02 "
            "7C E8 2B 47 06 3B 47 02 7F E0 F6 06 EA 27 01 75 E0 "
            "C6 06 EA 27 01 C7 06 32 0A 09 00 5B C3 C6 06 EA 27 "
            "00 A1 36 0A A3 32 0A 5B C3"
        ),
        (),
    ),
    (
        "nav_chart_list_build",
        0x00721A,
        bytes.fromhex(
            "53 51 52 56 E8 00 00 BA 00 00 BE 00 00 31 C9 89 D3 "
            "83 C2 02 8B 07 85 C0 7C 17 C4 1E 00 00 01 C3 26 F7 "
            "07 18 01 74 E8 89 F3 83 C6 02 89 07 41 EB DE 89 04 "
            "89 C8 5E 5A 59 5B CB"
        ),
        bytes.fromhex(
            "53 51 52 56 E8 2D EE BA 16 6A BE D3 2A 31 C9 89 D3 "
            "83 C2 02 8B 07 85 C0 7C 17 C4 1E 24 67 01 C3 26 F7 "
            "07 18 01 74 E8 89 F3 83 C6 02 89 07 41 EB DE 89 04 "
            "89 C8 5E 5A 59 5B CB"
        ),
        (),
    ),
    (
        "nav_kind2_target_list_build",
        0x0071CF,
        bytes.fromhex(
            "53 52 56 06 E8 00 00 BA 00 00 BE 00 00 31 C9 89 D3 "
            "83 C2 02 8B 07 3D FF FF 74 22 3B 06 00 00 74 EE 3B "
            "06 00 00 74 E8 C4 1E 00 00 01 C3 26 83 3F 02 75 DC "
            "89 F3 83 C6 02 89 07 41 EB D2 89 04 89 C8 07 5E 5A "
            "5B CB"
        ),
        bytes.fromhex(
            "53 52 56 06 E8 78 EE BA 16 6A BE 13 2B 31 C9 89 D3 "
            "83 C2 02 8B 07 3D FF FF 74 22 3B 06 54 67 74 EE 3B "
            "06 56 67 74 E8 C4 1E 24 67 01 C3 26 83 3F 02 75 DC "
            "89 F3 83 C6 02 89 07 41 EB D2 89 04 89 C8 07 5E 5A "
            "5B CB"
        ),
        (),
    ),
    (
        "ship_3d_navigation_candidate_build",
        0x0070EE,
        bytes.fromhex(
            "56 8C C0 BB 00 00 BE 00 00 9A 00 00 00 00 BF 00 00 "
            "B8 00 00 8E C0 26 8B 04 83 C6 02 3D FF FF 74 22 3B "
            "06 00 00 74 EA C4 1E 00 00 01 C3 26 83 3F 02 75 DE "
            "26 F6 47 02 01 74 D7 89 FB 83 C7 02 89 07 EB CE C7 "
            "05 00 00 5E CB"
        ),
        bytes.fromhex(
            "55 56 66 50 1E 0F A8 1F BD 86 68 BE 86 68 0E E8 "
            "4B F1 BF 53 2B 8B 04 83 C6 02 3D FF FF 74 22 3B 06 "
            "54 67 74 F0 C4 1E 24 67 01 C3 26 83 3F 02 75 E4 26 "
            "F6 47 02 01 74 DD 89 FB 83 C7 02 89 07 EB D4 C7 05 "
            "00 00 C4 3E 24 67 1F 66 58 5E 5D CB"
        ),
        (),
    ),
    (
        "ship_3d_position_field_resolve",
        0x0061A6,
        bytes.fromhex(
            "53 51 52 56 57 06 89 D7 89 F1 8B 1C 81 FB 00 01 "
            "74 23 83 FB 08 74 42 83 FB 10 74 3D 81 FB 00 02 74 "
            "37 B8 11 00 89 DA E8 00 00 01 C6 8B 34 83 FE FF 74 "
            "2B EB D3 B8 0C 00 89 DA E8 00 00 01 C6 3B 3C 75 11 "
            "B8 09 00 89 DA E8 00 00 01 C8 07 5F 5E 5A 59 5B C3 "
            "B8 0A 00 EB ED B8 0B 00 EB E8 B8 00 00 8E C0 26 8B "
            "36 00 00 EB 9E"
        ),
        bytes.fromhex(
            "53 51 52 56 57 06 66 0F B7 F6 66 31 C0 89 D7 89 "
            "F1 8B 1C 81 FB 00 01 74 21 83 FB 08 74 3C 83 FB 10 "
            "74 37 81 FB 00 02 74 31 B8 11 00 E8 4E FE 01 C6 8B "
            "34 83 FE FF 74 27 EB D5 B8 0C 00 E8 3D FE 01 C6 3B "
            "3C 75 0F B8 09 00 E8 31 FE 01 C8 07 5F 5E 5A 59 5B "
            "C3 B8 0A 00 EB EF B8 0B 00 EB EA 0F A8 07 26 8B 36 "
            "52 67 EB A6"
        ),
        (),
    ),
    (
        "entity_flag_state_transition",
        0x0041D1,
        bytes.fromhex(
            "53 BB 00 00 C1 E0 05 01 C3 8B 07 A8 80 74 08 "
            "A8 01 74 04 24 FC 0C 02 89 07 5B CB"
        ),
        bytes.fromhex(
            "50 53 BB 12 62 C1 E0 05 01 C3 65 8B 07 0A C0 "
            "79 08 A8 01 74 04 24 FC 0C 02 65 89 07 5B 58 CB"
        ),
        (),
    ),
    (
        "palette_upload_if_dirty",
        0x00178B,
        bytes.fromhex(
            "50 F6 06 00 00 01 75 02 58 C3 9A 00 00 00 00 "
            "BE 00 00 9A 00 00 00 00 30 C0 A2 00 00 A2 00 00 "
            "A2 00 00 58 C3"
        ),
        bytes.fromhex(
            "50 F6 06 55 5B 01 75 02 58 C3 9A D7 05 00 00 "
            "BE 51 52 9A 00 00 99 02 30 C0 A2 55 5B A2 40 0A "
            "A2 3E 0A 58 C3"
        ),
        ((0x1795, 0x1798), (0x179D, 0x17A0)),
    ),
)

# Turbo C 2.01 reproduces these bodies exactly; only OMF relocation words are
# zero in the standalone object.  They are enabled only when an archived
# Turbo C tree is explicitly supplied to the package builder.
BLOODPRG_TURBO_PATCH_FUNCTIONS = (
    ("list_d8c_bounds_init", 0x00A73E, 0x19, ()),
    ("list_d8c_wrap_bounds_reset", 0x00A744, 0x13, ()),
    ("nav_choice_handler_0", 0x008713, 0x19, ()),
    ("presentation_update_1fb2", 0x009F53, 0x2D, ()),
)


def sha256_bytes(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def sha256(path: Path) -> str:
    return sha256_bytes(path.read_bytes())


def move_mz_relocations(
    image: bytearray, moves: tuple[tuple[int, int], ...]
) -> None:
    """Move verified DOS MZ relocation entries when a far call moves."""
    header_size = int.from_bytes(image[8:10], "little") * 16
    relocation_count = int.from_bytes(image[6:8], "little")
    relocation_table = int.from_bytes(image[0x18:0x1A], "little")
    entries: list[tuple[int, int, int, int]] = []
    for index in range(relocation_count):
        position = relocation_table + index * 4
        offset = int.from_bytes(image[position : position + 2], "little")
        segment = int.from_bytes(image[position + 2 : position + 4], "little")
        file_offset = header_size + segment * 16 + offset
        entries.append((index, position, segment, file_offset))

    for old_file_offset, new_file_offset in moves:
        matches = [
            entry for entry in entries if entry[3] == old_file_offset
        ]
        if len(matches) != 1:
            raise SystemExit(
                f"expected one MZ relocation at 0x{old_file_offset:04x}, "
                f"found {len(matches)}"
            )
        _index, position, segment, _old = matches[0]
        new_offset = new_file_offset - header_size - segment * 16
        if not 0 <= new_offset <= 0xFFFF:
            raise SystemExit(
                f"MZ relocation target is not representable: "
                f"0x{new_file_offset:04x}"
            )
        if any(entry[3] == new_file_offset for entry in entries):
            raise SystemExit(
                f"MZ relocation target already occupied: 0x{new_file_offset:04x}"
            )
        image[position : position + 2] = new_offset.to_bytes(2, "little")
        entries[entries.index(matches[0])] = (
            matches[0][0],
            position,
            segment,
            new_file_offset,
        )


def metadata_path(path: Path) -> str:
    """Keep package metadata readable when an input lives outside ROOT."""
    try:
        return str(path.relative_to(ROOT))
    except ValueError:
        return str(path)


def resolve_executable(value: str) -> str:
    resolved = shutil.which(value)
    if resolved:
        return resolved
    if Path(value).is_file():
        return value
    raise SystemExit(f"executable not found: {value}")


def run_checked(command: list[str]) -> None:
    process = subprocess.run(command, cwd=ROOT, text=True, capture_output=True)
    if process.returncode == 0:
        return
    output = "\n".join(part for part in (process.stdout, process.stderr) if part)
    raise SystemExit(f"command failed: {' '.join(command)}\n{output}")


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--cd-root", type=Path, default=DEFAULT_CD_ROOT)
    parser.add_argument("--xdb-dir", type=Path, default=DEFAULT_XDB_DIR)
    parser.add_argument("--source-dir", type=Path, default=DEFAULT_SOURCE_DIR)
    parser.add_argument("--reference-dir", type=Path, default=DEFAULT_REFERENCE_DIR)
    parser.add_argument("--output-dir", type=Path, default=DEFAULT_OUTPUT_DIR)
    parser.add_argument("--wcl", default="wcl")
    parser.add_argument("--wdis", default="wdis")
    parser.add_argument("--wasm", default="wasm")
    parser.add_argument("--wlink", default="wlink")
    parser.add_argument(
        "--include-bloodprg-runtime",
        action="store_true",
        help="compile and link the recovered BLOODPRG runtime as cd/BPRG_RE.EXE",
    )
    parser.add_argument(
        "--include-bloodprg-fixed-patch",
        action="store_true",
        help="emit a game-loadable BLOODPRG copy containing only verified fixed-layout C patches",
    )
    parser.add_argument(
        "--turbo-c-toolchain",
        type=Path,
        help="archived Turbo C tree; enables Turbo-generated fixed-layout patches",
    )
    parser.add_argument("--dosbox", default="dosbox-x")
    parser.add_argument(
        "--cbvm",
        type=Path,
        help="prebuilt cbvm executable; useful when Watcom and Rust use separate shells",
    )
    return parser.parse_args()


def run_link_probe(
    output_dir: Path,
    main_object: Path,
    object_dir: Path,
    extra_objects: list[Path],
    name: str,
) -> int:
    command = [
        sys.executable,
        str(ROOT / "re/tools/link_recovered_objects.py"),
        "--main-object",
        str(main_object),
        "--object-dir",
        str(object_dir),
        "--output-dir",
        str(output_dir),
        "--name",
        name,
        "--map",
    ]
    for extra_object in extra_objects:
        command.extend(("--extra-object", str(extra_object)))
    process = subprocess.run(command, cwd=ROOT, text=True, capture_output=True)
    diagnostics = "\n".join(
        part for part in (process.stdout, process.stderr) if part
    )
    (output_dir / "driver.log").write_text(
        "$ " + " ".join(command) + "\n" + diagnostics + "\n",
        encoding="utf-8",
    )
    return process.returncode


def build_bloodprg_runtime(
    args: argparse.Namespace, output: Path
) -> list[dict[str, str]]:
    """Link the recovered entrypoint, runtime data owners, and C routines."""
    validation_dir = output / "validation" / "bloodprg_runtime"
    validation_dir.mkdir(parents=True, exist_ok=True)
    object_dir = output / "bloodprg_objects"

    main_object = validation_dir / "bloodprg_relinked_main.obj"
    run_checked(
        [
            args.wcl,
            "-q",
            "-c",
            "-3",
            "-ox",
            "-mm",
            "-zdp",
            "-we",
            "-dBLOODPRG_RELINKED_RUNTIME",
            "-i=" + str(ROOT / "re/source/bloodprg/candidates/include"),
            "-fo=" + str(main_object),
            str(ROOT / "re/integration/dos/bloodprg_relinked_main.c"),
        ]
    )

    adapter_object = validation_dir / "platform_adapters.obj"
    run_checked(
        [
            args.wcl,
            "-q",
            "-c",
            "-3",
            "-ox",
            "-mm",
            "-zdp",
            "-we",
            "-dBLOODPRG_RELINKED_RUNTIME",
            "-i=" + str(ROOT / "re/source/bloodprg/candidates/include"),
            "-fo=" + str(adapter_object),
            str(ROOT / "re/integration/dos/bloodprg_platform_adapters.c"),
        ]
    )

    initial_dir = validation_dir / "initial"
    initial_dir.mkdir(parents=True, exist_ok=True)
    initial_status = run_link_probe(
        initial_dir,
        main_object,
        object_dir,
        [adapter_object],
        "BLOODPRG_INITIAL.EXE",
    )
    unresolved = initial_dir / "unresolved.tsv"
    if not unresolved.is_file():
        raise SystemExit(f"initial link did not write unresolved report: {unresolved}")
    if initial_status == 0:
        raise SystemExit("initial BLOODPRG link unexpectedly resolved without data owner")

    owner_dir = validation_dir / "data_owner"
    run_checked(
        [
            sys.executable,
            str(ROOT / "re/tools/bloodprg_data_layout_probe.py"),
            "--unresolved",
            str(unresolved),
            "--image",
            str((args.cd_root / "BLOODPRG.EXE").resolve()),
            "--runtime-layout",
            "--output-dir",
            str(owner_dir),
        ]
    )
    owner_object = owner_dir / "bloodprg_data_layout_probe.obj"
    run_checked(
        [
            args.wasm,
            "-q",
            str(owner_dir / "bloodprg_data_layout_probe.asm"),
            "-fo=" + str(owner_object),
        ]
    )

    final_dir = validation_dir / "final"
    final_dir.mkdir(parents=True, exist_ok=True)
    final_status = run_link_probe(
        final_dir,
        main_object,
        object_dir,
        [adapter_object, owner_object],
        "BPRG_RE.EXE",
    )
    final_executable = final_dir / "BPRG_RE.EXE"
    final_report = final_dir / "unresolved.tsv"
    unresolved_lines = final_report.read_text(encoding="ascii").splitlines()
    if final_status != 0 or not final_executable.is_file() or len(unresolved_lines) != 1:
        raise SystemExit(
            "relinked BLOODPRG runtime did not resolve cleanly; see "
            f"{final_dir / 'link.log'} and {final_report}"
        )

    runtime_executable = output / "cd" / "BPRG_RE.EXE"
    runtime_executable.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(final_executable, runtime_executable)
    original_sha = sha256(args.cd_root / "BLOODPRG.EXE")
    return [
        {
            "component": final_executable.name,
            "source": str(final_executable.relative_to(output)),
            "output": str(final_executable.relative_to(output)),
            "status": "c_relinked_runtime_zero_unresolved",
            "offset": "-",
            "original_sha256": original_sha,
            "output_sha256": sha256(final_executable),
        },
        {
            "component": runtime_executable.name,
            "source": str(final_executable.relative_to(output)),
            "output": str(runtime_executable.relative_to(output)),
            "status": "c_relinked_runtime_dos_alias",
            "offset": "-",
            "original_sha256": original_sha,
            "output_sha256": sha256(runtime_executable),
        },
    ]


def link_or_copy(source: Path, destination: Path) -> None:
    destination.parent.mkdir(parents=True, exist_ok=True)
    try:
        os.link(source, destination)
    except OSError:
        shutil.copy2(source, destination)


def copy_cd_tree(source: Path, destination: Path) -> None:
    if not source.is_dir():
        raise SystemExit(f"missing CD tree: {source}")
    for directory, _, filenames in os.walk(source):
        directory_path = Path(directory)
        relative = directory_path.relative_to(source)
        target_dir = destination / relative
        target_dir.mkdir(parents=True, exist_ok=True)
        for filename in filenames:
            if filename.upper() == "BLOOD.DAT":
                continue
            link_or_copy(directory_path / filename, target_dir / filename)


def compile_sources(args: argparse.Namespace, output: Path) -> None:
    scripts = output / "scripts"
    objects = output / "xdb_objects"
    scripts.mkdir(parents=True, exist_ok=True)
    if args.cbvm is not None:
        command = ["--cbvm", str(args.cbvm.resolve())]
    else:
        command = []
    run_checked(
        [
            sys.executable,
            str(ROOT / "re/tools/compile_bloodscript_bundle.py"),
            "--source-dir",
            str(args.source_dir.resolve()),
            "--output-dir",
            str(scripts.resolve()),
            "--reference-dir",
            str(args.reference_dir.resolve()),
            *command,
        ]
    )
    run_checked(
        [
            sys.executable,
            str(ROOT / "re/tools/build_xdb_objects.py"),
            "--manifest",
            str(XDB_MANIFEST),
            "--wcl",
            args.wcl,
            "--object-dir",
            str(objects.resolve()),
        ]
    )


def build_bloodprg_objects(args: argparse.Namespace, output: Path) -> Path:
    object_dir = output / "bloodprg_objects"
    run_checked(
        [
            sys.executable,
            str(ROOT / "re/tools/build_xdb_objects.py"),
            "--manifest",
            str(ROOT / "re/source/bloodprg/candidates/manifest.tsv"),
            "--module-prefix",
            "",
            "--output-label",
            "bloodprg",
            "--define",
            "BLOODPRG_RELINKED_RUNTIME",
            "--wcl",
            args.wcl,
            "--object-dir",
            str(object_dir),
        ]
    )
    return object_dir


def build_bloodprg_turbo_objects(
    args: argparse.Namespace, output: Path
) -> dict[str, Path]:
    manifest = ROOT / "re/source/bloodprg/candidates/manifest.tsv"
    with manifest.open(newline="", encoding="ascii") as handle:
        rows = {row["function"]: row for row in csv.DictReader(handle, delimiter="\t")}
    object_dir = output / "validation" / "bloodprg_fixed" / "turbo_objects"
    objects: dict[str, Path] = {}
    for function, _offset, _length, _allowed_changes in BLOODPRG_TURBO_PATCH_FUNCTIONS:
        row = rows.get(function)
        if row is None:
            raise SystemExit(f"manifest has no Turbo C patch function: {function}")
        source = (manifest.parent / row["source"]).resolve()
        destination = object_dir / f"{Path(row['source']).stem}.OBJ"
        run_checked(
            [
                sys.executable,
                str(ROOT / "re/tools/build_turbo_c_object.py"),
                "--toolchain",
                str(args.turbo_c_toolchain),
                "--source",
                str(source),
                "--output",
                str(destination),
                "--dosbox",
                args.dosbox,
            ]
        )
        objects[function] = destination
    return objects


def wdis_code_bytes(wdis: str, object_path: Path, function: str) -> tuple[bytes, str]:
    process = subprocess.run(
        [wdis, "-p", str(object_path)],
        cwd=ROOT,
        text=True,
        capture_output=True,
        check=False,
    )
    listing = process.stdout + process.stderr
    if process.returncode != 0:
        raise SystemExit(f"wdis failed for {object_path}:\n{listing}")
    segments = re.split(r"(?=^Segment: )", listing, flags=re.MULTILINE)
    for segment in segments:
        if (
            "Routine Size:" not in segment
            or (f"{function}_" not in segment and f"_{function}" not in segment)
        ):
            continue
        data = bytearray()
        for line in segment.splitlines():
            match = re.match(
                r"\s*[0-9A-Fa-f]+\s+((?:[0-9A-Fa-f]{2}\s+)+)", line
            )
            if match is not None:
                data.extend(int(value, 16) for value in match.group(1).split())
        return bytes(data), segment
    raise SystemExit(f"{function} was not found in {object_path}")


def build_bloodprg_fixed_patch(
    args: argparse.Namespace,
    output: Path,
    object_dir: Path,
    turbo_objects: dict[str, Path] | None = None,
) -> list[dict[str, str]]:
    manifest = ROOT / "re/source/bloodprg/candidates/manifest.tsv"
    with manifest.open(newline="", encoding="ascii") as handle:
        rows = {row["function"]: row for row in csv.DictReader(handle, delimiter="\t")}
    original_path = args.cd_root / "BLOODPRG.EXE"
    original = original_path.read_bytes()
    validation_dir = output / "validation" / "bloodprg_fixed"
    validation_dir.mkdir(parents=True, exist_ok=True)
    patched = bytearray(original)
    report_rows = ["function\toffset\tlength\tstatus\toriginal_sha256\tgenerated_sha256"]

    patch_specs = [
        (function, offset, length, "byte_exact_c_patch")
        for function, offset, length in BLOODPRG_FIXED_PATCH_FUNCTIONS
    ]
    patch_specs.extend(
        (function, offset, length, "relocation_masked_c_patch", allowed_changes)
        for function, offset, length, allowed_changes
        in BLOODPRG_RELOCATION_MASKED_PATCH_FUNCTIONS
    )
    if turbo_objects is not None:
        patch_specs.extend(
            (function, offset, length, "turbo_relocation_masked_c_patch", allowed_changes)
            for function, offset, length, allowed_changes in BLOODPRG_TURBO_PATCH_FUNCTIONS
        )

    for spec in patch_specs:
        function, offset, length, patch_status = spec[:4]
        allowed_changes = spec[4] if len(spec) == 5 else ()
        row = rows.get(function)
        if row is None:
            raise SystemExit(f"manifest has no fixed-patch function: {function}")
        source_stem = Path(row["source"]).stem
        object_path = (
            turbo_objects.get(function)
            if turbo_objects is not None
            else None
        ) or (object_dir / "bloodprg" / f"{source_stem}.OBJ")
        generated, listing = wdis_code_bytes(args.wdis, object_path, function)
        (validation_dir / f"{source_stem}.asm").write_text(listing, encoding="ascii")
        if len(generated) > length or (
            patch_status == "byte_exact_c_patch" and len(generated) != length
        ):
            raise SystemExit(
                f"{function} generated {len(generated)} bytes, expected {length}"
            )
        expected = original[offset : offset + length]
        if len(expected) != length:
            raise SystemExit(f"{function} exceeds BLOODPRG image at 0x{offset:06x}")
        differences = [
            index
            for index, (actual, compiled) in enumerate(zip(expected, generated))
            if actual != compiled
        ]
        if patch_status == "byte_exact_c_patch" and differences:
            raise SystemExit(
                f"{function} is not byte-identical at 0x{offset:06x}; "
                "production patch refused"
            )
        if patch_status in (
            "relocation_masked_c_patch",
            "turbo_relocation_masked_c_patch",
        ):
            actual_changes = tuple(
                (index, expected[index], generated[index])
                for index in differences
                if generated[index] != 0
            )
            if actual_changes != allowed_changes:
                raise SystemExit(
                    f"{function} has unexpected non-relocation differences "
                    "production patch refused"
                )
            replacement = bytearray(expected)
            for index, _original, compiled in allowed_changes:
                replacement[index] = compiled
            generated = bytes(replacement)
        patched[offset : offset + length] = generated
        report_rows.append(
            "\t".join(
                (
                    function,
                    f"0x{offset:06x}",
                    str(length),
                    patch_status,
                    sha256_bytes(expected),
                    sha256_bytes(generated),
                )
            )
        )

    for function, offset, generated_template, replacement, relocation_moves in (
        BLOODPRG_SEMANTIC_PATCH_FUNCTIONS
    ):
        row = rows.get(function)
        if row is None:
            raise SystemExit(f"manifest has no semantic-patch function: {function}")
        source_stem = Path(row["source"]).stem
        object_path = object_dir / "bloodprg" / f"{source_stem}.OBJ"
        generated, listing = wdis_code_bytes(args.wdis, object_path, function)
        (validation_dir / f"{source_stem}.asm").write_text(listing, encoding="ascii")
        if generated != generated_template:
            raise SystemExit(
                f"{function} generated an unapproved semantic-patch body; "
                "production patch refused"
            )
        expected = original[offset : offset + len(replacement)]
        if len(expected) != len(replacement):
            raise SystemExit(f"{function} exceeds BLOODPRG image at 0x{offset:06x}")
        patched[offset : offset + len(replacement)] = replacement
        move_mz_relocations(patched, relocation_moves)
        report_rows.append(
            "\t".join(
                (
                    function,
                    f"0x{offset:06x}",
                    str(len(replacement)),
                    "semantic_c_patch_verified_runtime",
                    sha256_bytes(expected),
                    sha256_bytes(replacement),
                )
            )
        )

    patched_path = validation_dir / "BLOODPRG_C_PATCHED.EXE"
    patched_path.write_bytes(patched)
    (validation_dir / "patch.tsv").write_text(
        "\n".join(report_rows) + "\n", encoding="ascii"
    )
    short_path = output / "cd" / "BPRG_C.EXE"
    shutil.copy2(patched_path, short_path)
    return [
        {
            "component": patched_path.name,
            "source": str(patched_path.relative_to(output)),
            "output": str(patched_path.relative_to(output)),
            "status": "c_fixed_layout_verified_patch",
            "offset": "multiple",
            "original_sha256": sha256(original_path),
            "output_sha256": sha256(patched_path),
        },
        {
            "component": short_path.name,
            "source": str(patched_path.relative_to(output)),
            "output": str(short_path.relative_to(output)),
            "status": "c_fixed_layout_verified_patch_dos_alias",
            "offset": "multiple",
            "original_sha256": sha256(original_path),
            "output_sha256": sha256(short_path),
        },
    ]


def build_source_xdb_files(
    args: argparse.Namespace,
    output: Path,
) -> list[dict[str, str]]:
    xdb_output = output / "xdb"
    validation_root = output / "validation" / "source_xdb"
    xdb_output.mkdir(parents=True, exist_ok=True)
    validation_root.mkdir(parents=True, exist_ok=True)
    records: list[dict[str, str]] = []

    for module in XDB_MODULES:
        original = args.xdb_dir / f"{module}.xdb"
        if not original.is_file():
            raise SystemExit(f"missing source overlay: {original}")
        build_dir = validation_root / module
        if build_dir.exists():
            shutil.rmtree(build_dir)
        run_checked(
            [
                sys.executable,
                str(ROOT / "re/tools/build_source_xdb.py"),
                "--module",
                module,
                "--object-dir",
                str(output / "xdb_objects" / f"xdb_{module}"),
                "--raw-xdb",
                str(original.resolve()),
                "--output-dir",
                str(build_dir),
                "--wasm",
                args.wasm,
                "--wlink",
                args.wlink,
            ]
        )
        built = build_dir / f"{module}.xdb"
        report = build_dir / "build.tsv"
        if not built.is_file() or not report.is_file():
            raise SystemExit(f"source XDB build omitted artifacts for {module}")
        destination = xdb_output / built.name
        shutil.copy2(built, destination)
        records.append(
            {
                "component": built.name,
                "source": metadata_path(XDB_MANIFEST.parent / module),
                "output": str(destination.relative_to(output)),
                "status": "c_source_linked_overlay",
                "offset": "raw_entry_0000:0000",
                "original_sha256": sha256(original),
                "output_sha256": sha256(destination),
            }
        )

    return records


def archive_entry_records(
    data: bytes | bytearray,
) -> list[tuple[str, int, int, int]]:
    records: list[tuple[str, int, int, int]] = []
    cursor = 2
    while cursor < min(65536, len(data)):
        record_offset = cursor
        name_bytes = bytes(data[cursor : cursor + 16])
        if not name_bytes or name_bytes[0] == 0:
            break
        cursor += 16
        size = int.from_bytes(data[cursor : cursor + 4], "little", signed=True)
        cursor += 4
        offset = int.from_bytes(data[cursor : cursor + 4], "little", signed=True)
        cursor += 4
        cursor += 1
        name = name_bytes.split(b"\0", 1)[0].decode("ascii").lower().replace("\\", "/")
        if size > 0 and offset >= 0 and offset + size <= len(data):
            records.append((name, record_offset, offset, size))
        if cursor <= record_offset:
            raise SystemExit("BLOOD.DAT directory parser did not advance")
    return records


def archive_entries(data: bytes | bytearray) -> dict[str, tuple[int, int]]:
    entries: dict[str, tuple[int, int]] = {}
    for name, _record_offset, offset, size in archive_entry_records(data):
        entries.setdefault(name, (offset, size))
    return entries


def replace_archive_members(
    data: bytearray,
    replacements: dict[str, bytes],
) -> tuple[bytearray, dict[str, tuple[int, int, int, int]]]:
    records = archive_entry_records(data)
    records_by_name: dict[str, list[tuple[int, int, int]]] = {}
    for name, record_offset, offset, size in records:
        records_by_name.setdefault(name, []).append((record_offset, offset, size))

    intervals: list[tuple[int, int, str, bytes]] = []
    for name, replacement in replacements.items():
        matches = records_by_name.get(name, [])
        if not matches:
            raise SystemExit(f"resource is absent from BLOOD.DAT: {name}")
        locations = {(offset, size) for _record, offset, size in matches}
        if len(locations) != 1:
            raise SystemExit(f"resource has conflicting BLOOD.DAT entries: {name}")
        offset, size = locations.pop()
        intervals.append((offset, size, name, replacement))

    intervals.sort()
    for left, right in zip(intervals, intervals[1:]):
        if left[0] + left[1] > right[0]:
            raise SystemExit(
                f"overlapping BLOOD.DAT replacements: {left[2]} and {right[2]}"
            )

    rebuilt = bytearray()
    cursor = 0
    locations: dict[str, tuple[int, int, int, int]] = {}
    for offset, size, name, replacement in intervals:
        rebuilt.extend(data[cursor:offset])
        new_offset = len(rebuilt)
        rebuilt.extend(replacement)
        locations[name] = (offset, size, new_offset, len(replacement))
        cursor = offset + size
    rebuilt.extend(data[cursor:])

    for name, record_offset, old_offset, old_size in records:
        if name in locations:
            _source_offset, _source_size, new_offset, new_size = locations[name]
        else:
            new_size = old_size
            new_offset = old_offset
            for replaced_offset, replaced_size, replaced_name, replacement in intervals:
                replaced_end = replaced_offset + replaced_size
                if old_offset >= replaced_end:
                    new_offset += len(replacement) - replaced_size
                elif old_offset >= replaced_offset:
                    raise SystemExit(
                        f"{name} starts inside replaced BLOOD.DAT member {replaced_name}"
                    )
        rebuilt[record_offset + 16 : record_offset + 20] = new_size.to_bytes(
            4, "little", signed=True
        )
        rebuilt[record_offset + 20 : record_offset + 24] = new_offset.to_bytes(
            4, "little", signed=True
        )

    return rebuilt, locations


def patch_archive(
    args: argparse.Namespace,
    output: Path,
    xdb_records: list[dict[str, str]],
) -> list[dict[str, str]]:
    source = args.cd_root / "BLOOD.DAT"
    if not source.is_file():
        raise SystemExit(f"missing BLOOD.DAT: {source}")
    data = bytearray(source.read_bytes())
    archive_output = output / "cd" / "BLOOD.DAT"
    archive_output.parent.mkdir(parents=True, exist_ok=True)
    records = list(xdb_records)

    for script in range(1, 6):
        for extension in ("COD", "BAS"):
            name = f"SCRIPT{script}.{extension}"
            generated = output / "scripts" / name
            original = args.cd_root / name
            if not original.is_file():
                raise SystemExit(f"missing loose CD script: {original}")
            if generated.read_bytes() != original.read_bytes():
                raise SystemExit(f"generated script is not byte-exact: {name}")
            destination = output / "cd" / name
            shutil.copy2(generated, destination)
            records.append(
                {
                    "component": name,
                    "source": str(generated.relative_to(output)),
                    "output": str(destination.relative_to(output)),
                    "status": "bloodscript_byte_exact",
                    "offset": "loose_cd_file",
                    "original_sha256": sha256(original),
                    "output_sha256": sha256(destination),
                }
            )

    replacements: dict[str, bytes] = {}
    replacement_paths: dict[str, Path] = {}
    replacement_statuses: dict[str, str] = {}
    expected_original_hashes: dict[str, str] = {}
    for record in xdb_records:
        path = output / record["output"]
        key = record["component"].lower().replace("\\", "/")
        if key in replacements:
            raise SystemExit(f"duplicate BLOOD.DAT replacement: {key}")
        replacements[key] = path.read_bytes()
        replacement_paths[key] = path
        replacement_statuses[key] = record["status"]
        expected_original_hashes[key] = record["original_sha256"]

    rebuilt, locations = replace_archive_members(data, replacements)
    rebuilt_entries = archive_entries(rebuilt)
    for name, replacement in replacements.items():
        old_offset, old_size, new_offset, new_size = locations[name]
        if rebuilt_entries.get(name) != (new_offset, new_size):
            raise SystemExit(f"rebuilt BLOOD.DAT directory mismatch for {name}")
        if rebuilt[new_offset : new_offset + new_size] != replacement:
            raise SystemExit(f"rebuilt BLOOD.DAT payload mismatch for {name}")
        replacement_path = replacement_paths[name]
        original_hash = sha256_bytes(data[old_offset : old_offset + old_size])
        if original_hash != expected_original_hashes[name]:
            raise SystemExit(
                f"loose and archived original XDB hashes differ for {name}"
            )
        records.append(
            {
                "component": name,
                "source": str(replacement_path.relative_to(output)),
                "output": "cd/BLOOD.DAT",
                "status": replacement_statuses[name],
                "offset": f"0x{old_offset:08x}->0x{new_offset:08x}",
                "original_sha256": original_hash,
                "output_sha256": sha256_bytes(replacement),
            }
        )
    archive_output.write_bytes(rebuilt)
    return records


def write_package_metadata(output: Path, records: list[dict[str, str]], cd_root: Path) -> None:
    fields = ("component", "source", "output", "status", "offset", "original_sha256", "output_sha256")
    with (output / "package_manifest.tsv").open("w", encoding="ascii", newline="") as handle:
        handle.write("\t".join(fields) + "\n")
        for record in records:
            handle.write("\t".join(record[field] for field in fields) + "\n")
    bloodprg = output / "cd" / "BLOODPRG.EXE"
    relinked = output / "cd" / "BPRG_RE.EXE"
    relinked_hash = (
        f"BPRG_RE.EXE sha256: {sha256(relinked)}\n" if relinked.is_file() else ""
    )
    readme = (
        "Commander Blood recovered source package\n"
        "=========================================\n\n"
        "This package keeps the shipped BLOODPRG.EXE as a fallback launcher.\n"
        "When requested, BPRG_RE.EXE is a normal DOS executable linked from all\n"
        "recovered BLOODPRG C routines, the recovered entrypoint, DOS adapters,\n"
        "and byte-backed runtime data owners. It has passed the opening cinematic\n"
        "runtime smoke test, but is not yet claimed to have full-game parity.\n"
        "The fixed-patch copy remains limited to routines whose compiled bytes\n"
        "are proven compatible at the original fixed offsets.\n\n"
        "The generated SCRIPT1..5.COD/BAS files are compiled from re/vm/bloodscript\n"
        "and compared byte-for-byte with the installed reference. AMER.XDB,\n"
        "CROOLIS.XDB, MANU3.XDB, and SCRUT.XDB are linked from all 169 recovered\n"
        "C routines. Their original data payloads are retained byte-for-byte\n"
        "apart from verified callback-pointer rebindings to the linked C code.\n"
        "The size-changing overlays are installed by rebuilding the BLOOD.DAT\n"
        "resource offsets without changing unrelated resource payloads.\n"
        "package_manifest.tsv records every source verification and hash.\n\n"
        f"BLOODPRG.EXE sha256: {sha256(bloodprg)}\n"
        f"{relinked_hash}"
        f"Source CD tree: {cd_root}\n"
    )
    (output / "README.txt").write_text(readme, encoding="ascii")


def main() -> int:
    args = parse_args()
    for path in (args.cd_root, args.xdb_dir, args.source_dir, args.reference_dir):
        if not path.exists():
            raise SystemExit(f"missing input path: {path}")
    wcl = resolve_executable(args.wcl)
    args.wcl = wcl
    args.wasm = resolve_executable(args.wasm)
    args.wlink = resolve_executable(args.wlink)
    if args.include_bloodprg_fixed_patch:
        args.wdis = resolve_executable(args.wdis)
    turbo_objects = None
    if args.turbo_c_toolchain is not None:
        args.turbo_c_toolchain = args.turbo_c_toolchain.resolve()
        if not args.turbo_c_toolchain.is_dir():
            raise SystemExit(f"Turbo C toolchain does not exist: {args.turbo_c_toolchain}")
        args.dosbox = resolve_executable(args.dosbox)
    output = args.output_dir.resolve()
    output.mkdir(parents=True, exist_ok=True)
    runtime_alias = output / "cd" / "BPRG_RE.EXE"
    if not args.include_bloodprg_runtime and runtime_alias.exists():
        runtime_alias.unlink()
    compile_sources(args, output)
    runtime_records: list[dict[str, str]] = []
    fixed_records: list[dict[str, str]] = []
    bloodprg_objects = None
    if args.include_bloodprg_runtime or args.include_bloodprg_fixed_patch:
        bloodprg_objects = build_bloodprg_objects(args, output)
    if args.include_bloodprg_fixed_patch and args.turbo_c_toolchain is not None:
        turbo_objects = build_bloodprg_turbo_objects(args, output)
    if args.include_bloodprg_runtime:
        runtime_records = build_bloodprg_runtime(args, output)
    copy_cd_tree(args.cd_root.resolve(), output / "cd")
    if args.include_bloodprg_fixed_patch:
        assert bloodprg_objects is not None
        fixed_records = build_bloodprg_fixed_patch(
            args, output, bloodprg_objects, turbo_objects
        )
    xdb_records = build_source_xdb_files(args, output)
    records = patch_archive(args, output, xdb_records)
    records.extend(fixed_records)
    records.extend(runtime_records)
    write_package_metadata(output, records, args.cd_root.resolve())
    print(f"wrote recovered package: {output}")
    print("BLOODPRG.EXE status: original_shipped_fallback")
    print("XDB status: four_c_source_linked_overlays")
    if runtime_records:
        print("BPRG_RE.EXE status: c_relinked_runtime_zero_unresolved")
    if fixed_records:
        print("BPRG_C.EXE status: c_fixed_layout_verified_patch")
    print(f"recorded package components: {len(records)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
