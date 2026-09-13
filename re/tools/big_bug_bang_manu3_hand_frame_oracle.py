#!/usr/bin/env python3
"""Verify BBB's relocated MANU3 hand-frame dispatcher."""

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

from unicorn import UC_ARCH_X86, UC_HOOK_CODE, UC_HOOK_MEM_WRITE, UC_MODE_16, Uc
from unicorn.x86_const import (
    UC_X86_REG_AX,
    UC_X86_REG_BP,
    UC_X86_REG_BX,
    UC_X86_REG_CS,
    UC_X86_REG_CX,
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
DEFAULT_OUTPUT = ROOT / "re/tools/oracle_vectors/big_bug_bang_manu3_hand_frame.json"
COMMANDER_FIXTURE = ROOT / "re/tools/oracle_vectors/func_1610_natural.json"
EXECUTABLE_SHA256 = "4b65ffca3e113a1826371e3436177861640a1b7aae24caafebb4c2f7aa467834"
COMMANDER_FIXTURE_SHA256 = (
    "565285c39e74dec3e2c270461ea08872cc780d82062bcd03d430c56d549e6813"
)

ENTRY = 0x17CE
END = 0x1865
BODY_SHA256 = "65632bb70f4eabcedfcb0a780820b70997f103456028618fc44a8fc8e0fb7e9f"
DEAD_MOUSE_BLOCK_START = 0x17F4
DEAD_MOUSE_BLOCK_END = 0x1807
CALLBACK_RETURN_OFFSET = 0x1860

PRESENTATION_MODE_OFFSET = 0x2A80
HUD_MODE_OFFSET = 0x0CE8
MOUSE_X_OFFSET = 0x0C22
MOUSE_Y_OFFSET = 0x0C24
MOUSE_BUTTONS_OFFSET = 0x0C26
REQUESTED_SELECTOR_OFFSET = 0x0C2A
CURRENT_SELECTOR_OFFSET = 0x0C2C
REQUEST_OFFSET = 0x0CAC
CALLBACK_POINTER_OFFSET = 0x0C8E
DELAY_OFFSET = 0x0CF0
SCENE_BLOCKED_OFFSET = 0x277F
PRESENTATION_FLAGS_OFFSET = 0x6B80
FRAMEBUFFER_WINDOW_OFFSET = 0x55E9

COMMANDER_PRESENTATION_MODE_OFFSET = 0x27E0
COMMANDER_HUD_MODE_OFFSET = 0x0ADF
COMMANDER_MOUSE_X_OFFSET = 0x0A2A
COMMANDER_MOUSE_Y_OFFSET = 0x0A2C
COMMANDER_MOUSE_BUTTONS_OFFSET = 0x0A2E
COMMANDER_REQUESTED_SELECTOR_OFFSET = 0x0A32
COMMANDER_CURRENT_SELECTOR_OFFSET = 0x0A34
COMMANDER_REQUEST_OFFSET = 0x0AB4
COMMANDER_CALLBACK_POINTER_OFFSET = 0x0A96
COMMANDER_DELAY_OFFSET = 0x0AE7
COMMANDER_SCENE_BLOCKED_OFFSET = 0x252D
COMMANDER_PRESENTATION_FLAGS_OFFSET = 0x67AA
COMMANDER_FRAMEBUFFER_WINDOW_OFFSET = 0x5219

DATA_SEGMENT = 0x2000
EXTRA_SEGMENT = 0x4000
FS_SEGMENT = 0x6000
GAME_SEGMENT = 0x8000
CALLBACK_SEGMENT = 0xA000
CALLBACK_OFFSET = 0x0100
CALLBACK_ADDRESS = CALLBACK_SEGMENT * 16 + CALLBACK_OFFSET
DECOY_CALLBACK_OFFSET = 0x0200
STACK_SEGMENT = 0xC000
SEGMENT_SIZE = 0x10000
CALLER_SP = 0xFF00
RETURN_OFFSET = 0xF610
RETURN_ADDRESS = RETURN_OFFSET
STACK_SENTINEL = bytes.fromhex("5aa596698778c33c")
DEFINED_FLAG_MASK = 0x08C5

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
        (offset * multiplier + (offset >> 8) * 17 + case_index * 31 + salt) & 0xFF
        for offset in range(SEGMENT_SIZE)
    )


def write_wrapped(target: bytearray, offset: int, data: bytes) -> None:
    for index, value in enumerate(data):
        target[(offset + index) & 0xFFFF] = value


