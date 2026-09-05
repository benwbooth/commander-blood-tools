#!/usr/bin/env python3
"""Run the original sequel D6 handler and actor-selection helper on synthetic state.

No native calls are replaced. Unicorn is an offline reference tool, not a game
dependency. Only synthetic inputs and native results are saved. Use python3 -P.
"""

import argparse
import hashlib
import json
from pathlib import Path
import random
import struct

from unicorn import Uc, UC_ARCH_X86, UC_MODE_16, UC_HOOK_INTR
from unicorn.x86_const import (
    UC_X86_REG_CS, UC_X86_REG_DS, UC_X86_REG_ES, UC_X86_REG_GS,
    UC_X86_REG_SS, UC_X86_REG_SP, UC_X86_REG_SI, UC_X86_REG_IP,
)

EXECUTABLE_SHA256 = "4b65ffca3e113a1826371e3436177861640a1b7aae24caafebb4c2f7aa467834"
HANDLER_START = 0x728B
RETURN_IP = 0x200
SCRIPT_SEGMENT = 0x2000
GLOBAL_SEGMENT = 0x3000
STATE_SEGMENT = 0x4000
STACK_SEGMENT = 0x5000
DIRECTORY_SEGMENT = 0x6000
SCRIPT_OFFSET = 64
VAR_POINTER = 0x6AEC
DIRECTORY_POINTER = 0x6AF0
EXCLUDED_LOCATION = 0x6B2A
COUNTDOWN = 0x0CC6
QUERY_MODE = 0x6B83
GROWTH_OPCODE = 0xD6
PLAYER_SIZE = 34
ACTOR_SIZE = 74
LOCATION_SIZE = 26
ACTOR_COUNT = 4
ACTOR_OFFSETS = [PLAYER_SIZE + index * ACTOR_SIZE for index in range(ACTOR_COUNT)]
LOCATION_OFFSETS = [PLAYER_SIZE + ACTOR_COUNT * ACTOR_SIZE + index * LOCATION_SIZE for index in range(2)]
STATE_BYTES = LOCATION_OFFSETS[-1] + LOCATION_SIZE
FLAGS = 2
GROUP = 20
QUANTITY = 22
LOCATION = 24
AGGRESSIVENESS = 50
GROWTH_BALANCE = 52
PRESSURE_RELIEF = 56


def word(data, offset, value):
    struct.pack_into("<H", data, offset, value & 65535)


def fixture():
    state = bytearray((index * 17 + 3) % 256 for index in range(STATE_BYTES))
    objects = [(0, 1, "blood")]
    objects += [(offset, 2, f"actor{index}") for index, offset in enumerate(ACTOR_OFFSETS)]
    objects += [(offset, 128, name) for offset, name in zip(LOCATION_OFFSETS, ["place", "Trashlando"])]
    directory = bytearray()
    for offset, kind, name in objects:
        word(state, offset, kind)
        word(state, offset + FLAGS, 5)
        directory.extend(struct.pack("<16sHH", name.encode(), offset, 1))
    directory.extend(bytes(20))
    for offset in ACTOR_OFFSETS:
        for field, value in [(GROUP, 1), (QUANTITY, 500), (LOCATION, LOCATION_OFFSETS[0]),
                             (AGGRESSIVENESS, 500), (GROWTH_BALANCE, 500), (PRESSURE_RELIEF, 500)]:
            word(state, offset + field, value)
    return state, bytes(directory)


