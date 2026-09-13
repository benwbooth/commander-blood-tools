#!/usr/bin/env python3
"""Execute Big Bug Bang's C6 post-frame travel dispatcher.

The complete dispatcher and its native field-offset helper run unmodified.
Only the established camera-transition and ship-HUD far-call boundaries are
captured, so the vectors isolate state owned by the dispatcher. Run with
``python3 -P`` so the adjacent dis.py cannot shadow the standard library.
"""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
import struct

from unicorn import Uc, UC_ARCH_X86, UC_HOOK_CODE, UC_HOOK_MEM_WRITE, UC_MODE_16
from unicorn.x86_const import (
    UC_X86_REG_AX,
    UC_X86_REG_BX,
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
    UC_X86_REG_IP,
    UC_X86_REG_SP,
    UC_X86_REG_SS,
)


EXECUTABLE_SHA256 = "4b65ffca3e113a1826371e3436177861640a1b7aae24caafebb4c2f7aa467834"
HEADER_SIZE = 0x800
DISPATCHER = (0x613F, 0x65E8)
C6_ARM = (0x6432, 0x652D)
FIELD_HELPER = (0x6633, 0x6644)
DISPATCHER_SHA256 = "c897f0f92da4b6befc887b569b7269e100caab140489fe28b987b6c41ec3d57e"
C6_ARM_SHA256 = "a409990649c6f232c2d553364d8e22fe0afbb07ada265135b399c20cd5b1b662"
FIELD_HELPER_SHA256 = "fb7ec0e721e99c38e166f3c8538a44a18b99e12810b183c3b7659b660f955520"

CODE_SEGMENT = 0x502
GLOBALS = 0x30000
STATE = 0x50000
DECOY = 0x70000
STACK = 0x90000
SEGMENT_SIZE = 0x10000
STACK_POINTER = 0xFF00
RETURN_IP = 0x1800
RETURN_LINEAR = CODE_SEGMENT * 16 + RETURN_IP
STACK_SENTINEL = bytes.fromhex("5aa596698778c33c")

OWNER = 0x1000
RELATED = 0x2000
ACTION = 0x3000
NAVIGATION_KIND = 0x0010
BLACK_HOLE_KIND = 0x0100

PHASE = 0x2A2D
CAMERA_COUNTDOWN = 0x2A26
ACTOR_BUSY = 0x2D1B
CAMERA_VIEW_ACTIVE = 0x2A25
PRESENTATION_GATE = 0x2200
SCREEN_REBUILD = 0x2A79
UI_FLAGS = 0x2A33
ACTIVE_LINE = 0x6B5A
FIELD_MATRIX = 0x7128
FIELD_MATRIX_FILE = 0x16918
FIELD_MATRIX_SIZE = 21 * 16

EXTERNALS = {
    0x464E: "transition",
    0x9EA6: "hud_reset",
}

CASES = (
    {
        "name": "phase_zero_waits_for_actor",
        "phase": 0,
        "actor_busy": 0,
    },
    {
        "name": "phase_zero_starts_camera_transition",
        "phase": 0,
        "actor_busy": 1,
    },
    {
        "name": "phase_one_waits_for_camera",
        "phase": 1,
        "camera_countdown": 5,
    },
    {
        "name": "phase_two_also_waits_for_camera",
        "phase": 2,
        "camera_countdown": 1,
    },
    {
        "name": "phase_one_starts_line_44",
        "phase": 1,
        "camera_countdown": 0,
        "actor_busy": 1,
        "camera_view_active": 1,
    },
    {
        "name": "final_phase_waits_for_presentation_gate",
        "phase": 2,
        "camera_countdown": 0,
        "presentation_gate": 1,
    },
    {
        "name": "final_phase_uses_matching_position_pair",
        "phase": 2,
        "camera_countdown": 0,
        "owner_relation": 0x1234,
        "related_comparison": 0x1234,
    },
    {
        "name": "final_phase_uses_mismatching_position_pair",
        "phase": 2,
        "camera_countdown": 0,
        "owner_relation": 0x1234,
        "related_comparison": 0x5678,
    },
)