def physical_address(segment: int, offset: int) -> int:
    return segment * 16 + (offset & 0xFFFF)


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
    result["sp"] = machine.reg_read(UC_X86_REG_SP)
    return result


def commander_vectors() -> list[dict[str, Any]]:
    digest = hashlib.sha256(COMMANDER_FIXTURE.read_bytes()).hexdigest()
    assert digest == COMMANDER_FIXTURE_SHA256, digest
    return json.loads(COMMANDER_FIXTURE.read_text())


def seed_real_state(data: bytearray, inputs: dict[str, Any]) -> None:
    data[PRESENTATION_MODE_OFFSET] = inputs["presentation_mode"]
    data[HUD_MODE_OFFSET] = inputs["hud_mode"]
    data[SCENE_BLOCKED_OFFSET] = inputs["scene_blocked"]
    data[PRESENTATION_FLAGS_OFFSET] = inputs["presentation_flags"]
    data[DELAY_OFFSET] = inputs["delay"]
    write_wrapped(data, MOUSE_X_OFFSET, struct.pack("<h", inputs["mouse_x"]))
    write_wrapped(data, MOUSE_Y_OFFSET, struct.pack("<h", inputs["mouse_y"]))
    write_wrapped(
        data,
        MOUSE_BUTTONS_OFFSET,
        struct.pack("<H", inputs["mouse_buttons"]),
    )
    write_wrapped(
        data,
        REQUESTED_SELECTOR_OFFSET,
        struct.pack("<H", inputs["requested_selector"]),
    )
    write_wrapped(
        data,
        CURRENT_SELECTOR_OFFSET,
        struct.pack("<H", inputs["current_selector"]),
    )
    write_wrapped(
        data,
        FRAMEBUFFER_WINDOW_OFFSET,
        struct.pack("<H", inputs["framebuffer_window_offset"]),
    )
    write_wrapped(
        data,
        CALLBACK_POINTER_OFFSET,
        struct.pack("<HH", CALLBACK_OFFSET, CALLBACK_SEGMENT),
    )


def seed_commander_decoys(data: bytearray, inputs: dict[str, Any]) -> None:
    data[COMMANDER_PRESENTATION_MODE_OFFSET] = inputs["presentation_mode"] ^ 1
    data[COMMANDER_HUD_MODE_OFFSET] = inputs["hud_mode"] ^ 1
    data[COMMANDER_SCENE_BLOCKED_OFFSET] = inputs["scene_blocked"] ^ 1
    data[COMMANDER_PRESENTATION_FLAGS_OFFSET] = inputs["presentation_flags"] ^ 2
    data[COMMANDER_DELAY_OFFSET] = inputs["delay"] ^ 0xA5
    for offset, value in (
        (COMMANDER_MOUSE_X_OFFSET, inputs["mouse_x"] & 0xFFFF),
        (COMMANDER_MOUSE_Y_OFFSET, inputs["mouse_y"] & 0xFFFF),
        (COMMANDER_MOUSE_BUTTONS_OFFSET, inputs["mouse_buttons"]),
        (COMMANDER_REQUESTED_SELECTOR_OFFSET, inputs["requested_selector"]),
        (COMMANDER_CURRENT_SELECTOR_OFFSET, inputs["current_selector"]),
        (COMMANDER_FRAMEBUFFER_WINDOW_OFFSET, inputs["framebuffer_window_offset"]),
    ):
        write_wrapped(data, offset, struct.pack("<H", value ^ 0xA5A5))
    write_wrapped(
        data,
        COMMANDER_CALLBACK_POINTER_OFFSET,
        struct.pack("<HH", DECOY_CALLBACK_OFFSET, CALLBACK_SEGMENT),
    )


