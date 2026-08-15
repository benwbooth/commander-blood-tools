#!/usr/bin/env python3
"""Verify the shipped record domain used by BLOODPRG 0x00713D."""

from __future__ import annotations

import argparse
import re
import struct
from pathlib import Path

from check_hud_position_domain import (
    DIRECTORY_ACTIVE_KIND,
    DIRECTORY_ENTRY_SIZE,
    FIELD_SELECTOR_POSITION,
    field_matrix,
    kind_column,
)


FIELD_SELECTOR_PARENT_LINK = 0x11
PRESENTABLE_KIND_MASK = 0x0098
LINKED_KIND_MASK = 0x0018


def read_word(image: bytes, offset: int, path: Path, label: str) -> int:
    if offset + 2 > len(image):
        raise ValueError(f"{path}: {label} at 0x{offset:04X} lies outside image")
    return struct.unpack_from("<H", image, offset)[0]


def field_offset(matrix: bytes, selector: int, kind: int) -> int:
    return matrix[selector * 16 + kind_column(kind)]


def checked_field_end(
    profile: int,
    record_offset: int,
    offset: int,
    width: int,
    description: str,
) -> int:
    if offset & 0x80:
        raise ValueError(
            f"SCRIPT{profile}: signed {description} offset 0x{offset:02X}"
        )
    end = record_offset + offset + width
    if end > 0x10000:
        raise ValueError(
            f"SCRIPT{profile}: {description} at record 0x{record_offset:04X} "
            "crosses 64 KiB"
        )
    return end


