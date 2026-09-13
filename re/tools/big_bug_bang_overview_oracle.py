#!/usr/bin/env python3
"""Execute BLOOD2PG's simulation-overview controller and record draw calls.

The original roster builders and navigation-position resolver execute unchanged.
Only the established graphics API boundaries are captured; this fixture proves
controller state, input consumption, filtering, layout, color and draw order,
not planar VGA pixels inside those already independently recovered primitives.
"""

import argparse
import hashlib
import itertools
import json
from pathlib import Path
import struct

from unicorn import Uc, UC_ARCH_X86, UC_HOOK_CODE, UC_MODE_16
from unicorn.x86_const import (
    UC_X86_REG_AX,
    UC_X86_REG_BP,
    UC_X86_REG_BX,
    UC_X86_REG_CS,
    UC_X86_REG_CX,
    UC_X86_REG_DI,
    UC_X86_REG_DS,
    UC_X86_REG_DX,
    UC_X86_REG_EFLAGS,
    UC_X86_REG_ES,
    UC_X86_REG_GS,
    UC_X86_REG_IP,
    UC_X86_REG_SI,
    UC_X86_REG_SP,
    UC_X86_REG_SS,
)

EXECUTABLE_SHA256 = "4b65ffca3e113a1826371e3436177861640a1b7aae24caafebb4c2f7aa467834"
MZ_HEADER_SIZE = 0x800
DATA_FILE_START = 0xF7F0
GLOBAL_BASE = 0x30000
STATE_BASE = 0x40000
DIRECTORY_BASE = 0x60000
SEGMENT_SIZE = 0x10000
STACK_TOP = 0xFF00
RETURN_IP = 0x100
CODE_SEGMENT = 0x502
ENTRY_FILE_OFFSET = 0xA286

SECONDARY = 0x0C37
PRESS_PENDING = 0x0C38
PRIMARY = 0x0C36
POINTER_X = 0x0C22
POINTER_Y = 0x0C24
HAND_SELECTOR = 0x0C2A
BLOCKED_WORD = 0x2A5F
UI_FLAGS = 0x2A22
HOVERED_MASK = 0x2A18
SELECTED_MASK = 0x2A1A
AVAILABLE_MASK = 0x2A1C
CONFLICT_MASK = 0x2A1E
UNSTABLE_MASK = 0x2A20
ACTIVE = 0x2A30
AVAILABLE_COUNT = 0x2A31
SHOW_ALL = 0x2A32
CAMERA_SLOT = 0x2CBB
SCRATCH_LIST = 0x2E13
STATE_POINTER = 0x6AEC
DIRECTORY_POINTER = 0x6AF0
ARCHETYPE_POINTER = 0x6B22
EXCLUDED_HOLDER_POINTER = 0x6B2A

ACTOR_BASE = 0x0100
ACTOR_SIZE = 74
HOLDER_BASE = 0x2000
HOLDER_SIZE = 32
ARCHETYPE = 0x4000
EXCLUDED_HOLDER = 0x4100

GRAPHICS = {
    0x40E9: "fill",
    0x3ED3: "outline",
    0x3B03: "line",
    0x362C: "horizontal",
    0x36A1: "vertical",
    0x3512: "text",
}


def word(data, offset):
    return struct.unpack_from("<H", data, offset)[0]


def write_word(data, offset, value):
    struct.pack_into("<H", data, offset, value & 0xFFFF)


def far_return(cpu):
    stack = cpu.reg_read(UC_X86_REG_SS) * 16 + cpu.reg_read(UC_X86_REG_SP)
    return_ip, return_cs = struct.unpack("<HH", cpu.mem_read(stack, 4))
    cpu.reg_write(UC_X86_REG_SP, (cpu.reg_read(UC_X86_REG_SP) + 4) & 0xFFFF)
    cpu.reg_write(UC_X86_REG_CS, return_cs)
    cpu.reg_write(UC_X86_REG_IP, return_ip)


def c_string(cpu, segment, offset):
    data = bytes(cpu.mem_read(segment * 16 + offset, 256))
    return data[:data.index(0)].decode("ascii")