def word(data: bytes | bytearray, offset: int) -> int:
    return struct.unpack_from("<H", data, offset)[0]


def dword(data: bytes | bytearray, offset: int) -> int:
    return struct.unpack_from("<I", data, offset)[0]


def put_word(data: bytearray, offset: int, value: int) -> None:
    struct.pack_into("<H", data, offset, value & 0xFFFF)


def put_dword(data: bytearray, offset: int, value: int) -> None:
    struct.pack_into("<I", data, offset, value & 0xFFFFFFFF)


def image_address(file_offset: int) -> int:
    return file_offset - HEADER_SIZE


def field_offset(executable: bytes, selector: int, kind: int) -> int:
    column = (kind & -kind).bit_length() - 1
    assert 0 <= selector < 21 and 0 <= column < 16
    return executable[FIELD_MATRIX_FILE + selector * 16 + column]


def far_return(cpu: Uc) -> None:
    stack = cpu.reg_read(UC_X86_REG_SS) * 16 + cpu.reg_read(UC_X86_REG_SP)
    return_ip, return_cs = struct.unpack("<HH", cpu.mem_read(stack, 4))
    cpu.reg_write(UC_X86_REG_SP, (cpu.reg_read(UC_X86_REG_SP) + 4) & 0xFFFF)
    cpu.reg_write(UC_X86_REG_CS, return_cs)
    cpu.reg_write(UC_X86_REG_IP, return_ip)


def initial_images(executable: bytes, case: dict[str, object]):
    globals_before = bytearray(
        (offset * 37 + (offset >> 8) * 11 + 0x31) & 0xFF
        for offset in range(SEGMENT_SIZE)
    )
    globals_before[FIELD_MATRIX : FIELD_MATRIX + FIELD_MATRIX_SIZE] = executable[
        FIELD_MATRIX_FILE : FIELD_MATRIX_FILE + FIELD_MATRIX_SIZE
    ]
    globals_before[PHASE] = int(case["phase"])
    globals_before[CAMERA_COUNTDOWN] = int(case.get("camera_countdown", 0))
    globals_before[ACTOR_BUSY] = int(case.get("actor_busy", 0))
    globals_before[CAMERA_VIEW_ACTIVE] = int(case.get("camera_view_active", 0))
    globals_before[PRESENTATION_GATE] = int(case.get("presentation_gate", 0))
    globals_before[SCREEN_REBUILD] = 0
    globals_before[UI_FLAGS] = 0xAF
    put_word(globals_before, ACTIVE_LINE, 0)

    state_before = bytearray(
        (offset * 29 + (offset >> 8) * 13 + 0x53) & 0xFF
        for offset in range(SEGMENT_SIZE)
    )
    put_word(state_before, OWNER, NAVIGATION_KIND)
    put_word(state_before, RELATED, BLACK_HOLE_KIND)
    put_word(state_before, ACTION, 0xC6)
    put_word(state_before, ACTION + 2, RELATED)
    put_word(state_before, ACTION + 4, 0xA55A)

    owner_relation = field_offset(executable, 0x0E, NAVIGATION_KIND)
    owner_position = field_offset(executable, 0x0B, NAVIGATION_KIND)
    related_comparison = field_offset(executable, 0x0C, BLACK_HOLE_KIND)
    related_match_relation = field_offset(executable, 0x0D, BLACK_HOLE_KIND)
    related_position_a = field_offset(executable, 0x09, BLACK_HOLE_KIND)
    related_position_b = field_offset(executable, 0x0A, BLACK_HOLE_KIND)
    assert (
        owner_relation,
        owner_position,
        related_comparison,
        related_match_relation,
        related_position_a,
        related_position_b,
    ) == (34, 24, 20, 22, 24, 28)

    put_word(
        state_before,
        OWNER + owner_relation,
        int(case.get("owner_relation", 0x1234)),
    )
    put_dword(state_before, OWNER + owner_position, 0x01020304)
    put_word(
        state_before,
        RELATED + related_comparison,
        int(case.get("related_comparison", 0x1234)),
    )
    put_word(state_before, RELATED + related_match_relation, 0x9ABC)
    put_dword(state_before, RELATED + related_position_a, 0x11223344)
    put_dword(state_before, RELATED + related_position_b, 0xA1B2C3D4)
    return globals_before, state_before


