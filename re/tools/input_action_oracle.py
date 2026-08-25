#!/usr/bin/env python3
"""Verify the recovered BLOODPRG input-action table and handler semantics."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import struct
import sys
from pathlib import Path

_HERE = os.path.dirname(os.path.abspath(__file__))
sys.path[:] = [
    path for path in sys.path if os.path.abspath(path or os.curdir) != _HERE
]

from unicorn import Uc
from unicorn.x86_const import UC_X86_REG_SP

sys.path.insert(0, _HERE)
from natural_candidate_oracle import EXE, execute


ROOT = Path(__file__).resolve().parents[2]
DEFAULT_OUTPUT = ROOT / "re/tools/oracle_vectors/input_action_handlers_natural.json"
RETURN_ADDRESS = 0xF400
DATA_SEGMENT = 0x2000
DIRECTORY_SEGMENT = 0x3000
STACK_SEGMENT = 0x9000
CALLER_SP = 0xFF00
STACK_SENTINEL = bytes.fromhex("5aa596698778c33c")

HANDLERS = (
    (0, 0x2140, 77, "input_action_move_previous"),
    (1, 0x218D, 116, "input_action_move_next"),
    (2, 0x2201, 1, "input_action_noop_2"),
    (3, 0x2202, 1, "input_action_noop_3"),
    (4, 0x2203, 6, "input_action_request_shutdown"),
    (5, 0x2209, 1, "input_action_noop_5"),
    (6, 0x2224, 41, "input_action_accept"),
    (7, 0x224D, 101, "input_action_cancel"),
    (8, 0x22D0, 5, "input_action_latch_text_key"),
    (9, 0x220A, 1, "input_action_noop_9"),
    (10, 0x220B, 1, "input_action_noop_10"),
    (11, 0x220C, 1, "input_action_noop_11"),
    (12, 0x220D, 9, "input_action_noop_12"),
    (13, 0x2216, 9, "input_action_noop_13"),
    (14, 0x221F, 5, "input_action_noop_14"),
    (15, 0x22B2, 30, "input_action_toggle_pause"),
)


def word(data: bytes | bytearray, offset: int) -> int:
    return struct.unpack_from("<H", data, offset)[0]


def dword(data: bytes | bytearray, offset: int) -> int:
    return struct.unpack_from("<I", data, offset)[0]


def set_word(data: bytearray, offset: int, value: int) -> None:
    struct.pack_into("<H", data, offset, value & 0xFFFF)


def set_dword(data: bytearray, offset: int, value: int) -> None:
    struct.pack_into("<I", data, offset, value & 0xFFFFFFFF)


def run_handler(
    entry: int,
    data: bytearray,
    raw_low_byte: int = 0,
    extra_memory: list[tuple[int, int, bytes]] | None = None,
    code_handler=None,
) -> tuple[Uc, bytes]:
    initial = {
        "eax": 0xA5A51234,
        "ebx": 0xB6B62345,
        "ecx": 0xC7C73456,
        "edx": 0xD8D84500 | (raw_low_byte & 0xFF),
        "esi": 0xE9E95678,
        "edi": 0xFAFA6789,
        "ebp": 0xABCD789A,
        "sp": CALLER_SP,
        "ds": DATA_SEGMENT,
        "es": DATA_SEGMENT,
        "fs": 0x4000,
        "gs": DATA_SEGMENT,
        "ss": STACK_SEGMENT,
        "flags": 0x0202,
    }
    memory = [
        (0, RETURN_ADDRESS, b"\xCC"),
        (DATA_SEGMENT, 0, bytes(data)),
        (
            STACK_SEGMENT,
            CALLER_SP,
            struct.pack("<H", RETURN_ADDRESS) + STACK_SENTINEL,
        ),
    ]
    if extra_memory is not None:
        memory.extend(extra_memory)
    machine = execute(
        entry,
        RETURN_ADDRESS,
        initial,
        memory,
        code_handler=code_handler,
        instruction_count=1000,
    )
    if machine.reg_read(UC_X86_REG_SP) != CALLER_SP + 2:
        raise AssertionError(f"{entry:#x}: near-return stack mismatch")
    if bytes(
        machine.mem_read(
            STACK_SEGMENT * 16 + CALLER_SP + 2, len(STACK_SENTINEL)
        )
    ) != STACK_SENTINEL:
        raise AssertionError(f"{entry:#x}: stack sentinel changed")
    result = bytes(machine.mem_read(DATA_SEGMENT * 16, 0x10000))
    return machine, result


def inventory() -> dict[str, object]:
    expected_offsets = tuple(entry - 0x0EB0 for _, entry, _, _ in HANDLERS)
    actual_offsets = struct.unpack_from("<16H", EXE, 0x20EE)
    if actual_offsets != expected_offsets:
        raise AssertionError(
            f"input handler table changed: {actual_offsets!r} != {expected_offsets!r}"
        )

    expected_hashes = {
        0x2140: "fc33da9380345cdb12641f81268d3e4c98d44f78d8e94e13ec2e0f7bb1af5d8b",
        0x218D: "33054a8ce9a76051c99544a3028d3aefb393f5848753f12d8b69e7c453548548",
        0x2201: "ae3f4619b0413d70d3004b9131c3752153074e45725be13b9a148978895e359e",
        0x2202: "ae3f4619b0413d70d3004b9131c3752153074e45725be13b9a148978895e359e",
        0x2203: "eedf9d68683b58593f0a258026640f0482661296a9311a1d88da921926562ccb",
        0x2209: "ae3f4619b0413d70d3004b9131c3752153074e45725be13b9a148978895e359e",
        0x220A: "ae3f4619b0413d70d3004b9131c3752153074e45725be13b9a148978895e359e",
        0x220B: "ae3f4619b0413d70d3004b9131c3752153074e45725be13b9a148978895e359e",
        0x220C: "ae3f4619b0413d70d3004b9131c3752153074e45725be13b9a148978895e359e",
        0x220D: "ac56c7e5e73ead184b39d0dfb7ee9bd454f6a0c21529d178f44550866eb38bbf",
        0x2216: "ac56c7e5e73ead184b39d0dfb7ee9bd454f6a0c21529d178f44550866eb38bbf",
        0x221F: "b66e39c8fcae7b58bf2073ec5ee5f739f9291df82306d4a6068f0b1a95c2edba",
        0x2224: "154e446bbe3ec08a22491b2223ebb1eb72dc45490d73b1c29c59b5cc23769ac6",
        0x224D: "4978dfa9eddbcaaca95aab48168cb64904776bf96260fde5bffd2f65bf00d75e",
        0x22B2: "3204f91cc26b18fbce4eed5a97e53e9474e64811520bf139dcaba701a713797c",
        0x22D0: "d025dfd0d91c8157f4e869024a98523b9afaa828e4690b9fdfcab45864a64231",
    }
    for _index, entry, length, _name in HANDLERS:
        actual = hashlib.sha256(EXE[entry : entry + length]).hexdigest()
        if actual != expected_hashes[entry]:
            raise AssertionError(f"{entry:#x}: original handler bytes changed")

    translation = EXE[0x1FEE : 0x20EE]
    mapped = sorted(set(value for value in translation if value < 0x80))
    if mapped != [0, 1, 2, 3, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15]:
        raise AssertionError(f"input translation actions changed: {mapped!r}")
    expected_keys = {
        0x08: 8,
        0x0D: 6,
        0x1B: 7,
        0x20: 7,
        0x50: 15,
        0x70: 15,
        0xBB: 5,
        0xBC: 9,
        0xBD: 10,
        0xBE: 11,
        0xBF: 12,
        0xC0: 13,
        0xC1: 14,
        0xC8: 0,
        0xCB: 3,
        0xCD: 2,
        0xD0: 1,
    }
    for key, action in expected_keys.items():
        if translation[key] != action:
            raise AssertionError(f"key {key:#x}: action changed")

    return {
        "table_file_offset": "0x0020ee",
        "translation_file_offset": "0x001fee",
        "translation_table": list(translation),
        "mapped_action_indices": mapped,
        "unmapped_handler_indices": [4],
        "handlers": [
            {
                "index": index,
                "entry": f"0x{entry:06x}",
                "near_offset": f"0x{entry - 0x0EB0:04x}",
                "function": name,
            }
            for index, entry, _length, name in HANDLERS
        ],
    }


def move_previous_vectors() -> list[dict[str, object]]:
    vectors = []
    cases = (
        ("directory_scroll", 3, 0xFFFF, 5, 5, 0, 4, 4),
        ("directory_within_window", 3, 0xFFFF, 6, 5, 0, 5, 5),
        ("directory_at_first", 3, 0xFFFF, 0, 0, 0, 0, 0),
        ("committed_directory_ignored", 3, 0x003C, 5, 5, 0, 5, 5),
        ("inactive", 0, 0, 5, 5, 0, 5, 5),
    )
    for name, flags, committed, selected, first, save, want_selected, want_first in cases:
        data = bytearray(0x10000)
        data[0x67A6] = flags
        set_word(data, 0x679E, committed)
        set_word(data, 0x67A2, selected)
        set_word(data, 0x67A0, first)
        data[0x2736] = save
        _, result = run_handler(0x2140, data)
        if word(result, 0x67A2) != want_selected or word(result, 0x67A0) != want_first:
            raise AssertionError(f"0x2140 {name}: directory selection mismatch")
        vectors.append(
            {
                "name": name,
                "mode": "selection" if flags & 3 else "inactive",
                "source": "profile" if flags & 1 else "builtin",
                "committed": committed != 0xFFFF,
                "selected_before": selected,
                "selected": word(result, 0x67A2),
                "first_visible_before": first,
                "first_visible": word(result, 0x67A0),
            }
        )

    data = bytearray(0x10000)
    data[0x2736] = 1
    set_word(data, 0x2732, 2)
    set_word(data, 0x2734, 0x3040)
    source = bytes(range(0x40, 0x50))
    data[0x3020:0x3030] = source
    _, result = run_handler(0x2140, data)
    if word(result, 0x2732) != 1 or word(result, 0x2734) != 0x3020:
        raise AssertionError("0x2140 save-slot previous index/pointer mismatch")
    if result[0x273B:0x274B] != source:
        raise AssertionError("0x2140 save-slot previous copy mismatch")
    vectors.append(
        {
            "name": "save_slot_previous",
            "mode": "save_menu",
            "slot_before": 2,
            "slot": word(result, 0x2732),
            "active_name_offset": word(result, 0x2734),
            "edit_name_hex": result[0x273B:0x274B].hex(),
            "edit_name_bytes": list(result[0x273B:0x274B]),
        }
    )
    return vectors


def move_next_vectors() -> list[dict[str, object]]:
    vectors = []
    for name, flags, selected, first, entry_kind, expected_selected, expected_first in (
        ("dynamic_directory", 1, 2, 0, 1, 3, 0),
        ("fixed_directory_scroll", 2, 14, 0, 1, 15, 1),
        ("directory_terminator", 1, 2, 0, 0, 2, 0),
    ):
        data = bytearray(0x10000)
        directory = bytearray(0x10000)
        data[0x67A6] = flags
        set_word(data, 0x679E, 0xFFFF)
        set_word(data, 0x67A2, selected)
        set_word(data, 0x67A0, first)
        next_index = (selected + 1) & 0xFF
        if flags & 1:
            set_word(data, 0x672C, 0x1000)
            set_word(data, 0x672E, DIRECTORY_SEGMENT)
            set_word(directory, 0x1000 + next_index * 20 + 18, entry_kind)
            extra = [(DIRECTORY_SEGMENT, 0, bytes(directory))]
        else:
            set_word(data, 0x6F80 + next_index * 20 + 18, entry_kind)
            extra = None
        _, result = run_handler(0x218D, data, extra_memory=extra)
        if word(result, 0x67A2) != expected_selected:
            raise AssertionError(f"0x218d {name}: selected index mismatch")
        if word(result, 0x67A0) != expected_first:
            raise AssertionError(f"0x218d {name}: first visible mismatch")
        vectors.append(
            {
                "name": name,
                "mode": "selection",
                "source": "profile" if flags & 1 else "builtin",
                "committed": False,
                "selected_before": selected,
                "selected": word(result, 0x67A2),
                "first_visible_before": first,
                "first_visible": word(result, 0x67A0),
                "next_entry_kind": entry_kind,
            }
        )

    data = bytearray(0x10000)
    data[0x2736] = 1
    set_word(data, 0x2732, 7)
    set_word(data, 0x2734, 0x3020)
    source = bytes(range(0x70, 0x80))
    data[0x3040:0x3050] = source
    _, result = run_handler(0x218D, data)
    if word(result, 0x2732) != 8 or word(result, 0x2734) != 0x3040:
        raise AssertionError("0x218d save-slot next index/pointer mismatch")
    if result[0x273B:0x274B] != source:
        raise AssertionError("0x218d save-slot next copy mismatch")
    vectors.append(
        {
            "name": "save_slot_next",
            "mode": "save_menu",
            "slot_before": 7,
            "slot": word(result, 0x2732),
            "active_name_offset": word(result, 0x2734),
            "edit_name_hex": result[0x273B:0x274B].hex(),
            "edit_name_bytes": list(result[0x273B:0x274B]),
        }
    )
    return vectors


def simple_handler_vectors() -> list[dict[str, object]]:
    vectors = []
    for index, entry, _length, name in HANDLERS:
        if index not in {2, 3, 5, 9, 10, 11, 12, 13, 14}:
            continue
        data = bytearray((value * 37 + 11) & 0xFF for value in range(0x10000))
        _machine, result = run_handler(entry, data, raw_low_byte=0xA5)
        if result != bytes(data):
            raise AssertionError(f"{entry:#x}: semantic no-op changed memory")
        vectors.append({"name": name, "action_index": index, "memory": "unchanged"})

    data = bytearray(0x10000)
    _, result = run_handler(0x2203, data)
    if result[0x0B13] != 1:
        raise AssertionError("0x2203: shutdown latch not set")
    vectors.append(
        {
            "name": "input_action_request_shutdown",
            "action_index": 4,
            "shutdown_latch": result[0x0B13],
        }
    )

    data = bytearray(0x10000)
    _, result = run_handler(0x22D0, data, raw_low_byte=0x7A)
    if result[0x0B15] != 0x7A:
        raise AssertionError("0x22d0: raw key not latched")
    vectors.append(
        {
            "name": "input_action_latch_text_key",
            "action_index": 8,
            "latched_key": result[0x0B15],
        }
    )
    return vectors


def accept_vectors() -> list[dict[str, object]]:
    vectors = []
    for name, flags, selected, kind, expected_offset in (
        ("latch_only", 0, 3, 1, 0x7777),
        ("active_record", 1, 0x0103, 1, 60),
        ("inactive_record", 1, 3, 2, 0x7777),
    ):
        data = bytearray(0x10000)
        directory = bytearray(0x10000)
        data[0x67A6] = flags
        set_word(data, 0x67A2, selected)
        set_word(data, 0x679E, 0x7777)
        set_word(data, 0x672E, DIRECTORY_SEGMENT)
        set_word(directory, ((selected & 0xFF) * 20 + 18) & 0xFFFF, kind)
        _, result = run_handler(
            0x2224,
            data,
            raw_low_byte=0x0D,
            extra_memory=[(DIRECTORY_SEGMENT, 0, bytes(directory))],
        )
        if result[0x0B15] != 0x0D:
            raise AssertionError(f"0x2224 {name}: Enter not latched")
        if word(result, 0x679E) != expected_offset:
            raise AssertionError(f"0x2224 {name}: record offset mismatch")
        vectors.append(
            {
                "name": name,
                "profile_selection": bool(flags & 1),
                "selected_word": selected,
                "selected_index": selected & 0xFF,
                "record_kind": kind,
                "committed_offset": word(result, 0x679E),
                "latched_key": result[0x0B15],
            }
        )
    return vectors


def cancel_vectors() -> list[dict[str, object]]:
    vectors = []
    for name, gate, dialogue, ship, line, cancels in (
        ("presentation_gate_clear", 0, 0, 0, 4, False),
        ("blocked_line_window", 1, 0, 0, 8, False),
        ("cancel_dialogue_line", 1, 0, 0, 4, True),
        ("cancel_other_line", 1, 0, 0, 3, True),
    ):
        data = bytearray(0x10000)
        data[0x0ADF] = 1
        data[0x0B15] = 0
        data[0x1FB2] = gate
        data[0x2534] = dialogue
        set_word(data, 0x24F3, ship)
        set_word(data, 0x6788, line)
        set_dword(data, 0x0D78, 0x12345678)
        set_dword(data, 0x0D7C, 0x89ABCDEF)
        data[0x5251:0x53D1] = bytes([0xA5]) * 384
        calls = []

        def capture(_machine: Uc, address: int, _size: int) -> None:
            if address == 0xA157:
                calls.append("list_d8c_init")

        _, result = run_handler(
            0x224D,
            data,
            raw_low_byte=0x1B,
            extra_memory=[(0, 0xA157, b"\xCB")],
            code_handler=capture,
        )
        if result[0x0ADF] != 0:
            raise AssertionError(f"0x224d {name}: pause not cleared")
        if cancels:
            if calls != ["list_d8c_init"]:
                raise AssertionError(f"0x224d {name}: queue reset call mismatch")
            if dword(result, 0x0D84) != 0x12345678:
                raise AssertionError(f"0x224d {name}: source offset not rewound")
            if dword(result, 0x0D88) != 0x89ABCDEF:
                raise AssertionError(f"0x224d {name}: source extent not rewound")
            if result[0x5251:0x53D1] != bytes(384):
                raise AssertionError(f"0x224d {name}: palette prefix not cleared")
            if result[0x5B55] != 1 or result[0x0B15] != 0:
                raise AssertionError(f"0x224d {name}: cancel latches mismatch")
            expected_dialogue = int(line == 4)
            if result[0x2534] != expected_dialogue:
                raise AssertionError(f"0x224d {name}: dialogue-ready mismatch")
        else:
            if calls or result[0x0B15] != 0x1B:
                raise AssertionError(f"0x224d {name}: Escape forwarding mismatch")
        vectors.append(
            {
                "name": name,
                "presentation_active": bool(gate & 1),
                "dialogue_ready_before": bool(dialogue & 1),
                "ship_active": bool(ship & 4),
                "active_line": line,
                "cancelled": cancels,
                "latched_key": result[0x0B15],
                "dialogue_ready": result[0x2534],
                "calls": calls,
            }
        )
    return vectors


def pause_vectors() -> list[dict[str, object]]:
    vectors = []
    for name, save_active, initial_pause, expected_pause in (
        ("pause", 0, 0, 1),
        ("unpause", 0, 1, 0),
        ("normalize_set_bits", 0, 3, 0),
        ("save_ui_blocks_toggle", 1, 1, 1),
    ):
        data = bytearray(0x10000)
        data[0x2736] = save_active
        data[0x0ADF] = initial_pause
        _, result = run_handler(0x22B2, data, raw_low_byte=0x70)
        if result[0x0ADF] != expected_pause or result[0x0B15] != 0x70:
            raise AssertionError(f"0x22b2 {name}: pause/key mismatch")
        vectors.append(
            {
                "name": name,
                "save_active": save_active,
                "pause_before": initial_pause,
                "pause_after": result[0x0ADF],
                "latched_key": result[0x0B15],
            }
        )
    return vectors


def report() -> dict[str, object]:
    return {
        "artifact_sha256": hashlib.sha256(EXE).hexdigest(),
        "inventory": inventory(),
        "vectors": {
            "move_previous": move_previous_vectors(),
            "move_next": move_next_vectors(),
            "simple_handlers": simple_handler_vectors(),
            "accept": accept_vectors(),
            "cancel": cancel_vectors(),
            "toggle_pause": pause_vectors(),
        },
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, default=DEFAULT_OUTPUT)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    encoded = json.dumps(report(), indent=2, sort_keys=True) + "\n"
    if args.check:
        if not args.output.is_file() or args.output.read_text(encoding="ascii") != encoded:
            raise SystemExit(f"{args.output}: stale or missing")
        print(f"PASS {args.output}")
        return 0
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(encoded, encoding="ascii")
    print(f"wrote {args.output}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
