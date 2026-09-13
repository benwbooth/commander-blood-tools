#!/usr/bin/env python3
"""Compare Commander Blood and Big Bug Bang dialogue/chatter coordination."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import struct
import sys
from pathlib import Path
from typing import Any

_HERE = os.path.dirname(os.path.abspath(__file__))
sys.path[:] = [path for path in sys.path if os.path.abspath(path or os.curdir) != _HERE]

import capstone
from capstone.x86 import X86_INS_JMP, X86_INS_LJMP, X86_OP_IMM
from unicorn import UC_ARCH_X86, UC_HOOK_CODE, UC_MODE_16, Uc, UcError
from unicorn.x86_const import (
    UC_X86_REG_AX,
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

ROOT = Path(__file__).resolve().parents[2]
COMMANDER_PATH = ROOT / "re/bin/BLOODPRG.EXE"
COMMANDER_FIXTURE = ROOT / "re/tools/oracle_vectors/func_b7e3_natural.json"
COMMANDER_SHA256 = "7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823"
SEQUEL_SHA256 = "4b65ffca3e113a1826371e3436177861640a1b7aae24caafebb4c2f7aa467834"

BRANCHES = {
    "commander": {
        "routine": (0xB7E3, 0xB8CD),
        "sha256": "64cde5b00846af7e05e9d3bc48fa6088a5d2381d09d16bb322768d0162eec490",
        "fields": {
            "sound": 0x0ADE,
            "mode": 0x0ADF,
            "cooldown": 0x0B2F,
            "delay": 0x0B33,
            "delay_base": 0x0BBD,
            "delay_limit": 0x0BBE,
            "last_clip": 0x0C4D,
            "clip_count": 0x0C53,
            "seed": 0x0C55,
            "menu_pending": 0x0CF9,
            "dialogue_armed": 0x0CFA,
            "voice_requested": 0x0CFB,
        },
        "dictionary_pointer": 0x6728,
        "word_list_pointer": 0x674A,
        "play_helper": 0xB8CD,
        "primary_play_return": 0xB898,
        "voice_play_return": 0xB8C3,
        "prng": (0x01CE, 0x0B02),
        "prng_return": 0xB8B3,
    },
    "sequel": {
        "routine": (0xCF73, 0xD05D),
        "sha256": "cdb50354c0ee003a28a04f0c4770b9afb0354af754378c211f9ae104d9c75fe0",
        "fields": {
            "sound": 0x0CE7,
            "mode": 0x0CE8,
            "cooldown": 0x0D39,
            "delay": 0x0D3D,
            "delay_base": 0x0DC7,
            "delay_limit": 0x0DC8,
            "last_clip": 0x0E57,
            "clip_count": 0x0E5D,
            "seed": 0x0E5F,
            "menu_pending": 0x0F47,
            "dialogue_armed": 0x0F48,
            "voice_requested": 0x0F49,
        },
        "dictionary_pointer": 0x6AFC,
        "word_list_pointer": 0x6B1A,
        "play_helper": 0xD05D,
        "primary_play_return": 0xD028,
        "voice_play_return": 0xD053,
        "prng": (0x01E6, 0x0B03),
        "prng_return": 0xD043,
    },
}

MACHINE_SIZE = 0xF0000
SEGMENT_SIZE = 0x10000
GAME_SEGMENT = 0x3000
SPLIT_GS_SEGMENT = 0x5000
DICTIONARY_SEGMENT = 0x7000
EXTRA_SEGMENT = 0x9000
STACK_SEGMENT = 0xB000
UNOWNED_SEGMENT = 0xD000
WORD_LIST_OFFSET = 0x7500
STACK_POINTER = 0xFF00
RETURN_IP = 0x6F00
STACK_SENTINEL = bytes.fromhex("69965aa5c33c")

DICTIONARY_WORDS = {
    0x20: b"A\x80\xff\0",
    0x40: b"Commander\0",
    0x60: b"\0",
}

REGISTER_IDS = {
    "eax": UC_X86_REG_EAX,
    "ebx": UC_X86_REG_EBX,
    "ecx": UC_X86_REG_ECX,
    "edx": UC_X86_REG_EDX,
    "esi": UC_X86_REG_ESI,
    "edi": UC_X86_REG_EDI,
    "ebp": UC_X86_REG_EBP,
    "sp": UC_X86_REG_SP,
    "ds": UC_X86_REG_DS,
    "es": UC_X86_REG_ES,
    "fs": UC_X86_REG_FS,
    "gs": UC_X86_REG_GS,
    "ss": UC_X86_REG_SS,
}
FLAG_MASKS = {
    "cf": 0x0001,
    "pf": 0x0004,
    "af": 0x0010,
    "zf": 0x0040,
    "sf": 0x0080,
    "of": 0x0800,
}


def case(
    name: str,
    sound: int,
    mode: int,
    menu_pending: int,
    dialogue_armed: int,
    voice_requested: int,
    cooldown: int,
    delay: int,
    seed: int,
    last_clip: int,
    clip_count: int,
    delay_base: int,
    delay_limit: int,
    words: list[int],
    prng: list[int],
    *,
    split: bool = False,
) -> dict[str, Any]:
    return locals()


CASES = [
    case("sound_disabled", 0, 0, 1, 1, 1, 0, 0, 7, 3, 12, 4, 20, [0x20, 0xFFFF], []),
    case(
        "mode_suppresses_dialogue",
        1,
        1,
        1,
        1,
        0,
        0,
        0,
        19,
        5,
        9,
        3,
        18,
        [0x20, 0xFFFF],
        [],
    ),
    case(
        "mode_allows_chatter",
        1,
        1,
        1,
        1,
        1,
        0,
        0,
        21,
        5,
        10,
        4,
        20,
        [0x20, 0xFFFF],
        [3],
    ),
    case("hash_empty", 1, 0, 1, 0, 0, 0, 0x1234, 0x4567, 8, 16, 5, 24, [0], []),
    case(
        "hash_signed_words",
        1,
        0,
        1,
        0,
        0,
        2,
        0x4321,
        0x9999,
        7,
        17,
        6,
        24,
        [0x20, 0x40, 0x60, 0xFFFF],
        [],
    ),
    case(
        "split_ds_gs_hash",
        1,
        0,
        1,
        0,
        0,
        3,
        0x2222,
        0x3333,
        4,
        14,
        7,
        25,
        [0x40, 0x20, 0],
        [],
        split=True,
    ),
    case("armed_delay_busy", 1, 0, 0, 1, 1, 2, 3, 7, 2, 20, 4, 20, [0], []),
    case("armed_primary", 1, 0, 0, 1, 0, 0, 0, 7, 9, 32, 4, 20, [0], []),
    case(
        "primary_range_and_duplicate_retry",
        1,
        0,
        0,
        1,
        0,
        0,
        0,
        40,
        2,
        10,
        18,
        20,
        [0],
        [],
    ),
    case("voice_prng_reroll", 1, 0, 0, 0, 1, 0, 0, 11, 3, 18, 3, 17, [0], [3, 3, 6]),
    case("primary_then_voice", 1, 0, 0, 1, 1, 0, 0, 7, 9, 32, 4, 20, [0], [2, 4]),
]


def image(seed: int) -> bytearray:
    return bytearray(
        (offset * 17 + (offset >> 8) * 13 + seed * 29 + 0x53) & 0xFF
        for offset in range(SEGMENT_SIZE)
    )


def put_byte(memory: bytearray, offset: int, value: int) -> None:
    memory[offset] = value & 0xFF


def put_word(memory: bytearray, offset: int, value: int) -> None:
    struct.pack_into("<H", memory, offset, value & 0xFFFF)


def read_byte(memory: bytes, offset: int) -> int:
    return memory[offset]


def read_word(memory: bytes, offset: int) -> int:
    return struct.unpack_from("<H", memory, offset)[0]


def first_difference(actual: bytes, expected: bytes) -> str:
    for offset, (left, right) in enumerate(zip(actual, expected, strict=True)):
        if left != right:
            return f"{offset:#x}: {left:#x} != {right:#x}"
    return "length"


def conditional_edges(body: bytes, start: int) -> set[tuple[int, int]]:
    decoder = capstone.Cs(capstone.CS_ARCH_X86, capstone.CS_MODE_16)
    decoder.detail = True
    result = set()
    for instruction in decoder.disasm(body, start):
        if capstone.CS_GRP_JUMP not in instruction.groups:
            continue
        if instruction.id in (X86_INS_JMP, X86_INS_LJMP):
            continue
        immediate = next(
            (
                operand.imm
                for operand in instruction.operands
                if operand.type == X86_OP_IMM
            ),
            None,
        )
        assert immediate is not None
        source = instruction.address - start
        result.add((source, int(immediate) - start))
        result.add((source, instruction.address + instruction.size - start))
    return result


def initialize_state(
    memory: bytearray,
    fields: dict[str, int],
    vector: dict[str, Any],
) -> None:
    put_byte(memory, fields["sound"], vector["sound"])
    put_byte(memory, fields["mode"], vector["mode"])
    put_byte(memory, fields["cooldown"], vector["cooldown"])
    put_word(memory, fields["delay"], vector["delay"])
    put_byte(memory, fields["delay_base"], vector["delay_base"])
    put_byte(memory, fields["delay_limit"], vector["delay_limit"])
    put_word(memory, fields["last_clip"], vector["last_clip"])
    put_word(memory, fields["clip_count"], vector["clip_count"])
    put_word(memory, fields["seed"], vector["seed"])
    put_byte(memory, fields["menu_pending"], vector["menu_pending"])
    put_byte(memory, fields["dialogue_armed"], vector["dialogue_armed"])
    put_byte(memory, fields["voice_requested"], vector["voice_requested"])


def execute(
    executable: bytes,
    branch_name: str,
    vector: dict[str, Any],
    expected_row: dict[str, Any],
    case_index: int,
) -> tuple[dict[str, Any], set[tuple[int, int]]]:
    branch = BRANCHES[branch_name]
    fields = branch["fields"]
    start, stop = branch["routine"]
    split = bool(vector["split"])
    gs_segment = SPLIT_GS_SEGMENT if split else GAME_SEGMENT

    game = image(case_index + 1)
    split_gs = image(case_index + 17)
    dictionary = image(case_index + 33)
    extra = image(case_index + 49)
    stack = image(case_index + 65)
    unowned = image(case_index + 81)
    initialize_state(game, fields, vector)
    if split:
        put_word(split_gs, fields["seed"], vector["seed"])
        put_byte(split_gs, fields["dialogue_armed"], vector["dialogue_armed"])

    struct.pack_into(
        "<HH",
        game,
        int(branch["dictionary_pointer"]),
        0,
        DICTIONARY_SEGMENT,
    )
    struct.pack_into(
        "<HH",
        game,
        int(branch["word_list_pointer"]),
        WORD_LIST_OFFSET,
        GAME_SEGMENT,
    )
    words = b"".join(struct.pack("<H", value) for value in vector["words"])
    game[WORD_LIST_OFFSET : WORD_LIST_OFFSET + len(words)] = words
    for offset, contents in DICTIONARY_WORDS.items():
        dictionary[offset : offset + len(contents)] = contents

    expected_game = bytearray(game)
    expected_split_gs = bytearray(split_gs)
    active_hash = (
        bool(vector["sound"] & 1)
        and not bool(vector["mode"] & 1)
        and bool(vector["menu_pending"] & 1)
    )
    if active_hash:
        put_byte(expected_game, fields["menu_pending"], 0)
        target = expected_split_gs if split else expected_game
        put_word(target, fields["seed"], expected_row["dialogue_seed_after"])
        put_byte(target, fields["dialogue_armed"], 1)
    if not split:
        put_word(expected_game, fields["seed"], expected_row["dialogue_seed_after"])
    put_word(expected_game, fields["delay"], expected_row["dialogue_delay_after"])
    put_word(expected_game, fields["last_clip"], expected_row["last_clip_after"])
    put_byte(expected_game, fields["cooldown"], expected_row["cooldown_after"])

    initial = {
        UC_X86_REG_EAX: 0xA1A11234 + case_index,
        UC_X86_REG_EBX: 0xB2B22345,
        UC_X86_REG_ECX: 0xC3C33456,
        UC_X86_REG_EDX: 0xD4D44567,
        UC_X86_REG_ESI: 0xE5E55678,
        UC_X86_REG_EDI: 0xF6F66789,
        UC_X86_REG_EBP: 0x9797789A,
        UC_X86_REG_SP: STACK_POINTER,
        UC_X86_REG_CS: 0,
        UC_X86_REG_DS: GAME_SEGMENT,
        UC_X86_REG_ES: EXTRA_SEGMENT,
        UC_X86_REG_FS: 0xA000,
        UC_X86_REG_GS: gs_segment,
        UC_X86_REG_SS: STACK_SEGMENT,
        UC_X86_REG_EFLAGS: 0x0202,
    }
    stack[STACK_POINTER : STACK_POINTER + 10] = (
        struct.pack("<HH", RETURN_IP, 0) + STACK_SENTINEL
    )
    expected_stack = bytearray(stack)
    struct.pack_into(
        "<HHHHHHHHH",
        expected_stack,
        0xFEEE,
        EXTRA_SEGMENT,
        GAME_SEGMENT,
        initial[UC_X86_REG_ESI] & 0xFFFF,
        initial[UC_X86_REG_EDI] & 0xFFFF,
        initial[UC_X86_REG_EBP] & 0xFFFF,
        initial[UC_X86_REG_EDX] & 0xFFFF,
        initial[UC_X86_REG_ECX] & 0xFFFF,
        initial[UC_X86_REG_EBX] & 0xFFFF,
        initial[UC_X86_REG_EAX] & 0xFFFF,
    )

    play_helper = int(branch["play_helper"])
    prng_segment, prng_offset = branch["prng"]
    prng_address = int(prng_segment) * 16 + int(prng_offset)
    patched_executable = bytearray(executable)
    patched_executable[play_helper] = 0xCB
    patched_executable[prng_address] = 0xCB

    machine = Uc(UC_ARCH_X86, UC_MODE_16)
    machine.mem_map(0, MACHINE_SIZE)
    machine.mem_write(0, bytes(patched_executable))
    for segment, contents in (
        (GAME_SEGMENT, game),
        (DICTIONARY_SEGMENT, dictionary),
        (EXTRA_SEGMENT, extra),
        (STACK_SEGMENT, stack),
        (UNOWNED_SEGMENT, unowned),
    ):
        machine.mem_write(segment * 16, bytes(contents))
    if split:
        machine.mem_write(SPLIT_GS_SEGMENT * 16, bytes(split_gs))
    for register, value in initial.items():
        machine.reg_write(register, value)

    body = executable[start:stop]
    expected_edges = conditional_edges(body, start)
    conditional_sources = {source for source, _ in expected_edges}
    covered_edges: set[tuple[int, int]] = set()
    previous_source: int | None = None
    reached_return = False
    helper_events: list[dict[str, int | str]] = []
    prng_index = 0

    def instruction(cpu: Uc, address: int, _size: int, _context: object) -> None:
        nonlocal previous_source, prng_index, reached_return
        if address == RETURN_IP and cpu.reg_read(UC_X86_REG_CS) == 0:
            reached_return = True
            cpu.emu_stop()
            return
        if address == play_helper and cpu.reg_read(UC_X86_REG_CS) == 0:
            clip = cpu.reg_read(UC_X86_REG_AX)
            return_ip, return_cs = struct.unpack(
                "<HH", cpu.mem_read(STACK_SEGMENT * 16 + 0xFEEA, 4)
            )
            expected_return = (
                int(branch["primary_play_return"])
                if clip & 0x8000
                else int(branch["voice_play_return"])
            )
            assert (return_ip, return_cs) == (expected_return, 0), (
                vector["name"],
                branch_name,
                "play frame",
                return_ip,
                return_cs,
            )
            assert cpu.reg_read(UC_X86_REG_DS) == GAME_SEGMENT
            assert cpu.reg_read(UC_X86_REG_SP) == 0xFEEA
            helper_events.append({"kind": "play", "argument": clip})
            struct.pack_into("<HH", expected_stack, 0xFEEA, return_ip, return_cs)
            previous_source = None
            return
        if address == prng_address:
            return_ip, return_cs = struct.unpack(
                "<HH", cpu.mem_read(STACK_SEGMENT * 16 + 0xFEEA, 4)
            )
            assert (return_ip, return_cs) == (int(branch["prng_return"]), 0), (
                vector["name"],
                branch_name,
                "PRNG frame",
                return_ip,
                return_cs,
            )
            assert cpu.reg_read(UC_X86_REG_AX) == 10
            assert cpu.reg_read(UC_X86_REG_DS) == GAME_SEGMENT
            assert cpu.reg_read(UC_X86_REG_SP) == 0xFEEA
            results = vector["prng"]
            assert prng_index < len(results), (vector["name"], "unexpected PRNG call")
            result = int(results[prng_index])
            prng_index += 1
            helper_events.append({"kind": "prng", "argument": 10, "result": result})
            struct.pack_into("<HH", expected_stack, 0xFEEA, return_ip, return_cs)
            cpu.reg_write(UC_X86_REG_AX, result)
            previous_source = None
            return
        assert start <= address < stop, (vector["name"], branch_name, hex(address))
        offset = address - start
        if previous_source is not None:
            covered_edges.add((previous_source, offset))
        previous_source = offset if offset in conditional_sources else None

    machine.hook_add(UC_HOOK_CODE, instruction)
    try:
        machine.emu_start(start, 0, count=50_000)
    except UcError as error:
        raise RuntimeError(
            f"{vector['name']} {branch_name}: failed at "
            f"{machine.reg_read(UC_X86_REG_CS):#x}:"
            f"{machine.reg_read(UC_X86_REG_IP):#x}"
        ) from error
    assert reached_return, (vector["name"], branch_name, "did not return")
    assert prng_index == len(vector["prng"]), (
        vector["name"],
        branch_name,
        prng_index,
        len(vector["prng"]),
    )
    assert [
        event["argument"] for event in helper_events if event["kind"] == "play"
    ] == expected_row["play_calls"], (vector["name"], branch_name, helper_events)

    for label, segment, expected in (
        ("game", GAME_SEGMENT, expected_game),
        ("dictionary", DICTIONARY_SEGMENT, dictionary),
        ("extra", EXTRA_SEGMENT, extra),
        ("stack", STACK_SEGMENT, expected_stack),
        ("unowned", UNOWNED_SEGMENT, unowned),
    ):
        actual = bytes(machine.mem_read(segment * 16, SEGMENT_SIZE))
        assert actual == bytes(expected), (
            vector["name"],
            branch_name,
            label,
            first_difference(actual, bytes(expected)),
        )
    if split:
        actual = bytes(machine.mem_read(SPLIT_GS_SEGMENT * 16, SEGMENT_SIZE))
        assert actual == bytes(expected_split_gs), (
            vector["name"],
            branch_name,
            "split GS",
            first_difference(actual, bytes(expected_split_gs)),
        )
    assert bytes(machine.mem_read(0, len(executable))) == bytes(patched_executable), (
        vector["name"],
        branch_name,
        "patched executable changed",
    )

    expected_registers = {
        "eax": initial[UC_X86_REG_EAX],
        "ebx": initial[UC_X86_REG_EBX],
        "ecx": initial[UC_X86_REG_ECX],
        "edx": initial[UC_X86_REG_EDX],
        "esi": initial[UC_X86_REG_ESI],
        "edi": initial[UC_X86_REG_EDI],
        "ebp": initial[UC_X86_REG_EBP],
        "sp": STACK_POINTER + 4,
        "ds": GAME_SEGMENT,
        "es": EXTRA_SEGMENT,
        "fs": initial[UC_X86_REG_FS],
        "gs": gs_segment,
        "ss": STACK_SEGMENT,
    }
    actual_registers = {
        name: machine.reg_read(register) for name, register in REGISTER_IDS.items()
    }
    assert actual_registers == expected_registers, (
        vector["name"],
        branch_name,
        actual_registers,
        expected_registers,
    )
    flags = machine.reg_read(UC_X86_REG_EFLAGS)
    flags_after = {name: bool(flags & mask) for name, mask in FLAG_MASKS.items()}
    assert (machine.reg_read(UC_X86_REG_CS), machine.reg_read(UC_X86_REG_IP)) == (
        0,
        RETURN_IP,
    )

    actual_game = bytes(machine.mem_read(GAME_SEGMENT * 16, SEGMENT_SIZE))
    actual_gs = (
        bytes(machine.mem_read(SPLIT_GS_SEGMENT * 16, SEGMENT_SIZE))
        if split
        else actual_game
    )
    assert read_word(actual_gs, fields["seed"]) == expected_row["dialogue_seed_after"]
    assert (
        read_word(actual_game, fields["delay"]) == expected_row["dialogue_delay_after"]
    )
    assert (
        read_word(actual_game, fields["last_clip"]) == expected_row["last_clip_after"]
    )
    assert read_byte(actual_game, fields["cooldown"]) == expected_row["cooldown_after"]

    row = dict(expected_row)
    row.update(
        {
            "flags_after": flags_after,
            "helper_events": helper_events,
            "registers_after": actual_registers,
            "return": "far",
        }
    )
    return row, covered_edges


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("sequel_executable", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()

    commander = COMMANDER_PATH.read_bytes()
    sequel = args.sequel_executable.read_bytes()
    for name, executable, expected in (
        ("Commander", commander, COMMANDER_SHA256),
        ("BBB", sequel, SEQUEL_SHA256),
    ):
        digest = hashlib.sha256(executable).hexdigest()
        if digest != expected:
            raise SystemExit(f"unsupported {name} executable SHA-256 {digest}")
    expected_rows = json.loads(COMMANDER_FIXTURE.read_text())
    assert len(CASES) == len(expected_rows) == 11
    assert [vector["name"] for vector in CASES] == [
        row["name"] for row in expected_rows
    ]

    rows = []
    coverage = {branch_name: set() for branch_name in BRANCHES}
    for case_index, (vector, expected_row) in enumerate(
        zip(CASES, expected_rows, strict=True)
    ):
        commander_row, commander_edges = execute(
            commander, "commander", vector, expected_row, case_index
        )
        sequel_row, sequel_edges = execute(
            sequel, "sequel", vector, expected_row, case_index
        )
        assert commander_row == sequel_row, (vector["name"], commander_row, sequel_row)
        coverage["commander"].update(commander_edges)
        coverage["sequel"].update(sequel_edges)
        rows.append(sequel_row)

    for branch_name, executable in (("commander", commander), ("sequel", sequel)):
        branch = BRANCHES[branch_name]
        start, stop = branch["routine"]
        body = executable[start:stop]
        digest = hashlib.sha256(body).hexdigest()
        assert digest == branch["sha256"], (branch_name, digest)
        expected_edges = conditional_edges(body, start)
        assert coverage[branch_name] == expected_edges, (
            branch_name,
            "conditional coverage",
            sorted(expected_edges - coverage[branch_name]),
            sorted(coverage[branch_name] - expected_edges),
        )

    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(
        "".join(
            json.dumps(row, sort_keys=True, separators=(",", ":")) + "\n"
            for row in rows
        )
    )
    print(
        f"verified {len(rows)} dual audio-event cases and "
        f"{len(coverage['sequel'])} conditional edges across "
        f"{len({source for source, _ in coverage['sequel']})} branch sites"
    )


if __name__ == "__main__":
    main()