def expected_images(
    executable: bytes,
    case: dict[str, object],
    globals_before: bytearray,
    state_before: bytearray,
):
    globals_after = bytearray(globals_before)
    state_after = bytearray(state_before)
    calls: list[dict[str, object]] = []
    phase = globals_after[PHASE]

    if phase == 0:
        if globals_after[ACTOR_BUSY] == 1:
            globals_after[PHASE] = 1
            globals_after[CAMERA_COUNTDOWN] = 8
            calls.append({"name": "transition", "object_id": 4})
        return globals_after, state_after, calls

    if globals_after[CAMERA_COUNTDOWN] != 0:
        return globals_after, state_after, calls
    if phase == 1:
        globals_after[PHASE] = 2
        globals_after[ACTOR_BUSY] = 0
        globals_after[CAMERA_VIEW_ACTIVE] = 0
        put_word(globals_after, ACTIVE_LINE, 44)
        return globals_after, state_after, calls
    if globals_after[PRESENTATION_GATE] & 1:
        return globals_after, state_after, calls

    globals_after[PHASE] = 0
    globals_after[SCREEN_REBUILD] = 1
    calls.append({"name": "hud_reset"})
    globals_after[UI_FLAGS] &= 0xFB
    state_after[ACTION : ACTION + 6] = bytes(6)

    def field(selector: int, kind: int) -> int:
        result = field_offset(executable, selector, kind)
        calls.append(
            {"name": "field", "selector": selector, "kind": kind, "result": result}
        )
        return result

    owner_relation = field(0x0E, NAVIGATION_KIND)
    relation = word(state_after, OWNER + owner_relation)
    owner_position = field(0x0B, NAVIGATION_KIND)
    comparison = word(state_after, RELATED + field(0x0C, BLACK_HOLE_KIND))
    if relation == comparison:
        relation = word(state_after, RELATED + field(0x0D, BLACK_HOLE_KIND))
        source_position = field(0x0A, BLACK_HOLE_KIND)
    else:
        field(0x0C, BLACK_HOLE_KIND)
        relation = comparison
        source_position = field(0x09, BLACK_HOLE_KIND)
    state_after[OWNER + owner_position : OWNER + owner_position + 4] = state_after[
        RELATED + source_position : RELATED + source_position + 4
    ]
    put_word(state_after, OWNER + owner_relation, relation)
    return globals_after, state_after, calls