def vectors(executable: bytes) -> list[dict[str, Any]]:
    rows: list[dict[str, Any]] = []
    for case_index, expected in enumerate(commander_vectors()):
        name = expected["name"]
        inputs = expected["inputs"]
        result = expected["result"]
        requested = inputs["requested_selector"]
        current = inputs["current_selector"]
        delay = inputs["delay"]
        direction_flag = bool(case_index & 1)

        data_before = seeded_segment(case_index, 11, 0x19)
        seed_real_state(data_before, inputs)
        seed_commander_decoys(data_before, inputs)
        data_expected = bytearray(data_before)
        extra_before = seeded_segment(case_index, 13, 0x2B)
        fs_before = seeded_segment(case_index, 17, 0x3D)
        game_before = seeded_segment(case_index, 19, 0x4F)
        for decoy in (extra_before, fs_before, game_before):
            seed_real_state(decoy, inputs)
            write_wrapped(
                decoy,
                CALLBACK_POINTER_OFFSET,
                struct.pack("<HH", DECOY_CALLBACK_OFFSET, CALLBACK_SEGMENT),
            )

        callback_before = seeded_segment(case_index, 23, 0x61)
        callback_before[CALLBACK_OFFSET] = 0xCB
        callback_before[DECOY_CALLBACK_OFFSET] = 0xCC
        stack_before = seeded_segment(case_index, 29, 0x0B)
        write_wrapped(
            stack_before,
            CALLER_SP,
            struct.pack("<H", RETURN_OFFSET) + STACK_SENTINEL,
        )
        stack_expected = bytearray(stack_before)
        expected_write_events: list[tuple[int, int, int]] = []

        def data_write(
            offset: int,
            value: int,
            size: int,
            target: bytearray = data_expected,
            events: list[tuple[int, int, int]] = expected_write_events,
        ) -> None:
            encoded = value.to_bytes(size, "little")
            write_wrapped(target, offset, encoded)
            events.append((physical_address(DATA_SEGMENT, offset), size, value))

        def stack_write(
            offset: int,
            value: int,
            target: bytearray = stack_expected,
            events: list[tuple[int, int, int]] = expected_write_events,
        ) -> None:
            write_wrapped(target, offset, struct.pack("<H", value & 0xFFFF))
            events.append((physical_address(STACK_SEGMENT, offset), 2, value & 0xFFFF))

        early_gate = (
            inputs["presentation_mode"] & 1 != 0
            or inputs["hud_mode"] & 1 != 0
            or requested & 0x8000 != 0
        )
        expected_requested = requested
        expected_current = current
        expected_delay = delay
        expect_callback = False
        if not early_gate:
            if requested == current:
                expected_requested = 0
                data_write(REQUESTED_SELECTOR_OFFSET, 0, 2)
            else:
                data_write(REQUESTED_SELECTOR_OFFSET, requested, 2)
                if requested != 0:
                    expected_current = requested
                    data_write(CURRENT_SELECTOR_OFFSET, requested, 2)

            if (
                inputs["scene_blocked"] & 1 == 0
                and inputs["presentation_flags"] & 2 != 0
            ):
                expected_delay = 2
                data_write(DELAY_OFFSET, expected_delay, 1)
            elif delay != 0:
                expected_delay = (delay - 1) & 0xFF
                data_write(DELAY_OFFSET, expected_delay, 1)
            else:
                expect_callback = True

        assert expected_requested == result["requested_selector"], name
        assert expected_current == result["current_selector"], name
        assert expected_delay == result["delay"], name
        assert expect_callback == bool(result["callback_calls"]), name

        initial = {
            "eax": 0xA5A51234 + case_index,
            "ebx": 0xB6B62345 + case_index,
            "ecx": 0xC7C73456 + case_index,
            "edx": 0xD8D84567 + case_index,
            "esi": 0xE9E95678 + case_index,
            "edi": 0xFAFA6789 + case_index,
            "ebp": 0xABCD789A + case_index,
            "ds": DATA_SEGMENT,
            "es": EXTRA_SEGMENT,
            "fs": FS_SEGMENT,
            "gs": GAME_SEGMENT,
            "ss": STACK_SEGMENT,
        }

        if expect_callback:
            request_bytes = bytes.fromhex(result["stack_request_hex"])
            request_words = struct.unpack("<4H", request_bytes)
            for index, value in enumerate(request_words):
                stack_write(REQUEST_OFFSET + index * 2, value)
            stack_write(CALLER_SP - 2, DATA_SEGMENT)
            stack_write(CALLER_SP - 4, EXTRA_SEGMENT)
            stack_write(CALLER_SP - 6, FS_SEGMENT)
            stack_write(CALLER_SP - 8, 0)
            stack_write(CALLER_SP - 10, CALLBACK_RETURN_OFFSET)

        machine = Uc(UC_ARCH_X86, UC_MODE_16)
        machine.mem_map(0, 0x100000)
        machine.mem_write(0, executable)
        machine.mem_write(RETURN_ADDRESS, b"\xcc")
        module_before = bytes(machine.mem_read(0, len(executable)))
        machine.mem_write(DATA_SEGMENT * 16, bytes(data_before))
        machine.mem_write(EXTRA_SEGMENT * 16, bytes(extra_before))
        machine.mem_write(FS_SEGMENT * 16, bytes(fs_before))
        machine.mem_write(GAME_SEGMENT * 16, bytes(game_before))
        machine.mem_write(CALLBACK_SEGMENT * 16, bytes(callback_before))
        machine.mem_write(STACK_SEGMENT * 16, bytes(stack_before))
        for register_name, register in GENERAL_REGISTERS.items():
            machine.reg_write(register, initial[register_name])
        for register_name, register in SEGMENT_REGISTERS.items():
            machine.reg_write(register, initial[register_name])
        machine.reg_write(UC_X86_REG_CS, 0)
        machine.reg_write(UC_X86_REG_SP, CALLER_SP)
        machine.reg_write(
            UC_X86_REG_EFLAGS,
            0x0293 | (0x0400 if direction_flag else 0),
        )

        reached_return: list[int] = []
        calls: list[dict[str, Any]] = []
        executed: set[int] = set()
        observed_write_events: list[tuple[int, int, int]] = []

        def instruction(
            cpu: Uc,
            address: int,
            _size: int,
            _data: object,
            calls_for_case: list[dict[str, Any]] = calls,
            reached: list[int] = reached_return,
            executed_addresses: set[int] = executed,
            expected_result: dict[str, Any] = result,
            case_name: str = name,
            expected_data: bytes = bytes(data_expected),
            expected_stack: bytes = bytes(stack_expected),
        ) -> None:
            if address == RETURN_ADDRESS:
                reached.append(address)
                cpu.emu_stop()
                return
            if address != CALLBACK_ADDRESS:
                assert ENTRY <= address < END, hex(address)
                executed_addresses.add(address)
                return

            raw_call = {
                "call": "manu3_overlay_entry",
                "request_offset": cpu.reg_read(UC_X86_REG_BP),
                "request_hex": bytes(
                    cpu.mem_read(STACK_SEGMENT * 16 + REQUEST_OFFSET, 8)
                ).hex(),
                "ax": cpu.reg_read(UC_X86_REG_AX),
                "bx": cpu.reg_read(UC_X86_REG_BX),
                "cx": cpu.reg_read(UC_X86_REG_CX),
                "ds": cpu.reg_read(UC_X86_REG_DS),
                "es": cpu.reg_read(UC_X86_REG_ES),
                "fs": cpu.reg_read(UC_X86_REG_FS),
                "sp": cpu.reg_read(UC_X86_REG_SP),
                "frame": list(
                    struct.unpack(
                        "<6H",
                        cpu.mem_read(STACK_SEGMENT * 16 + CALLER_SP - 10, 12),
                    )
                ),
            }
            normalized = dict(raw_call)
            normalized["request_offset"] = COMMANDER_REQUEST_OFFSET
            normalized["es"] = expected_result["callback_calls"][0]["es"]
            normalized["fs"] = expected_result["callback_calls"][0]["fs"]
            normalized["frame"] = list(normalized["frame"])
            normalized["frame"][0] = 0x16A2
            normalized["frame"][2:5] = expected_result["callback_calls"][0]["frame"][
                2:5
            ]
            assert normalized == expected_result["callback_calls"][0], case_name
            assert bytes(cpu.mem_read(DATA_SEGMENT * 16, SEGMENT_SIZE)) == bytes(
                expected_data
            )
            assert bytes(cpu.mem_read(STACK_SEGMENT * 16, SEGMENT_SIZE)) == bytes(
                expected_stack
            )
            calls_for_case.append(normalized)
            cpu.reg_write(UC_X86_REG_EAX, 0x11112222)
            cpu.reg_write(UC_X86_REG_EBX, 0x33334444)
            cpu.reg_write(UC_X86_REG_ECX, 0x55556666)
            cpu.reg_write(UC_X86_REG_EDX, 0x77778888)
            cpu.reg_write(UC_X86_REG_ESI, 0x9999AAAA)
            cpu.reg_write(UC_X86_REG_EDI, 0xBBBBCCCC)
            cpu.reg_write(UC_X86_REG_EBP, 0xDDDDBEEF)
            cpu.reg_write(UC_X86_REG_DS, 0x3100)
            cpu.reg_write(UC_X86_REG_ES, 0x3200)
            cpu.reg_write(UC_X86_REG_FS, 0x3300)

        def write_hook(
            _cpu: Uc,
            _access: int,
            address: int,
            size: int,
            value: int,
            _data: object,
            events: list[tuple[int, int, int]] = observed_write_events,
        ) -> None:
            events.append((address, size, value))

        machine.hook_add(UC_HOOK_CODE, instruction)
        machine.hook_add(UC_HOOK_MEM_WRITE, write_hook)
        machine.emu_start(ENTRY, 0, count=200)

        assert reached_return == [RETURN_ADDRESS], name
        assert calls == result["callback_calls"], name
        assert not executed.intersection(
            range(DEAD_MOUSE_BLOCK_START, DEAD_MOUSE_BLOCK_END)
        )
        assert observed_write_events == expected_write_events, (
            name,
            observed_write_events,
            expected_write_events,
        )
        assert bytes(machine.mem_read(DATA_SEGMENT * 16, SEGMENT_SIZE)) == bytes(
            data_expected
        )
        assert bytes(machine.mem_read(EXTRA_SEGMENT * 16, SEGMENT_SIZE)) == bytes(
            extra_before
        )
        assert bytes(machine.mem_read(FS_SEGMENT * 16, SEGMENT_SIZE)) == bytes(
            fs_before
        )
        assert bytes(machine.mem_read(GAME_SEGMENT * 16, SEGMENT_SIZE)) == bytes(
            game_before
        )
        assert bytes(machine.mem_read(CALLBACK_SEGMENT * 16, SEGMENT_SIZE)) == bytes(
            callback_before
        )
        assert bytes(machine.mem_read(STACK_SEGMENT * 16, SEGMENT_SIZE)) == bytes(
            stack_expected
        )
        assert bytes(machine.mem_read(0, len(executable))) == module_before
        assert (
            bytes(
                machine.mem_read(
                    STACK_SEGMENT * 16 + CALLER_SP + 2,
                    len(STACK_SENTINEL),
                )
            )
            == STACK_SENTINEL
        )

        actual_registers = snapshot_registers(machine)
        for register, value in expected["final_registers"].items():
            if register == "flags":
                continue
            expected_value = {
                "es": EXTRA_SEGMENT,
                "fs": FS_SEGMENT,
                "gs": GAME_SEGMENT,
            }.get(register, value)
            assert actual_registers[register] == expected_value, (name, register)
        flags = machine.reg_read(UC_X86_REG_EFLAGS)
        assert (
            flags & DEFINED_FLAG_MASK
            == expected["final_registers"]["flags"] & DEFINED_FLAG_MASK
        ), name
        assert bool(flags & 0x0200), name
        assert bool(flags & 0x0400) == direction_flag, name
        assert machine.reg_read(UC_X86_REG_CS) == 0
        assert machine.reg_read(UC_X86_REG_IP) == RETURN_OFFSET

        rows.append(dict(expected))
    return rows


