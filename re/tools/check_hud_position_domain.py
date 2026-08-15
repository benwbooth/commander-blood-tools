#!/usr/bin/env python3
"""Verify the shipped record-address domain used by BLOODPRG 0x006FF3."""

from __future__ import annotations

import argparse
import struct
from pathlib import Path


DATA_SEGMENT = 0x0CE2
FIELD_TABLE_OFFSET = 0x6D60
FIELD_SELECTOR_KIND100_POSITION = 0x09
FIELD_SELECTOR_POSITION = 0x0B
DIRECTORY_ENTRY_SIZE = 20
DIRECTORY_ACTIVE_KIND = 1
KIND100 = 0x0100


def field_matrix(executable: Path) -> bytes:
    image = executable.read_bytes()
    header_paragraphs = struct.unpack_from("<H", image, 0x08)[0]
    image_start = header_paragraphs * 16
    if image[image_start : image_start + 3] != b"\xB8\xE2\x0C":
        raise ValueError(f"{executable}: unexpected BLOODPRG entry sequence")
    start = image_start + DATA_SEGMENT * 16 + FIELD_TABLE_OFFSET
    table = image[start : start + 0x15 * 16]
    if len(table) != 0x15 * 16:
        raise ValueError(f"{executable}: truncated field-offset matrix")
    return table


def kind_column(kind: int) -> int:
    if kind == 0:
        raise ValueError("zero-kind active directory entry")
    return (kind & -kind).bit_length() - 1


def audit_profile(
    number: int, game_dir: Path, matrix: bytes
) -> tuple[int, int, int, int, set[int]]:
    deb_path = game_dir / f"SCRIPT{number}.DEB"
    var_path = game_dir / f"SCRIPT{number}.VAR"
    deb = deb_path.read_bytes()
    var = var_path.read_bytes()
    if len(deb) % DIRECTORY_ENTRY_SIZE:
        raise ValueError(f"{deb_path}: size is not a multiple of 20")

    active_count = 0
    eligible_count = 0
    maximum_base = 0
    maximum_end = 0
    field_offsets: set[int] = set()
    arche: tuple[int, int, int] | None = None

    for entry_offset in range(0, len(deb), DIRECTORY_ENTRY_SIZE):
        name_bytes, record_offset, entry_kind = struct.unpack_from(
            "<16sHH", deb, entry_offset
        )
        if entry_kind != DIRECTORY_ACTIVE_KIND:
            continue
        active_count += 1
        maximum_base = max(maximum_base, record_offset)
        if record_offset + 2 > len(var):
            raise ValueError(
                f"{var_path}: record 0x{record_offset:04X} lies outside VAR"
            )
        kind = struct.unpack_from("<H", var, record_offset)[0]
        column = kind_column(kind)
        selector = (
            FIELD_SELECTOR_KIND100_POSITION
            if kind == KIND100
            else FIELD_SELECTOR_POSITION
        )
        field_offset = matrix[selector * 16 + column]
        if name_bytes.split(b"\0", 1)[0] == b"arche":
            arche = (record_offset, kind, field_offset)
        if field_offset == 0:
            continue

        width = 8 if kind == KIND100 else 4
        end = record_offset + field_offset + width
        if field_offset & 0x80:
            raise ValueError(
                f"SCRIPT{number}: signed field offset 0x{field_offset:02X} "
                f"for kind 0x{kind:04X}"
            )
        if end > 0x10000:
            raise ValueError(
                f"SCRIPT{number}: record 0x{record_offset:04X} crosses 64 KiB"
            )
        eligible_count += 1
        maximum_end = max(maximum_end, end)
        field_offsets.add(field_offset)

    if arche is None:
        raise ValueError(f"{deb_path}: no active arche symbol")
    arche_offset, arche_kind, arche_field = arche
    if arche_kind != 0x0010 or arche_field != 0x18:
        raise ValueError(
            f"SCRIPT{number}: arche is 0x{arche_offset:04X}, "
            f"kind 0x{arche_kind:04X}, field 0x{arche_field:02X}"
        )

    print(
        f"SCRIPT{number}: active={active_count} eligible={eligible_count} "
        f"max_base=0x{maximum_base:04X} max_read_end=0x{maximum_end:04X} "
        f"arche=0x{arche_offset:04X}/kind=0x{arche_kind:04X}"
    )
    return active_count, eligible_count, maximum_base, maximum_end, field_offsets


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--executable", type=Path, default=Path("re/bin/BLOODPRG.EXE")
    )
    parser.add_argument(
        "--game-dir", type=Path, default=Path("accuracy/cdrive/CBLOOD")
    )
    args = parser.parse_args()

    matrix = field_matrix(args.executable)
    totals = [audit_profile(number, args.game_dir, matrix) for number in range(1, 6)]
    offsets = set().union(*(result[4] for result in totals))
    if offsets != {0x06, 0x18}:
        raise ValueError(
            "unexpected shipped HUD position offsets: "
            + ", ".join(f"0x{offset:02X}" for offset in sorted(offsets))
        )

    summary = (
        sum(result[0] for result in totals),
        sum(result[1] for result in totals),
        max(result[2] for result in totals),
        max(result[3] for result in totals),
    )
    if summary != (640, 216, 0x14E6, 0x14F0):
        raise ValueError(f"unexpected shipped HUD domain summary: {summary!r}")

    print(
        f"OK: {summary[0]} active entries, {summary[1]} position reads, "
        f"offsets=0x06/0x18, no signed offsets or 64-KiB crossings"
    )


if __name__ == "__main__":
    main()
