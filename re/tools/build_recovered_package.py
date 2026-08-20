#!/usr/bin/env python3
"""Build a runnable hybrid package from recovered C and BloodScript sources.

The DOS executable is intentionally kept as the shipped BLOODPRG.EXE until its
startup, shared data, DOS adapters, and cross-overlay link boundaries are
recovered.  An opt-in fixed-layout patch emits only independently verified C
replacements.  The archive is still patched through the real resource directory:
the generated scripts are byte-exact, and the three one-byte C no-op routines
are verified with Watcom and written at their original fixed offsets.
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

NOOP_PATCHES = (
    ("amer", "func_001dd6_method_noop.c", 0x1DD6),
    ("croolis", "func_001d27_method_noop.c", 0x1D27),
    ("manu3", "func_000848_span_setup_next.c", 0x0848),
    ("scrut", "func_001de7_method_noop.c", 0x1DE7),
)

# These candidates have compiler-verified instruction shapes whose only
# relocations resolve to the original fixed overlay operands.
SHAPE_PATCHES = (
    ("amer", "func_000347_mouse_position_set.c", 0x0347, 14, "mouse_position"),
    ("croolis", "func_00035c_mouse_position_set.c", 0x035C, 14, "mouse_position"),
    ("scrut", "func_00035c_mouse_position_set.c", 0x035C, 14, "mouse_position"),
    ("manu3", "func_00017c_anim_select_entry.c", 0x017C, 4, "manu3_entry"),
    ("amer", "func_000b0f_method_slot_11_anchor_state.c", 0x0B0F, 16, "method_slot_11"),
    ("croolis", "func_000b50_method_slot_11_anchor_state.c", 0x0B50, 16, "method_slot_11"),
    ("scrut", "func_000b55_method_slot_11_anchor_state.c", 0x0B55, 16, "method_slot_11"),
)

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
    parser.add_argument(
        "--include-bloodprg-link-probe",
        action="store_true",
        help="compile and link the recovered BLOODPRG C objects into a validation executable",
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


def build_bloodprg_link_probe(args: argparse.Namespace, output: Path) -> dict[str, str]:
    """Build a real C aggregate link with an explicit startup harness."""
    validation_dir = output / "validation" / "bloodprg_link"
    validation_dir.mkdir(parents=True, exist_ok=True)
    object_dir = output / "bloodprg_objects"

    startup_object = validation_dir / "startup_gate.obj"
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
            "-i=" + str(ROOT / "re/source/bloodprg/candidates/include"),
            "-fo=" + str(startup_object),
            str(ROOT / "re/integration/dos/bloodprg_startup_options.c"),
        ]
    )

    initial_dir = validation_dir / "initial"
    initial_dir.mkdir(parents=True, exist_ok=True)
    initial_status = run_link_probe(
        initial_dir,
        startup_object,
        object_dir,
        [],
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
            "-i=" + str(ROOT / "re/source/bloodprg/candidates/include"),
            "-fo=" + str(adapter_object),
            str(ROOT / "re/integration/dos/bloodprg_platform_adapters.c"),
        ]
    )

    final_dir = validation_dir / "final"
    final_dir.mkdir(parents=True, exist_ok=True)
    final_status = run_link_probe(
        final_dir,
        startup_object,
        object_dir,
        [adapter_object, owner_object],
        "BLOODPRG_C_LINK.EXE",
    )
    final_executable = final_dir / "BLOODPRG_C_LINK.EXE"
    final_report = final_dir / "unresolved.tsv"
    unresolved_lines = final_report.read_text(encoding="ascii").splitlines()
    if final_status != 0 or not final_executable.is_file() or len(unresolved_lines) != 1:
        raise SystemExit(
            "final BLOODPRG C link did not resolve cleanly; see "
            f"{final_dir / 'link.log'} and {final_report}"
        )
    # DOS 8.3 lookup is part of the runtime check; keep a short command name
    # beside the descriptive host-side artifact.
    shutil.copy2(final_executable, final_dir / "BPRG.EXE")
    return {
        "component": final_executable.name,
        "source": str(final_executable.relative_to(output)),
        "output": str(final_executable.relative_to(output)),
        "status": "c_aggregate_link_zero_unresolved_startup_harness",
        "offset": "-",
        "original_sha256": "-",
        "output_sha256": sha256(final_executable),
    }


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


def read_manifest_rows() -> dict[str, dict[str, str]]:
    with XDB_MANIFEST.open(newline="", encoding="ascii") as handle:
        return {row["source"]: row for row in csv.DictReader(handle, delimiter="\t")}


def verify_noop_object(
    source: Path,
    row: dict[str, str],
    object_dir: Path,
    wdis: str,
    validation_dir: Path,
) -> None:
    entry_module = row["entry"].split(":", 1)[0]
    object_path = object_dir / entry_module / f"{source.stem}.OBJ"
    if not object_path.is_file():
        raise SystemExit(f"missing compiled object for {source}: {object_path}")
    process = subprocess.run(
        [wdis, "-a", str(object_path)],
        cwd=ROOT,
        text=True,
        capture_output=True,
    )
    if process.returncode != 0:
        raise SystemExit(f"wdis failed for {object_path}: {process.stderr}")
    listing = process.stdout
    validation_dir.mkdir(parents=True, exist_ok=True)
    (validation_dir / f"{source.stem}.asm").write_text(listing, encoding="ascii")
    instruction_lines = re.findall(
        r"^\s{4,}([a-z][a-z0-9]*)\b.*$", listing, flags=re.IGNORECASE | re.MULTILINE
    )
    if instruction_lines != ["ret"]:
        raise SystemExit(
            f"C replacement is not a one-instruction RET for {source}: "
            f"{instruction_lines!r}"
        )


def read_mz_image(path: Path) -> bytes:
    data = path.read_bytes()
    if data[:2] not in (b"MZ", b"ZM"):
        raise SystemExit(f"linked C probe is not a DOS MZ executable: {path}")
    header_size = int.from_bytes(data[8:10], "little") * 16
    pages = int.from_bytes(data[4:6], "little")
    last_page = int.from_bytes(data[2:4], "little")
    image_total = pages * 512 if last_page == 0 else (pages - 1) * 512 + last_page
    return data[header_size:image_total]


def verify_shape_probe(
    args: argparse.Namespace,
    output: Path,
    source: Path,
    length: int,
    kind: str,
) -> bytes:
    validation_dir = output / "validation" / "shape_probes"
    validation_dir.mkdir(parents=True, exist_ok=True)
    stem = source.stem
    harness = validation_dir / f"{stem}_{kind}_harness.c"
    executable = validation_dir / f"{stem}_{kind}.EXE"
    map_file = validation_dir / f"{stem}_{kind}.map"
    if kind == "mouse_position":
        harness.write_text(
            '#include "re/source/xdb/candidates/include/xdb_mouse.h"\n'
            "volatile xdb_mouse_state xdb_alien_mouse_state;\n"
            "int main(void) { return 0; }\n",
            encoding="ascii",
        )
    elif kind == "manu3_entry":
        harness.write_text(
            '#include "re/source/xdb/candidates/include/xdb_manu3.h"\n'
            "void pad(void) {}\n"
            "void XDB_NEAR xdb_manu3_anim_select(xdb_u16 selector) "
            "{ (void)selector; }\n"
            "int main(void) { return 0; }\n",
            encoding="ascii",
        )
    elif kind == "method_slot_11":
        harness.write_text(
            '#include "re/source/xdb/candidates/include/xdb_alien.h"\n'
            "xdb_alien_cursor XDB_CODE_DATA xdb_amer_slot11_cursor;\n"
            "xdb_alien_cursor XDB_CODE_DATA xdb_croolis_slot11_cursor;\n"
            "xdb_alien_cursor XDB_CODE_DATA xdb_scrut_slot11_cursor;\n"
            "int main(void) { return 0; }\n",
            encoding="ascii",
        )
    else:
        raise SystemExit(f"unknown shape probe kind: {kind}")
    command = [
        args.wcl,
        "-q",
        "-3",
        "-ox",
        "-mm",
        "-zdp",
        "-we",
        "-i=" + str(ROOT),
        f"-fm={map_file}",
        f"-fe={executable}",
        str(source),
        str(harness),
    ]
    process = subprocess.run(
        command, cwd=validation_dir, text=True, capture_output=True, check=False
    )
    if process.returncode != 0 or not executable.is_file():
        diagnostics = "\n".join(
            part for part in (process.stdout, process.stderr) if part
        )
        raise SystemExit(f"C shape probe failed for {source}: {diagnostics}")
    image = read_mz_image(executable)
    if len(image) < length:
        raise SystemExit(f"shape probe image is shorter than {length} bytes: {executable}")
    generated = image[:length]
    (validation_dir / f"{stem}_{kind}.bin").write_bytes(generated)
    return generated


def verify_shape_patch(
    args: argparse.Namespace,
    output: Path,
    source: Path,
    original: bytes,
    offset: int,
    length: int,
    kind: str,
) -> bytes:
    generated = verify_shape_probe(args, output, source, length, kind)
    expected = original[offset : offset + length]
    if len(expected) != length:
        raise SystemExit(f"fixed overlay routine exceeds {source}: 0x{offset:04x}")
    ignored = {(2, 4), (6, 8)} if kind == "mouse_position" else set()
    if kind == "method_slot_11":
        ignored = {(13, 15)}
    replacement = bytearray(generated)
    for index, (actual, reference) in enumerate(zip(generated, expected)):
        if any(start <= index < end for start, end in ignored):
            replacement[index] = reference
            continue
        if kind == "method_slot_11" and 6 <= index < 10:
            continue
        if actual != reference:
            raise SystemExit(
                f"C shape mismatch for {source} at byte {index}: "
                f"generated 0x{actual:02x}, expected 0x{reference:02x}"
            )
    return bytes(replacement)


def patch_xdb_files(
    args: argparse.Namespace,
    output: Path,
    rows: dict[str, dict[str, str]],
) -> list[dict[str, str]]:
    xdb_output = output / "xdb"
    validation_dir = output / "validation"
    object_dir = output / "xdb_objects"
    records: list[dict[str, str]] = []
    for module in ("amer", "croolis", "manu3", "scrut"):
        original_path = args.xdb_dir / f"{module}.xdb"
        if not original_path.is_file():
            raise SystemExit(f"missing source overlay: {original_path}")
        destination = xdb_output / f"{module}.xdb"
        destination.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(original_path, destination)

    for module, source_name, offset in NOOP_PATCHES:
        source = XDB_MANIFEST.parent / module / source_name
        row = rows.get(f"{module}/{source_name}")
        if row is None:
            raise SystemExit(f"manifest has no C source row: {source}")
        verify_noop_object(source, row, object_dir, args.wdis, validation_dir)

        original_path = args.xdb_dir / f"{module}.xdb"
        if not original_path.is_file():
            raise SystemExit(f"missing source overlay: {original_path}")
        original = original_path.read_bytes()
        if offset >= len(original) or original[offset] != 0xC3:
            raise SystemExit(
                f"fixed RET invariant failed for {original_path} at 0x{offset:04x}"
            )
        patched = bytearray((xdb_output / f"{module}.xdb").read_bytes())
        patched[offset] = 0xC3
        destination = xdb_output / f"{module}.xdb"
        destination.parent.mkdir(parents=True, exist_ok=True)
        destination.write_bytes(patched)
        records.append(
            {
                "component": f"{module}.xdb",
                "source": metadata_path(source),
                "output": str(destination.relative_to(output)),
                "status": "c_source_fixed_offset_patch",
                "offset": f"0x{offset:04x}",
                "original_sha256": sha256_bytes(original),
                "output_sha256": sha256_bytes(patched),
            }
        )

    for module, source_name, offset, length, kind in SHAPE_PATCHES:
        source = XDB_MANIFEST.parent / module / source_name
        row = rows.get(f"{module}/{source_name}")
        if row is None:
            raise SystemExit(f"manifest has no C source row: {source}")
        original_path = args.xdb_dir / f"{module}.xdb"
        if not original_path.is_file():
            raise SystemExit(f"missing source overlay: {original_path}")
        original = original_path.read_bytes()
        replacement = verify_shape_patch(
            args, output, source, original, offset, length, kind
        )
        patched = bytearray((xdb_output / f"{module}.xdb").read_bytes())
        patched[offset : offset + length] = replacement
        destination = xdb_output / f"{module}.xdb"
        destination.parent.mkdir(parents=True, exist_ok=True)
        destination.write_bytes(patched)
        records.append(
            {
                "component": f"{module}.xdb@0x{offset:04x}",
                "archive_name": f"{module}.xdb",
                "source": metadata_path(source),
                "output": str(destination.relative_to(output)),
                "status": "c_source_fixed_layout_verified",
                "offset": f"0x{offset:04x}",
                "original_sha256": sha256_bytes(original),
                "output_sha256": sha256_bytes(patched),
            }
        )

    return records


def archive_entries(data: bytearray) -> dict[str, tuple[int, int]]:
    entries: dict[str, tuple[int, int]] = {}
    cursor = 2
    while cursor < min(65536, len(data)):
        name_start = cursor
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
            entries.setdefault(name, (offset, size))
        if cursor <= name_start:
            raise SystemExit("BLOOD.DAT directory parser did not advance")
    return entries


def patch_archive(
    args: argparse.Namespace,
    output: Path,
    xdb_records: list[dict[str, str]],
) -> list[dict[str, str]]:
    source = args.cd_root / "BLOOD.DAT"
    if not source.is_file():
        raise SystemExit(f"missing BLOOD.DAT: {source}")
    data = bytearray(source.read_bytes())
    entries = archive_entries(data)
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

    replacements: list[tuple[str, Path, str]] = []
    for record in xdb_records:
        path = output / record["output"]
        replacements.append((record["component"], path, record["status"]))

    for name, replacement_path, status in replacements:
        archive_name = name.split("@", 1)[0]
        key = archive_name.lower().replace("\\", "/")
        if key not in entries:
            raise SystemExit(f"resource is absent from BLOOD.DAT: {archive_name}")
        offset, size = entries[key]
        replacement = replacement_path.read_bytes()
        if len(replacement) != size:
            raise SystemExit(
                f"size-changing archive replacement refused for {archive_name}: "
                f"{len(replacement)} != {size}"
            )
        original_hash = sha256_bytes(data[offset : offset + size])
        data[offset : offset + size] = replacement
        records.append(
            {
                "component": name,
                "source": str(replacement_path.relative_to(output)),
                "output": "cd/BLOOD.DAT",
                "status": status,
                "offset": f"0x{offset:08x}",
                "original_sha256": original_hash,
                "output_sha256": sha256_bytes(replacement),
            }
        )
    archive_output.write_bytes(data)
    return records


def write_package_metadata(output: Path, records: list[dict[str, str]], cd_root: Path) -> None:
    fields = ("component", "source", "output", "status", "offset", "original_sha256", "output_sha256")
    with (output / "package_manifest.tsv").open("w", encoding="ascii", newline="") as handle:
        handle.write("\t".join(fields) + "\n")
        for record in records:
            handle.write("\t".join(record[field] for field in fields) + "\n")
    bloodprg = output / "cd" / "BLOODPRG.EXE"
    readme = (
        "Commander Blood recovered hybrid package\n"
        "==========================================\n\n"
        "This package keeps the shipped BLOODPRG.EXE as the default launcher.\n"
        "It also records optional C-derived validation artifacts: the aggregate\n"
        "link uses a startup harness, and the fixed-patch copy is emitted only\n"
        "for routines whose compiled bytes are proven compatible at the original\n"
        "fixed offsets. Neither artifact is mislabeled as a full decompilation.\n\n"
        "The generated SCRIPT1..5.COD/BAS files are compiled from re/vm/bloodscript\n"
        "and compared byte-for-byte with the installed reference. The four\n"
        "alien-overlay no-op routines are verified by wdis, while the three\n"
        "mouse-position routines, MANU3 entry, and three slot-11 routines are\n"
        "linked in small DOS shape probes. Their fixed-layout machine-code\n"
        "shapes are compared against the\n"
        "original offsets, with only approved relocation words restored, before\n"
        "the XDB files and BLOOD.DAT are emitted.\n"
        "package_manifest.tsv records every source verification and hash.\n\n"
        f"BLOODPRG.EXE sha256: {sha256(bloodprg)}\n"
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
    args.wdis = resolve_executable(args.wdis)
    if args.include_bloodprg_link_probe:
        args.wasm = resolve_executable(args.wasm)
    turbo_objects = None
    if args.turbo_c_toolchain is not None:
        args.turbo_c_toolchain = args.turbo_c_toolchain.resolve()
        if not args.turbo_c_toolchain.is_dir():
            raise SystemExit(f"Turbo C toolchain does not exist: {args.turbo_c_toolchain}")
        args.dosbox = resolve_executable(args.dosbox)
    output = args.output_dir.resolve()
    output.mkdir(parents=True, exist_ok=True)
    compile_sources(args, output)
    link_record = None
    fixed_records: list[dict[str, str]] = []
    bloodprg_objects = None
    if args.include_bloodprg_link_probe or args.include_bloodprg_fixed_patch:
        bloodprg_objects = build_bloodprg_objects(args, output)
    if args.include_bloodprg_fixed_patch and args.turbo_c_toolchain is not None:
        turbo_objects = build_bloodprg_turbo_objects(args, output)
    if args.include_bloodprg_link_probe:
        link_record = build_bloodprg_link_probe(args, output)
    copy_cd_tree(args.cd_root.resolve(), output / "cd")
    shutil.copy2(args.cd_root / "BLOOD.DAT", output / "cd" / "BLOOD.DAT")
    if args.include_bloodprg_fixed_patch:
        assert bloodprg_objects is not None
        fixed_records = build_bloodprg_fixed_patch(
            args, output, bloodprg_objects, turbo_objects
        )
    rows = read_manifest_rows()
    xdb_records = patch_xdb_files(args, output, rows)
    records = patch_archive(args, output, xdb_records)
    records.extend(fixed_records)
    if link_record is not None:
        records.append(link_record)
    write_package_metadata(output, records, args.cd_root.resolve())
    print(f"wrote hybrid package: {output}")
    print("BLOODPRG.EXE status: original_shipped_fallback")
    if link_record is not None:
        print("BLOODPRG_C_LINK.EXE status: c_aggregate_link_zero_unresolved_startup_harness")
    if fixed_records:
        print("BPRG_C.EXE status: c_fixed_layout_verified_patch")
    print(f"recorded package components: {len(records)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