def verify_executable(path: Path) -> bytes:
    executable = path.read_bytes()
    digest = hashlib.sha256(executable).hexdigest()
    if digest != EXECUTABLE_SHA256:
        raise SystemExit(f"unsupported {path.name} SHA-256 {digest}")
    body_digest = hashlib.sha256(executable[ENTRY:END]).hexdigest()
    if body_digest != BODY_SHA256:
        raise SystemExit(f"BBB MANU3 hand-frame body changed: {body_digest}")
    return executable


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("executable", nargs="?", type=Path, default=DEFAULT_EXECUTABLE)
    parser.add_argument("output", nargs="?", type=Path, default=DEFAULT_OUTPUT)
    args = parser.parse_args()

    rows = vectors(verify_executable(args.executable))
    assert rows == commander_vectors()
    result = {
        "format": "big_bug_bang_manu3_hand_frame_oracle_v1",
        "executable_sha256": EXECUTABLE_SHA256,
        "commander_fixture_sha256": COMMANDER_FIXTURE_SHA256,
        "routine": {
            "entry": f"0x{ENTRY:04x}",
            "end": f"0x{END:04x}",
            "body_sha256": BODY_SHA256,
            "request_offset": REQUEST_OFFSET,
            "callback_pointer_offset": CALLBACK_POINTER_OFFSET,
        },
        "rows": rows,
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(result, indent=2) + "\n")
    print(f"verified {len(rows)} BBB MANU3 hand-frame cases")


if __name__ == "__main__":
    main()
