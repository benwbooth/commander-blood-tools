#!/usr/bin/env python3
"""Classify Big Bug Bang's field-display dispatch as a diagnostic inspector.

The generic indirect-dispatch atlas discovers 21 table entries but cannot say
what they do. This audit binds the table to the adjacent field-name and
field-offset matrices, verifies its guarded main-loop owner, and records the
formatter shape selected for every field. It is static ownership evidence, not
proof that the retail game can activate the inspector without external state.
"""

from __future__ import annotations

import argparse
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import sys


_HERE = Path(__file__).resolve().parent
sys.path[:] = [
    path
    for path in sys.path
    if Path(os.path.abspath(path or os.curdir)) != _HERE
]

from capstone import CS_AC_READ, CS_AC_WRITE  # noqa: E402
from capstone.x86 import X86_OP_MEM  # noqa: E402


def _load_atlas():
    spec = importlib.util.spec_from_file_location(
        "big_bug_bang_indirect_dispatch_atlas",
        _HERE / "big_bug_bang_indirect_dispatch_atlas.py",
    )
    assert spec is not None and spec.loader is not None
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


ATLAS = _load_atlas()

EXECUTABLE_SHA256 = ATLAS.EXECUTABLE_SHA256
GLOBAL_DATA_FILE_BASE = 0xF7F0
MAIN_CALL_SITE = 0x13BD
DISPLAY_OWNER_ENTRY = 0x7C0E
DIRECTORY_LIST_ENTRY = 0x25DE
RECORD_DISPLAY_ENTRY = 0x7C65
EMPTY_SELECTION_ENTRY = 0x7F21
MODE_OFFSET = 0x6B7C
FIELD_MATRIX_OFFSET = 0x7128
FIELD_LABEL_OFFSET = 0x736C
FIELD_STRIDE = 16
OBJECT_KIND_COUNT = 9

OBJECT_KIND_NAMES = (
    "player",
    "actor",
    "celestial_body",
    "navigation_entity",
    "auxiliary",
    "location",
    "black_hole",
    "world_state",
    "inventory_item",
)

FIELD_NAMES = (
    "POP",
    "BASE",
    "AGR",
    "NRJ",
    "CONNAIS",
    "PORTEE",
    "EVL",
    "RENCONTRE",
    "POSITION1",
    "POSITION2",
    "POSITION",
    "UNIVERS1",
    "UNIVERS2",
    "UNIVERS",
    "SUJET",
    "RACE",
    "LIEU",
    "MESSAGE",
    "ACTION",
    "ICONE",
    "ATTAQUE",
)

HANDLER_FORMATS = {
    0x7D48: "decimal_word",
    0x7D67: "present_absent_label",
    0x7D83: "known_object_name_list",
    0x7DCB: "decimal_word_pair",
    0x7E06: "directory_object_name",
    0x7E40: "state_object_name_or_blank",
    0x7E5C: "dictionary_text",
    0x7E70: "lowest_set_bit_name",
    0x7E8E: "label_only",
    0x7E8F: "message_opcode_and_object",
    0x7EF1: "eight_dictionary_names",
}

EXPECTED_MODE_READ_SITES = (
    0x23D5,
    0x2424,
    0x2445,
    0x2495,
    0x24A3,
    0x2519,
    0x7C0F,
    0x7C17,
)