def default_actors():
    return [
        {"group": 0x8000, "flags": 0x0005, "holder_flags": 0x0003,
         "quantity": 3, "balance": 3, "position": [40, 60], "opponent": None},
        {"group": 0x0020, "flags": 0x000D, "holder_flags": 0x0003,
         "quantity": 2, "balance": 7, "position": [80, 90], "opponent": 0},
        {"group": 0x0001, "flags": 0x0005, "holder_flags": 0x0003,
         "quantity": 9, "balance": 4, "position": [120, 130], "opponent": None},
        {"group": 0x0004, "flags": 0x0005, "holder_flags": 0x0001,
         "quantity": 1, "balance": 1, "position": [150, 150], "opponent": None},
        {"group": 0x0400, "flags": 0x0001, "holder_flags": 0x0003,
         "quantity": 1, "balance": 1, "position": [170, 150], "opponent": None},
        {"group": 0x0100, "flags": 0x0005, "holder_flags": 0x0003,
         "quantity": 1, "balance": 1, "position": [190, 150], "opponent": None,
         "excluded_holder": True},
    ]


def run(executable, name, inputs, actors=None):
    actors = default_actors() if actors is None else actors
    cpu = Uc(UC_ARCH_X86, UC_MODE_16)
    cpu.mem_map(0, 0x100000)
    cpu.mem_write(0, executable[MZ_HEADER_SIZE:])

    globals_before = bytearray(SEGMENT_SIZE)
    native_data = executable[DATA_FILE_START:]
    globals_before[:len(native_data)] = native_data
    for offset, address in ((STATE_POINTER, STATE_BASE), (DIRECTORY_POINTER, DIRECTORY_BASE)):
        struct.pack_into("<HH", globals_before, offset, 0, address // 16)
    write_word(globals_before, ARCHETYPE_POINTER, ARCHETYPE)
    write_word(globals_before, EXCLUDED_HOLDER_POINTER, EXCLUDED_HOLDER)
    for offset, value in (
        (BLOCKED_WORD, inputs.get("blocked_word", 0)),
        (UI_FLAGS, inputs.get("ui_flags", 0)),
        (POINTER_X, inputs.get("pointer", [0, 0])[0]),
        (POINTER_Y, inputs.get("pointer", [0, 0])[1]),
        (HOVERED_MASK, inputs.get("hovered_mask", 0)),
        (SELECTED_MASK, inputs.get("selected_mask", 0)),
        (AVAILABLE_MASK, inputs.get("available_mask", 0xA55A)),
        (CONFLICT_MASK, inputs.get("conflict_mask", 0x5AA5)),
        (UNSTABLE_MASK, inputs.get("unstable_mask", 0x55AA)),
        (HAND_SELECTOR, inputs.get("hand", 23)),
    ):
        write_word(globals_before, offset, value)
    globals_before[SECONDARY] = inputs.get("secondary", 0)
    globals_before[PRESS_PENDING] = inputs.get("press_pending", 0)
    globals_before[PRIMARY] = inputs.get("primary", 0)
    globals_before[ACTIVE] = inputs.get("active", 0)
    globals_before[SHOW_ALL] = inputs.get("show_all", 0)
    globals_before[AVAILABLE_COUNT] = inputs.get("available_count", 0xA5)
    globals_before[CAMERA_SLOT] = inputs.get("camera_slot", 0xF)
    write_word(globals_before, STACK_TOP, RETURN_IP)
    cpu.mem_write(GLOBAL_BASE, bytes(globals_before))

    state = bytearray(SEGMENT_SIZE)
    write_word(state, ARCHETYPE, 8)
    write_word(state, ARCHETYPE + 24, 100)
    write_word(state, ARCHETYPE + 26, 110)
    write_word(state, EXCLUDED_HOLDER, 8)
    excluded_holder_flags = next(
        (actor["holder_flags"] for actor in actors if actor.get("excluded_holder")), 3
    )
    write_word(state, EXCLUDED_HOLDER + 2, excluded_holder_flags)
    excluded_position = next(
        (actor["position"] for actor in actors if actor.get("excluded_holder")), [200, 180]
    )
    write_word(state, EXCLUDED_HOLDER + 24, excluded_position[0])
    write_word(state, EXCLUDED_HOLDER + 26, excluded_position[1])
    for index, actor in enumerate(actors):
        actor_offset = ACTOR_BASE + index * ACTOR_SIZE
        holder_offset = (EXCLUDED_HOLDER if actor.get("excluded_holder")
                         else HOLDER_BASE + index * HOLDER_SIZE)
        if holder_offset != EXCLUDED_HOLDER:
            write_word(state, holder_offset, 8)
            write_word(state, holder_offset + 2, actor["holder_flags"])
            write_word(state, holder_offset + 24, actor["position"][0])
            write_word(state, holder_offset + 26, actor["position"][1])
        write_word(state, actor_offset, actor.get("state_flags", 2))
        write_word(state, actor_offset + 2, actor["flags"])
        write_word(state, actor_offset + 20, actor["group"])
        write_word(state, actor_offset + 22, actor["quantity"])
        write_word(state, actor_offset + 24, holder_offset)
        write_word(state, actor_offset + 52, actor["balance"])
        opponent = actor.get("opponent")
        write_word(state, actor_offset + 72,
                   0xFFFF if opponent is None else ACTOR_BASE + opponent * ACTOR_SIZE)
    cpu.mem_write(STATE_BASE, bytes(state))

    directory = bytearray(SEGMENT_SIZE)
    for index in range(len(actors)):
        entry = index * 20
        write_word(directory, entry + 16, ACTOR_BASE + index * ACTOR_SIZE)
        write_word(directory, entry + 18, 1)
    cpu.mem_write(DIRECTORY_BASE, bytes(directory))

    initial_registers = {
        UC_X86_REG_AX: 0x1111,
        UC_X86_REG_BX: 0x2222,
        UC_X86_REG_CX: 0x3333,
        UC_X86_REG_DX: 0x4444,
        UC_X86_REG_BP: 0x5555,
        UC_X86_REG_SI: 0x6666,
        UC_X86_REG_DI: 0x7777,
        UC_X86_REG_DS: GLOBAL_BASE // 16,
        UC_X86_REG_ES: 0x7000,
        UC_X86_REG_GS: GLOBAL_BASE // 16,
        UC_X86_REG_SS: GLOBAL_BASE // 16,
        UC_X86_REG_SP: STACK_TOP,
        UC_X86_REG_CS: CODE_SEGMENT,
    }
    for register, value in initial_registers.items():
        cpu.reg_write(register, value)
    cpu.reg_write(UC_X86_REG_EFLAGS, 2)

    draws = []
    helper_visits = set()

    def instruction(machine, address, _size, _context):
        file_address = address + MZ_HEADER_SIZE
        if file_address in (0x6FF2, 0x706E, 0x67B8):
            helper_visits.add(file_address)
        kind = GRAPHICS.get(file_address)
        if kind is None:
            return
        ax = machine.reg_read(UC_X86_REG_AX) & 0xFFFF
        bx = machine.reg_read(UC_X86_REG_BX) & 0xFFFF
        cx = machine.reg_read(UC_X86_REG_CX) & 0xFFFF
        dx = machine.reg_read(UC_X86_REG_DX) & 0xFFFF
        bp = machine.reg_read(UC_X86_REG_BP) & 0xFFFF
        if kind == "text":
            draws.append({"kind": kind, "text": c_string(
                machine, machine.reg_read(UC_X86_REG_DS), machine.reg_read(UC_X86_REG_SI)),
                "origin": [bx, dx], "color": ax & 0xFF})
        elif kind == "line":
            draws.append({"kind": kind, "start": [bx, cx], "end": [dx, bp],
                          "color": ax & 0xFF})
        elif kind in ("fill", "outline"):
            draws.append({"kind": kind, "origin": [bx, cx], "extent": [dx, bp],
                          "color": ax & 0xFF})
        else:
            draws.append({"kind": kind, "origin": [bx, cx], "extent": dx,
                          "color": ax & 0xFF})
        far_return(machine)

    cpu.hook_add(UC_HOOK_CODE, instruction)
    entry = ENTRY_FILE_OFFSET - MZ_HEADER_SIZE
    sentinel = CODE_SEGMENT * 16 + RETURN_IP
    cpu.emu_start(entry, sentinel, count=200000)
    assert cpu.reg_read(UC_X86_REG_CS) == CODE_SEGMENT
    assert cpu.reg_read(UC_X86_REG_IP) == RETURN_IP
    for register, value in initial_registers.items():
        if register not in (UC_X86_REG_SP, UC_X86_REG_CS):
            assert cpu.reg_read(register) == value, (name, register)
    assert cpu.reg_read(UC_X86_REG_SP) == STACK_TOP + 2
    assert bytes(cpu.mem_read(STATE_BASE, SEGMENT_SIZE)) == bytes(state)
    assert bytes(cpu.mem_read(DIRECTORY_BASE, SEGMENT_SIZE)) == bytes(directory)

    after = bytes(cpu.mem_read(GLOBAL_BASE, SEGMENT_SIZE))
    outputs = {
        "secondary": after[SECONDARY],
        "press_pending": after[PRESS_PENDING],
        "primary": after[PRIMARY],
        "hand": word(after, HAND_SELECTOR),
        "active": after[ACTIVE],
        "show_all": after[SHOW_ALL],
        "available_mask": word(after, AVAILABLE_MASK),
        "available_count": after[AVAILABLE_COUNT],
        "hovered_mask": word(after, HOVERED_MASK),
        "selected_mask": word(after, SELECTED_MASK),
        "conflict_mask": word(after, CONFLICT_MASK),
        "unstable_mask": word(after, UNSTABLE_MASK),
        "camera_slot": after[CAMERA_SLOT],
        "draws": draws,
        "helpers": [f"{address:#06x}" for address in sorted(helper_visits)],
    }
    return {"name": name, "input": inputs, "actors": actors, "output": outputs}


def cases(executable):
    yield run(executable, "blocked_word", {"blocked_word": 1, "secondary": 1,
                                           "press_pending": 7, "primary": 1})
    yield run(executable, "blocked_ui_low", {"ui_flags": 1, "secondary": 1,
                                             "press_pending": 7, "primary": 1})
    yield run(executable, "inactive", {"active": 0, "secondary": 0})
    yield run(executable, "open", {"active": 0, "secondary": 1,
                                    "press_pending": 7, "primary": 1})
    yield run(executable, "close", {"active": 1, "show_all": 1, "secondary": 1,
                                     "press_pending": 7, "camera_slot": 3})
    for pointer, primary in itertools.product(
        ([5, 167], [6, 167], [36, 174], [37, 174], [46, 168], [47, 168],
         [136, 174], [137, 174], [47, 175], [136, 181]),
        (0, 1),
    ):
        yield run(executable, f"filtered_{pointer[0]}_{pointer[1]}_{primary}",
                  {"active": 1, "show_all": 0, "available_mask": 0x8121,
                   "available_count": 4, "hovered_mask": 0x0020,
                   "selected_mask": 0x0001, "pointer": pointer, "primary": primary,
                   "press_pending": 4})
    for count, mask in ((1, 0x0001), (4, 0x8007), (5, 0x001F),
                        (6, 0x003F), (10, 0x03FF), (16, 0xFFFF)):
        yield run(executable, f"layout_{count}",
                  {"active": 1, "show_all": 1, "available_mask": mask,
                   "available_count": count, "pointer": [319, 199]})
    no_available = [{**actor, "holder_flags": 1} for actor in default_actors()]
    yield run(executable, "open_empty", {"secondary": 1, "press_pending": 3}, no_available)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("executable", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    executable = args.executable.read_bytes()
    if hashlib.sha256(executable).hexdigest() != EXECUTABLE_SHA256:
        raise SystemExit("unsupported BLOOD2PG.EXE build")
    results = list(cases(executable))
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text("".join(json.dumps(row, separators=(",", ":")) + "\n"
                                   for row in results))
    print(f"wrote {len(results)} original simulation-overview cases")


if __name__ == "__main__":
    main()
