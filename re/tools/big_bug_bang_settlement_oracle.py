#!/usr/bin/env python3
"""Execute the complete original D5 settlement path on synthetic object graphs.

The selector, distance search, position resolver, and recursive descendant
collector all execute unchanged native instructions. No call is stubbed.
Only synthetic state and native results are saved; run with python3 -P.
"""

import argparse
import hashlib
import json
from pathlib import Path
import random
import struct

from unicorn import Uc, UC_ARCH_X86, UC_MODE_16, UC_HOOK_CODE, UC_HOOK_INTR
from unicorn.x86_const import (
    UC_X86_REG_CS, UC_X86_REG_DS, UC_X86_REG_ES, UC_X86_REG_GS,
    UC_X86_REG_SS, UC_X86_REG_SP, UC_X86_REG_SI, UC_X86_REG_IP,
)

EXECUTABLE_SHA256 = "4b65ffca3e113a1826371e3436177861640a1b7aae24caafebb4c2f7aa467834"
HANDLER = 0x7367
RETURN_IP = 0x200
SCRIPT_SEGMENT = 0x2000
GLOBAL_SEGMENT = 0x3000
STATE_SEGMENT = 0x4000
DIRECTORY_SEGMENT = 0x6000
SCRIPT_OFFSET = 64
VAR_POINTER = 0x6AEC
DIRECTORY_POINTER = 0x6AF0
ARCHETYPE = 0x6B22
HONK = 0x6B24
EXCLUDED_DESTINATION = 0x6B28
EXCLUDED_SOURCE = 0x6B2A
COUNTDOWN = 0x0CC6
QUERY_MODE = 0x6B83
RANGE_OVERRIDE = 0x6B72
FIELD_TABLE = 0x7128
FIELD_TABLE_FILE = 0x16918
FLAGS = 2
GROUP = 20
QUANTITY = 22
ACTOR_LOCATION = 24
BALANCE = 52
RELIEF = 56
LOCATION_PARENT = 20
BODY_PARENT = 22
BODY_POSITION = 24
BODY_Y = 26
SOURCE = 34
CLONE_A = 108
CLONE_B = 182
HONK_ACTOR = 256
OTHER_GROUP = 330
NESTED = 404
HOME = 478
NEAR = 504
TIE = 530
ARK_LOCATION = 556
TRASH = 582
BODIES = [608, 638, 668, 698, 728]
ARCHE = 758
STATE_SIZE = ARCHE + 36
HANDLER_ENTRIES = [HANDLER, 0x706E, 0x6F17, 0x6F52, 0x67B8, 0x6633, 0x8103, 0x685D]
CONFLICT_HANDLER = 0x70CD
ATTACK_RATE = 0x6B70
INITIAL_ATTACK_RATE = 444
GUARD_DEPTH = 0x6C2C
GUARD_TARGET = 0x6C04
FAILURE_SCRIPT_OFFSET = 384


def word(data, offset, value):
    struct.pack_into("<H", data, offset, value & 65535)


def fixture():
    state = bytearray((index * 17 + 3) % 256 for index in range(STATE_SIZE))
    objects = [(0, 1, "blood")]
    objects += [(offset, 2, name) for offset, name in zip(
        [SOURCE, CLONE_A, CLONE_B, HONK_ACTOR, OTHER_GROUP, NESTED],
        ["source", "clone_a", "clone_b", "Honk", "other", "nested"])]
    objects += [(offset, 128, name) for offset, name in zip(
        [HOME, NEAR, TIE, ARK_LOCATION, TRASH], ["home", "near", "tie", "Arche", "Trashlando"])]
    objects += [(offset, 8, f"body{index}") for index, offset in enumerate(BODIES)]
    objects.append((ARCHE, 16, "arche"))
    directory = bytearray()
    for offset, kind, name in objects:
        word(state, offset, kind)
        word(state, offset + FLAGS, 1)
        directory.extend(struct.pack("<16sHH", name.encode(), offset, 1))
    directory.extend(bytes(20))
    word(state, 6, 65535)
    for index, offset in enumerate([SOURCE, CLONE_A, CLONE_B, HONK_ACTOR, OTHER_GROUP, NESTED]):
        word(state, offset + GROUP, 2 if offset == OTHER_GROUP else 1)
        word(state, offset + QUANTITY, 500 if offset == SOURCE else 30 + index)
        word(state, offset + ACTOR_LOCATION, CLONE_A if offset == NESTED else HOME)
        word(state, offset + RELIEF, 200 + index)
        word(state, offset + BALANCE, 100 + index)
    word(state, SOURCE + FLAGS, 5)
    word(state, HOME + FLAGS, 5)
    for location, body, position in zip([HOME, NEAR, TIE, ARK_LOCATION, TRASH], BODIES,
                                       [(0, 0), (10, 0), (0, 10), (1, 0), (1000, 1000)]):
        word(state, location + LOCATION_PARENT, body)
        word(state, body + BODY_PARENT, 65535)
        word(state, body + BODY_POSITION, position[0])
        word(state, body + BODY_Y, position[1])
    word(state, ARCHE + BODY_PARENT, 65535)
    word(state, ARCHE + BODY_POSITION, 0)
    word(state, ARCHE + BODY_Y, 0)
    return state, bytes(directory)