def run(executable, name, state, directory, rate=10, group_mask=1, countdown=0, query=0):
    token = struct.pack("<BHH", GROWTH_OPCODE, group_mask, rate & 65535)
    machine = Uc(UC_ARCH_X86, UC_MODE_16)
    machine.mem_map(0, 1048576)
    machine.mem_write(0, executable)
    machine.mem_write(SCRIPT_SEGMENT * 16 + SCRIPT_OFFSET, token)
    globals_before = bytearray(32768)
    struct.pack_into("<HH", globals_before, VAR_POINTER, 0, STATE_SEGMENT)
    struct.pack_into("<HH", globals_before, DIRECTORY_POINTER, 0, DIRECTORY_SEGMENT)
    word(globals_before, EXCLUDED_LOCATION, LOCATION_OFFSETS[1])
    word(globals_before, COUNTDOWN, countdown)
    globals_before[QUERY_MODE] = query
    machine.mem_write(GLOBAL_SEGMENT * 16, bytes(globals_before))
    machine.mem_write(STATE_SEGMENT * 16, bytes(state))
    machine.mem_write(DIRECTORY_SEGMENT * 16, directory)
    stack_pointer = 65520
    machine.mem_write(STACK_SEGMENT * 16 + stack_pointer, struct.pack("<H", RETURN_IP))
    for register, value in [
        (UC_X86_REG_CS, 0), (UC_X86_REG_DS, SCRIPT_SEGMENT),
        (UC_X86_REG_GS, GLOBAL_SEGMENT), (UC_X86_REG_SS, STACK_SEGMENT),
        (UC_X86_REG_SP, stack_pointer), (UC_X86_REG_ES, 0),
        (UC_X86_REG_SI, SCRIPT_OFFSET + 1),
    ]:
        machine.reg_write(register, value)
    interrupts = []

    def interrupt(cpu, number, _context):
        interrupts.append(number)
        cpu.emu_stop()

    machine.hook_add(UC_HOOK_INTR, interrupt)
    machine.emu_start(HANDLER_START, RETURN_IP, count=10000)
    assert interrupts in ([], [0]), interrupts
    if not interrupts:
        assert machine.reg_read(UC_X86_REG_IP) == RETURN_IP, name
        assert machine.reg_read(UC_X86_REG_SI) == SCRIPT_OFFSET + len(token), name
    assert machine.mem_read(GLOBAL_SEGMENT * 16, len(globals_before)) == globals_before, name
    assert machine.mem_read(DIRECTORY_SEGMENT * 16, len(directory)) == directory, name
    return {"name": name, "token": list(token), "countdown": countdown,
            "query_mode": query, "excluded_location": LOCATION_OFFSETS[1],
            "directory": list(directory), "state_before": list(state),
            "state_after": list(machine.mem_read(STATE_SEGMENT * 16, len(state))),
            "divide_error": bool(interrupts)}


def vectors(executable):
    cases = [
        (500, 500, 500, 500, 10), (1, -1, 0, 1000, 0),
        (500, 2000, -900, 2000, 10), (500, 500, 0, 0, 10),
        (65535, 500, -1000, 1000, 10), (32767, 32767, 32767, 1000, 32767),
        (500, 500, 500, 500, -1), (500, 500, 500, 500, -32768),
        (65535, -32768, 500, -32768, 10), (32768, 1000, -32768, 0, 10),
        (5, 500, 1000, 1000, 0), (32767, 500, 1000, 1000, 32767),
    ]
    for query in [0, 1]:
        for index, (quantity, aggression, balance, relief, rate) in enumerate(cases):
            state, directory = fixture()
            for field, value in [(QUANTITY, quantity), (AGGRESSIVENESS, aggression),
                                 (GROWTH_BALANCE, balance), (PRESSURE_RELIEF, relief)]:
                word(state, ACTOR_OFFSETS[1] + field, value)
            yield run(executable, f"q{query}_edge{index}", state, directory, rate=rate, query=query)
        for index, (field, value) in enumerate([
            (FLAGS, 0), (FLAGS, 1), (FLAGS, 4), (FLAGS, 13),
            (GROUP, 2), (GROUP, 0), (LOCATION, LOCATION_OFFSETS[1]),
        ]):
            state, directory = fixture()
            word(state, ACTOR_OFFSETS[1] + field, value)
            word(state, ACTOR_OFFSETS[1] + AGGRESSIVENESS, 2000)
            yield run(executable, f"q{query}_filter{index}", state, directory, query=query)
        for countdown in [1, 65535]:
            state, directory = fixture()
            yield run(executable, f"q{query}_countdown{countdown}", state, directory, countdown=countdown, query=query)
        state, directory = fixture()
        word(state, LOCATION_OFFSETS[0] + FLAGS, 4)
        yield run(executable, f"q{query}_inactive_location", state, directory, query=query)
        state, directory = fixture()
        yield run(executable, f"q{query}_empty_group", state, directory, group_mask=0, query=query)
    randomizer = random.Random(20260906)
    for index in range(80):
        state, directory = fixture()
        for offset in ACTOR_OFFSETS:
            for field in [QUANTITY, AGGRESSIVENESS, GROWTH_BALANCE, PRESSURE_RELIEF]:
                word(state, offset + field, randomizer.randrange(65536))
            word(state, offset + FLAGS, randomizer.choice([5, 5, 13, 1]))
            word(state, offset + GROUP, randomizer.choice([1, 2, 3]))
        yield run(executable, f"random{index}", state, directory,
                  rate=randomizer.randrange(65536), group_mask=randomizer.choice([1, 2, 3]), query=index % 2)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("executable", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    executable = args.executable.read_bytes()
    if hashlib.sha256(executable).hexdigest() != EXECUTABLE_SHA256:
        raise SystemExit("unsupported BLOOD2PG.EXE build; refusing fixed-offset oracle")
    results = list(vectors(executable))
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text("".join(json.dumps(item, separators=(",", ":")) + "\n" for item in results))
    print(f"wrote {len(results)} original-handler vectors ({sum(item['divide_error'] for item in results)} divide errors)")


if __name__ == "__main__":
    main()