def audit_profile(
    number: int, game_dir: Path, structured_dir: Path, matrix: bytes
) -> tuple[int, int, int, int, int, int, int, set[int], set[int], set[int]]:
    deb_path = game_dir / f"SCRIPT{number}.DEB"
    var_path = game_dir / f"SCRIPT{number}.VAR"
    deb = deb_path.read_bytes()
    var = var_path.read_bytes()
    if len(deb) % DIRECTORY_ENTRY_SIZE:
        raise ValueError(f"{deb_path}: size is not a multiple of 20")

    entries: list[tuple[bytes, int]] = []
    for entry_offset in range(0, len(deb), DIRECTORY_ENTRY_SIZE):
        name_bytes, record_offset, entry_kind = struct.unpack_from(
            "<16sHH", deb, entry_offset
        )
        if (entry_kind & 0x00FF) != DIRECTORY_ACTIVE_KIND:
            break
        entries.append((name_bytes.split(b"\0", 1)[0], record_offset))

    names_by_offset = {record_offset: name for name, record_offset in entries}
    kinds_by_offset = {
        record_offset: read_word(var, record_offset, var_path, "record kind")
        for _, record_offset in entries
    }
    arche_entries = [entry for entry in entries if entry[0] == b"arche"]
    if len(arche_entries) != 1:
        raise ValueError(f"{deb_path}: expected one active arche symbol")
    arche_offset = arche_entries[0][1]
    arche_kind = read_word(var, arche_offset, var_path, "arche kind")
    arche_position_offset = field_offset(
        matrix, FIELD_SELECTOR_POSITION, arche_kind
    )
    maximum_end = checked_field_end(
        number, arche_offset, arche_position_offset, 4, "arche position"
    )

    candidate_count = 0
    linked_count = 0
    linked_active_count = 0
    linked_targets: set[int] = set()
    linked_target_kinds: set[int] = set()
    position_offsets: set[int] = {arche_position_offset}
    parent_offsets: set[int] = set()
    parent_field_addresses: set[int] = set()

    for name, candidate_offset in entries:
        kind = read_word(var, candidate_offset, var_path, "candidate kind")
        if (kind & PRESENTABLE_KIND_MASK) == 0 or candidate_offset == arche_offset:
            continue
        candidate_count += 1
        effective_offset = candidate_offset
        effective_kind = kind

        if kind & 0x0080:
            linked_count += 1
            parent_offset = field_offset(
                matrix, FIELD_SELECTOR_PARENT_LINK, 0x0080
            )
            parent_offsets.add(parent_offset)
            parent_field_addresses.add((candidate_offset + parent_offset) & 0xFFFF)
            maximum_end = max(
                maximum_end,
                checked_field_end(
                    number, candidate_offset, parent_offset, 2, "parent link"
                ),
            )
            effective_offset = read_word(
                var,
                candidate_offset + parent_offset,
                var_path,
                f"parent link for {name.decode('ascii', 'replace')}",
            )
            linked_targets.add(effective_offset)
            effective_kind = read_word(
                var, effective_offset, var_path, "linked-record kind"
            )
            linked_target_kinds.add(effective_kind)
            effective_flags = read_word(
                var, effective_offset + 2, var_path, "linked-record flags"
            )
            if (effective_kind & LINKED_KIND_MASK) == 0:
                target_name = names_by_offset.get(effective_offset, b"<unnamed>")
                raise ValueError(
                    f"SCRIPT{number}: {name!r} links to {target_name!r} "
                    f"kind 0x{effective_kind:04X}"
                )
            if effective_flags & 1:
                linked_active_count += 1

        position_offset = field_offset(
            matrix, FIELD_SELECTOR_POSITION, effective_kind
        )
        position_offsets.add(position_offset)
        maximum_end = max(
            maximum_end,
            checked_field_end(
                number,
                effective_offset,
                position_offset,
                4,
                "effective position",
            ),
        )

    vm_text = "\n".join(
        (structured_dir / f"script{number}.{image}.blood").read_text()
        for image in ("cod", "bas")
    )
    if "VAR kind 0x0080, selector(s) 11" in vm_text:
        raise ValueError(f"SCRIPT{number}: VM source aliases a kind-0x80 parent field")
    direct_parent_refs = {
        int(value, 16)
        for value in re.findall(
            r"(?i)(?:state|flags)\[0x([0-9a-f]{4})\]", vm_text
        )
    } & parent_field_addresses
    if direct_parent_refs:
        raise ValueError(
            f"SCRIPT{number}: VM directly references kind-0x80 parent fields "
            + ", ".join(f"0x{value:04X}" for value in sorted(direct_parent_refs))
        )

    cd_related = [
        int(value, 16)
        for value in re.findall(
            r"(?im)^\s*record_triple\s+\S+\s+0x([0-9a-f]{4})\s+", vm_text
        )
    ]
    c2_related = [
        int(value, 16)
        for value in re.findall(
            r"(?im)^\s*record_state\s+0xC2\s+\S+\s+0x([0-9a-f]{4})\s+",
            vm_text,
        )
    ]
    cd_kinds = {kinds_by_offset.get(offset) for offset in cd_related}
    c2_kinds = {kinds_by_offset.get(offset) for offset in c2_related}
    if cd_kinds - {0x0400} or c2_kinds - {0x0002}:
        raise ValueError(
            f"SCRIPT{number}: unexpected selector-0x11 writer kinds "
            f"CD={sorted(cd_kinds)!r}, C2={sorted(c2_kinds)!r}"
        )

    print(
        f"SCRIPT{number}: scanned={len(entries)} candidates={candidate_count} "
        f"linked={linked_count} linked_active={linked_active_count} "
        f"linked_targets={len(linked_targets)} max_read_end=0x{maximum_end:04X} "
        f"arche=0x{arche_offset:04X}/kind=0x{arche_kind:04X} "
        f"CD_writes={len(cd_related)} C2_writes={len(c2_related)}"
    )
    return (
        len(entries),
        candidate_count,
        linked_count,
        linked_active_count,
        maximum_end,
        len(cd_related),
        len(c2_related),
        position_offsets,
        parent_offsets,
        linked_target_kinds,
    )


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--executable", type=Path, default=Path("re/bin/BLOODPRG.EXE")
    )
    parser.add_argument(
        "--game-dir", type=Path, default=Path("accuracy/cdrive/CBLOOD")
    )
    parser.add_argument(
        "--structured-dir", type=Path, default=Path("re/vm/structured")
    )
    args = parser.parse_args()

    matrix = field_matrix(args.executable)
    totals = [
        audit_profile(number, args.game_dir, args.structured_dir, matrix)
        for number in range(1, 6)
    ]
    expected_profiles = [
        (122, 63, 22, 22, 0x0FCC, 0, 0),
        (122, 62, 21, 21, 0x0FFC, 86, 2),
        (130, 62, 22, 22, 0x1136, 82, 0),
        (136, 61, 21, 21, 0x11F6, 10, 0),
        (130, 66, 25, 25, 0x10C4, 4, 0),
    ]
    observed_profiles = [result[:7] for result in totals]
    if observed_profiles != expected_profiles:
        raise ValueError(f"unexpected shipped profile domains: {observed_profiles!r}")

    position_offsets = set().union(*(result[7] for result in totals))
    parent_offsets = set().union(*(result[8] for result in totals))
    linked_target_kinds = set().union(*(result[9] for result in totals))
    summary = (
        sum(result[0] for result in totals),
        sum(result[1] for result in totals),
        sum(result[2] for result in totals),
        sum(result[3] for result in totals),
        max(result[4] for result in totals),
        sum(result[5] for result in totals),
        sum(result[6] for result in totals),
    )
    if summary != (640, 314, 111, 111, 0x11F6, 182, 2):
        raise ValueError(f"unexpected shipped state-record summary: {summary!r}")
    if position_offsets != {0x18} or parent_offsets != {0x14}:
        raise ValueError(
            f"unexpected field offsets: position={position_offsets!r}, "
            f"parent={parent_offsets!r}"
        )
    if linked_target_kinds != {0x0008, 0x0010}:
        raise ValueError(f"unexpected linked target kinds: {linked_target_kinds!r}")

    print(
        f"OK: {summary[0]} scanned entries, {summary[1]} candidates, "
        f"{summary[2]} linked ({summary[3]} initially active), "
        f"position_offsets={','.join(f'0x{x:02X}' for x in sorted(position_offsets))}, "
        f"parent_offsets={','.join(f'0x{x:02X}' for x in sorted(parent_offsets))}, "
        f"linked_kinds=0x08/0x10, max_read_end=0x{summary[4]:04X}; "
        f"all {summary[5]} CD and {summary[6]} C2 writes target other kinds"
    )


if __name__ == "__main__":
    main()