def run(executable, name, state, directory, group=1, countdown=0, query=0, override=0, attack_rate=None):
    conflict = attack_rate is not None
    token = struct.pack("<BHH", 0xD4, group, attack_rate) if conflict else struct.pack("<BH", 0xD5, group)
    machine = Uc(UC_ARCH_X86, UC_MODE_16)
    machine.mem_map(0, 1048576)
    machine.mem_write(0, executable)
    machine.mem_write(SCRIPT_SEGMENT * 16 + SCRIPT_OFFSET, token)
    globals_before = bytearray(32768)
    struct.pack_into("<HH", globals_before, VAR_POINTER, 0, STATE_SEGMENT)
    struct.pack_into("<HH", globals_before, DIRECTORY_POINTER, 0, DIRECTORY_SEGMENT)
    for offset, value in [(ARCHETYPE, ARCHE), (HONK, HONK_ACTOR), (EXCLUDED_DESTINATION, ARK_LOCATION),
                          (EXCLUDED_SOURCE, TRASH), (COUNTDOWN, countdown)]:
        word(globals_before, offset, value)
    globals_before[QUERY_MODE] = query
    globals_before[RANGE_OVERRIDE] = override
    if conflict:
        word(globals_before, ATTACK_RATE, INITIAL_ATTACK_RATE)
        word(globals_before, GUARD_DEPTH, 2 if query else 0)
        word(globals_before, GUARD_TARGET, FAILURE_SCRIPT_OFFSET)
    globals_before[FIELD_TABLE:FIELD_TABLE + 352] = executable[FIELD_TABLE_FILE:FIELD_TABLE_FILE + 352]
    machine.mem_write(GLOBAL_SEGMENT * 16, bytes(globals_before))
    machine.mem_write(STATE_SEGMENT * 16, bytes(state))
    machine.mem_write(DIRECTORY_SEGMENT * 16, directory)
    stack_pointer = 65520
    machine.mem_write(GLOBAL_SEGMENT * 16 + stack_pointer, struct.pack("<H", RETURN_IP))
    # The original scratch lists use SS, and the nested collector reads them
    # through GS as well. Preserve that original SS=GS relationship in the oracle.
    for register, value in [(UC_X86_REG_CS, 0), (UC_X86_REG_DS, SCRIPT_SEGMENT),
                            (UC_X86_REG_GS, GLOBAL_SEGMENT), (UC_X86_REG_SS, GLOBAL_SEGMENT),
                            (UC_X86_REG_ES, 0), (UC_X86_REG_SP, stack_pointer),
                            (UC_X86_REG_SI, SCRIPT_OFFSET + 1)]:
        machine.reg_write(register, value)
    called = set()
    entries = [CONFLICT_HANDLER, 0x724E, 0x697A, *HANDLER_ENTRIES[1:]] if conflict else HANDLER_ENTRIES

    def instruction(_cpu, address, _size, _context):
        if address in entries:
            called.add(address)

    machine.hook_add(UC_HOOK_CODE, instruction)
    interrupts = []

    def interrupt(cpu, number, _context):
        interrupts.append(number)
        cpu.emu_stop()

    machine.hook_add(UC_HOOK_INTR, interrupt)
    machine.emu_start(CONFLICT_HANDLER if conflict else HANDLER, RETURN_IP, count=50000)
    assert interrupts in ([], [0]) and (conflict or not interrupts), (name, interrupts)
    branch_taken = conflict and machine.reg_read(UC_X86_REG_SI) == FAILURE_SCRIPT_OFFSET
    if not interrupts:
        assert machine.reg_read(UC_X86_REG_IP) == RETURN_IP, f"{name}: handler failed to return"
        assert machine.reg_read(UC_X86_REG_SI) == (FAILURE_SCRIPT_OFFSET if branch_taken else SCRIPT_OFFSET + len(token)), name
    globals_after = bytearray(machine.mem_read(GLOBAL_SEGMENT * 16, len(globals_before)))
    override_after = globals_after[RANGE_OVERRIDE]
    extra = {}
    if conflict:
        extra = {"attack_rate_before": INITIAL_ATTACK_RATE,
                 "attack_rate_after": struct.unpack_from("<H", globals_after, ATTACK_RATE)[0],
                 "query_mode_after": globals_after[QUERY_MODE],
                 "guard_depth_after": struct.unpack_from("<H", globals_after, GUARD_DEPTH)[0],
                 "branch_taken": branch_taken, "divide_error": bool(interrupts)}
        for start, length in [(ATTACK_RATE, 2), (QUERY_MODE, 1), (GUARD_DEPTH, 2)]:
            globals_after[start:start + length] = globals_before[start:start + length]
    for start, end in [(0x2E03, 0x2F63), (0x6C2E, 0x6E00), (RANGE_OVERRIDE, RANGE_OVERRIDE + 1)]:
        globals_after[start:end] = globals_before[start:end]
    assert globals_after == globals_before, f"{name}: unexpected global write"
    assert machine.mem_read(DIRECTORY_SEGMENT * 16, len(directory)) == directory, name
    return {"name": name, "token": list(token), "countdown": countdown, "query_mode": query,
            "range_override_before": override, "range_override_after": override_after,
            "directory": list(directory), "state_before": list(state),
            "state_after": list(machine.mem_read(STATE_SEGMENT * 16, len(state))),
            "native_handlers_called": sorted(called), **extra}


