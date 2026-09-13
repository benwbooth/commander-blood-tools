#!/usr/bin/env python3
"""Verify BBB's relocated main-loop HUD refresh."""

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

from unicorn import (  # noqa: E402
    UC_ARCH_X86,
    UC_HOOK_CODE,
    UC_HOOK_INSN,
    UC_HOOK_MEM_WRITE,
    UC_MODE_16,
    Uc,
)
from unicorn.x86_const import (  # noqa: E402
    UC_X86_INS_OUT,
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
DEFAULT_EXECUTABLE = ROOT / "output/big-bug-bang/disc/BLOOD2PG.EXE"
DEFAULT_OUTPUT = ROOT / "re/tools/oracle_vectors/big_bug_bang_hud_refresh.json"
COMMANDER_FIXTURE = ROOT / "re/tools/oracle_vectors/func_1a93_natural.json"
EXECUTABLE_SHA256 = "4b65ffca3e113a1826371e3436177861640a1b7aae24caafebb4c2f7aa467834"
COMMANDER_FIXTURE_SHA256 = (
    "5a50f6c275d32e697498205b01a9254437fafd95c8eeea55212788a97b0f2b12"
)

ENTRY = 0x1C55
END = 0x1C95
BODY_SHA256 = "c211a631b03765a60bfc469a513a0d1d21b15c0ccb8018463bdb9cb54c9ccfa0"
GATE_OFFSET = 0x0CE8
SCREEN_POINTER_OFFSET = 0x55ED
TEXT_OFFSET = 0x01AE
COMMANDER_TEXT_OFFSET = 0x0166
CLEAR_DISPLACEMENT = 0x1D2E
CLEAR_WIDTH = 20
CLEAR_ROWS = 14
ROW_ADVANCE = 0x003C

RENDERER_SEGMENT = 0x02B1
RENDERER_OFFSET = 0x0498
RENDERER_ADDRESS = RENDERER_SEGMENT * 16 + RENDERER_OFFSET
COMMANDER_RENDERER_SEGMENT = 0x0299
RETRACE_SEGMENT = 0
RETRACE_OFFSET = 0x05D2
RETRACE_ADDRESS = RETRACE_OFFSET
COMMANDER_RETRACE_OFFSET = 0x05D7
RENDERER_RETURN_OFFSET = 0x1C8D
RETRACE_RETURN_OFFSET = 0x1C92
RENDERER_FLAGS = 0x0246
RETRACE_FLAGS = 0x0297
DEFINED_FLAG_MASK = 0x0CC5

DATA_SEGMENT = 0x3000
FS_SEGMENT = 0x2000
GAME_SEGMENT = 0x7000
STACK_SEGMENT = 0x9000
SEGMENT_SIZE = 0x10000
CALLER_SP = 0xFF00
RETURN_OFFSET = 0x2800
RETURN_ADDRESS = RETURN_OFFSET
STACK_SENTINEL = bytes.fromhex("5aa596698778c33c")

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


def seeded_segment(case_index: int, multiplier: int, salt: int) -> bytearray:
    return bytearray(
        (offset * multiplier + (offset >> 8) * 13 + case_index * 29 + salt) & 0xFF
        for offset in range(SEGMENT_SIZE)
    )


def write_wrapped(target: bytearray, offset: int, data: bytes) -> None:
    for index, value in enumerate(data):
        target[(offset + index) & 0xFFFF] = value


def physical_addresses(segment: int, offset: int, size: int) -> set[int]:
    return {segment * 16 + ((offset + index) & 0xFFFF) for index in range(size)}


def snapshot_registers(machine: Uc) -> dict[str, int]:
    result = {
        name: machine.reg_read(register) for name, register in GENERAL_REGISTERS.items()
    }
    result.update(
        {
            name: machine.reg_read(register)
            for name, register in SEGMENT_REGISTERS.items()
        }
    )
    return result


def commander_vectors() -> list[dict[str, Any]]:
    digest = hashlib.sha256(COMMANDER_FIXTURE.read_bytes()).hexdigest()
    assert digest == COMMANDER_FIXTURE_SHA256, digest
    return json.loads(COMMANDER_FIXTURE.read_text())


def vectors(executable: bytes) -> list[dict[str, Any]]:
    rows: list[dict[str, Any]] = []
    for case_index, expected in enumerate(commander_vectors()):
        name = expected["name"]
        gate = expected["gate"]
        enabled = bool(gate & 1)
        backward = expected["direction"] == "backward"
        screen_offset = expected["screen_pointer"]["offset"]
        screen_segment = expected["screen_pointer"]["segment"]
        incoming_es_segment = 0x4000 + case_index
        initial = {
            "eax": 0xA5A51234 + case_index,
            "ebx": 0xB6B62468 + case_index,
            "ecx": 0xC7C7369C + case_index,
            "edx": 0xD8D855AA + case_index,
            "esi": 0xE9E96789 + case_index,
            "edi": 0xFAFA789A + case_index,
            "ebp": 0x0B0B1357 + case_index,
            "ds": DATA_SEGMENT,
            "es": incoming_es_segment,
            "fs": FS_SEGMENT,
            "gs": GAME_SEGMENT,
            "ss": STACK_SEGMENT,
        }

        data_before = seeded_segment(case_index, 17, 0x21)
        data_before[GATE_OFFSET] = gate
        write_wrapped(
            data_before,
            SCREEN_POINTER_OFFSET,
            struct.pack("<HH", screen_offset, screen_segment),
        )
        text = b"BBB HUD STATUS\x00" + bytes((0xA5,)) * 5
        write_wrapped(data_before, TEXT_OFFSET, text)
        incoming_es_before = bytes(seeded_segment(case_index, 19, 0x31))
        fs_before = bytes(seeded_segment(case_index, 23, 0x43))
        game_before = seeded_segment(case_index, 29, 0x59)
        game_before[GATE_OFFSET] = gate ^ 0xFF
        write_wrapped(
            game_before,
            SCREEN_POINTER_OFFSET,
            struct.pack("<HH", screen_offset ^ 0xA5A5, incoming_es_segment),
        )
        write_wrapped(game_before, TEXT_OFFSET, bytes(value ^ 0xFF for value in text))

        screen_before = bytes(
            (index * 37 + case_index * 19 + 5) & 0xFF for index in range(SEGMENT_SIZE)
        )
        screen_expected = bytearray(screen_before)
        clear_start = (screen_offset + CLEAR_DISPLACEMENT) & 0xFFFF
        cursor = clear_start
        allowed_addresses: set[int] = set()
        if enabled:
            direction = -1 if backward else 1
            for _row in range(CLEAR_ROWS):
                for _column in range(CLEAR_WIDTH):
                    screen_expected[cursor] = 0
                    allowed_addresses |= physical_addresses(screen_segment, cursor, 1)
                    cursor = (cursor + direction) & 0xFFFF
                cursor = (cursor + ROW_ADVANCE) & 0xFFFF
        final_di = cursor if enabled else initial["edi"] & 0xFFFF
        assert clear_start == expected["clear_start"], name
        assert final_di == expected["final_di"], name
        assert (
            hashlib.sha256(bytes(screen_expected)).hexdigest()
            == expected["framebuffer_sha256"]
        ), name

        stack_before = seeded_segment(case_index, 31, 0x67)
        write_wrapped(stack_before, CALLER_SP, struct.pack("<H", RETURN_OFFSET))
        write_wrapped(stack_before, CALLER_SP + 2, STACK_SENTINEL)
        stack_expected = bytearray(stack_before)
        write_wrapped(
            stack_expected,
            CALLER_SP - 2,
            struct.pack("<H", initial["esi"] & 0xFFFF),
        )
        write_wrapped(stack_expected, CALLER_SP - 4, struct.pack("<H", initial["es"]))
        allowed_addresses |= physical_addresses(STACK_SEGMENT, CALLER_SP - 4, 4)
        if enabled:
            write_wrapped(
                stack_expected, CALLER_SP - 10, struct.pack("<H", RETRACE_FLAGS)
            )
            write_wrapped(
                stack_expected,
                CALLER_SP - 8,
                struct.pack("<HH", RETRACE_RETURN_OFFSET, 0),
            )
            allowed_addresses |= physical_addresses(STACK_SEGMENT, CALLER_SP - 10, 6)

        renderer_stub = b"\x68" + struct.pack("<H", RENDERER_FLAGS) + b"\x9d\xcb"
        retrace_stub = b"\x68" + struct.pack("<H", RETRACE_FLAGS) + b"\x9d\xcb"
        machine = Uc(UC_ARCH_X86, UC_MODE_16)
        machine.mem_map(0, 0x100000)
        machine.mem_write(0, executable)
        machine.mem_write(RETURN_ADDRESS, b"\xcc")
        machine.mem_write(RENDERER_ADDRESS, renderer_stub)
        machine.mem_write(RETRACE_ADDRESS, retrace_stub)
        module_before = bytes(machine.mem_read(0, len(executable)))
        machine.mem_write(FS_SEGMENT * 16, fs_before)
        machine.mem_write(DATA_SEGMENT * 16, bytes(data_before))
        machine.mem_write(incoming_es_segment * 16, incoming_es_before)
        machine.mem_write(screen_segment * 16, screen_before)
        machine.mem_write(GAME_SEGMENT * 16, bytes(game_before))
        machine.mem_write(STACK_SEGMENT * 16, bytes(stack_before))
        for register_name, register in GENERAL_REGISTERS.items():
            machine.reg_write(register, initial[register_name])
        for register_name, register in SEGMENT_REGISTERS.items():
            machine.reg_write(register, initial[register_name])
        machine.reg_write(UC_X86_REG_CS, 0)
        machine.reg_write(UC_X86_REG_SP, CALLER_SP)
        machine.reg_write(UC_X86_REG_EFLAGS, 0x0202 | (0x0400 if backward else 0))

        calls: list[dict[str, Any]] = []
        port_writes: list[list[int]] = []
        reached_return: list[int] = []
        observed_write_addresses: set[int] = set()

        def instruction(cpu: Uc, address: int, _size: int, _data: object) -> None:
            if address == RETURN_ADDRESS:
                reached_return.append(address)
                cpu.emu_stop()
                return
            if address not in (RENDERER_ADDRESS, RETRACE_ADDRESS):
                assert (
                    ENTRY <= address < END
                    or RENDERER_ADDRESS
                    <= address
                    < RENDERER_ADDRESS + len(renderer_stub)
                    or RETRACE_ADDRESS <= address < RETRACE_ADDRESS + len(retrace_stub)
                ), hex(address)
                return

            is_renderer = address == RENDERER_ADDRESS
            raw_call = {
                "callee": (
                    "planar_ui_text_render_10row"
                    if is_renderer
                    else "video_retrace_phase_wait"
                ),
                "eax": cpu.reg_read(UC_X86_REG_EAX),
                "ebx": cpu.reg_read(UC_X86_REG_EBX),
                "ecx": cpu.reg_read(UC_X86_REG_ECX),
                "edx": cpu.reg_read(UC_X86_REG_EDX),
                "esi": cpu.reg_read(UC_X86_REG_ESI),
                "edi": cpu.reg_read(UC_X86_REG_EDI),
                "ds": cpu.reg_read(UC_X86_REG_DS),
                "es": cpu.reg_read(UC_X86_REG_ES),
                "gs": cpu.reg_read(UC_X86_REG_GS),
                "ss": cpu.reg_read(UC_X86_REG_SS),
                "sp": cpu.reg_read(UC_X86_REG_SP),
                "cs": cpu.reg_read(UC_X86_REG_CS),
                "ip": cpu.reg_read(UC_X86_REG_IP),
                "flags": cpu.reg_read(UC_X86_REG_EFLAGS) & 0x0CD7,
            }
            expected_call = expected["calls"][len(calls)]
            expected_raw = dict(expected_call)
            expected_raw["esi"] = (expected_raw["esi"] & 0xFFFF0000) | TEXT_OFFSET
            expected_raw["cs"] = RENDERER_SEGMENT if is_renderer else RETRACE_SEGMENT
            expected_raw["ip"] = RENDERER_OFFSET if is_renderer else RETRACE_OFFSET
            assert raw_call == expected_raw, (name, raw_call, expected_raw)

            return_offset = (
                RENDERER_RETURN_OFFSET if is_renderer else RETRACE_RETURN_OFFSET
            )
            stack_words = struct.unpack(
                "<5H", cpu.mem_read(STACK_SEGMENT * 16 + CALLER_SP - 8, 10)
            )
            assert stack_words == (
                return_offset,
                0,
                initial["es"],
                initial["esi"] & 0xFFFF,
                RETURN_OFFSET,
            ), (name, stack_words)
            assert bytes(cpu.mem_read(screen_segment * 16, SEGMENT_SIZE)) == bytes(
                screen_expected
            ), name

            normalized_call = dict(raw_call)
            normalized_call["esi"] = (
                normalized_call["esi"] & 0xFFFF0000
            ) | COMMANDER_TEXT_OFFSET
            normalized_call["cs"] = COMMANDER_RENDERER_SEGMENT if is_renderer else 0
            normalized_call["ip"] = (
                RENDERER_OFFSET if is_renderer else COMMANDER_RETRACE_OFFSET
            )
            assert normalized_call == expected_call, name
            calls.append(normalized_call)

        def write_hook(
            _cpu: Uc,
            _access: int,
            address: int,
            size: int,
            _value: int,
            _data: object,
        ) -> None:
            addresses = set(range(address, address + size))
            assert addresses <= allowed_addresses, hex(address)
            observed_write_addresses.update(addresses)

        def output_port(
            _cpu: Uc, port: int, size: int, value: int, _data: object
        ) -> None:
            port_writes.append([port, size, value])

        machine.hook_add(UC_HOOK_CODE, instruction)
        machine.hook_add(UC_HOOK_MEM_WRITE, write_hook)
        machine.hook_add(UC_HOOK_INSN, output_port, None, 1, 0, UC_X86_INS_OUT)
        machine.emu_start(ENTRY, 0, count=5_000)

        assert reached_return == [RETURN_ADDRESS], name
        assert observed_write_addresses == allowed_addresses, name
        assert calls == expected["calls"], name
        assert port_writes == expected["port_writes"], name
        expected_registers = dict(initial)
        if enabled:
            expected_registers["eax"] = (initial["eax"] & 0xFFFF0000) | 0x0FE8
            expected_registers["ebx"] = (initial["ebx"] & 0xFFFF0000) | 0x0087
            expected_registers["ecx"] &= 0xFFFF0000
            expected_registers["edx"] = (initial["edx"] & 0xFFFF0000) | 0x0060
            expected_registers["edi"] = (initial["edi"] & 0xFFFF0000) | final_di
        assert snapshot_registers(machine) == expected_registers, name
        assert machine.reg_read(UC_X86_REG_CS) == 0
        assert machine.reg_read(UC_X86_REG_IP) == RETURN_OFFSET
        assert machine.reg_read(UC_X86_REG_SP) == CALLER_SP + 2
        final_flags = machine.reg_read(UC_X86_REG_EFLAGS) & DEFINED_FLAG_MASK
        assert final_flags == expected["final_flags"], name
        assert bytes(machine.mem_read(0, len(executable))) == module_before
        assert bytes(machine.mem_read(FS_SEGMENT * 16, SEGMENT_SIZE)) == fs_before
        assert bytes(machine.mem_read(DATA_SEGMENT * 16, SEGMENT_SIZE)) == bytes(
            data_before
        )
        assert (
            bytes(machine.mem_read(incoming_es_segment * 16, SEGMENT_SIZE))
            == incoming_es_before
        )
        assert bytes(machine.mem_read(screen_segment * 16, SEGMENT_SIZE)) == bytes(
            screen_expected
        )
        assert bytes(machine.mem_read(GAME_SEGMENT * 16, SEGMENT_SIZE)) == bytes(
            game_before
        )
        assert bytes(machine.mem_read(STACK_SEGMENT * 16, SEGMENT_SIZE)) == bytes(
            stack_expected
        )

        rows.append(
            {
                "name": name,
                "gate": gate,
                "enabled": enabled,
                "direction": expected["direction"],
                "screen_pointer": {
                    "offset": screen_offset,
                    "segment": screen_segment,
                },
                "clear_start": clear_start,
                "final_di": final_di,
                "port_writes": port_writes,
                "calls": calls,
                "framebuffer_sha256": hashlib.sha256(
                    bytes(screen_expected)
                ).hexdigest(),
                "final_flags": final_flags,
            }
        )
    return rows


def verify_executable(path: Path) -> bytes:
    executable = path.read_bytes()
    digest = hashlib.sha256(executable).hexdigest()
    if digest != EXECUTABLE_SHA256:
        raise SystemExit(f"unsupported {path.name} SHA-256 {digest}")
    body_digest = hashlib.sha256(executable[ENTRY:END]).hexdigest()
    if body_digest != BODY_SHA256:
        raise SystemExit(f"BBB HUD-refresh body changed: {body_digest}")
    return executable


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("executable", nargs="?", type=Path, default=DEFAULT_EXECUTABLE)
    parser.add_argument("output", nargs="?", type=Path, default=DEFAULT_OUTPUT)
    args = parser.parse_args()

    rows = vectors(verify_executable(args.executable))
    assert rows == commander_vectors()
    result = {
        "format": "big_bug_bang_hud_refresh_oracle_v1",
        "executable_sha256": EXECUTABLE_SHA256,
        "commander_fixture_sha256": COMMANDER_FIXTURE_SHA256,
        "routine": {
            "entry": f"0x{ENTRY:04x}",
            "end": f"0x{END:04x}",
            "body_sha256": BODY_SHA256,
            "gate_offset": GATE_OFFSET,
            "screen_pointer_offset": SCREEN_POINTER_OFFSET,
            "text_offset": TEXT_OFFSET,
            "renderer": f"{RENDERER_SEGMENT:04x}:{RENDERER_OFFSET:04x}",
            "retrace": f"{RETRACE_SEGMENT:04x}:{RETRACE_OFFSET:04x}",
        },
        "rows": rows,
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(result, indent=2) + "\n")
    print(f"verified {len(rows)} BBB HUD-refresh cases")


if __name__ == "__main__":
    main()
