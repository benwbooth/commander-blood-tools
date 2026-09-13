#!/usr/bin/env python3
"""Verify BBB's unchanged keyboard dispatcher and active input handlers."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import struct
import sys
from typing import Any

_HERE = os.path.dirname(os.path.abspath(__file__))
sys.path[:] = [path for path in sys.path if os.path.abspath(path or os.curdir) != _HERE]

from unicorn import Uc, UC_ARCH_X86, UC_HOOK_CODE, UC_MODE_16  # noqa: E402
from unicorn.x86_const import (  # noqa: E402
    UC_X86_REG_CS,
    UC_X86_REG_DS,
    UC_X86_REG_EAX,
    UC_X86_REG_EBP,
    UC_X86_REG_EBX,
    UC_X86_REG_ECX,
    UC_X86_REG_EDI,
    UC_X86_REG_EDX,
    UC_X86_REG_EFLAGS,
    UC_X86_REG_ES,
    UC_X86_REG_ESI,
    UC_X86_REG_FS,
    UC_X86_REG_GS,
    UC_X86_REG_SP,
    UC_X86_REG_SS,
)


ROOT = Path(__file__).resolve().parents[2]
DEFAULT_EXECUTABLE = ROOT / "output/big-bug-bang/disc/BLOOD2PG.EXE"
DEFAULT_OUTPUT = ROOT / "re/tools/oracle_vectors/big_bug_bang_input_handlers.json"
EXECUTABLE_SHA256 = "4b65ffca3e113a1826371e3436177861640a1b7aae24caafebb4c2f7aa467834"

CODE_SEGMENT = 0x00F7
GLOBALS_ADDRESS = 0x30000
GLOBALS_SEGMENT = GLOBALS_ADDRESS // 16
DIRECTORY_ADDRESS = 0x50000
DIRECTORY_SEGMENT = DIRECTORY_ADDRESS // 16
STACK_SEGMENT = 0x9000
CALLER_SP = 0xFE00
NEAR_RETURN_OFFSET = 0x7000
FAR_RETURN_SEGMENT = 0x6000
FAR_RETURN_OFFSET = 0x1000
FAR_RETURN_ADDRESS = FAR_RETURN_SEGMENT * 16 + FAR_RETURN_OFFSET
POLL_ADDRESS = 0x1E6 * 16 + 0x039D
QUEUE_RESET_ADDRESS = 0x0ACB * 16 + 0x0A91
STACK_SENTINEL = bytes.fromhex("5aa596698778c33c")

TEXT_LATCH = 0x0D1F
PAUSE = 0x0CE8
PRESENTATION_GATE = 0x2200
DIALOGUE_READY = 0x2786
SHIP_FLAGS = 0x2745
ACTIVE_LINE = 0x6B5A
REWIND_POSITION = 0x0FC6
REWIND_REMAINING = 0x0FCA
READ_POSITION = 0x0FD2
READ_REMAINING = 0x0FD6
SCENE_PALETTE = 0x5621
SCENE_PALETTE_PREFIX_SIZE = 384
PALETTE_DIRTY = 0x5F25
PRESENTATION_COMPLETE = 0x6B7E
SELECTION_MODE = 0x6B7C
COMMITTED_ROW = 0x6B74
SELECTED_ROW = 0x6B78
FIRST_VISIBLE_ROW = 0x6B76
PROFILE_DIRECTORY_OFFSET = 0x6AF0
PROFILE_DIRECTORY_SEGMENT = 0x6AF2
BUILTIN_DIRECTORY = 0x785E
SAVE_ACTIVE = 0x29C4
SAVE_SLOT = 0x29C0
SAVE_NAME_POINTER = 0x29C2
SAVE_EDIT_NAME = 0x29C9

ROUTINES = (
    (0x23A2, 0x23D4, "input_action_dispatch"),
    (0x23D4, 0x2421, "input_action_move_previous"),
    (0x2421, 0x2495, "input_action_move_next"),
    (0x2514, 0x253D, "input_action_accept"),
    (0x253D, 0x25A7, "input_action_cancel"),
    (0x25A7, 0x25C5, "input_action_toggle_pause"),
    (0x25C5, 0x25CA, "input_action_latch_text_key"),
)
ROUTINE_HASHES = {
    0x23A2: "da0918fb023211af394db5c330c039bca58345340c5f6d9768a8d62b15d61f47",
    0x23D4: "6ea909c1b3ef0c591f40f3e8467253de11a400ca359133ce43cb5866cfdfb843",
    0x2421: "1d55ab10b3f4c029ae32c171b207237505d199a0bb1de7a0088de12645c554e7",
    0x2514: "9853d5cfc47de9386bb40378a54e4ad5cc5f8515e000aac98aef370e9d9ab832",
    0x253D: "0d1950fcd0f8ae23c706094220450e6c1ec673235b7bc9040cf890fdcb22b437",
    0x25A7: "217993305705b5e7de77e45229c5c761aa4dc0563d4243bcdaa18d6d2c9f42f0",
    0x25C5: "25e0a587d11b11e771b3bae454e2e8b7e6b3c8bba8f5452e86d3bbf519477e2b",
}
HANDLERS = (
    (0, 0x23D4, "input_action_move_previous"),
    (1, 0x2421, "input_action_move_next"),
    (2, 0x2495, "diagnostic_field_next"),
    (3, 0x24A3, "diagnostic_field_previous"),
    (4, 0x24B1, "input_action_noop_escape"),
    (5, 0x24B2, "input_action_noop_f1"),
    (6, 0x2514, "input_action_accept"),
    (7, 0x253D, "input_action_cancel"),
    (8, 0x25C5, "input_action_latch_text_key"),
    (9, 0x24B3, "input_action_noop_f2"),
    (10, 0x24B4, "input_action_noop_f3"),
    (11, 0x24B5, "input_action_noop_f4"),
    (12, 0x24B6, "input_action_noop_f5"),
    (13, 0x24BF, "input_action_noop_f6"),
    (14, 0x24C8, "input_action_abort_conversation"),
    (15, 0x25A7, "input_action_toggle_pause"),
)

GENERAL_REGISTERS = {
    "eax": UC_X86_REG_EAX,
    "ebx": UC_X86_REG_EBX,
    "ecx": UC_X86_REG_ECX,
    "edx": UC_X86_REG_EDX,
    "esi": UC_X86_REG_ESI,
    "edi": UC_X86_REG_EDI,
    "ebp": UC_X86_REG_EBP,
}
SEGMENT_REGISTERS = {
    "ds": UC_X86_REG_DS,
    "es": UC_X86_REG_ES,
    "fs": UC_X86_REG_FS,
    "gs": UC_X86_REG_GS,
    "ss": UC_X86_REG_SS,
}
INITIAL_REGISTERS = {
    "eax": 0xA5A51234,
    "ebx": 0xB6B62345,
    "ecx": 0xC7C73456,
    "edx": 0xD8D84500,
    "esi": 0xE9E95678,
    "edi": 0xFAFA6789,
    "ebp": 0xABCD789A,
    "ds": GLOBALS_SEGMENT,
    "es": GLOBALS_SEGMENT,
    "fs": 0x4000,
    "gs": GLOBALS_SEGMENT,
    "ss": STACK_SEGMENT,
}


def word(data: bytes | bytearray, offset: int) -> int:
    return struct.unpack_from("<H", data, offset)[0]


def dword(data: bytes | bytearray, offset: int) -> int:
    return struct.unpack_from("<I", data, offset)[0]


def set_word(data: bytearray, offset: int, value: int) -> None:
    struct.pack_into("<H", data, offset, value & 0xFFFF)


def set_dword(data: bytearray, offset: int, value: int) -> None:
    struct.pack_into("<I", data, offset, value & 0xFFFFFFFF)


def register_snapshot(machine: Uc) -> dict[str, str]:
    result = {
        name: f"0x{machine.reg_read(register):08x}"
        for name, register in GENERAL_REGISTERS.items()
    }
    result.update(
        {
            name: f"0x{machine.reg_read(register):04x}"
            for name, register in SEGMENT_REGISTERS.items()
        }
    )
    result["flags"] = f"0x{machine.reg_read(UC_X86_REG_EFLAGS):08x}"
    return result


def initial_registers(raw_low_byte: int = 0) -> dict[str, int]:
    result = dict(INITIAL_REGISTERS)
    result["edx"] = (result["edx"] & ~0xFF) | (raw_low_byte & 0xFF)
    return result


def make_machine(
    executable: bytes,
    data: bytearray,
    stack: bytes,
    registers: dict[str, int],
    patches: list[tuple[int, bytes]] | None = None,
    extra_memory: list[tuple[int, bytes]] | None = None,
) -> tuple[Uc, bytes]:
    machine = Uc(UC_ARCH_X86, UC_MODE_16)
    machine.mem_map(0, 0x100000)
    machine.mem_write(0, executable)
    expected_image = bytearray(executable)
    for address, patch in patches or []:
        machine.mem_write(address, patch)
        if address < len(expected_image):
            end = min(address + len(patch), len(expected_image))
            expected_image[address:end] = patch[: end - address]
    machine.mem_write(GLOBALS_ADDRESS, bytes(data))
    for address, content in extra_memory or []:
        machine.mem_write(address, content)
    machine.mem_write(STACK_SEGMENT * 16 + CALLER_SP, stack + STACK_SENTINEL)
    for name, register in GENERAL_REGISTERS.items():
        machine.reg_write(register, registers[name])
    for name, register in SEGMENT_REGISTERS.items():
        machine.reg_write(register, registers[name])
    machine.reg_write(UC_X86_REG_SP, CALLER_SP)
    machine.reg_write(UC_X86_REG_EFLAGS, 0x0202)
    return machine, bytes(expected_image)


def execute_case(
    executable: bytes,
    entry: int,
    data: bytearray,
    expected: bytearray,
    *,
    raw_low_byte: int = 0,
    far_return: bool = False,
    code_segment: int = 0,
    patches: list[tuple[int, bytes]] | None = None,
    extra_memory: list[tuple[int, bytes]] | None = None,
    allowed_ranges: tuple[tuple[int, int], ...] | None = None,
    preserved: tuple[str, ...] = (),
    capture: dict[int, str] | None = None,
) -> tuple[Uc, list[int], list[str]]:
    registers = initial_registers(raw_low_byte)
    stack = (
        struct.pack("<HH", FAR_RETURN_OFFSET, FAR_RETURN_SEGMENT)
        if far_return
        else struct.pack("<H", NEAR_RETURN_OFFSET)
    )
    machine, expected_image = make_machine(
        executable,
        data,
        stack,
        registers,
        patches=patches,
        extra_memory=extra_memory,
    )
    machine.reg_write(UC_X86_REG_CS, code_segment)
    return_address = FAR_RETURN_ADDRESS if far_return else NEAR_RETURN_OFFSET
    ranges = allowed_ranges or (
        (entry, next(end for start, end, _ in ROUTINES if start == entry)),
    )
    trace: list[int] = []
    calls: list[str] = []

    def instruction(current: Uc, address: int, size: int, _context: object) -> None:
        if address == return_address:
            current.emu_stop()
            return
        if capture is not None and address in capture:
            calls.append(capture[address])
        if POLL_ADDRESS <= address < POLL_ADDRESS + 4 or address == QUEUE_RESET_ADDRESS:
            return
        if not any(start <= address < address + size <= end for start, end in ranges):
            raise AssertionError(f"{entry:#x}: unexpected instruction at {address:#x}")
        trace.append(address)

    machine.hook_add(UC_HOOK_CODE, instruction)
    machine.emu_start(entry, 0, count=2000)
    return_size = 4 if far_return else 2
    if machine.reg_read(UC_X86_REG_SP) != CALLER_SP + return_size:
        raise AssertionError(
            f"{entry:#x}: return stack mismatch: {machine.reg_read(UC_X86_REG_SP):#x}"
        )
    sentinel_address = STACK_SEGMENT * 16 + CALLER_SP + return_size
    if bytes(machine.mem_read(sentinel_address, len(STACK_SENTINEL))) != STACK_SENTINEL:
        raise AssertionError(f"{entry:#x}: stack sentinel changed")
    if bytes(machine.mem_read(0, len(executable))) != expected_image:
        raise AssertionError(f"{entry:#x}: executable memory changed")
    after = bytes(machine.mem_read(GLOBALS_ADDRESS, 0x10000))
    if after != bytes(expected):
        mismatches = [
            offset
            for offset, (actual, wanted) in enumerate(zip(after, expected, strict=True))
            if actual != wanted
        ]
        raise AssertionError(
            f"{entry:#x}: unexpected global writes at "
            f"{[hex(offset) for offset in mismatches[:16]]}"
        )
    for address, content in extra_memory or []:
        if bytes(machine.mem_read(address, len(content))) != content:
            raise AssertionError(
                f"{entry:#x}: auxiliary memory changed at {address:#x}"
            )
    for name in preserved:
        register = GENERAL_REGISTERS.get(name, SEGMENT_REGISTERS.get(name))
        if register is None:
            raise AssertionError(f"unknown preserved register {name}")
        if machine.reg_read(register) != registers[name]:
            raise AssertionError(f"{entry:#x}: {name} was not preserved")
    return machine, trace, calls


def fixture_registers(machine: Uc) -> dict[str, str]:
    return register_snapshot(machine)


def inventory(executable: bytes) -> dict[str, Any]:
    for start, end, _name in ROUTINES:
        digest = hashlib.sha256(executable[start:end]).hexdigest()
        if digest != ROUTINE_HASHES[start]:
            raise AssertionError(f"{start:#x}: original routine bytes changed")

    actual_handlers = tuple(
        offset + CODE_SEGMENT * 16
        for offset in struct.unpack_from("<16H", executable, 0x2382)
    )
    expected_handlers = tuple(entry for _index, entry, _name in HANDLERS)
    if actual_handlers != expected_handlers:
        raise AssertionError(f"input handler table changed: {actual_handlers!r}")

    translation = executable[0x2281:0x2381]
    mapped = sorted(set(value for value in translation if value < 0x80))
    if mapped != list(range(16)):
        raise AssertionError(f"input translation actions changed: {mapped!r}")
    expected_keys = {
        0x08: 8,
        0x0D: 6,
        0x1B: 4,
        0x20: 7,
        0x50: 15,
        0x70: 15,
        0x7F: 8,
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
        "dispatcher_code_segment": f"0x{CODE_SEGMENT:04x}",
        "handler_table_file_offset": "0x002382",
        "translation_file_offset": "0x002281",
        "translation_table": list(translation),
        "mapped_action_indices": mapped,
        "routines": [
            {
                "entry": f"0x{start:06x}",
                "length": end - start,
                "function": name,
                "sha256": ROUTINE_HASHES[start],
            }
            for start, end, name in ROUTINES
        ],
        "handlers": [
            {
                "index": index,
                "entry": f"0x{entry:06x}",
                "near_offset": f"0x{entry - CODE_SEGMENT * 16:04x}",
                "function": name,
            }
            for index, entry, name in HANDLERS
        ],
    }


def dispatch_vectors(executable: bytes) -> list[dict[str, Any]]:
    translation = executable[0x2281:0x2381]
    vectors = []
    cases = (
        ("no_key", 0x0000, None, None),
        ("unmapped_control", 0x0001, 0x01, None),
        ("latch_printable", 0x0071, 0x71, 8),
        ("move_previous_extended", 0x4800, 0xC8, 0),
        ("move_next_extended", 0x5000, 0xD0, 1),
        ("accept_enter", 0x000D, 0x0D, 6),
        ("ignore_escape", 0x001B, 0x1B, 4),
        ("cancel_space", 0x0020, 0x20, 7),
        ("toggle_pause", 0x0070, 0x70, 15),
    )
    allowed = (
        (0x23A2, 0x23D4),
        (0x23D4, 0x2495),
        (0x24B1, 0x24B2),
        (0x2514, 0x25CA),
    )
    preserved = tuple(GENERAL_REGISTERS) + tuple(SEGMENT_REGISTERS)
    for name, poll_word, translated_code, expected_action in cases:
        data = bytearray(0x10000)
        data[TEXT_LATCH] = 0xA5
        data[PAUSE] = 0
        expected = data[:]
        expected[TEXT_LATCH] = 0
        if name == "latch_printable":
            expected[TEXT_LATCH] = poll_word & 0xFF
        elif name == "accept_enter":
            expected[TEXT_LATCH] = poll_word & 0xFF
        elif name == "cancel_space":
            expected[PAUSE] = 0
            expected[TEXT_LATCH] = poll_word & 0xFF
        elif name == "toggle_pause":
            expected[PAUSE] = 1
            expected[TEXT_LATCH] = poll_word & 0xFF

        poll_stub = b"\xb8" + struct.pack("<H", poll_word) + b"\xcb"
        machine, trace, calls = execute_case(
            executable,
            0x23A2,
            data,
            expected,
            far_return=True,
            code_segment=CODE_SEGMENT,
            patches=[(POLL_ADDRESS, poll_stub)],
            allowed_ranges=allowed,
            preserved=preserved,
        )
        if translated_code is None:
            if poll_word != 0 or 0x23B3 in trace:
                raise AssertionError(f"dispatcher {name}: zero-key path mismatch")
            action = None
        else:
            action = translation[translated_code]
            if action >= 0x80:
                action = None
            if action != expected_action:
                raise AssertionError(f"dispatcher {name}: action mismatch")
        handler_entry = None if action is None else HANDLERS[action][1]
        if handler_entry is not None and handler_entry not in trace:
            raise AssertionError(f"dispatcher {name}: handler was not entered")
        if calls:
            raise AssertionError(f"dispatcher {name}: unexpected external calls")
        vectors.append(
            {
                "name": name,
                "bios_key_word": poll_word,
                "translated_code": translated_code,
                "action_index": action,
                "handler_entry": None
                if handler_entry is None
                else f"0x{handler_entry:06x}",
                "text_before": 0xA5,
                "text_after": expected[TEXT_LATCH],
                "pause_before": 0,
                "pause_after": expected[PAUSE],
                "registers_after": fixture_registers(machine),
            }
        )
    return vectors


def move_previous_vectors(executable: bytes) -> list[dict[str, Any]]:
    vectors = []
    cases = (
        ("selection_scroll", 3, 0xFFFF, 5, 5, 0, 4, 4),
        ("selection_within_window", 3, 0xFFFF, 6, 5, 0, 5, 5),
        ("selection_at_first", 3, 0xFFFF, 0, 0, 0, 0, 0),
        ("committed_selection_ignored", 3, 0x003C, 5, 5, 0, 5, 5),
        ("inactive", 0, 0, 5, 5, 0, 5, 5),
        ("save_slot_previous", 0, 0, 0, 0, 1, 0, 0),
        ("save_slot_at_first", 0, 0, 0, 0, 1, 0, 0),
    )
    for (
        name,
        mode,
        committed,
        selected,
        first,
        save,
        want_selected,
        want_first,
    ) in cases:
        data = bytearray(0x10000)
        data[SELECTION_MODE] = mode
        set_word(data, COMMITTED_ROW, committed)
        set_word(data, SELECTED_ROW, selected)
        set_word(data, FIRST_VISIBLE_ROW, first)
        data[SAVE_ACTIVE] = save
        slot_before = None
        slot_after = None
        edit_name = None
        expected = data[:]
        if name == "save_slot_previous":
            slot_before = 2
            slot_after = 1
            set_word(data, SAVE_SLOT, slot_before)
            set_word(data, SAVE_NAME_POINTER, 0x3040)
            source = bytes(range(0x40, 0x50))
            data[0x3020:0x3030] = source
            expected = data[:]
            set_word(expected, SAVE_SLOT, slot_after)
            set_word(expected, SAVE_NAME_POINTER, 0x3020)
            expected[SAVE_EDIT_NAME : SAVE_EDIT_NAME + 16] = source
            edit_name = list(source)
        elif name == "save_slot_at_first":
            slot_before = slot_after = 0
            set_word(data, SAVE_SLOT, 0)
            set_word(data, SAVE_NAME_POINTER, 0x3000)
            expected = data[:]
            edit_name = list(expected[SAVE_EDIT_NAME : SAVE_EDIT_NAME + 16])
        else:
            set_word(expected, SELECTED_ROW, want_selected)
            set_word(expected, FIRST_VISIBLE_ROW, want_first)

        machine, _trace, calls = execute_case(
            executable,
            0x23D4,
            data,
            expected,
            preserved=("ebx", "ecx", "edx", "ebp", "ds", "es", "fs", "gs", "ss"),
        )
        if calls:
            raise AssertionError(f"previous {name}: unexpected external calls")
        vectors.append(
            {
                "name": name,
                "mode": "selection"
                if mode & 3
                else ("save_menu" if save & 1 else "inactive"),
                "source": "profile" if mode & 1 else ("builtin" if mode & 2 else None),
                "committed": committed != 0xFFFF,
                "selected_before": selected if mode & 3 else None,
                "selected": word(expected, SELECTED_ROW) if mode & 3 else None,
                "first_visible_before": first if mode & 3 else None,
                "first_visible": word(expected, FIRST_VISIBLE_ROW)
                if mode & 3
                else None,
                "slot_before": slot_before,
                "slot": slot_after,
                "edit_name_bytes": edit_name,
                "registers_after": fixture_registers(machine),
            }
        )
    return vectors


def move_next_vectors(executable: bytes) -> list[dict[str, Any]]:
    vectors = []
    cases = (
        ("profile_directory", 1, 0xFFFF, 2, 0, 1, 0, 3, 0),
        ("builtin_directory_scroll", 2, 0xFFFF, 14, 0, 1, 0, 15, 1),
        ("directory_terminator", 1, 0xFFFF, 2, 0, 0, 0, 2, 0),
        ("committed_selection_ignored", 1, 60, 2, 0, 1, 0, 2, 0),
        ("inactive", 0, 0, 2, 0, 1, 0, 2, 0),
        ("save_slot_next", 0, 0, 0, 0, 0, 1, 0, 0),
        ("save_slot_at_last", 0, 0, 0, 0, 0, 1, 0, 0),
    )
    for (
        name,
        mode,
        committed,
        selected,
        first,
        kind,
        save_slot,
        want_selected,
        want_first,
    ) in cases:
        data = bytearray(0x10000)
        directory = bytearray(0x10000)
        data[SELECTION_MODE] = mode
        set_word(data, COMMITTED_ROW, committed)
        set_word(data, SELECTED_ROW, selected)
        set_word(data, FIRST_VISIBLE_ROW, first)
        next_index = (selected + 1) & 0xFF
        extra: list[tuple[int, bytes]] = []
        next_kind = None
        if mode & 3 and committed == 0xFFFF:
            next_kind = kind
            if mode & 1:
                set_word(data, PROFILE_DIRECTORY_OFFSET, 0x1000)
                set_word(data, PROFILE_DIRECTORY_SEGMENT, DIRECTORY_SEGMENT)
                set_word(directory, 0x1000 + next_index * 20 + 18, kind)
                extra.append((DIRECTORY_ADDRESS, bytes(directory)))
            else:
                set_word(data, BUILTIN_DIRECTORY + next_index * 20 + 18, kind)
        data[SAVE_ACTIVE] = 1 if save_slot else 0
        slot_before = None
        slot_after = None
        edit_name = None
        expected = data[:]
        if name == "save_slot_next":
            slot_before = 7
            slot_after = 8
            set_word(data, SAVE_SLOT, slot_before)
            set_word(data, SAVE_NAME_POINTER, 0x3020)
            source = bytes(range(0x70, 0x80))
            data[0x3040:0x3050] = source
            expected = data[:]
            set_word(expected, SAVE_SLOT, slot_after)
            set_word(expected, SAVE_NAME_POINTER, 0x3040)
            expected[SAVE_EDIT_NAME : SAVE_EDIT_NAME + 16] = source
            edit_name = list(source)
        elif name == "save_slot_at_last":
            slot_before = slot_after = 8
            set_word(data, SAVE_SLOT, 8)
            set_word(data, SAVE_NAME_POINTER, 0x3040)
            expected = data[:]
            edit_name = list(expected[SAVE_EDIT_NAME : SAVE_EDIT_NAME + 16])
        else:
            set_word(expected, SELECTED_ROW, want_selected)
            set_word(expected, FIRST_VISIBLE_ROW, want_first)

        machine, _trace, calls = execute_case(
            executable,
            0x2421,
            data,
            expected,
            extra_memory=extra,
            preserved=("ecx", "esi", "ebp", "ds", "es", "fs", "gs", "ss"),
        )
        if calls:
            raise AssertionError(f"next {name}: unexpected external calls")
        vectors.append(
            {
                "name": name,
                "mode": "selection"
                if mode & 3
                else ("save_menu" if save_slot else "inactive"),
                "source": "profile" if mode & 1 else ("builtin" if mode & 2 else None),
                "committed": committed != 0xFFFF,
                "selected_before": selected if mode & 3 else None,
                "selected": word(expected, SELECTED_ROW) if mode & 3 else None,
                "first_visible_before": first if mode & 3 else None,
                "first_visible": word(expected, FIRST_VISIBLE_ROW)
                if mode & 3
                else None,
                "next_entry_kind": next_kind,
                "slot_before": slot_before,
                "slot": slot_after,
                "edit_name_bytes": edit_name,
                "registers_after": fixture_registers(machine),
            }
        )
    return vectors


def accept_vectors(executable: bytes) -> list[dict[str, Any]]:
    vectors = []
    cases = (
        ("inactive_latch_only", 0, 3, 1, 0x7777),
        ("builtin_latch_only", 2, 3, 1, 0x7777),
        ("profile_object", 1, 3, 1, 60),
        ("profile_sentinel", 1, 3, 0, 0x7777),
    )
    for name, mode, selected, kind, expected_commit in cases:
        data = bytearray(0x10000)
        directory = bytearray(0x10000)
        data[SELECTION_MODE] = mode
        set_word(data, SELECTED_ROW, selected)
        set_word(data, COMMITTED_ROW, 0x7777)
        set_word(data, PROFILE_DIRECTORY_SEGMENT, DIRECTORY_SEGMENT)
        set_word(directory, (selected & 0xFF) * 20 + 18, kind)
        expected = data[:]
        expected[TEXT_LATCH] = 0x0D
        set_word(expected, COMMITTED_ROW, expected_commit)
        machine, _trace, calls = execute_case(
            executable,
            0x2514,
            data,
            expected,
            raw_low_byte=0x0D,
            extra_memory=[(DIRECTORY_ADDRESS, bytes(directory))],
            preserved=("ecx", "esi", "edi", "ebp", "ds", "es", "fs", "gs", "ss"),
        )
        if calls:
            raise AssertionError(f"accept {name}: unexpected external calls")
        vectors.append(
            {
                "name": name,
                "profile_selection": bool(mode & 1),
                "selected_index": selected,
                "record_kind": kind,
                "committed_offset": word(expected, COMMITTED_ROW),
                "latched_key": expected[TEXT_LATCH],
                "registers_after": fixture_registers(machine),
            }
        )
    return vectors


def cancel_vectors(executable: bytes) -> list[dict[str, Any]]:
    vectors = []
    cases = (
        ("presentation_inactive", 0, 0, 0, 4, False),
        ("dialogue_already_ready", 1, 1, 0, 4, False),
        ("ship_active", 1, 0, 4, 4, False),
        ("blocked_line_first", 1, 0, 0, 8, False),
        ("blocked_line_last", 1, 0, 0, 40, False),
        ("cancel_dialogue_line", 1, 0, 0, 4, True),
        ("cancel_low_line", 1, 0, 0, 3, True),
        ("cancel_high_line", 1, 0, 0, 41, True),
    )
    for name, gate, dialogue, ship, line, cancels in cases:
        data = bytearray(0x10000)
        data[TEXT_LATCH] = 0xA5
        data[PAUSE] = 1
        data[PRESENTATION_GATE] = gate
        data[DIALOGUE_READY] = dialogue
        set_word(data, SHIP_FLAGS, ship)
        set_word(data, ACTIVE_LINE, line)
        set_dword(data, REWIND_POSITION, 0x12345678)
        set_dword(data, REWIND_REMAINING, 0x89ABCDEF)
        set_dword(data, READ_POSITION, 0x0BADF00D)
        set_dword(data, READ_REMAINING, 0x00C0FFEE)
        data[SCENE_PALETTE : SCENE_PALETTE + SCENE_PALETTE_PREFIX_SIZE] = (
            bytes([0xA5]) * SCENE_PALETTE_PREFIX_SIZE
        )
        expected = data[:]
        expected[PAUSE] = 0
        expected_calls: list[str] = []
        if cancels:
            expected[DIALOGUE_READY] = int(line == 4)
            set_dword(expected, READ_POSITION, 0x12345678)
            set_dword(expected, READ_REMAINING, 0x89ABCDEF)
            expected[SCENE_PALETTE : SCENE_PALETTE + SCENE_PALETTE_PREFIX_SIZE] = bytes(
                SCENE_PALETTE_PREFIX_SIZE
            )
            expected[PALETTE_DIRTY] = 1
            expected[PRESENTATION_COMPLETE] = 1
            expected_calls.append("reset_presentation_queue")
        else:
            expected[TEXT_LATCH] = 0x1B
        machine, _trace, calls = execute_case(
            executable,
            0x253D,
            data,
            expected,
            raw_low_byte=0x1B,
            patches=[(QUEUE_RESET_ADDRESS, b"\xcb")],
            allowed_ranges=((0x253D, 0x25A7), (0x25C5, 0x25CA)),
            preserved=(
                "eax",
                "ebx",
                "ecx",
                "edx",
                "esi",
                "ebp",
                "ds",
                "es",
                "fs",
                "gs",
                "ss",
            ),
            capture={QUEUE_RESET_ADDRESS: "reset_presentation_queue"},
        )
        if calls != expected_calls:
            raise AssertionError(f"cancel {name}: external call mismatch {calls!r}")
        vectors.append(
            {
                "name": name,
                "presentation_active": bool(gate & 1),
                "dialogue_ready_before": bool(dialogue & 1),
                "ship_active": bool(ship & 4),
                "active_line": line,
                "cancelled": cancels,
                "latched_key_before": 0xA5,
                "latched_key": expected[TEXT_LATCH],
                "dialogue_ready": expected[DIALOGUE_READY],
                "read_position_before": 0x0BADF00D,
                "remaining_before": 0x00C0FFEE,
                "rewind_position": 0x12345678,
                "rewind_remaining": 0x89ABCDEF,
                "read_position": dword(expected, READ_POSITION),
                "remaining": dword(expected, READ_REMAINING),
                "palette_dirty": bool(expected[PALETTE_DIRTY]),
                "presentation_complete": bool(expected[PRESENTATION_COMPLETE]),
                "calls": calls,
                "registers_after": fixture_registers(machine),
            }
        )
    return vectors


def pause_vectors(executable: bytes) -> list[dict[str, Any]]:
    vectors = []
    cases = (
        ("pause", 0, 0, 1),
        ("unpause", 0, 1, 0),
        ("normalize_set_bits", 0, 3, 0),
        ("save_ui_blocks_toggle", 1, 1, 1),
    )
    preserved = tuple(GENERAL_REGISTERS) + tuple(SEGMENT_REGISTERS)
    for name, save_active, pause_before, pause_after in cases:
        data = bytearray(0x10000)
        data[TEXT_LATCH] = 0xA5
        data[SAVE_ACTIVE] = save_active
        data[PAUSE] = pause_before
        expected = data[:]
        expected[PAUSE] = pause_after
        expected[TEXT_LATCH] = 0x70
        machine, _trace, calls = execute_case(
            executable,
            0x25A7,
            data,
            expected,
            raw_low_byte=0x70,
            allowed_ranges=((0x25A7, 0x25CA),),
            preserved=preserved,
        )
        if calls:
            raise AssertionError(f"pause {name}: unexpected external calls")
        vectors.append(
            {
                "name": name,
                "save_active": save_active,
                "pause_before": pause_before,
                "pause_after": pause_after,
                "latched_key_before": 0xA5,
                "latched_key": expected[TEXT_LATCH],
                "registers_after": fixture_registers(machine),
            }
        )
    return vectors


def latch_vectors(executable: bytes) -> list[dict[str, Any]]:
    vectors = []
    preserved = tuple(GENERAL_REGISTERS) + tuple(SEGMENT_REGISTERS)
    for name, text_byte in (("zero", 0), ("printable", 0x71), ("maximum", 0xFF)):
        data = bytearray(0x10000)
        data[TEXT_LATCH] = 0xA5
        expected = data[:]
        expected[TEXT_LATCH] = text_byte
        machine, _trace, calls = execute_case(
            executable,
            0x25C5,
            data,
            expected,
            raw_low_byte=text_byte,
            preserved=preserved,
        )
        if calls:
            raise AssertionError(f"latch {name}: unexpected external calls")
        vectors.append(
            {
                "name": name,
                "latched_key_before": 0xA5,
                "latched_key": text_byte,
                "registers_after": fixture_registers(machine),
            }
        )
    return vectors


def report(executable: bytes) -> dict[str, Any]:
    digest = hashlib.sha256(executable).hexdigest()
    if digest != EXECUTABLE_SHA256:
        raise AssertionError(f"unexpected BLOOD2PG.EXE SHA-256 {digest}")
    return {
        "artifact_sha256": digest,
        "inventory": inventory(executable),
        "vectors": {
            "dispatch": dispatch_vectors(executable),
            "move_previous": move_previous_vectors(executable),
            "move_next": move_next_vectors(executable),
            "accept": accept_vectors(executable),
            "cancel": cancel_vectors(executable),
            "toggle_pause": pause_vectors(executable),
            "latch_text": latch_vectors(executable),
        },
    }


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("executable", nargs="?", type=Path, default=DEFAULT_EXECUTABLE)
    parser.add_argument("output", nargs="?", type=Path, default=DEFAULT_OUTPUT)
    args = parser.parse_args()
    encoded = (
        json.dumps(report(args.executable.read_bytes()), indent=2, sort_keys=True)
        + "\n"
    )
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(encoded, encoding="ascii")
    print("verified BBB input dispatcher plus 7 active handlers")


if __name__ == "__main__":
    main()