def vectors(executable):
    for query in [0, 1]:
        state, directory = fixture()
        yield run(executable, f"q{query}_nested_and_tie", state, directory, query=query)
        for index, (offset, value) in enumerate([
            (SOURCE + QUANTITY, 299), (SOURCE + QUANTITY, 300), (SOURCE + QUANTITY, 32767),
            (SOURCE + QUANTITY, 32768), (SOURCE + QUANTITY, 65535), (SOURCE + FLAGS, 1),
            (SOURCE + FLAGS, 13), (SOURCE + ACTOR_LOCATION, TRASH), (HOME + FLAGS, 4),
            (NEAR + FLAGS, 5), (NEAR + FLAGS, 0), (CLONE_A + FLAGS, 0),
            (CLONE_A + GROUP, 2), (CLONE_B + ACTOR_LOCATION, NEAR),
            (HOME + FLAGS, 1),
        ]):
            state, directory = fixture()
            word(state, offset, value)
            yield run(executable, f"q{query}_gate{index}", state, directory, query=query)
        for index, point in enumerate([(250, 0), (250, 1), (65535, 0), (32768, 32768)]):
            state, directory = fixture()
            word(state, TIE + FLAGS, 5)
            word(state, BODIES[1] + BODY_POSITION, point[0])
            word(state, BODIES[1] + BODY_Y, point[1])
            yield run(executable, f"q{query}_distance{index}", state, directory, query=query)
        for countdown in [1, 65535]:
            state, directory = fixture()
            yield run(executable, f"q{query}_clock{countdown}", state, directory, countdown=countdown, query=query, override=1)
        for group in [0, 2, 3, 65535]:
            state, directory = fixture()
            yield run(executable, f"q{query}_mask{group}", state, directory, group=group, query=query, override=1)
    randomizer = random.Random(20260907)
    for index in range(48):
        state, directory = fixture()
        for actor in [SOURCE, CLONE_A, CLONE_B, OTHER_GROUP, NESTED]:
            word(state, actor + FLAGS, randomizer.choice([1, 5, 13]))
            word(state, actor + QUANTITY, randomizer.choice([10, 299, 300, 1000, 65535]))
            word(state, actor + GROUP, randomizer.choice([1, 2, 3]))
        for location, body in zip([NEAR, TIE, ARK_LOCATION, TRASH], BODIES[1:]):
            word(state, location + FLAGS, randomizer.choice([0, 1, 1, 5]))
            word(state, body + BODY_POSITION, randomizer.choice([0, 10, 250, 300, 65535]))
            word(state, body + BODY_Y, randomizer.choice([0, 10, 300]))
        yield run(executable, f"random{index}", state, directory, group=randomizer.choice([1, 2, 3]), query=index % 2)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("executable", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    executable = args.executable.read_bytes()
    if hashlib.sha256(executable).hexdigest() != EXECUTABLE_SHA256:
        raise SystemExit("unsupported BLOOD2PG.EXE build; refusing fixed-offset oracle")
    results = list(vectors(executable))
    assert set().union(*(set(item["native_handlers_called"]) for item in results)) == set(HANDLER_ENTRIES)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text("".join(json.dumps(item, separators=(",", ":")) + "\n" for item in results))
    print(f"wrote {len(results)} native settlement cases covering all {len(HANDLER_ENTRIES)} handler/helper entries")


if __name__ == "__main__":
    main()