def execute(executable: bytes, case: dict[str, object]) -> dict[str, object]:
    cpu = Uc(UC_ARCH_X86, UC_MODE_16)
    cpu.mem_map(0, 0x100000)
    module = executable[HEADER_SIZE:]
    cpu.mem_write(0, module)
    globals_before, state_before = initial_images(executable, case)
    expected_globals, expected_state, expected_calls = expected_images(
        executable, case, globals_before, state_before
    )
    decoy_before = bytes(
        (offset * 17 + (offset >> 8) * 23 + 0x75) & 0xFF
        for offset in range(SEGMENT_SIZE)
    )
    stack_before = bytearray(
        (offset * 31 + (offset >> 8) * 7 + 0x97) & 0xFF
        for offset in range(SEGMENT_SIZE)
    )
    stack_before[STACK_POINTER : STACK_POINTER + 2 + len(STACK_SENTINEL)] = (
        struct.pack("<H", RETURN_IP) + STACK_SENTINEL
    )
    cpu.mem_write(GLOBALS, bytes(globals_before))
    cpu.mem_write(STATE, bytes(state_before))
    cpu.mem_write(DECOY, decoy_before)
    cpu.mem_write(STACK, bytes(stack_before))

    initial_registers = {
        UC_X86_REG_EAX: 0xA1A1BEEF,
        UC_X86_REG_EBX: 0xB2B22345,
        UC_X86_REG_ECX: 0xC3C33456,
        UC_X86_REG_EDX: 0xD4D44567,
        UC_X86_REG_ESI: 0xE5E50000 | OWNER,
        UC_X86_REG_EDI: 0xF6F66789,
        UC_X86_REG_EBP: 0x97970000 | ACTION,
        UC_X86_REG_CS: CODE_SEGMENT,
        UC_X86_REG_DS: STATE // 16,
        UC_X86_REG_ES: DECOY // 16,
        UC_X86_REG_FS: 0xB000,
        UC_X86_REG_GS: GLOBALS // 16,
        UC_X86_REG_SS: STACK // 16,
        UC_X86_REG_SP: STACK_POINTER,
        UC_X86_REG_EFLAGS: 0x0AD6,
    }
    for register, value in initial_registers.items():
        cpu.reg_write(register, value)

    calls: list[dict[str, object]] = []
    pending_field: dict[str, object] | None = None
    reached_return = False
    allowed_global_writes = {
        PHASE,
        CAMERA_COUNTDOWN,
        ACTOR_BUSY,
        CAMERA_VIEW_ACTIVE,
        SCREEN_REBUILD,
        UI_FLAGS,
        ACTIVE_LINE,
        ACTIVE_LINE + 1,
    }
    allowed_state_writes = set(range(ACTION, ACTION + 6))
    allowed_state_writes.update(range(OWNER + 24, OWNER + 28))
    allowed_state_writes.update(range(OWNER + 34, OWNER + 36))

    def instruction(machine: Uc, address: int, size: int, _context) -> None:
        nonlocal pending_field, reached_return
        file_offset = address + HEADER_SIZE
        if address == RETURN_LINEAR:
            reached_return = True
            machine.emu_stop()
            return
        if file_offset in EXTERNALS:
            name = EXTERNALS[file_offset]
            call: dict[str, object] = {"name": name}
            if name == "transition":
                call["object_id"] = machine.reg_read(UC_X86_REG_AX)
            calls.append(call)
            far_return(machine)
            return
        if file_offset == FIELD_HELPER[0]:
            assert pending_field is None
            pending_field = {
                "name": "field",
                "selector": machine.reg_read(UC_X86_REG_AX),
                "kind": machine.reg_read(UC_X86_REG_BX),
            }
        elif file_offset == FIELD_HELPER[1] - 1:
            assert pending_field is not None
            pending_field["result"] = machine.reg_read(UC_X86_REG_AX)
            calls.append(pending_field)
            pending_field = None
        assert any(
            image_address(start) <= address < address + size <= image_address(end)
            for start, end in (DISPATCHER, FIELD_HELPER)
        ), hex(file_offset)

    def write_hook(_machine, _access, address, size, _value, _context) -> None:
        offsets = range(address, address + size)
        allowed = (
            all(offset - GLOBALS in allowed_global_writes for offset in offsets)
            or all(offset - STATE in allowed_state_writes for offset in offsets)
            or STACK + STACK_POINTER - 32 <= address
            and address + size <= STACK + STACK_POINTER
        )
        assert allowed, (hex(address), size)

    cpu.hook_add(UC_HOOK_CODE, instruction)
    cpu.hook_add(UC_HOOK_MEM_WRITE, write_hook)
    cpu.emu_start(image_address(DISPATCHER[0]), 0, count=2000)
    assert reached_return
    assert pending_field is None
    assert calls == expected_calls, (case["name"], calls, expected_calls)

    globals_after = bytes(cpu.mem_read(GLOBALS, SEGMENT_SIZE))
    state_after = bytes(cpu.mem_read(STATE, SEGMENT_SIZE))
    assert globals_after == bytes(expected_globals)
    assert state_after == bytes(expected_state)
    assert bytes(cpu.mem_read(DECOY, SEGMENT_SIZE)) == decoy_before
    assert bytes(cpu.mem_read(0, len(module))) == module
    assert (
        bytes(cpu.mem_read(STACK + STACK_POINTER + 2, len(STACK_SENTINEL)))
        == STACK_SENTINEL
    )

    expected_registers = {
        UC_X86_REG_EAX: initial_registers[UC_X86_REG_EAX],
        UC_X86_REG_EBX: initial_registers[UC_X86_REG_EBX],
        UC_X86_REG_ECX: initial_registers[UC_X86_REG_ECX],
        UC_X86_REG_EDX: initial_registers[UC_X86_REG_EDX],
        UC_X86_REG_ESI: OWNER,
        UC_X86_REG_EDI: initial_registers[UC_X86_REG_EDI] & 0xFFFF,
        UC_X86_REG_EBP: initial_registers[UC_X86_REG_EBP],
        UC_X86_REG_CS: CODE_SEGMENT,
        UC_X86_REG_DS: STATE // 16,
        UC_X86_REG_ES: DECOY // 16,
        UC_X86_REG_FS: initial_registers[UC_X86_REG_FS],
        UC_X86_REG_GS: GLOBALS // 16,
        UC_X86_REG_SS: STACK // 16,
        UC_X86_REG_SP: STACK_POINTER + 2,
    }
    for register, expected in expected_registers.items():
        assert cpu.reg_read(register) == expected, (
            register,
            cpu.reg_read(register),
            expected,
        )

    return {
        "name": case["name"],
        "phase_before": globals_before[PHASE],
        "camera_countdown_before": globals_before[CAMERA_COUNTDOWN],
        "actor_busy_before": globals_before[ACTOR_BUSY],
        "camera_view_active_before": globals_before[CAMERA_VIEW_ACTIVE],
        "presentation_gate_before": globals_before[PRESENTATION_GATE],
        "ui_flags_before": globals_before[UI_FLAGS],
        "owner_relation_before": word(state_before, OWNER + 34),
        "owner_position_before": dword(state_before, OWNER + 24),
        "related_comparison": word(state_before, RELATED + 20),
        "related_match_relation": word(state_before, RELATED + 22),
        "related_position_a": dword(state_before, RELATED + 24),
        "related_position_b": dword(state_before, RELATED + 28),
        "phase_after": globals_after[PHASE],
        "camera_countdown_after": globals_after[CAMERA_COUNTDOWN],
        "actor_busy_after": globals_after[ACTOR_BUSY],
        "camera_view_active_after": globals_after[CAMERA_VIEW_ACTIVE],
        "active_line_after": word(globals_after, ACTIVE_LINE),
        "screen_rebuild_after": globals_after[SCREEN_REBUILD],
        "ui_flags_after": globals_after[UI_FLAGS],
        "record_after": list(struct.unpack_from("<3H", state_after, ACTION)),
        "owner_relation_after": word(state_after, OWNER + 34),
        "owner_position_after": dword(state_after, OWNER + 24),
        "calls": calls,
        "globals_sha256": hashlib.sha256(globals_after).hexdigest(),
        "state_sha256": hashlib.sha256(state_after).hexdigest(),
        "flags_after": cpu.reg_read(UC_X86_REG_EFLAGS) & 0xFFFF,
    }


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("executable", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    executable = args.executable.read_bytes()
    digest = hashlib.sha256(executable).hexdigest()
    if digest != EXECUTABLE_SHA256:
        raise SystemExit(f"unsupported BLOOD2PG.EXE SHA-256 {digest}")
    for (start, end), expected in (
        (DISPATCHER, DISPATCHER_SHA256),
        (C6_ARM, C6_ARM_SHA256),
        (FIELD_HELPER, FIELD_HELPER_SHA256),
    ):
        actual = hashlib.sha256(executable[start:end]).hexdigest()
        if actual != expected:
            raise SystemExit(f"native span {start:#x}..{end:#x} changed")

    rows = [execute(executable, case) for case in CASES]
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(
        "".join(
            json.dumps(row, sort_keys=True, separators=(",", ":")) + "\n"
            for row in rows
        )
    )
    print(f"verified {len(rows)} original C6 travel-dispatch cases")


if __name__ == "__main__":
    main()