def _digest(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def _fixed_string(data: bytes, offset: int, capacity: int = FIELD_STRIDE) -> str:
    raw = data[offset : offset + capacity]
    if len(raw) != capacity:
        raise ValueError(f"truncated fixed string at {offset:#x}")
    return raw.split(b"\0", 1)[0].decode("ascii")


def _expanded_builder(executable: Path):
    mz = ATLAS.GRAPH.MZ(str(executable))
    builder = ATLAS.GRAPH.FunctionGraphBuilder(mz)
    builder.build()
    for table in ATLAS.TABLES:
        _entries, roots = ATLAS.table_entries(mz, table)
        for entry, segment in sorted(roots.items()):
            builder.walk_function(entry, segment)
    return builder


def _mode_accesses(builder) -> list[dict[str, object]]:
    accesses = []
    visited = set()
    for instructions in builder.function_instructions.values():
        for address in sorted(instructions):
            if address in visited:
                continue
            visited.add(address)
            instruction = builder.one_instruction(address)
            for operand in instruction.operands:
                if operand.type != X86_OP_MEM or operand.mem.disp != MODE_OFFSET:
                    continue
                access = []
                if operand.access & CS_AC_READ:
                    access.append("read")
                if operand.access & CS_AC_WRITE:
                    access.append("write")
                accesses.append(
                    {
                        "file_offset": f"0x{address:06x}",
                        "mnemonic": instruction.mnemonic,
                        "operand": instruction.op_str,
                        "segment": instruction.reg_name(operand.mem.segment) or "default_ds",
                        "access": access,
                    }
                )
    return sorted(accesses, key=lambda row: str(row["file_offset"]))


def audit(executable: Path) -> dict[str, object]:
    digest = _digest(executable)
    if digest != EXECUTABLE_SHA256:
        raise ValueError(f"unrecognized BLOOD2PG.EXE SHA-256 {digest}")

    builder = _expanded_builder(executable)
    mz = builder.mz
    field_table = next(
        table for table in ATLAS.TABLES if table["name"] == "field_display_handlers"
    )
    entries, _targets = ATLAS.table_entries(mz, field_table)
    if len(entries) != len(FIELD_NAMES):
        raise ValueError("field-display dispatch no longer has 21 selectors")

    call = builder.one_instruction(MAIN_CALL_SITE)
    far_target = builder.far_target(call)
    if far_target is None or far_target[0] != DISPLAY_OWNER_ENTRY:
        raise ValueError("main-loop field-display call changed")
    expected_callees = {
        DIRECTORY_LIST_ENTRY,
        RECORD_DISPLAY_ENTRY,
        EMPTY_SELECTION_ENTRY,
    }
    if builder.callgraph[DISPLAY_OWNER_ENTRY] != expected_callees:
        raise ValueError("field-display owner call graph changed")

    mode_accesses = _mode_accesses(builder)
    read_sites = tuple(int(str(row["file_offset"]), 16) for row in mode_accesses)
    if read_sites != EXPECTED_MODE_READ_SITES:
        raise ValueError(f"field-display mode accesses changed: {read_sites}")
    if any("write" in row["access"] for row in mode_accesses):
        raise ValueError("field-display mode gained a direct static write")

    initial_mode = mz.data[GLOBAL_DATA_FILE_BASE + MODE_OFFSET]
    if initial_mode != 0:
        raise ValueError(f"field-display mode initial value changed to {initial_mode:#x}")
    state_label = _fixed_string(
        mz.data, GLOBAL_DATA_FILE_BASE + FIELD_LABEL_OFFSET
    )
    if state_label != "ETAT":
        raise ValueError(f"state label changed to {state_label!r}")

    selectors = []
    for index, (name, entry) in enumerate(zip(FIELD_NAMES, entries, strict=True), start=1):
        label = _fixed_string(
            mz.data,
            GLOBAL_DATA_FILE_BASE + FIELD_LABEL_OFFSET + index * FIELD_STRIDE,
        )
        if label != name:
            raise ValueError(f"selector {index} label changed to {label!r}")
        if int(str(entry["selector"]), 16) != index:
            raise ValueError(f"selector {index} dispatch metadata is off by one")
        handler = int(str(entry["target_file_offset"]), 16)
        formatter = HANDLER_FORMATS.get(handler)
        if formatter is None:
            raise ValueError(f"selector {index} has unknown handler {handler:#x}")
        row_start = GLOBAL_DATA_FILE_BASE + FIELD_MATRIX_OFFSET + index * FIELD_STRIDE
        offsets = mz.data[row_start : row_start + OBJECT_KIND_COUNT]
        if len(offsets) != OBJECT_KIND_COUNT:
            raise ValueError(f"selector {index} field row is truncated")
        selectors.append(
            {
                "selector": f"0x{index:02x}",
                "name": name,
                "handler_file_offset": f"0x{handler:06x}",
                "formatter": formatter,
                "object_offsets": {
                    kind: (value if value else None)
                    for kind, value in zip(OBJECT_KIND_NAMES, offsets, strict=True)
                },
            }
        )

    return {
        "format": "big-bug-bang-field-display-audit-v1",
        "scope": (
            "Static diagnostic ownership; this does not prove runtime activation "
            "or gameplay reachability"
        ),
        "input": {
            "executable": os.fspath(executable),
            "sha256": digest,
        },
        "owner": {
            "main_call_site": f"0x{MAIN_CALL_SITE:06x}",
            "entry": f"0x{DISPLAY_OWNER_ENTRY:06x}",
            "directory_list_entry": f"0x{DIRECTORY_LIST_ENTRY:06x}",
            "selected_record_entry": f"0x{RECORD_DISPLAY_ENTRY:06x}",
            "empty_selection_entry": f"0x{EMPTY_SELECTION_ENTRY:06x}",
            "mode_offset": f"gs:0x{MODE_OFFSET:04x}",
            "initial_mode": initial_mode,
            "explicit_mode_accesses": mode_accesses,
            "explicit_mode_write_count": 0,
        },
        "display": {
            "state_label": state_label,
            "field_matrix_file_offset": f"0x{GLOBAL_DATA_FILE_BASE + FIELD_MATRIX_OFFSET:06x}",
            "field_label_table_file_offset": f"0x{GLOBAL_DATA_FILE_BASE + FIELD_LABEL_OFFSET:06x}",
            "dispatch_table_file_offset": f"0x{int(field_table['table']):06x}",
            "selector_count": len(selectors),
            "distinct_handler_count": len({row["handler_file_offset"] for row in selectors}),
            "selectors": selectors,
        },
    }


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("executable", type=Path)
    parser.add_argument("output", type=Path)
    return parser.parse_args()


def main() -> None:
    args = parse_args()
    report = audit(args.executable)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(report, indent=2) + "\n")
    display = report["display"]
    print(
        f"classified {display['selector_count']} field selectors across "
        f"{display['distinct_handler_count']} diagnostic formatters"
    )
    print(args.output)


if __name__ == "__main__":
    main()
