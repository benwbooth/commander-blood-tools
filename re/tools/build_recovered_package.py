#!/usr/bin/env python3
"""Build a runnable hybrid package from recovered C and BloodScript sources.

The DOS executable is intentionally kept as the shipped BLOODPRG.EXE until its
startup, shared data, DOS adapters, and cross-overlay link boundaries are
recovered.  The archive is still patched through the real resource directory:
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
    ("scrut", "func_001de7_method_noop.c", 0x1DE7),
)

# These candidates have compiler-verified instruction shapes whose only
# relocations resolve to the original fixed overlay operands.
SHAPE_PATCHES = (
    ("amer", "func_000347_mouse_position_set.c", 0x0347, 14, "mouse_position"),
    ("croolis", "func_00035c_mouse_position_set.c", 0x035C, 14, "mouse_position"),
    ("scrut", "func_00035c_mouse_position_set.c", 0x035C, 14, "mouse_position"),
    ("manu3", "func_00017c_anim_select_entry.c", 0x017C, 4, "manu3_entry"),
)


def sha256_bytes(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def sha256(path: Path) -> str:
    return sha256_bytes(path.read_bytes())


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
    parser.add_argument(
        "--cbvm",
        type=Path,
        help="prebuilt cbvm executable; useful when Watcom and Rust use separate shells",
    )
    return parser.parse_args()


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
        f"-fm={map_file}",
        f"-fe={executable}",
        str(source),
        str(harness),
    ]
    process = subprocess.run(
        command, cwd=ROOT, text=True, capture_output=True, check=False
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
    for index, (actual, reference) in enumerate(zip(generated, expected)):
        if any(start <= index < end for start, end in ignored):
            continue
        if actual != reference:
            raise SystemExit(
                f"C shape mismatch for {source} at byte {index}: "
                f"generated 0x{actual:02x}, expected 0x{reference:02x}"
            )
    return expected


def patch_xdb_files(
    args: argparse.Namespace,
    output: Path,
    rows: dict[str, dict[str, str]],
) -> list[dict[str, str]]:
    xdb_output = output / "xdb"
    validation_dir = output / "validation"
    object_dir = output / "xdb_objects"
    records: list[dict[str, str]] = []
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
        patched = bytearray(original)
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
        patched = bytearray(original)
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

    manu3 = args.xdb_dir / "manu3.xdb"
    if not manu3.is_file():
        raise SystemExit(f"missing source overlay: {manu3}")
    manu3_output = xdb_output / "manu3.xdb"
    manu3_output.write_bytes(manu3.read_bytes())
    records.append(
        {
            "component": "manu3.xdb",
            "source": metadata_path(manu3),
            "output": str(manu3_output.relative_to(output)),
            "status": "original_overlay_no_c_patch",
            "offset": "-",
            "original_sha256": sha256(manu3),
            "output_sha256": sha256(manu3_output),
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
    (output / "README.txt").write_text(
        "Commander Blood recovered hybrid package\n"
        "==========================================\n\n"
        "This package uses the shipped BLOODPRG.EXE. It is not yet a fully\n"
        "C-linked replacement: the current aggregate link still has unresolved\n"
        "startup, shared-data, DOS/XMS/EMS, and cross-XDB symbols.\n\n"
        "The generated SCRIPT1..5.COD/BAS files are compiled from re/vm/bloodscript\n"
        "and compared byte-for-byte with the installed reference. The three\n"
        "alien-overlay no-op routines are verified by wdis, while the three\n"
        "mouse-position routines and MANU3 entry are linked in small DOS shape\n"
        "probes. Their fixed-layout machine-code shapes are compared against the\n"
        "original offsets before the XDB files and BLOOD.DAT are emitted.\n"
        "package_manifest.tsv records every source verification and hash.\n\n"
        f"BLOODPRG.EXE sha256: {sha256(bloodprg)}\n"
        f"Source CD tree: {cd_root}\n",
        encoding="ascii",
    )


def main() -> int:
    args = parse_args()
    for path in (args.cd_root, args.xdb_dir, args.source_dir, args.reference_dir):
        if not path.exists():
            raise SystemExit(f"missing input path: {path}")
    wcl = resolve_executable(args.wcl)
    args.wcl = wcl
    args.wdis = resolve_executable(args.wdis)
    output = args.output_dir.resolve()
    output.mkdir(parents=True, exist_ok=True)
    compile_sources(args, output)
    copy_cd_tree(args.cd_root.resolve(), output / "cd")
    shutil.copy2(args.cd_root / "BLOOD.DAT", output / "cd" / "BLOOD.DAT")
    rows = read_manifest_rows()
    xdb_records = patch_xdb_files(args, output, rows)
    records = patch_archive(args, output, xdb_records)
    write_package_metadata(output, records, args.cd_root.resolve())
    print(f"wrote hybrid package: {output}")
    print("BLOODPRG.EXE status: original_shipped_fallback")
    print(f"recorded package components: {len(records)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
