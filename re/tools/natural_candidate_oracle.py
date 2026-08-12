#!/usr/bin/env python3
"""Verify selected natural-C semantics against direct BLOODPRG execution."""

from __future__ import annotations

import os
import sys

_HERE = os.path.dirname(os.path.abspath(__file__))
sys.path[:] = [
    path for path in sys.path if os.path.abspath(path or os.curdir) != _HERE
]

import argparse
import hashlib
import json
import math
import struct
from collections.abc import Callable
from pathlib import Path

from unicorn import (
    UC_ARCH_X86,
    UC_HOOK_CODE,
    UC_HOOK_INSN,
    UC_HOOK_INTR,
    UC_MODE_16,
    Uc,
    UcError,
)
from unicorn.x86_const import (
    UC_X86_INS_IN,
    UC_X86_INS_OUT,
    UC_X86_REG_AX,
    UC_X86_REG_BP,
    UC_X86_REG_EBP,
    UC_X86_REG_BX,
    UC_X86_REG_EBX,
    UC_X86_REG_CS,
    UC_X86_REG_CX,
    UC_X86_REG_ECX,
    UC_X86_REG_DI,
    UC_X86_REG_EDI,
    UC_X86_REG_DS,
    UC_X86_REG_DX,
    UC_X86_REG_EAX,
    UC_X86_REG_EDX,
    UC_X86_REG_EFLAGS,
    UC_X86_REG_ES,
    UC_X86_REG_FS,
    UC_X86_REG_GS,
    UC_X86_REG_IP,
    UC_X86_REG_SI,
    UC_X86_REG_ESI,
    UC_X86_REG_SP,
    UC_X86_REG_SS,
)


REPO_ROOT = Path(__file__).resolve().parents[2]
EXE = (REPO_ROOT / "re/bin/BLOODPRG.EXE").read_bytes()
VECTOR_ROOT = REPO_ROOT / "re/tools/oracle_vectors"

REGISTERS = {
    "eax": UC_X86_REG_EAX,
    "ax": UC_X86_REG_AX,
    "ebx": UC_X86_REG_EBX,
    "bx": UC_X86_REG_BX,
    "ecx": UC_X86_REG_ECX,
    "cx": UC_X86_REG_CX,
    "edx": UC_X86_REG_EDX,
    "dx": UC_X86_REG_DX,
    "esi": UC_X86_REG_ESI,
    "si": UC_X86_REG_SI,
    "edi": UC_X86_REG_EDI,
    "di": UC_X86_REG_DI,
    "bp": UC_X86_REG_BP,
    "ebp": UC_X86_REG_EBP,
    "sp": UC_X86_REG_SP,
    "ds": UC_X86_REG_DS,
    "es": UC_X86_REG_ES,
    "fs": UC_X86_REG_FS,
    "gs": UC_X86_REG_GS,
    "ss": UC_X86_REG_SS,
    "flags": UC_X86_REG_EFLAGS,
}


def execute(
    entry: int,
    return_address: int,
    registers: dict[str, int],
    memory: list[tuple[int, int, bytes]],
    interrupt_handler: Callable[[Uc, int], None] | None = None,
    code_handler: Callable[[Uc, int, int], None] | None = None,
    input_handler: Callable[[Uc, int, int], int] | None = None,
    output_handler: Callable[[Uc, int, int, int], None] | None = None,
    instruction_count: int = 20000,
) -> Uc:
    machine = Uc(UC_ARCH_X86, UC_MODE_16)
    machine.mem_map(0, 0x300000)
    machine.mem_write(0, EXE + bytes(0x120000 - len(EXE)))
    machine.reg_write(UC_X86_REG_CS, 0)
    machine.reg_write(UC_X86_REG_SS, 0x9000)
    machine.reg_write(UC_X86_REG_SP, 0xff00)

    for name, value in registers.items():
        machine.reg_write(REGISTERS[name], value)
    for segment, offset, data in memory:
        machine.mem_write(segment * 16 + offset, data)

    returned = []

    def stop_at_return(machine: Uc, address: int, _size: int, _data: object) -> None:
        if address == return_address:
            returned.append(address)
            machine.emu_stop()
        elif code_handler is not None:
            code_handler(machine, address, _size)

    machine.hook_add(UC_HOOK_CODE, stop_at_return)
    if interrupt_handler is not None:

        def handle_interrupt(
            machine: Uc, number: int, _data: object
        ) -> None:
            interrupt_handler(machine, number)

        machine.hook_add(UC_HOOK_INTR, handle_interrupt)
    if input_handler is not None:

        def handle_input(
            machine: Uc, port: int, size: int, _data: object
        ) -> int:
            return input_handler(machine, port, size)

        machine.hook_add(
            UC_HOOK_INSN, handle_input, None, 1, 0, UC_X86_INS_IN
        )
    if output_handler is not None:

        def handle_output(
            machine: Uc, port: int, size: int, value: int, _data: object
        ) -> None:
            output_handler(machine, port, size, value)

        machine.hook_add(
            UC_HOOK_INSN, handle_output, None, 1, 0, UC_X86_INS_OUT
        )
    # The stop address is a global execution boundary in Unicorn, so it cannot
    # be the routine's RET when a nested call targets a higher address.
    try:
        machine.emu_start(entry, 0x2ffff0, count=instruction_count)
    except UcError as error:
        raise RuntimeError(
            f"{entry:#x}: execution failed at "
            f"{machine.reg_read(UC_X86_REG_CS):#x}:"
            f"{machine.reg_read(UC_X86_REG_IP):#x}"
        ) from error
    if not returned:
        stack_pointer = machine.reg_read(UC_X86_REG_SP)
        stack_segment = machine.reg_read(UC_X86_REG_SS)
        stack_bytes = bytes(
            machine.mem_read(stack_segment * 16 + stack_pointer, 8)
        ).hex()
        raise RuntimeError(
            f"{entry:#x}: did not reach return at {return_address:#x}; "
            f"stopped at {machine.reg_read(UC_X86_REG_CS):#x}:"
            f"{machine.reg_read(UC_X86_REG_IP):#x}; "
            f"sp={stack_pointer:#x} stack={stack_bytes}"
        )
    return machine


def cmos_rtc_read_vectors() -> list[dict[str, object]]:
    vectors = []
    for seconds in (0x00, 0x01, 0x09, 0x10, 0x59, 0x80, 0xFE, 0xFF):
        inputs = []
        outputs = []
        initial = {
            "eax": 0xA5A51234,
            "bx": 0x2468,
            "cx": 0x369C,
            "dx": 0x55AA,
            "si": 0x6789,
            "di": 0x789A,
            "bp": 0x1357,
            "ds": 0x2000,
            "es": 0x2400,
            "gs": 0x2800,
        }

        def input_port(_machine: Uc, port: int, size: int) -> int:
            inputs.append((port, size))
            if (port, size) != (0x71, 1):
                raise AssertionError(f"0x2DD3 read unexpected port {port:#x}/{size}")
            return seconds

        def output_port(
            _machine: Uc, port: int, size: int, value: int
        ) -> None:
            outputs.append((port, size, value))

        machine = execute(
            0x2DD3,
            0x2DE1,
            initial,
            [(0, 0x0AEE, b"\x5a\xa5")],
            input_handler=input_port,
            output_handler=output_port,
        )
        if inputs != [(0x71, 1)]:
            raise AssertionError(f"0x2DD3 seconds={seconds:#x}: unexpected reads")
        if outputs != [(0x70, 1, 0)]:
            raise AssertionError(f"0x2DD3 seconds={seconds:#x}: unexpected writes")
        expected_word = seconds | (seconds << 8)
        actual_word = struct.unpack("<H", machine.mem_read(0x0AEE, 2))[0]
        if actual_word != expected_word:
            raise AssertionError(
                f"0x2DD3 seconds={seconds:#x}: stored {actual_word:#x}, "
                f"expected {expected_word:#x}"
            )
        if machine.reg_read(UC_X86_REG_EAX) != initial["eax"]:
            raise AssertionError(f"0x2DD3 seconds={seconds:#x}: did not preserve EAX")
        for name in ("bx", "cx", "dx", "si", "di", "bp", "ds", "es", "gs"):
            if machine.reg_read(REGISTERS[name]) != initial[name]:
                raise AssertionError(f"0x2DD3 did not preserve {name}")

        vectors.append(
            {
                "seconds": seconds,
                "port_writes": [list(item) for item in outputs],
                "port_reads": [list(item) for item in inputs],
                "stored_word": actual_word,
                "preserved_eax": machine.reg_read(UC_X86_REG_EAX),
            }
        )
    return vectors


def vga_palette_write_vectors() -> list[dict[str, object]]:
    data_segment = 0x2000
    vectors = []
    for source_offset, seed in ((0x0000, 3), (0x1357, 29), (0xFE80, 101)):
        source = bytes((index * 37 + seed) & 0xFF for index in range(0x10000))
        expected_palette = bytes(
            source[(source_offset + index) & 0xFFFF] for index in range(768)
        )
        outputs = []
        initial = {
            "eax": 0xA5A51234,
            "bx": 0x2468,
            "cx": 0x369C,
            "dx": 0x55AA,
            "si": source_offset,
            "di": 0x789A,
            "bp": 0x1357,
            "ds": data_segment,
            "es": 0x2400,
            "gs": 0x2800,
        }

        def output_port(
            _machine: Uc, port: int, size: int, value: int
        ) -> None:
            outputs.append((port, size, value))

        machine = execute(
            0x2F90,
            0x2FA5,
            initial,
            [(data_segment, 0, source)],
            output_handler=output_port,
        )
        if outputs[:1] != [(0x3C8, 1, 0)]:
            raise AssertionError(f"0x2F90 si={source_offset:#x}: bad DAC index write")
        actual_palette = bytes(value for port, size, value in outputs[1:])
        if len(outputs) != 769 or any(
            (port, size) != (0x3C9, 1) for port, size, _value in outputs[1:]
        ):
            raise AssertionError(f"0x2F90 si={source_offset:#x}: bad DAC data writes")
        if actual_palette != expected_palette:
            raise AssertionError(f"0x2F90 si={source_offset:#x}: bad palette payload")
        for name in ("eax", "bx", "cx", "dx", "si", "di", "bp", "ds", "es", "gs"):
            if machine.reg_read(REGISTERS[name]) != initial[name]:
                raise AssertionError(f"0x2F90 did not preserve {name}")

        vectors.append(
            {
                "source_offset": source_offset,
                "write_count": len(outputs),
                "palette_head": list(actual_palette[:12]),
                "palette_tail": list(actual_palette[-12:]),
                "palette_sha256": hashlib.sha256(actual_palette).hexdigest(),
                "preserved_eax": machine.reg_read(UC_X86_REG_EAX),
            }
        )
    return vectors


def vga_dac_clear_vectors() -> list[dict[str, object]]:
    vectors = []
    for eax, cx, dx in (
        (0xA5A51234, 0x369C, 0x55AA),
        (0x5A5AFFFF, 0x0000, 0x03C8),
        (0x12340000, 0xFFFF, 0x03C9),
    ):
        outputs = []
        initial = {
            "eax": eax,
            "bx": 0x2468,
            "cx": cx,
            "dx": dx,
            "si": 0x6789,
            "di": 0x789A,
            "bp": 0x1357,
            "ds": 0x2000,
            "es": 0x2400,
            "gs": 0x2800,
        }

        def output_port(
            _machine: Uc, port: int, size: int, value: int
        ) -> None:
            outputs.append((port, size, value))

        machine = execute(
            0x2FA6,
            0x2FBA,
            initial,
            [],
            output_handler=output_port,
        )
        if outputs[:1] != [(0x3C8, 1, 0)]:
            raise AssertionError("0x2FA6 produced a bad DAC index write")
        if len(outputs) != 769 or any(
            item != (0x3C9, 1, 0) for item in outputs[1:]
        ):
            raise AssertionError("0x2FA6 did not clear all 768 DAC bytes")
        for name in ("eax", "bx", "cx", "dx", "si", "di", "bp", "ds", "es", "gs"):
            if machine.reg_read(REGISTERS[name]) != initial[name]:
                raise AssertionError(f"0x2FA6 did not preserve {name}")

        vectors.append(
            {
                "initial_eax": eax,
                "initial_cx": cx,
                "initial_dx": dx,
                "write_count": len(outputs),
                "zero_data_writes": len(outputs) - 1,
                "preserved_eax": machine.reg_read(UC_X86_REG_EAX),
            }
        )
    return vectors


def set_video_mode_saved_vectors() -> list[dict[str, object]]:
    data_segment = 0x2000
    video_segment = 0x2800
    vectors = []
    for mode in (0x00, 0x03, 0x13, 0x7F, 0xFF):
        interrupts = []
        initial = {
            "eax": 0xA5A51234,
            "bx": 0x2468,
            "cx": 0x369C,
            "dx": 0x55AA,
            "si": 0x6789,
            "di": 0x789A,
            "bp": 0x1357,
            "ds": data_segment,
            "es": 0x2400,
            "gs": video_segment,
        }

        def interrupt(machine: Uc, number: int) -> None:
            interrupts.append((number, machine.reg_read(UC_X86_REG_AX)))
            machine.reg_write(UC_X86_REG_AX, 0xDEAD)

        machine = execute(
            0x0CC0,
            0x0CCA,
            initial,
            [
                (data_segment, 0x5232, bytes([mode ^ 0xFF])),
                (video_segment, 0x5232, bytes([mode])),
            ],
            interrupt_handler=interrupt,
        )
        if interrupts != [(0x10, mode)]:
            raise AssertionError(
                f"0x0CC0 mode={mode:#x}: interrupts={interrupts!r}"
            )
        if machine.reg_read(UC_X86_REG_AX) != (initial["eax"] & 0xFFFF):
            raise AssertionError(f"0x0CC0 mode={mode:#x}: did not restore AX")
        for name in ("bx", "cx", "dx", "si", "di", "bp", "ds", "es", "gs"):
            if machine.reg_read(REGISTERS[name]) != initial[name]:
                raise AssertionError(f"0x0CC0 did not preserve {name}")

        vectors.append(
            {
                "saved_mode": mode,
                "interrupt": interrupts[0][0],
                "interrupt_ax": interrupts[0][1],
                "restored_ax": machine.reg_read(UC_X86_REG_AX),
            }
        )
    return vectors


def bcd_to_binary_vectors() -> list[dict[str, object]]:
    vectors = []
    for value in range(0x100):
        initial = {
            "eax": 0xA5A55A00 | value,
            "bx": 0x2468,
            "cx": 0x369C,
            "dx": 0x55AA,
            "si": 0x6789,
            "di": 0x789A,
            "bp": 0x1357,
            "ds": 0x2000,
            "es": 0x2400,
            "gs": 0x2800,
        }
        machine = execute(0x0986, 0x0996, initial, [])
        expected = ((value >> 4) * 10) + (value & 0x0F)
        if machine.reg_read(UC_X86_REG_AX) != expected:
            raise AssertionError(
                f"0x0986 value={value:#x}: "
                f"AX={machine.reg_read(UC_X86_REG_AX):#x}, expected={expected:#x}"
            )
        for register in ("bx", "cx", "dx", "si", "di", "bp", "ds", "es", "gs"):
            if machine.reg_read(REGISTERS[register]) != initial[register]:
                raise AssertionError(f"0x0986 did not preserve {register}")
        vectors.append({"packed_bcd": value, "binary": expected})
    return vectors


def detect_cdrom_vectors() -> list[dict[str, object]]:
    data_segment = 0x2000
    state_segment = 0x2800
    vectors = []
    for drive_count in (0x0000, 0x0001, 0x0002, 0x7FFF, 0xFFFF):
        interrupts = []
        initial = {
            "eax": 0xA5A51234,
            "bx": 0x2468,
            "cx": 0x369C,
            "dx": 0x55AA,
            "si": 0x6789,
            "di": 0x789A,
            "bp": 0x1357,
            "ds": data_segment,
            "es": 0x2400,
            "gs": state_segment,
        }

        def interrupt(machine: Uc, number: int) -> None:
            interrupts.append(
                (
                    number,
                    machine.reg_read(UC_X86_REG_AX),
                    machine.reg_read(UC_X86_REG_BX),
                )
            )
            machine.reg_write(UC_X86_REG_AX, 0xADAD)
            machine.reg_write(UC_X86_REG_BX, drive_count)

        machine = execute(
            0x0B32,
            0x0B41,
            initial,
            [
                (data_segment, 0x0AE6, b"\xa5"),
                (state_segment, 0x0AE6, b"\x5a"),
            ],
            interrupt_handler=interrupt,
        )
        if interrupts != [(0x2F, 0x1500, 0x0000)]:
            raise AssertionError(
                f"0x0B32 count={drive_count:#x}: interrupts={interrupts!r}"
            )
        expected_present = int(drive_count != 0)
        actual_present = machine.mem_read(
            state_segment * 16 + 0x0AE6, 1
        )[0]
        if actual_present != expected_present:
            raise AssertionError(
                f"0x0B32 count={drive_count:#x}: present={actual_present}"
            )
        if machine.mem_read(data_segment * 16 + 0x0AE6, 1) != b"\xa5":
            raise AssertionError("0x0B32 wrote the DS decoy instead of GS state")
        if machine.reg_read(UC_X86_REG_AX) != 0xADAD:
            raise AssertionError("0x0B32 did not retain the interrupt AX result")
        if machine.reg_read(UC_X86_REG_BX) != drive_count:
            raise AssertionError("0x0B32 did not retain the MSCDEX drive count")
        flags = machine.reg_read(UC_X86_REG_EFLAGS)
        actual_flags = {
            "carry": flags & 1,
            "parity": (flags >> 2) & 1,
            "zero": (flags >> 6) & 1,
            "sign": (flags >> 7) & 1,
            "overflow": (flags >> 11) & 1,
        }
        expected_flags = {
            "carry": 0,
            "parity": int(bin(drive_count & 0xFF).count("1") % 2 == 0),
            "zero": int(drive_count == 0),
            "sign": (drive_count >> 15) & 1,
            "overflow": 0,
        }
        if actual_flags != expected_flags:
            raise AssertionError(
                f"0x0B32 count={drive_count:#x}: flags={actual_flags}, "
                f"expected={expected_flags}"
            )
        for name in ("cx", "dx", "si", "di", "bp", "ds", "es", "gs"):
            if machine.reg_read(REGISTERS[name]) != initial[name]:
                raise AssertionError(f"0x0B32 did not preserve {name}")

        vectors.append(
            {
                "drive_count": drive_count,
                "interrupt_ax": interrupts[0][1],
                "interrupt_bx": interrupts[0][2],
                "cdrom_present": actual_present,
                "result_ax": machine.reg_read(UC_X86_REG_AX),
                "result_bx": machine.reg_read(UC_X86_REG_BX),
                "flags": actual_flags,
            }
        )
    return vectors


def keyboard_read_vectors() -> list[dict[str, object]]:
    cases = [
        ("empty", False, 0x0000),
        ("ascii", True, 0x1E61),
        ("extended", True, 0x4800),
        ("high", True, 0xFFFF),
    ]
    vectors = []
    for name, available, key_code in cases:
        interrupts = []
        initial = {
            "eax": 0xA5A51234,
            "bx": 0x2468,
            "cx": 0x369C,
            "dx": 0x55AA,
            "si": 0x6789,
            "di": 0x789A,
            "bp": 0x1357,
            "ds": 0x2000,
            "es": 0x2400,
            "gs": 0x2800,
        }

        def interrupt(machine: Uc, number: int) -> None:
            ax = machine.reg_read(UC_X86_REG_AX)
            interrupts.append((number, ax))
            if len(interrupts) == 1:
                if (number, ax) != (0x16, 0x0100):
                    raise AssertionError(f"0x267D {name}: bad keyboard poll")
                flags = machine.reg_read(UC_X86_REG_EFLAGS)
                if available:
                    machine.reg_write(UC_X86_REG_EFLAGS, flags & ~0x40)
                    machine.reg_write(UC_X86_REG_AX, key_code)
                else:
                    machine.reg_write(UC_X86_REG_EFLAGS, flags | 0x40)
                    machine.reg_write(UC_X86_REG_AX, 0xBEEF)
                return
            if (number, ax) != (0x16, 0x0000):
                raise AssertionError(f"0x267D {name}: bad keyboard read")
            machine.reg_write(UC_X86_REG_AX, key_code)

        machine = execute(
            0x267D,
            0x268C,
            initial,
            [],
            interrupt_handler=interrupt,
        )
        expected_interrupts = 2 if available else 1
        if len(interrupts) != expected_interrupts:
            raise AssertionError(f"0x267D {name}: interrupts={interrupts!r}")
        expected_ax = key_code if available else 0
        if machine.reg_read(UC_X86_REG_AX) != expected_ax:
            raise AssertionError(f"0x267D {name}: returned the wrong key code")
        for register in ("bx", "cx", "dx", "si", "di", "bp", "ds", "es", "gs"):
            if machine.reg_read(REGISTERS[register]) != initial[register]:
                raise AssertionError(f"0x267D did not preserve {register}")

        vectors.append(
            {
                "name": name,
                "available": available,
                "key_code": key_code,
                "interrupts": [list(item) for item in interrupts],
                "result_ax": machine.reg_read(UC_X86_REG_AX),
            }
        )
    return vectors


def print_string_dos_vectors() -> list[dict[str, object]]:
    data_segment = 0x2000
    cases = [
        (0x0100, b""),
        (0x0100, b"A"),
        (0x1357, b"DOS$TEXT"),
        (0xFFFC, bytes([0x80, 0xFE, 0x24, 0x7F])),
    ]
    vectors = []
    for source_offset, payload in cases:
        source = bytearray(0x10000)
        for index, value in enumerate(payload + b"\0"):
            source[(source_offset + index) & 0xFFFF] = value
        interrupts = []
        initial = {
            "eax": 0xA5A51234,
            "bx": 0x2468,
            "cx": 0x369C,
            "dx": 0x55AA,
            "si": source_offset,
            "di": 0x789A,
            "bp": 0x1357,
            "ds": data_segment,
            "es": 0x2400,
            "gs": 0x2800,
        }

        def interrupt(machine: Uc, number: int) -> None:
            ax = machine.reg_read(UC_X86_REG_AX)
            dx = machine.reg_read(UC_X86_REG_DX)
            interrupts.append((number, ax, dx))
            if number != 0x21 or (ax >> 8) != 2:
                raise AssertionError("0x0D61 invoked the wrong DOS service")
            machine.reg_write(UC_X86_REG_AX, (ax & 0xFF00) | (dx & 0xFF))

        machine = execute(
            0x0D61,
            0x0D74,
            initial,
            [(data_segment, 0, bytes(source))],
            interrupt_handler=interrupt,
        )
        actual_payload = bytes(dx & 0xFF for _number, _ax, dx in interrupts)
        if actual_payload != payload:
            raise AssertionError(
                f"0x0D61 si={source_offset:#x}: output={actual_payload!r}"
            )
        for register in ("eax", "bx", "cx", "dx", "si", "di", "bp", "ds", "es", "gs"):
            if machine.reg_read(REGISTERS[register]) != initial[register]:
                raise AssertionError(f"0x0D61 did not preserve {register}")

        vectors.append(
            {
                "source_offset": source_offset,
                "payload": list(payload),
                "dos_calls": len(interrupts),
                "call_dx": [dx for _number, _ax, dx in interrupts],
                "preserved_eax": machine.reg_read(UC_X86_REG_EAX),
            }
        )
    return vectors


def rtc_time_read_vectors() -> list[dict[str, object]]:
    data_segment = 0x2000
    state_segment = 0x2800
    vectors = []
    for bcd_hour in (0x00, 0x09, 0x12, 0x23, 0x59, 0x99, 0xFF):
        interrupts = []
        initial = {
            "eax": 0xA5A51234,
            "bx": 0x2468,
            "cx": 0x369C,
            "dx": 0x55AA,
            "si": 0x6789,
            "di": 0x789A,
            "bp": 0x1357,
            "ds": data_segment,
            "es": 0x2400,
            "gs": state_segment,
        }

        def interrupt(machine: Uc, number: int) -> None:
            interrupts.append((number, machine.reg_read(UC_X86_REG_AX)))
            machine.reg_write(UC_X86_REG_CX, (bcd_hour << 8) | 0x5A)
            machine.reg_write(UC_X86_REG_DX, 0xBEEF)
            machine.reg_write(UC_X86_REG_AX, 0xDEAD)

        machine = execute(
            0x093B,
            0x094F,
            initial,
            [
                (data_segment, 0x0AA6, b"\xa5\xa5"),
                (state_segment, 0x0AA6, b"\x5a\x5a"),
            ],
            interrupt_handler=interrupt,
        )
        if interrupts != [(0x1A, 0x0234)]:
            raise AssertionError(
                f"0x093B hour={bcd_hour:#x}: interrupts={interrupts!r}"
            )
        binary_hour = (((bcd_hour >> 4) * 10) + (bcd_hour & 0x0F)) & 0xFF
        expected_word = binary_hour if binary_hour < 0x80 else binary_hour | 0xFF00
        actual_word = struct.unpack(
            "<H", machine.mem_read(state_segment * 16 + 0x0AA6, 2)
        )[0]
        if actual_word != expected_word:
            raise AssertionError(
                f"0x093B hour={bcd_hour:#x}: stored={actual_word:#x}, "
                f"expected={expected_word:#x}"
            )
        if machine.mem_read(data_segment * 16 + 0x0AA6, 2) != b"\xa5\xa5":
            raise AssertionError("0x093B wrote the DS decoy instead of GS state")
        for register in ("eax", "bx", "cx", "dx", "si", "di", "bp", "ds", "es", "gs"):
            if machine.reg_read(REGISTERS[register]) != initial[register]:
                raise AssertionError(f"0x093B did not preserve {register}")

        vectors.append(
            {
                "bcd_hour": bcd_hour,
                "binary_byte": binary_hour,
                "stored_word": actual_word,
                "interrupt_ax": interrupts[0][1],
                "preserved_eax": machine.reg_read(UC_X86_REG_EAX),
            }
        )
    return vectors


def mouse_set_range_vectors() -> list[dict[str, object]]:
    cases = [
        (0x0000, 0x0BB8, 0x0000, 0x00C8),
        (0x0001, 0x0002, 0x0003, 0x0004),
        (0xFFFF, 0x8000, 0x7FFF, 0x0000),
    ]
    vectors = []
    for min_x, max_x, min_y, max_y in cases:
        interrupts = []
        initial = {
            "eax": 0xA5A50000 | min_x,
            "bx": max_x,
            "cx": min_y,
            "dx": max_y,
            "si": 0x6789,
            "di": 0x789A,
            "bp": 0x1357,
            "ds": 0x2000,
            "es": 0x2400,
            "gs": 0x2800,
        }

        def interrupt(machine: Uc, number: int) -> None:
            registers = (
                number,
                machine.reg_read(UC_X86_REG_AX),
                machine.reg_read(UC_X86_REG_BX),
                machine.reg_read(UC_X86_REG_CX),
                machine.reg_read(UC_X86_REG_DX),
            )
            interrupts.append(registers)
            if len(interrupts) == 1:
                machine.reg_write(UC_X86_REG_AX, 0x1111)
                machine.reg_write(UC_X86_REG_BX, 0x2222)
                machine.reg_write(UC_X86_REG_CX, 0x3333)
                machine.reg_write(UC_X86_REG_DX, 0x4444)
            else:
                machine.reg_write(UC_X86_REG_AX, 0xAAAA)
                machine.reg_write(UC_X86_REG_BX, 0xBBBB)
                machine.reg_write(UC_X86_REG_CX, 0xCCCC)
                machine.reg_write(UC_X86_REG_DX, 0xDDDD)

        machine = execute(
            0x0D4A,
            0x0D60,
            initial,
            [],
            interrupt_handler=interrupt,
        )
        expected_interrupts = [
            (0x33, 7, max_x, min_x, max_x),
            (0x33, 8, 0x2222, min_y, max_y),
        ]
        if interrupts != expected_interrupts:
            raise AssertionError(
                f"0x0D4A {min_x:#x}/{max_x:#x}/{min_y:#x}/{max_y:#x}: "
                f"interrupts={interrupts!r}"
            )
        if machine.reg_read(UC_X86_REG_AX) != min_x:
            raise AssertionError("0x0D4A did not restore AX")
        if machine.reg_read(UC_X86_REG_BX) != max_x:
            raise AssertionError("0x0D4A did not restore BX")
        if machine.reg_read(UC_X86_REG_CX) != 0xCCCC:
            raise AssertionError("0x0D4A did not retain second-call CX")
        if machine.reg_read(UC_X86_REG_DX) != 0xDDDD:
            raise AssertionError("0x0D4A did not retain second-call DX")
        for register in ("si", "di", "bp", "ds", "es", "gs"):
            if machine.reg_read(REGISTERS[register]) != initial[register]:
                raise AssertionError(f"0x0D4A did not preserve {register}")

        vectors.append(
            {
                "min_x": min_x,
                "max_x": max_x,
                "min_y": min_y,
                "max_y": max_y,
                "interrupts": [list(item) for item in interrupts],
                "result_ax": machine.reg_read(UC_X86_REG_AX),
                "result_bx": machine.reg_read(UC_X86_REG_BX),
                "result_cx": machine.reg_read(UC_X86_REG_CX),
                "result_dx": machine.reg_read(UC_X86_REG_DX),
            }
        )
    return vectors


def text_width_vectors() -> list[dict[str, object]]:
    data_segment = 0x2000
    font_segment = 0x2600
    faces = {
        0: (0x7362, 0x7412, 0x14782, 0x14832),
        1: (0x7802, 0x78b2, 0x14c22, 0x14cd2),
    }
    texts = [
        b"",
        b"A",
        b"TALK",
        b"Commander Blood",
        b"               ",
        b"$?",
        bytes([0x82, 0x84, 0x87, 0x94]),
    ]
    vectors = []

    for selector in (0, 1, 0xffff):
        face = 0 if selector == 0 else 1
        map_offset, advance_offset, map_file, advance_file = faces[face]
        character_map = EXE[map_file:advance_file]
        advance_region = EXE[advance_file : advance_file + 256]
        if len(character_map) != 176 or len(advance_region) != 256:
            raise RuntimeError("font table extraction did not produce the expected extent")

        for text in texts:
            expected = sum(
                advance_region[character_map[character]] for character in text
            )
            expected = (expected - 2) & 0xffff
            initial = {
                "ax": selector,
                "bx": 0x1357,
                "cx": 0x2468,
                "dx": 0x369c,
                "si": 0x0100,
                "di": 0x55aa,
                "bp": 0x6789,
                "ds": data_segment,
                "gs": font_segment,
            }
            machine = execute(
                0x30CD,
                0x3105,
                initial,
                [
                    (data_segment, 0x0100, text + b"\0"),
                    (font_segment, map_offset, character_map),
                    (font_segment, advance_offset, advance_region),
                ],
            )
            actual = machine.reg_read(UC_X86_REG_AX)
            if actual != expected:
                raise AssertionError(
                    f"0x30CD selector={selector:#x} text={text!r}: "
                    f"actual={actual:#x}, expected={expected:#x}"
                )
            for name in ("bx", "cx", "dx", "si", "di", "bp"):
                actual_register = machine.reg_read(REGISTERS[name])
                if actual_register != initial[name]:
                    raise AssertionError(f"0x30CD did not preserve {name}")

            vectors.append(
                {
                    "selector": selector,
                    "text": list(text),
                    "width_minus_trailing_gap": actual,
                }
            )

    return vectors


def entity_flag_state_transition_vectors() -> list[dict[str, object]]:
    data_segment = 0x2000
    state_segment = 0x2600
    cases = [
        ("inactive", 0, 0x0000),
        ("state0_without_active", 1, 0x0001),
        ("active_without_state0", 2, 0x0080),
        ("active_state0", 0x15, 0x0081),
        ("active_state0_dirty", 0x1F, 0x0083),
        ("preserve_high_byte", 7, 0xA581),
        ("preserve_other_low_bits", 9, 0x55C1),
    ]
    vectors = []

    for name, object_id, flags in cases:
        offset = 0x6212 + object_id * 32
        record = bytearray((index * 13 + object_id) & 0xFF for index in range(32))
        record[0:2] = struct.pack("<H", flags)
        decoy = bytearray(record)
        decoy[0:2] = struct.pack("<H", flags ^ 0xFFFF)
        initial = {
            "eax": 0xA5A50000 | object_id,
            "bx": 0x2345,
            "cx": 0x3456,
            "dx": 0x4567,
            "si": 0x5678,
            "di": 0x6789,
            "bp": 0x789A,
            "ds": data_segment,
            "es": 0x2800,
            "gs": state_segment,
        }
        machine = execute(
            0x41D1,
            0x41EF,
            initial,
            [
                (state_segment, offset, bytes(record)),
                (data_segment, offset, bytes(decoy)),
            ],
        )

        expected_flags = flags
        low_flags = flags & 0xFF
        if (low_flags & 0x80) != 0 and (low_flags & 1) != 0:
            expected_flags = (flags & 0xFF00) | ((low_flags & 0xFE) | 2)
        expected = bytearray(record)
        expected[0:2] = struct.pack("<H", expected_flags)
        actual = bytes(machine.mem_read(state_segment * 16 + offset, 32))
        if actual != bytes(expected):
            raise AssertionError(
                f"0x41D1 {name}: actual={actual.hex()}, expected={expected.hex()}"
            )
        actual_decoy = bytes(machine.mem_read(data_segment * 16 + offset, 32))
        if actual_decoy != bytes(decoy):
            raise AssertionError(f"0x41D1 {name}: DS decoy record changed")
        for register, value in initial.items():
            if register in REGISTERS:
                actual_register = machine.reg_read(REGISTERS[register])
                if actual_register != value:
                    raise AssertionError(f"0x41D1 {name}: changed {register}")

        vectors.append(
            {
                "name": name,
                "object_id": object_id,
                "input_flags": flags,
                "output_flags": expected_flags,
                "record_offset": offset,
            }
        )

    return vectors


def sprite_slot_position_update_vectors() -> list[dict[str, object]]:
    data_segment = 0x2000
    state_segment = 0x2600
    cases = [
        ("inactive_ignores_both", 0, 0x0000, 10, 20, 30, 40),
        ("dirty_only_is_inactive", 1, 0x0002, 10, 20, 30, 40),
        ("state0_unchanged", 2, 0x0001, 10, 20, 10, 20),
        ("active_x_changes", 0x15, 0x0080, 10, 20, 30, 20),
        ("state0_y_changes", 0x1F, 0x0001, 10, 20, 10, 40),
        ("both_change_preserve_high", 7, 0xA580, 0xFFFF, 0, 0, 0xFFFF),
    ]
    vectors = []

    for name, object_id, flags, old_x, old_y, draw_x, draw_y in cases:
        offset = 0x6212 + object_id * 32
        record = bytearray((index * 17 + object_id) & 0xFF for index in range(32))
        record[0:2] = struct.pack("<H", flags)
        record[8:10] = struct.pack("<H", old_x)
        record[10:12] = struct.pack("<H", old_y)
        decoy = bytes(byte ^ 0xFF for byte in record)
        initial = {
            "eax": 0xB6B60000 | object_id,
            "bx": draw_x,
            "cx": draw_y,
            "dx": 0x4567,
            "si": 0x5678,
            "di": 0x6789,
            "bp": 0x789A,
            "ds": data_segment,
            "es": 0x2800,
            "gs": state_segment,
        }
        machine = execute(
            0x420D,
            0x423F,
            initial,
            [
                (state_segment, offset, bytes(record)),
                (data_segment, offset, decoy),
            ],
        )

        expected = bytearray(record)
        expected_flags = flags
        active = (flags & 0x81) != 0
        if active and (old_x != draw_x or old_y != draw_y):
            expected_flags |= 2
        if active and old_x != draw_x:
            expected[8:10] = struct.pack("<H", draw_x)
        if active and old_y != draw_y:
            expected[10:12] = struct.pack("<H", draw_y)
        expected[0:2] = struct.pack("<H", expected_flags)
        actual = bytes(machine.mem_read(state_segment * 16 + offset, 32))
        if actual != bytes(expected):
            raise AssertionError(
                f"0x420D {name}: actual={actual.hex()}, expected={expected.hex()}"
            )
        actual_decoy = bytes(machine.mem_read(data_segment * 16 + offset, 32))
        if actual_decoy != decoy:
            raise AssertionError(f"0x420D {name}: DS decoy record changed")
        for register, value in initial.items():
            if register in REGISTERS:
                actual_register = machine.reg_read(REGISTERS[register])
                if actual_register != value:
                    raise AssertionError(f"0x420D {name}: changed {register}")

        vectors.append(
            {
                "name": name,
                "object_id": object_id,
                "input_flags": flags,
                "input_position": [old_x, old_y],
                "requested_position": [draw_x, draw_y],
                "output_flags": expected_flags,
                "output_position": [
                    draw_x if active and old_x != draw_x else old_x,
                    draw_y if active and old_y != draw_y else old_y,
                ],
            }
        )

    return vectors


def sprite_slot_extent_update_vectors() -> list[dict[str, object]]:
    data_segment = 0x2000
    state_segment = 0x2600
    source_segment = 0x3200
    stack_segment = 0x9000
    cases = [
        ("inactive", 0, 0x0000, 64, 32, 65, 33, 64, 32),
        ("equal_source_flag_already_clear", 1, 0x0080, 64, 32, 65, 33, 64, 32),
        ("equal_source_clears_change", 2, 0x0090, 64, 32, 65, 33, 64, 32),
        ("equal_source_preserves_high", 0x15, 0xA591, 64, 32, 65, 33, 64, 32),
        ("different_source_same_extent", 0x1F, 0x0090, 80, 40, 80, 40, 64, 32),
        ("different_width_updates", 7, 0x0080, 80, 40, 64, 40, 64, 32),
        ("different_height_updates", 9, 0x0001, 80, 40, 80, 32, 64, 32),
        ("both_update_wrap_values", 11, 0xA581, 0, 0xFFFF, 1, 2, 3, 4),
    ]
    vectors = []

    for (
        name,
        object_id,
        flags,
        width,
        height,
        old_width,
        old_height,
        source_width,
        source_height,
    ) in cases:
        offset = 0x6212 + object_id * 32
        source_offset = 0x0100 + object_id * 4
        bp = 0x0200 + object_id * 8
        record = bytearray((index * 19 + object_id) & 0xFF for index in range(32))
        record[0:2] = struct.pack("<H", flags)
        record[12:14] = struct.pack("<H", old_width)
        record[14:16] = struct.pack("<H", old_height)
        decoy = bytes(byte ^ 0xA5 for byte in record)
        initial = {
            "eax": 0xC7C70000 | object_id,
            "bx": 0x2345,
            "cx": width,
            "dx": height,
            "si": 0x5678,
            "di": 0x6789,
            "bp": bp,
            "ds": data_segment,
            "es": 0x2800,
            "gs": state_segment,
        }
        machine = execute(
            0x42CD,
            0x4315,
            initial,
            [
                (state_segment, offset, bytes(record)),
                (data_segment, offset, decoy),
                (
                    stack_segment,
                    bp + 4,
                    struct.pack("<HH", source_offset, source_segment),
                ),
                (
                    source_segment,
                    source_offset,
                    struct.pack("<HH", source_width, source_height),
                ),
            ],
        )

        expected = bytearray(record)
        expected_flags = flags
        active = (flags & 0x81) != 0
        if active and width == source_width and height == source_height:
            if (flags & 0x10) != 0:
                expected_flags = (flags & 0xFFEF) | 2
        elif active and (width != old_width or height != old_height):
            expected_flags = flags | 0x12
            expected[12:14] = struct.pack("<H", width)
            expected[14:16] = struct.pack("<H", height)
        expected[0:2] = struct.pack("<H", expected_flags)
        actual = bytes(machine.mem_read(state_segment * 16 + offset, 32))
        if actual != bytes(expected):
            raise AssertionError(
                f"0x42CD {name}: actual={actual.hex()}, expected={expected.hex()}"
            )
        actual_decoy = bytes(machine.mem_read(data_segment * 16 + offset, 32))
        if actual_decoy != decoy:
            raise AssertionError(f"0x42CD {name}: DS decoy record changed")
        for register, value in initial.items():
            if register in REGISTERS:
                actual_register = machine.reg_read(REGISTERS[register])
                if actual_register != value:
                    raise AssertionError(f"0x42CD {name}: changed {register}")

        vectors.append(
            {
                "name": name,
                "object_id": object_id,
                "input_flags": flags,
                "requested_extent": [width, height],
                "stored_extent": [old_width, old_height],
                "source_extent": [source_width, source_height],
                "output_flags": expected_flags,
                "output_extent": [
                    width
                    if active
                    and (width != source_width or height != source_height)
                    and (width != old_width or height != old_height)
                    else old_width,
                    height
                    if active
                    and (width != source_width or height != source_height)
                    and (width != old_width or height != old_height)
                    else old_height,
                ],
                "source_pointer_boundary": "SS:BP+4",
            }
        )

    return vectors


def sprite_slot_range_mark_dirty_vectors() -> list[dict[str, object]]:
    data_segment = 0x2000
    state_segment = 0x2600
    cases = [
        ("single_inactive", 4, [0x0000]),
        ("single_active", 7, [0x0080]),
        ("single_preserve_high", 9, [0xA5FF]),
        ("mixed_range", 0x15, [0x0080, 0x0001, 0x0183, 0x55C0]),
    ]
    vectors = []

    for name, first_id, input_flags in cases:
        records = bytearray()
        for index, flags in enumerate(input_flags):
            object_id = first_id + index
            record = bytearray(
                (byte_index * 23 + object_id) & 0xFF for byte_index in range(32)
            )
            record[0:2] = struct.pack("<H", flags)
            records.extend(record)
        decoy = bytes(byte ^ 0x5A for byte in records)
        offset = 0x6212 + first_id * 32
        last_id = first_id + len(input_flags) - 1
        initial = {
            "eax": 0xD1D10000 | first_id,
            "bx": last_id,
            "cx": 0x3456,
            "dx": 0x4567,
            "si": 0x5678,
            "di": 0x6789,
            "bp": 0x789A,
            "ds": data_segment,
            "es": 0x2800,
            "gs": state_segment,
        }
        machine = execute(
            0x4240,
            0x426C,
            initial,
            [
                (state_segment, offset, bytes(records)),
                (data_segment, offset, decoy),
            ],
        )

        expected = bytearray(records)
        output_flags = []
        for index, flags in enumerate(input_flags):
            result = flags
            if (flags & 0x80) != 0:
                result = (flags & 0xFF00) | (((flags & 0xFF) & 0x7E) | 2)
            expected[index * 32 : index * 32 + 2] = struct.pack("<H", result)
            output_flags.append(result)
        actual = bytes(machine.mem_read(state_segment * 16 + offset, len(records)))
        if actual != bytes(expected):
            raise AssertionError(
                f"0x4240 {name}: actual={actual.hex()}, expected={expected.hex()}"
            )
        actual_decoy = bytes(
            machine.mem_read(data_segment * 16 + offset, len(records))
        )
        if actual_decoy != decoy:
            raise AssertionError(f"0x4240 {name}: DS decoy records changed")
        for register, value in initial.items():
            if register in REGISTERS:
                actual_register = machine.reg_read(REGISTERS[register])
                if actual_register != value:
                    raise AssertionError(f"0x4240 {name}: changed {register}")

        vectors.append(
            {
                "name": name,
                "first_object_id": first_id,
                "last_object_id": last_id,
                "input_flags": input_flags,
                "output_flags": output_flags,
            }
        )

    return vectors


def sprite_slot_commit_dirty_range_vectors() -> list[dict[str, object]]:
    data_segment = 0x2000
    state_segment = 0x2600
    cases = [
        {
            "name": "clip_snapshot_bit0",
            "first_id": 0,
            "flags": [0x0003],
            "snapshot_flags": 0x0001,
            "clip": [0, 319, 5, 194],
        },
        {
            "name": "clip_snapshot_clears_full_word",
            "first_id": 0x15,
            "flags": [0x0003, 0xA503],
            "snapshot_flags": 0xA5A3,
            "clip": [0xFFFF, 0, 0x8000, 0x7FFF],
        },
        {
            "name": "single_dirty_state0",
            "first_id": 4,
            "flags": [0x0003],
            "snapshot_flags": 0,
            "clip": [1, 2, 3, 4],
        },
        {
            "name": "snapshot_bit_clear_walks_range",
            "first_id": 7,
            "flags": [0xA503],
            "snapshot_flags": 0x0002,
            "clip": [5, 6, 7, 8],
        },
        {
            "name": "mixed_range",
            "first_id": 0x15,
            "flags": [0x0000, 0x0001, 0x0002, 0x0003, 0xFF83],
            "snapshot_flags": 0,
            "clip": [9, 10, 11, 12],
        },
    ]
    vectors = []

    for case in cases:
        name = str(case["name"])
        first_id = int(case["first_id"])
        input_flags = [int(value) for value in case["flags"]]
        snapshot_flags = int(case["snapshot_flags"])
        clip = [int(value) for value in case["clip"]]
        records = bytearray()
        current_geometry = []
        committed_geometry = []
        for index, flags in enumerate(input_flags):
            object_id = first_id + index
            current = [
                (0x1000 + object_id * 7 + field * 0x111) & 0xFFFF
                for field in range(4)
            ]
            committed = [
                (0x8000 + object_id * 11 + field * 0x101) & 0xFFFF
                for field in range(4)
            ]
            record = bytearray(
                (byte_index * 29 + object_id) & 0xFF for byte_index in range(32)
            )
            record[0:2] = struct.pack("<H", flags)
            record[8:16] = struct.pack("<HHHH", *current)
            record[16:24] = struct.pack("<HHHH", *committed)
            records.extend(record)
            current_geometry.append(current)
            committed_geometry.append(committed)

        offset = 0x6212 + first_id * 32
        decoy_records = bytes(byte ^ 0xC3 for byte in records)
        initial_dirty_list = bytes(
            (index * 31 + first_id) & 0xFF for index in range(24)
        )
        decoy_dirty_list = bytes(byte ^ 0x96 for byte in initial_dirty_list)
        initial = {
            "eax": 0xD2D20000 | first_id,
            "bx": first_id + len(input_flags) - 1,
            "cx": 0x3456,
            "dx": 0x4567,
            "si": 0x5678,
            "di": 0x6789,
            "ebp": 0xD3D389AB,
            "ds": data_segment,
            "es": 0x2800,
            "gs": state_segment,
        }
        memory = [
            (state_segment, offset, bytes(records)),
            (data_segment, offset, decoy_records),
            (state_segment, 0x5235, struct.pack("<HHHH", *clip)),
            (state_segment, 0x5249, struct.pack("<H", snapshot_flags)),
            (state_segment, 0x6612, initial_dirty_list),
            (data_segment, 0x6612, decoy_dirty_list),
        ]
        machine = execute(0x43F7, 0x446E, initial, memory)

        expected_records = bytearray(records)
        expected_dirty_list = bytearray(initial_dirty_list)
        expected_snapshot_flags = snapshot_flags
        if (snapshot_flags & 1) != 0:
            expected_dirty_list[0:8] = struct.pack("<HHHH", *clip)
            expected_dirty_list[8:10] = struct.pack("<H", 0xFFFF)
            expected_snapshot_flags = 0
        else:
            for index, flags in enumerate(input_flags):
                if (flags & 3) == 3:
                    start = index * 32
                    expected_records[start + 16 : start + 24] = expected_records[
                        start + 8 : start + 16
                    ]

        actual_records = bytes(
            machine.mem_read(state_segment * 16 + offset, len(records))
        )
        if actual_records != bytes(expected_records):
            raise AssertionError(
                f"0x43F7 {name}: records={actual_records.hex()}, "
                f"expected={expected_records.hex()}"
            )
        actual_dirty_list = bytes(machine.mem_read(state_segment * 16 + 0x6612, 24))
        if actual_dirty_list != bytes(expected_dirty_list):
            raise AssertionError(
                f"0x43F7 {name}: dirty list={actual_dirty_list.hex()}, "
                f"expected={expected_dirty_list.hex()}"
            )
        actual_snapshot_flags = struct.unpack(
            "<H", machine.mem_read(state_segment * 16 + 0x5249, 2)
        )[0]
        if actual_snapshot_flags != expected_snapshot_flags:
            raise AssertionError(
                f"0x43F7 {name}: snapshot flags={actual_snapshot_flags:#x}, "
                f"expected={expected_snapshot_flags:#x}"
            )
        actual_decoy_records = bytes(
            machine.mem_read(data_segment * 16 + offset, len(records))
        )
        if actual_decoy_records != decoy_records:
            raise AssertionError(f"0x43F7 {name}: DS decoy records changed")
        actual_decoy_dirty = bytes(machine.mem_read(data_segment * 16 + 0x6612, 24))
        if actual_decoy_dirty != decoy_dirty_list:
            raise AssertionError(f"0x43F7 {name}: DS decoy dirty list changed")
        for register, value in initial.items():
            if register in REGISTERS:
                actual_register = machine.reg_read(REGISTERS[register])
                if actual_register != value:
                    raise AssertionError(f"0x43F7 {name}: changed {register}")

        output_geometry = []
        for flags, current, committed in zip(
            input_flags, current_geometry, committed_geometry
        ):
            should_commit = (flags & 3) == 3 and (snapshot_flags & 1) == 0
            output_geometry.append(current if should_commit else committed)
        vectors.append(
            {
                "name": name,
                "first_object_id": first_id,
                "last_object_id": initial["bx"],
                "slot_flags": input_flags,
                "snapshot_flags_before": snapshot_flags,
                "snapshot_flags_after": expected_snapshot_flags,
                "clip_bounds": clip,
                "committed_geometry_after": output_geometry,
                "wrote_clip_snapshot": (snapshot_flags & 1) != 0,
            }
        )

    return vectors


def sprite_slot_dirty_range_render_vectors() -> list[dict[str, object]]:
    data_segment = 0x2000
    state_segment = 0x2600
    stub_offsets = [0x7000 + index * 0x10 for index in range(8)]
    cases = [
        {
            "name": "empty_dirty_list",
            "first_id": 2,
            "flags": [0x0003, 0x0023, 0x0043],
            "geometry": [10, 20, 8, 9],
            "dirty_rects": [],
            "sentinel_left": 0xFFFF,
        },
        {
            "name": "inactive_range_walks_backward",
            "first_id": 4,
            "flags": [0x0002, 0x0022, 0xA566],
            "geometry": [10, 20, 8, 9],
            "dirty_rects": [[5, 25, 15, 35]],
            "sentinel_left": 0xFFFF,
        },
        {
            "name": "nonintersecting_range_walks_backward",
            "first_id": 7,
            "flags": [0x0003, 0x0023, 0x0043],
            "geometry": [10, 20, 5, 5],
            "dirty_rects": [[100, 120, 100, 120]],
            "sentinel_left": 0xFFFF,
        },
        {
            "name": "reverse_dispatch_order",
            "first_id": 10,
            "flags": [0x0001, 0x0029, 0x004F],
            "geometry": [10, 20, 10, 10],
            "dirty_rects": [[5, 25, 15, 35]],
            "sentinel_left": 0xFFFF,
        },
        {
            "name": "multiple_intersecting_rectangles",
            "first_id": 14,
            "flags": [0x0021],
            "geometry": [10, 10, 10, 10],
            "dirty_rects": [[0, 15, 0, 15], [15, 30, 15, 30]],
            "sentinel_left": 0xFFFF,
        },
        {
            "name": "signed_wrapping_geometry",
            "first_id": 17,
            "flags": [0x0045],
            "geometry": [0xFFF0, 2, 0x30, 8],
            "dirty_rects": [[0, 0x10, 0, 0x10]],
            "sentinel_left": 0x8000,
        },
    ]
    vectors = []

    def signed_word(value: int) -> int:
        value &= 0xFFFF
        return value - 0x10000 if (value & 0x8000) != 0 else value

    for case in cases:
        name = str(case["name"])
        first_id = int(case["first_id"])
        input_flags = [int(value) for value in case["flags"]]
        geometry = [int(value) for value in case["geometry"]]
        dirty_rects = [
            [int(value) for value in rect] for rect in case["dirty_rects"]
        ]
        last_id = first_id + len(input_flags) - 1
        records = bytearray()
        for index, flags in enumerate(input_flags):
            object_id = first_id + index
            record = bytearray(
                (byte_index * 37 + object_id * 11) & 0xFF
                for byte_index in range(32)
            )
            record[0:2] = struct.pack("<H", flags)
            record[8:16] = struct.pack("<HHHH", *geometry)
            records.extend(record)

        dirty_list = bytearray()
        for rect in dirty_rects:
            dirty_list.extend(struct.pack("<HHHH", *rect))
        dirty_list.extend(
            struct.pack(
                "<HHHH", int(case["sentinel_left"]), 0x1357, 0x2468, 0x369C
            )
        )
        record_offset = 0x6212 + first_id * 32
        decoy_records = bytes(byte ^ 0xA6 for byte in records)
        decoy_dirty_list = bytes(byte ^ 0x69 for byte in dirty_list)
        initial_selected = 0x6F00
        initial_flip_x = 0xA5
        initial_flip_y = 0x5A
        initial = {
            "eax": 0xD4D40000 | first_id,
            "bx": last_id,
            "cx": 0x3456,
            "dx": 0x4567,
            "si": 0x5678,
            "di": 0x6789,
            "ebp": 0xE5E589AB,
            "ds": data_segment,
            "es": 0x2800,
            "gs": state_segment,
        }
        stub_bytes = bytearray(0x80)
        for index in range(8):
            stub_bytes[index * 0x10] = 0xC3
        calls = []

        def record_blitter_call(
            _machine: Uc, address: int, _size: int
        ) -> None:
            if address in stub_offsets:
                calls.append(stub_offsets.index(address))

        machine = execute(
            0x4471,
            0x4521,
            initial,
            [
                (state_segment, record_offset, bytes(records)),
                (data_segment, record_offset, decoy_records),
                (state_segment, 0x6612, bytes(dirty_list)),
                (data_segment, 0x6612, decoy_dirty_list),
                (0, 0x1592, struct.pack("<8H", *stub_offsets)),
                (0, 0x15A2, struct.pack("<H", initial_selected)),
                (0, 0x14DF, bytes([initial_flip_x, initial_flip_y])),
                (0, 0x7000, bytes(stub_bytes)),
            ],
            code_handler=record_blitter_call,
        )

        expected_records = bytearray(records)
        expected_calls = []
        expected_selected = initial_selected
        expected_flip_x = initial_flip_x
        expected_flip_y = initial_flip_y
        if dirty_rects:
            for index in range(len(input_flags) - 1, -1, -1):
                record_start = index * 32
                flags = input_flags[index]
                if (flags & 1) != 0:
                    mode = (flags >> 2) & 7
                    expected_selected = stub_offsets[mode]
                    expected_flip_x = 1 if (flags & 0x20) != 0 else 0
                    expected_flip_y = 1 if (flags & 0x40) != 0 else 0
                    slot_right = (geometry[0] + geometry[2]) & 0xFFFF
                    slot_bottom = (geometry[1] + geometry[3]) & 0xFFFF
                    for rect in dirty_rects:
                        expected_records[record_start + 24 : record_start + 32] = (
                            struct.pack("<HHHH", *rect)
                        )
                        intersects = (
                            signed_word(geometry[0]) < signed_word(rect[1])
                            and signed_word(geometry[1]) < signed_word(rect[3])
                            and signed_word(slot_right) > signed_word(rect[0])
                            and signed_word(slot_bottom) > signed_word(rect[2])
                        )
                        if intersects:
                            expected_calls.append(mode)
                flags &= ~2
                expected_records[record_start : record_start + 2] = struct.pack(
                    "<H", flags
                )

        actual_records = bytes(
            machine.mem_read(state_segment * 16 + record_offset, len(records))
        )
        if actual_records != bytes(expected_records):
            raise AssertionError(
                f"0x4471 {name}: records={actual_records.hex()}, "
                f"expected={expected_records.hex()}"
            )
        if calls != expected_calls:
            raise AssertionError(
                f"0x4471 {name}: calls={calls}, expected={expected_calls}"
            )
        actual_selected = struct.unpack("<H", machine.mem_read(0x15A2, 2))[0]
        if actual_selected != expected_selected:
            raise AssertionError(
                f"0x4471 {name}: selected={actual_selected:#x}, "
                f"expected={expected_selected:#x}"
            )
        actual_flip_x, actual_flip_y = machine.mem_read(0x14DF, 2)
        if (actual_flip_x, actual_flip_y) != (expected_flip_x, expected_flip_y):
            raise AssertionError(
                f"0x4471 {name}: flips={(actual_flip_x, actual_flip_y)}, "
                f"expected={(expected_flip_x, expected_flip_y)}"
            )
        actual_decoy_records = bytes(
            machine.mem_read(data_segment * 16 + record_offset, len(records))
        )
        if actual_decoy_records != decoy_records:
            raise AssertionError(f"0x4471 {name}: DS decoy records changed")
        actual_decoy_dirty = bytes(
            machine.mem_read(data_segment * 16 + 0x6612, len(dirty_list))
        )
        if actual_decoy_dirty != decoy_dirty_list:
            raise AssertionError(f"0x4471 {name}: DS decoy dirty list changed")
        for register, value in initial.items():
            actual_register = machine.reg_read(REGISTERS[register])
            if actual_register != value:
                raise AssertionError(f"0x4471 {name}: changed {register}")

        vectors.append(
            {
                "name": name,
                "first_object_id": first_id,
                "last_object_id": last_id,
                "input_flags": input_flags,
                "output_flags": [
                    struct.unpack(
                        "<H", expected_records[index * 32 : index * 32 + 2]
                    )[0]
                    for index in range(len(input_flags))
                ],
                "dirty_rects": dirty_rects,
                "slot_geometry": geometry,
                "blitter_modes_called": expected_calls,
                "selected_mode_after": (
                    stub_offsets.index(expected_selected)
                    if expected_selected in stub_offsets
                    else None
                ),
                "flip_x_after": expected_flip_x,
                "flip_y_after": expected_flip_y,
            }
        )

    return vectors


def _sprite_blit_raw_vectors(
    entry: int, stop: int, opaque: bool
) -> list[dict[str, object]]:
    state_segment = 0x2600
    frame_segment = 0x3200
    frame_offset = 0x0200
    framebuffer_segment = 0x5000
    framebuffer_offset = 0x0100
    transparent_cases = [
        {
            "name": "transparent_zero_direct_copy",
            "flags": 0x0001,
            "flip_x": 0,
            "flip_y": 0,
            "draw": [5, 6],
            "extent": [4, 3],
            "frame_offset": [1, 2],
            "dirty": [6, 10, 8, 11],
            "stride": 4,
        },
        {
            "name": "destination_remap_5f11",
            "flags": 0x0101,
            "flip_x": 0,
            "flip_y": 0,
            "draw": [20, 12],
            "extent": [3, 2],
            "frame_offset": [0, 0],
            "dirty": [20, 23, 12, 14],
            "stride": 5,
        },
        {
            "name": "destination_remap_6011_mode3",
            "flags": 0x0301,
            "flip_x": 0,
            "flip_y": 0,
            "draw": [30, 18],
            "extent": [4, 2],
            "frame_offset": [0, 0],
            "dirty": [30, 34, 18, 20],
            "stride": 6,
        },
        {
            "name": "clip_all_edges",
            "flags": 0x0001,
            "flip_x": 0,
            "flip_y": 0,
            "draw": [10, 10],
            "extent": [6, 5],
            "frame_offset": [-2, 1],
            "dirty": [10, 13, 12, 15],
            "stride": 8,
            "advanced_cursor": 8,
            "advanced_x_offset": 2,
        },
        {
            "name": "horizontal_flip_with_clipping",
            "flags": 0x0001,
            "flip_x": 1,
            "flip_y": 0,
            "draw": [10, 10],
            "extent": [6, 5],
            "frame_offset": [-2, 1],
            "dirty": [10, 13, 11, 15],
            "stride": 8,
        },
        {
            "name": "vertical_flip_with_clipping",
            "flags": 0x0001,
            "flip_x": 0,
            "flip_y": 1,
            "draw": [10, 10],
            "extent": [6, 5],
            "frame_offset": [-2, 1],
            "dirty": [8, 14, 12, 16],
            "stride": 8,
        },
        {
            "name": "both_flips_and_remap_6011",
            "flags": 0x0201,
            "flip_x": 1,
            "flip_y": 1,
            "draw": [42, 24],
            "extent": [5, 4],
            "frame_offset": [-1, -1],
            "dirty": [42, 45, 24, 26],
            "stride": 7,
            "advanced_cursor": 7,
            "advanced_x_offset": 0,
        },
        {
            "name": "signed_negative_frame_origin",
            "flags": 0x0001,
            "flip_x": 0,
            "flip_y": 0,
            "draw": [1, 2],
            "extent": [5, 4],
            "frame_offset": [-3, -1],
            "dirty": [0, 3, 1, 5],
            "stride": 6,
        },
        {
            "name": "noncanonical_flip_x_uses_bit_for_clip_byte_for_direction",
            "flags": 0x0201,
            "flip_x": 2,
            "flip_y": 0,
            "draw": [12, 10],
            "extent": [5, 2],
            "frame_offset": [-2, 0],
            "dirty": [11, 15, 10, 12],
            "stride": 6,
        },
        {
            "name": "noncanonical_flip_y_uses_bit_for_clip_byte_for_direction",
            "flags": 0x0101,
            "flip_x": 0,
            "flip_y": 2,
            "draw": [20, 10],
            "extent": [4, 4],
            "frame_offset": [0, -1],
            "dirty": [20, 24, 10, 13],
            "stride": 6,
        },
    ]
    opaque_cases = [
        {
            "name": "opaque_zero_overwrite_mixed_dword_tail",
            "flags": 0x0301,
            "flip_x": 0,
            "flip_y": 0,
            "draw": [5, 6],
            "extent": [5, 3],
            "frame_offset": [1, 2],
            "dirty": [6, 11, 8, 11],
            "stride": 7,
        },
        {
            "name": "aligned_dword_rows",
            "flags": 0x0201,
            "flip_x": 0,
            "flip_y": 0,
            "draw": [20, 12],
            "extent": [4, 2],
            "frame_offset": [0, 0],
            "dirty": [20, 24, 12, 14],
            "stride": 6,
        },
        {
            "name": "short_byte_only_rows",
            "flags": 0x0101,
            "flip_x": 0,
            "flip_y": 0,
            "draw": [30, 18],
            "extent": [3, 2],
            "frame_offset": [0, 0],
            "dirty": [30, 33, 18, 20],
            "stride": 5,
        },
        {
            "name": "clip_all_edges",
            "flags": 0x0001,
            "flip_x": 0,
            "flip_y": 0,
            "draw": [10, 10],
            "extent": [6, 5],
            "frame_offset": [-2, 1],
            "dirty": [10, 13, 12, 15],
            "stride": 8,
            "advanced_cursor": 8,
            "advanced_x_offset": 2,
        },
        {
            "name": "horizontal_flip_with_clipping",
            "flags": 0x0301,
            "flip_x": 1,
            "flip_y": 0,
            "draw": [10, 10],
            "extent": [6, 5],
            "frame_offset": [-2, 1],
            "dirty": [10, 13, 11, 15],
            "stride": 8,
        },
        {
            "name": "vertical_flip_with_clipping",
            "flags": 0x0201,
            "flip_x": 0,
            "flip_y": 1,
            "draw": [10, 10],
            "extent": [6, 5],
            "frame_offset": [-2, 1],
            "dirty": [8, 14, 12, 16],
            "stride": 8,
        },
        {
            "name": "both_flips",
            "flags": 0x0101,
            "flip_x": 1,
            "flip_y": 1,
            "draw": [42, 24],
            "extent": [5, 4],
            "frame_offset": [-1, -1],
            "dirty": [42, 45, 24, 26],
            "stride": 7,
            "advanced_cursor": 7,
            "advanced_x_offset": 0,
        },
        {
            "name": "signed_negative_frame_origin",
            "flags": 0x0301,
            "flip_x": 0,
            "flip_y": 0,
            "draw": [1, 2],
            "extent": [5, 4],
            "frame_offset": [-3, -1],
            "dirty": [0, 3, 1, 5],
            "stride": 6,
        },
        {
            "name": "noncanonical_flip_x_uses_bit_for_clip_byte_for_direction",
            "flags": 0x0201,
            "flip_x": 2,
            "flip_y": 0,
            "draw": [12, 10],
            "extent": [5, 2],
            "frame_offset": [-2, 0],
            "dirty": [11, 15, 10, 12],
            "stride": 6,
        },
        {
            "name": "noncanonical_flip_y_uses_bit_for_clip_byte_for_direction",
            "flags": 0x0101,
            "flip_x": 0,
            "flip_y": 2,
            "draw": [20, 10],
            "extent": [4, 4],
            "frame_offset": [0, -1],
            "dirty": [20, 24, 10, 13],
            "stride": 6,
        },
    ]
    cases = opaque_cases if opaque else transparent_cases
    routine = f"{entry:#06x}"
    vectors = []

    def signed_word(value: int) -> int:
        value &= 0xFFFF
        return value - 0x10000 if (value & 0x8000) != 0 else value

    for case_index, case in enumerate(cases):
        name = str(case["name"])
        flags = int(case["flags"])
        flip_x = int(case["flip_x"])
        flip_y = int(case["flip_y"])
        draw_x, draw_y = (int(value) for value in case["draw"])
        extent_width, extent_height = (
            int(value) for value in case["extent"]
        )
        x_offset, y_offset = (
            int(value) for value in case["frame_offset"]
        )
        dirty = [int(value) for value in case["dirty"]]
        stride = int(case["stride"])
        frame_height = max(extent_height + 3, 8)
        pixels = bytearray()
        for row in range(frame_height):
            for column in range(stride):
                value = (row * 29 + column * 17 + case_index * 13 + 1) & 0xFF
                if (row + column + case_index) % 4 == 0:
                    value = 0
                pixels.append(value)
        advanced_cursor = int(case.get("advanced_cursor", 0))
        if advanced_cursor != 0:
            # Pin the word reinterpreted as x-origin after vertical SI movement.
            pixel_index = advanced_cursor - 4
            pixels[pixel_index : pixel_index + 2] = struct.pack(
                "<H", int(case["advanced_x_offset"]) & 0xFFFF
            )
        frame = struct.pack(
            "<HHHH",
            stride,
            frame_height,
            x_offset & 0xFFFF,
            y_offset & 0xFFFF,
        ) + bytes(pixels)

        slot_id = 3 + case_index
        record_offset = 0x6212 + slot_id * 32
        record = bytearray(
            (byte_index * 19 + case_index * 23) & 0xFF
            for byte_index in range(32)
        )
        record[0:2] = struct.pack("<H", flags)
        record[4:8] = struct.pack("<HH", frame_offset, frame_segment)
        record[8:16] = struct.pack(
            "<HHHH", draw_x, draw_y, extent_width, extent_height
        )
        record[24:32] = struct.pack("<HHHH", *dirty)
        framebuffer = bytearray(
            (index * 13 + case_index * 31 + 7) & 0xFF
            for index in range(64000)
        )
        expected_framebuffer = bytearray(framebuffer)
        remap_5f11 = bytes((255 - index) & 0xFF for index in range(256))
        remap_6011 = bytes((index * 3 + 11) & 0xFF for index in range(256))

        sprite_top = signed_word(draw_y + y_offset)
        sprite_right = signed_word(draw_x + extent_width + x_offset)
        sprite_bottom = signed_word(draw_y + extent_height + y_offset)
        destination_y = sprite_top
        draw_width = extent_width
        draw_height = extent_height
        source_index = 0
        if sprite_top < signed_word(dirty[2]):
            clipped = (signed_word(dirty[2]) - sprite_top) & 0xFFFF
            draw_height = (draw_height - clipped) & 0xFFFF
            if (flip_y & 1) == 0:
                source_index = (source_index + clipped * stride) & 0xFFFF
            destination_y = signed_word(dirty[2])
        if sprite_bottom >= signed_word(dirty[3]):
            clipped = (sprite_bottom - signed_word(dirty[3])) & 0xFFFF
            draw_height = (draw_height - clipped) & 0xFFFF
            if (flip_y & 1) != 0:
                source_index = (source_index + clipped * stride) & 0xFFFF
        cursor_x_offset = signed_word(
            struct.unpack("<H", frame[source_index + 4 : source_index + 6])[0]
        )
        sprite_left = signed_word(draw_x + cursor_x_offset)
        destination_x = sprite_left
        if sprite_left < signed_word(dirty[0]):
            clipped = (signed_word(dirty[0]) - sprite_left) & 0xFFFF
            draw_width = (draw_width - clipped) & 0xFFFF
            if (flip_x & 1) == 0:
                source_index = (source_index + clipped) & 0xFFFF
            destination_x = signed_word(dirty[0])
        if sprite_right >= signed_word(dirty[1]):
            clipped = (sprite_right - signed_word(dirty[1])) & 0xFFFF
            draw_width = (draw_width - clipped) & 0xFFFF
            if (flip_x & 1) != 0:
                source_index = (source_index + clipped) & 0xFFFF

        if (flip_y & 1) != 0:
            destination_y = signed_word(destination_y + draw_height - 1)
        if flip_x != 0:
            destination_x = signed_word(destination_x + draw_width - 1)
        initial_selected_remap = 0x7777
        remap_mode = (flags >> 8) & 3
        if opaque:
            expected_remap_offset = initial_selected_remap
            remap_table = None
        else:
            expected_remap_offset = (
                0
                if remap_mode == 0
                else (0x5F11 if remap_mode == 1 else 0x6011)
            )
            remap_table = (
                None
                if remap_mode == 0
                else (remap_5f11 if remap_mode == 1 else remap_6011)
            )
        changed_pixels = []
        row_source = source_index
        row_y = destination_y
        for _row in range(draw_height):
            source_cursor = row_source
            destination_cursor = (
                ((row_y & 0xFFFF) * 320 + (destination_x & 0xFFFF)) & 0xFFFF
            )
            for _column in range(draw_width):
                if source_cursor >= len(pixels):
                    raise AssertionError(
                        f"{routine} {name}: source cursor {source_cursor:#x} "
                        f"outside {len(pixels):#x}; width={draw_width:#x}, "
                        f"height={draw_height:#x}, start={source_index:#x}, "
                        f"sprite_left={sprite_left:#x}"
                    )
                source_pixel = pixels[source_cursor]
                source_cursor += 1
                if opaque or source_pixel != 0:
                    before = expected_framebuffer[destination_cursor]
                    after = (
                        source_pixel
                        if remap_table is None
                        else remap_table[before]
                    )
                    expected_framebuffer[destination_cursor] = after
                    changed_pixels.append(
                        [destination_cursor, before, source_pixel, after]
                    )
                destination_cursor = (
                    destination_cursor + (-1 if flip_x != 0 else 1)
                ) & 0xFFFF
            row_source += stride
            row_y = signed_word(row_y + (-1 if flip_y != 0 else 1))

        right = (draw_x + extent_width) & 0xFFFF
        bottom = (draw_y + extent_height) & 0xFFFF
        initial = {
            "eax": 0xA1A10000 | (draw_x & 0xFFFF),
            "ebx": 0xB2B20000 | (draw_y & 0xFFFF),
            "ecx": 0xC3C33456,
            "edx": 0xD4D40000 | right,
            "esi": 0xE5E55678,
            "edi": 0xF6F60000 | record_offset,
            "ebp": 0x97970000 | bottom,
            "ds": state_segment,
            "es": state_segment,
            "gs": state_segment,
        }
        machine = execute(
            entry,
            stop,
            initial,
            [
                (state_segment, record_offset, bytes(record)),
                (frame_segment, frame_offset, frame),
                (
                    state_segment,
                    0x5221,
                    struct.pack("<HH", framebuffer_offset, framebuffer_segment),
                ),
                (
                    state_segment,
                    0x524B,
                    struct.pack("<H", initial_selected_remap),
                ),
                (state_segment, 0x5F11, remap_5f11),
                (state_segment, 0x6011, remap_6011),
                (0, 0x14DF, bytes([flip_x, flip_y])),
                (framebuffer_segment, framebuffer_offset, bytes(framebuffer)),
            ],
        )

        actual_framebuffer = bytes(
            machine.mem_read(
                framebuffer_segment * 16 + framebuffer_offset, 64000
            )
        )
        if actual_framebuffer != bytes(expected_framebuffer):
            mismatch = next(
                index
                for index, (actual, expected) in enumerate(
                    zip(actual_framebuffer, expected_framebuffer)
                )
                if actual != expected
            )
            actual_writes = [
                [index, before, after]
                for index, (before, after) in enumerate(
                    zip(framebuffer, actual_framebuffer)
                )
                if before != after
            ][:20]
            expected_writes = [
                [index, before, after]
                for index, (before, after) in enumerate(
                    zip(framebuffer, expected_framebuffer)
                )
                if before != after
            ][:20]
            raise AssertionError(
                f"{routine} {name}: framebuffer[{mismatch:#x}]="
                f"{actual_framebuffer[mismatch]:#x}, "
                f"expected={expected_framebuffer[mismatch]:#x}; "
                f"actual_writes={actual_writes}; "
                f"expected_writes={expected_writes}"
            )
        actual_record = bytes(
            machine.mem_read(state_segment * 16 + record_offset, 32)
        )
        if actual_record != bytes(record):
            raise AssertionError(f"{routine} {name}: slot record changed")
        actual_frame = bytes(
            machine.mem_read(frame_segment * 16 + frame_offset, len(frame))
        )
        if actual_frame != frame:
            raise AssertionError(f"{routine} {name}: source frame changed")
        actual_remap_offset = struct.unpack(
            "<H", machine.mem_read(state_segment * 16 + 0x524B, 2)
        )[0]
        if actual_remap_offset != expected_remap_offset:
            raise AssertionError(
                f"{routine} {name}: remap={actual_remap_offset:#x}, "
                f"expected={expected_remap_offset:#x}"
            )
        for register, value in initial.items():
            actual_register = machine.reg_read(REGISTERS[register])
            if actual_register != value:
                raise AssertionError(f"{routine} {name}: changed {register}")

        vectors.append(
            {
                "name": name,
                "flags": flags,
                "flip_x": bool(flip_x),
                "flip_y": bool(flip_y),
                "flip_bytes": [flip_x, flip_y],
                "draw": [draw_x, draw_y],
                "extent": [extent_width, extent_height],
                "frame_origin_offset": [x_offset, y_offset],
                "dirty_rect": dirty,
                "frame_stride": stride,
                "clipped_extent": [draw_width, draw_height],
                "source_start_pixel": source_index,
                "destination_start": [destination_x, destination_y],
                "selected_remap_offset": expected_remap_offset,
                "changed_pixels": changed_pixels,
            }
        )

    return vectors


def sprite_blit_raw_transparent_vectors() -> list[dict[str, object]]:
    return _sprite_blit_raw_vectors(0x4536, 0x46B5, opaque=False)


def sprite_blit_raw_opaque_vectors() -> list[dict[str, object]]:
    return _sprite_blit_raw_vectors(0x4BA8, 0x4CD5, opaque=True)


def _sprite_blit_rle_vectors(
    entry: int, stop: int, opaque: bool
) -> list[dict[str, object]]:
    state_segment = 0x2600
    frame_segment = 0x3200
    frame_offset = 0x0200
    framebuffer_segment = 0x5000
    framebuffer_offset = 0x0100
    initial_selected_remap = 0x7777
    cases = [
        {
            "name": (
                "mixed_runs_copy_zero_opaquely"
                if opaque
                else "mixed_runs_skip_zero_directly"
            ),
            "flags": 0x0301 if opaque else 0x0001,
            "flip_x": 0,
            "flip_y": 0,
            "draw": [5, 6],
            "extent": [8, 3],
            "frame_offset": [1, 2],
            "dirty": [6, 14, 8, 11],
            "row_kind": "mixed",
        },
        {
            "name": "left_clip_splits_repeat_run",
            "flags": 0x0201,
            "flip_x": 0,
            "flip_y": 0,
            "draw": [10, 10],
            "extent": [8, 2],
            "frame_offset": [0, 0],
            "dirty": [12, 18, 10, 12],
            "row_kind": "repeat_prefix",
        },
        {
            "name": "left_clip_splits_literal_run",
            "flags": 0x0101,
            "flip_x": 0,
            "flip_y": 0,
            "draw": [10, 10],
            "extent": [8, 2],
            "frame_offset": [0, 0],
            "dirty": [12, 18, 10, 12],
            "row_kind": "literal_prefix",
        },
        {
            "name": "right_clip_splits_repeat_run",
            "flags": 0x0301,
            "flip_x": 0,
            "flip_y": 0,
            "draw": [10, 10],
            "extent": [8, 2],
            "frame_offset": [0, 0],
            "dirty": [10, 16, 10, 12],
            "row_kind": "repeat_suffix",
        },
        {
            "name": "right_clip_splits_literal_run",
            "flags": 0x0201,
            "flip_x": 0,
            "flip_y": 0,
            "draw": [10, 10],
            "extent": [8, 2],
            "frame_offset": [0, 0],
            "dirty": [10, 16, 10, 12],
            "row_kind": "literal_suffix",
        },
        {
            "name": "horizontal_flip_swaps_source_clips",
            "flags": 0x0101,
            "flip_x": 1,
            "flip_y": 0,
            "draw": [10, 10],
            "extent": [8, 3],
            "frame_offset": [-1, 0],
            "dirty": [11, 15, 10, 13],
            "row_kind": "mixed",
        },
        {
            "name": "top_clip_skips_encoded_row_and_reloads_x",
            "flags": 0x0301,
            "flip_x": 0,
            "flip_y": 0,
            "draw": [20, 10],
            "extent": [8, 4],
            "frame_offset": [-2, -1],
            "dirty": [21, 25, 10, 13],
            "row_kind": "split_literal",
            "advanced_x_offset": 1,
        },
        {
            "name": "vertical_flip_bottom_clip_skips_encoded_row",
            "flags": 0x0201,
            "flip_x": 0,
            "flip_y": 1,
            "draw": [20, 10],
            "extent": [8, 4],
            "frame_offset": [-1, 0],
            "dirty": [20, 27, 10, 13],
            "row_kind": "split_literal",
            "advanced_x_offset": 0,
        },
        {
            "name": "both_flips_and_all_edges",
            "flags": 0x0101,
            "flip_x": 1,
            "flip_y": 1,
            "draw": [42, 24],
            "extent": [8, 4],
            "frame_offset": [-1, -1],
            "dirty": [43, 48, 24, 26],
            "row_kind": "split_literal",
            "advanced_x_offset": -1,
        },
        {
            "name": "noncanonical_flip_bytes_split_bit_and_direction",
            "flags": 0x0301,
            "flip_x": 2,
            "flip_y": 2,
            "draw": [12, 10],
            "extent": [8, 4],
            "frame_offset": [-2, -1],
            "dirty": [12, 17, 10, 13],
            "row_kind": "split_literal",
            "advanced_x_offset": 0,
        },
    ]
    vectors = []

    def signed_word(value: int) -> int:
        value &= 0xFFFF
        return value - 0x10000 if (value & 0x8000) != 0 else value

    def encode_literal(values: list[int]) -> bytes:
        return bytes([len(values) - 1, *values])

    def encode_repeat(length: int, value: int) -> bytes:
        return bytes([(1 - length) & 0xFF, value & 0xFF])

    def make_row(
        kind: str,
        row: int,
        stride: int,
        case_index: int,
        advanced_x_offset: int | None,
    ) -> tuple[bytes, bytes]:
        values = [
            (row * 37 + column * 19 + case_index * 11 + 1) & 0xFF
            for column in range(stride)
        ]
        for column in range(stride):
            if (row + column + case_index) % 5 == 0:
                values[column] = 0
        if advanced_x_offset is not None:
            values[stride - 4 : stride - 2] = struct.pack(
                "<H", advanced_x_offset & 0xFFFF
            )

        if kind == "literal":
            encoded = encode_literal(values)
        elif kind == "split_literal":
            encoded = encode_literal(values[:4]) + encode_literal(values[4:])
        elif kind == "mixed":
            values[1] = values[0]
            values[2] = values[0]
            encoded = encode_repeat(3, values[0]) + encode_literal(values[3:])
        elif kind == "repeat_prefix":
            values[1:4] = [values[0]] * 3
            encoded = encode_repeat(4, values[0]) + encode_literal(values[4:])
        elif kind == "literal_prefix":
            values[5:8] = [values[4]] * 3
            encoded = encode_literal(values[:4]) + encode_repeat(4, values[4])
        elif kind == "repeat_suffix":
            values[5:8] = [values[4]] * 3
            encoded = encode_literal(values[:4]) + encode_repeat(4, values[4])
        elif kind == "literal_suffix":
            values[1:4] = [values[0]] * 3
            encoded = encode_repeat(4, values[0]) + encode_literal(values[4:])
        else:
            raise AssertionError(f"unknown RLE row kind {kind}")
        return encoded, bytes(values)

    for case_index, case in enumerate(cases):
        name = str(case["name"])
        flags = int(case["flags"])
        flip_x = int(case["flip_x"])
        flip_y = int(case["flip_y"])
        draw_x, draw_y = (int(value) for value in case["draw"])
        extent_width, extent_height = (
            int(value) for value in case["extent"]
        )
        x_offset, y_offset = (
            int(value) for value in case["frame_offset"]
        )
        dirty = [int(value) for value in case["dirty"]]
        stride = extent_width
        frame_height = max(extent_height + 2, 6)
        encoded = bytearray()
        decoded_rows = []
        for row in range(frame_height):
            row_encoded, row_decoded = make_row(
                str(case["row_kind"]),
                row,
                stride,
                case_index,
                (
                    int(case["advanced_x_offset"])
                    if "advanced_x_offset" in case
                    else None
                ),
            )
            encoded.extend(row_encoded)
            decoded_rows.append(row_decoded)
        frame = struct.pack(
            "<HHHH",
            stride,
            frame_height,
            x_offset & 0xFFFF,
            y_offset & 0xFFFF,
        ) + bytes(encoded)

        def decode_row(position: int) -> tuple[bytes, int]:
            decoded = bytearray()
            while len(decoded) < stride:
                control = encoded[position]
                position += 1
                if (control & 0x80) != 0:
                    run_length = ((-control) & 0xFF) + 1
                    pixel = encoded[position]
                    position += 1
                    decoded.extend([pixel] * run_length)
                else:
                    run_length = control + 1
                    decoded.extend(encoded[position : position + run_length])
                    position += run_length
            if len(decoded) != stride:
                raise AssertionError(
                    f"{entry:#06x} {name}: malformed generated row"
                )
            return bytes(decoded), position

        slot_id = 13 + case_index
        record_offset = 0x6212 + slot_id * 32
        record = bytearray(
            (byte_index * 19 + case_index * 23) & 0xFF
            for byte_index in range(32)
        )
        record[0:2] = struct.pack("<H", flags)
        record[4:8] = struct.pack("<HH", frame_offset, frame_segment)
        record[8:16] = struct.pack(
            "<HHHH", draw_x, draw_y, extent_width, extent_height
        )
        record[24:32] = struct.pack("<HHHH", *dirty)
        framebuffer = bytearray(
            (index * 13 + case_index * 31 + 7) & 0xFF
            for index in range(64000)
        )
        expected_framebuffer = bytearray(framebuffer)
        remap_5f11 = bytes((255 - index) & 0xFF for index in range(256))
        remap_6011 = bytes((index * 3 + 11) & 0xFF for index in range(256))
        remap_mode = (flags >> 8) & 3
        if opaque:
            expected_remap_offset = initial_selected_remap
            remap_table = None
        elif remap_mode == 0:
            expected_remap_offset = 0
            remap_table = None
        elif remap_mode == 1:
            expected_remap_offset = 0x5F11
            remap_table = remap_5f11
        else:
            expected_remap_offset = 0x6011
            remap_table = remap_6011

        sprite_top = signed_word(draw_y + y_offset)
        sprite_right = signed_word(draw_x + extent_width + x_offset)
        sprite_bottom = signed_word(draw_y + extent_height + y_offset)
        destination_y = sprite_top
        draw_width = extent_width
        draw_height = extent_height
        source_position = 0
        skipped_rows = 0
        if sprite_top < signed_word(dirty[2]):
            clipped = (signed_word(dirty[2]) - sprite_top) & 0xFFFF
            draw_height = (draw_height - clipped) & 0xFFFF
            if (flip_y & 1) == 0:
                for _ in range(clipped):
                    _decoded, source_position = decode_row(source_position)
                    skipped_rows += 1
            destination_y = signed_word(dirty[2])
        if sprite_bottom > signed_word(dirty[3]):
            clipped = (sprite_bottom - signed_word(dirty[3])) & 0xFFFF
            draw_height = (draw_height - clipped) & 0xFFFF
            if (flip_y & 1) != 0:
                for _ in range(clipped):
                    _decoded, source_position = decode_row(source_position)
                    skipped_rows += 1

        cursor_x_offset = signed_word(
            struct.unpack(
                "<H", frame[4 + source_position : 6 + source_position]
            )[0]
        )
        sprite_left = signed_word(draw_x + cursor_x_offset)
        destination_x = sprite_left
        left_clip = 0
        right_clip = 0
        if sprite_left < signed_word(dirty[0]):
            left_clip = (signed_word(dirty[0]) - sprite_left) & 0xFFFF
            draw_width = (draw_width - left_clip) & 0xFFFF
            destination_x = signed_word(dirty[0])
        if sprite_right >= signed_word(dirty[1]):
            right_clip = (sprite_right - signed_word(dirty[1])) & 0xFFFF
            draw_width = (draw_width - right_clip) & 0xFFFF

        if (flip_y & 1) != 0:
            destination_y = signed_word(destination_y + draw_height - 1)
        if flip_x != 0:
            destination_x = signed_word(destination_x + draw_width - 1)
        visible_start = right_clip if flip_x != 0 else left_clip
        changed_pixels = []
        row_y = destination_y
        rendered_rows = []
        for _ in range(draw_height):
            decoded, source_position = decode_row(source_position)
            selected = decoded[visible_start : visible_start + draw_width]
            rendered_rows.append(list(selected))
            destination_cursor = (
                ((row_y & 0xFFFF) * 320 + (destination_x & 0xFFFF)) & 0xFFFF
            )
            for source_pixel in selected:
                before = expected_framebuffer[destination_cursor]
                if opaque or source_pixel != 0:
                    after = (
                        source_pixel
                        if opaque or remap_table is None
                        else remap_table[before]
                    )
                    expected_framebuffer[destination_cursor] = after
                    change = [destination_cursor, before, source_pixel]
                    if not opaque:
                        change.append(after)
                    changed_pixels.append(change)
                destination_cursor = (
                    destination_cursor + (-1 if flip_x != 0 else 1)
                ) & 0xFFFF
            row_y = signed_word(row_y + (-1 if flip_y != 0 else 1))

        right = (draw_x + extent_width) & 0xFFFF
        bottom = (draw_y + extent_height) & 0xFFFF
        initial = {
            "eax": 0xA1A10000 | (draw_x & 0xFFFF),
            "ebx": 0xB2B20000 | (draw_y & 0xFFFF),
            "ecx": 0xC3C33456,
            "edx": 0xD4D40000 | right,
            "esi": 0xE5E55678,
            "edi": 0xF6F60000 | record_offset,
            "ebp": 0x97970000 | bottom,
            "ds": state_segment,
            "es": state_segment,
            "gs": state_segment,
        }
        initial_scratch = bytes.fromhex("a1b2c3d4e5f6")
        try:
            machine = execute(
                entry,
                stop,
                initial,
                [
                    (state_segment, record_offset, bytes(record)),
                    (frame_segment, frame_offset, frame),
                    (
                        state_segment,
                        0x5221,
                        struct.pack(
                            "<HH", framebuffer_offset, framebuffer_segment
                        ),
                    ),
                    (
                        state_segment,
                        0x524B,
                        struct.pack("<H", initial_selected_remap),
                    ),
                    (state_segment, 0x5F11, remap_5f11),
                    (state_segment, 0x6011, remap_6011),
                    (0, 0x14DF, bytes([flip_x, flip_y])),
                    (0, 0x1726, initial_scratch),
                    (
                        framebuffer_segment,
                        framebuffer_offset,
                        bytes(framebuffer),
                    ),
                ],
            )
        except RuntimeError as error:
            raise RuntimeError(f"{entry:#06x} {name}: {error}") from error

        actual_framebuffer = bytes(
            machine.mem_read(
                framebuffer_segment * 16 + framebuffer_offset, 64000
            )
        )
        if actual_framebuffer != bytes(expected_framebuffer):
            mismatch = next(
                index
                for index, (actual, expected) in enumerate(
                    zip(actual_framebuffer, expected_framebuffer)
                )
                if actual != expected
            )
            raise AssertionError(
                f"{entry:#06x} {name}: framebuffer[{mismatch:#x}]="
                f"{actual_framebuffer[mismatch]:#x}, "
                f"expected={expected_framebuffer[mismatch]:#x}"
            )
        actual_scratch = bytes(machine.mem_read(0x1726, 6))
        expected_scratch = struct.pack("<HHH", stride, left_clip, right_clip)
        if actual_scratch != expected_scratch:
            raise AssertionError(
                f"{entry:#06x} {name}: scratch={actual_scratch.hex()}, "
                f"expected={expected_scratch.hex()}"
            )
        actual_remap = struct.unpack(
            "<H", machine.mem_read(state_segment * 16 + 0x524B, 2)
        )[0]
        if actual_remap != expected_remap_offset:
            raise AssertionError(
                f"{entry:#06x} {name}: remap={actual_remap:#x}, "
                f"expected={expected_remap_offset:#x}"
            )
        if bytes(
            machine.mem_read(state_segment * 16 + 0x5F11, 256)
        ) != remap_5f11:
            raise AssertionError(f"{entry:#06x} {name}: changed first remap table")
        if bytes(
            machine.mem_read(state_segment * 16 + 0x6011, 256)
        ) != remap_6011:
            raise AssertionError(f"{entry:#06x} {name}: changed second remap table")
        if bytes(machine.mem_read(state_segment * 16 + record_offset, 32)) != bytes(
            record
        ):
            raise AssertionError(f"{entry:#06x} {name}: slot record changed")
        if bytes(
            machine.mem_read(frame_segment * 16 + frame_offset, len(frame))
        ) != frame:
            raise AssertionError(f"{entry:#06x} {name}: source frame changed")
        for register, value in initial.items():
            actual_register = machine.reg_read(REGISTERS[register])
            if actual_register != value:
                raise AssertionError(f"{entry:#06x} {name}: changed {register}")

        vector = {
            "name": name,
            "flags": flags,
            "flip_bytes": [flip_x, flip_y],
            "draw": [draw_x, draw_y],
            "extent": [extent_width, extent_height],
            "frame_origin_offset": [x_offset, y_offset],
            "dirty_rect": dirty,
            "row_kind": case["row_kind"],
            "encoded_bytes": len(encoded),
            "skipped_rows": skipped_rows,
            "cursor_x_offset": cursor_x_offset,
            "clipped_extent": [draw_width, draw_height],
            "source_clips": [left_clip, right_clip],
            "destination_start": [destination_x, destination_y],
            "rendered_rows": rendered_rows,
            "changed_pixels": changed_pixels,
        }
        if not opaque:
            vector["selected_remap_offset"] = expected_remap_offset
        vectors.append(vector)

    return vectors


def sprite_blit_rle_transparent_vectors() -> list[dict[str, object]]:
    return _sprite_blit_rle_vectors(0x46BC, 0x4BA7, opaque=False)


def sprite_blit_rle_opaque_vectors() -> list[dict[str, object]]:
    return _sprite_blit_rle_vectors(0x4CD6, 0x4F61, opaque=True)


def sprite_blit_scaled_transparent_vectors() -> list[dict[str, object]]:
    state_segment = 0x2600
    frame_segment = 0x3200
    frame_offset = 0x0200
    framebuffer_segment = 0x5000
    framebuffer_offset = 0x0100
    initial_selected_remap = 0x7777
    initial_scratch = bytes.fromhex("a1b2c3d4e5f6")
    cases = [
        {
            "name": "zero_destination_width_returns",
            "flags": 0x0301,
            "flip": [1, 2],
            "draw": [10, 12],
            "extent": [0, 4],
            "source": [5, 3],
            "frame_offset": [-17, 29],
            "dirty": [0, 320, 0, 200],
        },
        {
            "name": "zero_destination_height_returns",
            "flags": 0x0201,
            "flip": [2, 1],
            "draw": [10, 12],
            "extent": [4, 0],
            "source": [5, 3],
            "frame_offset": [31, -23],
            "dirty": [0, 320, 0, 200],
        },
        {
            "name": "one_to_one_transparent_zero",
            "flags": 0x0101,
            "flip": [0, 0],
            "draw": [10, 20],
            "extent": [4, 3],
            "source": [4, 3],
            "frame_offset": [0, 0],
            "dirty": [10, 14, 20, 23],
        },
        {
            "name": "fractional_upscale",
            "flags": 0x0301,
            "flip": [1, 1],
            "draw": [24, 30],
            "extent": [7, 5],
            "source": [3, 2],
            "frame_offset": [-101, 97],
            "dirty": [24, 31, 30, 35],
        },
        {
            "name": "fractional_downscale",
            "flags": 0x0201,
            "flip": [2, 129],
            "draw": [41, 16],
            "extent": [3, 2],
            "source": [8, 6],
            "frame_offset": [1234, -2345],
            "dirty": [41, 44, 16, 18],
        },
        {
            "name": "all_edges_clipped_advance_fixed_point",
            "flags": 0x0101,
            "flip": [255, 254],
            "draw": [5, 6],
            "extent": [10, 8],
            "source": [8, 6],
            "frame_offset": [-300, 400],
            "dirty": [7, 13, 8, 12],
        },
        {
            "name": "signed_negative_origin_clipped",
            "flags": 0x0301,
            "flip": [128, 127],
            "draw": [0xFFFD, 0xFFFE],
            "extent": [8, 6],
            "source": [6, 4],
            "frame_offset": [2047, -2048],
            "dirty": [0, 5, 0, 4],
        },
        {
            "name": "horizontal_clip_rejects_negative_width",
            "flags": 0x0201,
            "flip": [1, 1],
            "draw": [10, 10],
            "extent": [4, 3],
            "source": [7, 5],
            "frame_offset": [13, 17],
            "dirty": [15, 20, 10, 13],
        },
        {
            "name": "frame_offsets_flips_and_remap_are_ignored",
            "flags": 0x03FF,
            "flip": [0xFE, 0x81],
            "draw": [70, 40],
            "extent": [6, 4],
            "source": [5, 3],
            "frame_offset": [-32768, 32767],
            "dirty": [71, 75, 41, 44],
        },
        {
            "name": "zero_source_dimensions_repeat_first_pixel",
            "flags": 0x01A5,
            "flip": [3, 4],
            "draw": [90, 50],
            "extent": [4, 3],
            "source": [0, 0],
            "frame_offset": [2222, -3333],
            "dirty": [90, 94, 50, 53],
            "first_pixel": 0xA7,
        },
    ]
    vectors = []

    def signed_word(value: int) -> int:
        value &= 0xFFFF
        return value - 0x10000 if (value & 0x8000) != 0 else value

    for case_index, case in enumerate(cases):
        name = str(case["name"])
        flags = int(case["flags"])
        flip_x, flip_y = (int(value) for value in case["flip"])
        draw_x, draw_y = (int(value) for value in case["draw"])
        extent_width, extent_height = (
            int(value) for value in case["extent"]
        )
        source_width, source_height = (
            int(value) for value in case["source"]
        )
        x_offset, y_offset = (
            int(value) for value in case["frame_offset"]
        )
        dirty = [int(value) for value in case["dirty"]]

        pixels = bytearray()
        for row in range(source_height):
            for column in range(source_width):
                value = (
                    row * 53 + column * 29 + case_index * 17 + 1
                ) & 0xFF
                if (row * 3 + column + case_index) % 4 == 0:
                    value = 0
                pixels.append(value)
        if not pixels:
            pixels.append(int(case.get("first_pixel", 0xA7)))
        frame = struct.pack(
            "<HHHH",
            source_width,
            source_height,
            x_offset & 0xFFFF,
            y_offset & 0xFFFF,
        ) + bytes(pixels)

        slot_id = 23 + case_index
        record_offset = 0x6212 + slot_id * 32
        record = bytearray(
            (byte_index * 19 + case_index * 23) & 0xFF
            for byte_index in range(32)
        )
        record[0:2] = struct.pack("<H", flags)
        record[4:8] = struct.pack("<HH", frame_offset, frame_segment)
        record[8:16] = struct.pack(
            "<HHHH", draw_x, draw_y, extent_width, extent_height
        )
        record[24:32] = struct.pack("<HHHH", *dirty)

        framebuffer = bytearray(
            (index * 13 + case_index * 31 + 7) & 0xFF
            for index in range(64000)
        )
        expected_framebuffer = bytearray(framebuffer)
        remap_5f11 = bytes((255 - index) & 0xFF for index in range(256))
        remap_6011 = bytes((index * 3 + 11) & 0xFF for index in range(256))

        x_step = None
        y_step = None
        x_start = 0
        y_start = 0
        destination_x = signed_word(draw_x)
        destination_y = signed_word(draw_y)
        draw_width = extent_width & 0xFFFF
        draw_height = extent_height & 0xFFFF
        sampled_pixels = []
        changed_pixels = []

        if extent_width != 0:
            x_step = ((source_width << 16) // extent_width) & 0xFFFFFFFF
        if extent_width != 0 and extent_height != 0:
            y_step = ((source_height << 16) // extent_height) & 0xFFFFFFFF

            if destination_y < signed_word(dirty[2]):
                clipped = signed_word(dirty[2]) - destination_y
                draw_height = (draw_height - clipped) & 0xFFFF
                y_start = (clipped * y_step) & 0xFFFFFFFF
                destination_y = signed_word(dirty[2])
            bottom = signed_word(draw_y + extent_height)
            if bottom >= signed_word(dirty[3]):
                clipped = bottom - signed_word(dirty[3])
                draw_height = (draw_height - clipped) & 0xFFFF

            if destination_x < signed_word(dirty[0]):
                clipped = signed_word(dirty[0]) - destination_x
                draw_width = (draw_width - clipped) & 0xFFFF
                x_start = (clipped * x_step) & 0xFFFFFFFF
                destination_x = signed_word(dirty[0])
            right = signed_word(draw_x + extent_width)
            if right >= signed_word(dirty[1]):
                clipped = right - signed_word(dirty[1])
                draw_width = (draw_width - clipped) & 0xFFFF

            if signed_word(draw_width) > 0 and signed_word(draw_height) > 0:
                y_position = y_start
                for row in range(draw_height):
                    source_y = (y_position >> 16) & 0xFFFF
                    x_position = x_start
                    for column in range(draw_width):
                        source_x = (x_position >> 16) & 0xFFFF
                        source_index = (
                            source_y * source_width + source_x
                        ) & 0xFFFF
                        source_pixel = pixels[source_index]
                        destination_index = (
                            ((destination_y + row) & 0xFFFF) * 320
                            + ((destination_x + column) & 0xFFFF)
                        ) & 0xFFFF
                        before = expected_framebuffer[destination_index]
                        after = before
                        if source_pixel != 0:
                            after = source_pixel
                            expected_framebuffer[destination_index] = after
                        sampled_pixels.append(
                            [
                                destination_index,
                                source_x,
                                source_y,
                                source_pixel,
                                before,
                                after,
                            ]
                        )
                        if source_pixel != 0:
                            changed_pixels.append(
                                [destination_index, before, after]
                            )
                        x_position = (x_position + x_step) & 0xFFFFFFFF
                    y_position = (y_position + y_step) & 0xFFFFFFFF

        right = (draw_x + extent_width) & 0xFFFF
        bottom = (draw_y + extent_height) & 0xFFFF
        initial = {
            "eax": 0xA1A10000 | (draw_x & 0xFFFF),
            "ebx": 0xB2B20000 | (draw_y & 0xFFFF),
            "ecx": 0xC3C33456,
            "edx": 0xD4D40000 | right,
            "esi": 0xE5E55678,
            "edi": 0xF6F60000 | record_offset,
            "ebp": 0x97970000 | bottom,
            "ds": state_segment,
            "es": state_segment,
            "gs": state_segment,
        }
        machine = execute(
            0x4F62,
            0x5099,
            initial,
            [
                (state_segment, record_offset, bytes(record)),
                (frame_segment, frame_offset, frame),
                (
                    state_segment,
                    0x5221,
                    struct.pack("<HH", framebuffer_offset, framebuffer_segment),
                ),
                (
                    state_segment,
                    0x524B,
                    struct.pack("<H", initial_selected_remap),
                ),
                (state_segment, 0x5F11, remap_5f11),
                (state_segment, 0x6011, remap_6011),
                (0, 0x14DF, bytes([flip_x, flip_y])),
                (0, 0x1726, initial_scratch),
                (
                    framebuffer_segment,
                    framebuffer_offset,
                    bytes(framebuffer),
                ),
            ],
        )

        actual_framebuffer = bytes(
            machine.mem_read(
                framebuffer_segment * 16 + framebuffer_offset, 64000
            )
        )
        if actual_framebuffer != bytes(expected_framebuffer):
            mismatch = next(
                index
                for index, (actual, expected) in enumerate(
                    zip(actual_framebuffer, expected_framebuffer)
                )
                if actual != expected
            )
            raise AssertionError(
                f"0x4f62 {name}: framebuffer[{mismatch:#x}]="
                f"{actual_framebuffer[mismatch]:#x}, "
                f"expected={expected_framebuffer[mismatch]:#x}"
            )
        if bytes(machine.mem_read(0x1726, 6)) != initial_scratch:
            raise AssertionError(f"0x4f62 {name}: changed RLE scratch")
        if struct.unpack(
            "<H", machine.mem_read(state_segment * 16 + 0x524B, 2)
        )[0] != initial_selected_remap:
            raise AssertionError(f"0x4f62 {name}: changed remap selector")
        if bytes(
            machine.mem_read(state_segment * 16 + 0x5F11, 256)
        ) != remap_5f11:
            raise AssertionError(f"0x4f62 {name}: changed first remap table")
        if bytes(
            machine.mem_read(state_segment * 16 + 0x6011, 256)
        ) != remap_6011:
            raise AssertionError(f"0x4f62 {name}: changed second remap table")
        if bytes(machine.mem_read(0x14DF, 2)) != bytes([flip_x, flip_y]):
            raise AssertionError(f"0x4f62 {name}: changed flip bytes")
        if bytes(machine.mem_read(state_segment * 16 + record_offset, 32)) != bytes(
            record
        ):
            raise AssertionError(f"0x4f62 {name}: slot record changed")
        if bytes(
            machine.mem_read(frame_segment * 16 + frame_offset, len(frame))
        ) != frame:
            raise AssertionError(f"0x4f62 {name}: source frame changed")
        for register, value in initial.items():
            actual_register = machine.reg_read(REGISTERS[register])
            if actual_register != value:
                raise AssertionError(f"0x4f62 {name}: changed {register}")

        vectors.append(
            {
                "name": name,
                "flags": flags,
                "flip_bytes": [flip_x, flip_y],
                "draw": [draw_x, draw_y],
                "extent": [extent_width, extent_height],
                "source_extent": [source_width, source_height],
                "frame_origin_offset": [x_offset, y_offset],
                "dirty_rect": dirty,
                "x_step_16_16": x_step,
                "y_step_16_16": y_step,
                "fixed_start_16_16": [x_start, y_start],
                "clipped_extent": [draw_width, draw_height],
                "destination_start": [destination_x, destination_y],
                "sampled_pixels": sampled_pixels,
                "changed_pixels": changed_pixels,
            }
        )

    return vectors


def dirty_rects_copy_secondary_to_primary_vectors() -> list[dict[str, object]]:
    state_segment = 0x2600
    rectangle_segment = 0x3600
    rectangle_offset = 0x0100
    primary_segment = 0x5000
    primary_offset = 0x0000
    secondary_segment = 0x6000
    secondary_offset = 0x0000
    cases = [
        {
            "name": "gate_clear_ignores_rectangles",
            "flags": 0xA4,
            "rectangles": [[4, 11, 3, 6]],
        },
        {
            "name": "sentinel_first",
            "flags": 0xA5,
            "rectangles": [],
        },
        {
            "name": "byte_only_unaligned",
            "flags": 0xA5,
            "rectangles": [[1, 4, 2, 4]],
        },
        {
            "name": "aligned_dword_only",
            "flags": 0xA5,
            "rectangles": [[8, 16, 4, 7]],
        },
        {
            "name": "aligned_dword_with_tail",
            "flags": 0xA5,
            "rectangles": [[12, 18, 5, 7]],
        },
        {
            "name": "unaligned_leading_bytes_then_dword",
            "flags": 0xA5,
            "rectangles": [[5, 12, 6, 8]],
        },
        {
            "name": "unaligned_leading_dword_and_tail",
            "flags": 0xA5,
            "rectangles": [[6, 15, 7, 9]],
        },
        {
            "name": "multiple_rectangles_use_exclusive_edges",
            "flags": 0xA5,
            "rectangles": [
                [0, 1, 0, 1],
                [17, 30, 20, 23],
                [319, 320, 199, 200],
            ],
        },
    ]
    vectors = []

    for case_index, case in enumerate(cases):
        name = str(case["name"])
        flags = int(case["flags"])
        rectangles = [
            [int(value) for value in rectangle]
            for rectangle in case["rectangles"]
        ]
        records = bytearray()
        for rectangle in rectangles:
            records.extend(struct.pack("<HHHH", *rectangle))
        records.extend(struct.pack("<HHHH", 0xFFFF, 0x1357, 0x2468, 0x369C))

        primary = bytearray(
            (index * 13 + case_index * 31 + 7) & 0xFF
            for index in range(64000)
        )
        secondary = bytearray(
            (index * 29 + case_index * 17 + 3) & 0xFF
            for index in range(64000)
        )
        expected_primary = bytearray(primary)
        copied_rows = []
        if (flags & 1) != 0:
            for left, right, top, bottom in rectangles:
                width = (right - left) & 0xFFFF
                rows = (bottom - top) & 0xFFFF
                row_offset = (top * 320 + left) & 0xFFFF
                for _ in range(rows):
                    expected_primary[row_offset : row_offset + width] = (
                        secondary[row_offset : row_offset + width]
                    )
                    copied_rows.append([row_offset, width])
                    row_offset = (row_offset + 320) & 0xFFFF

        state = bytearray(
            (byte_index * 23 + case_index * 19 + 5) & 0xFF
            for byte_index in range(0x11)
        )
        state[0:4] = struct.pack("<HH", primary_offset, primary_segment)
        state[8:12] = struct.pack("<HH", secondary_offset, secondary_segment)
        state[16] = flags
        initial = {
            "eax": 0xA1A11234,
            "ebx": 0xB2B22345,
            "ecx": 0xC3C33456,
            "edx": 0xD4D44567,
            "esi": 0xE5E55678,
            "edi": 0xF6F60000 | rectangle_offset,
            "ebp": 0x9797789A,
            "sp": 0xFF00,
            "ds": 0x2400,
            "es": rectangle_segment,
            "gs": state_segment,
        }
        machine = execute(
            0x509D,
            0x5183,
            initial,
            [
                (state_segment, 0x5221, bytes(state)),
                (rectangle_segment, rectangle_offset, bytes(records)),
                (primary_segment, primary_offset, bytes(primary)),
                (secondary_segment, secondary_offset, bytes(secondary)),
            ],
        )

        actual_primary = bytes(
            machine.mem_read(primary_segment * 16 + primary_offset, 64000)
        )
        if actual_primary != bytes(expected_primary):
            mismatch = next(
                index
                for index, (actual, expected) in enumerate(
                    zip(actual_primary, expected_primary)
                )
                if actual != expected
            )
            raise AssertionError(
                f"0x509d {name}: primary[{mismatch:#x}]="
                f"{actual_primary[mismatch]:#x}, "
                f"expected={expected_primary[mismatch]:#x}"
            )
        if bytes(
            machine.mem_read(secondary_segment * 16 + secondary_offset, 64000)
        ) != bytes(secondary):
            raise AssertionError(f"0x509d {name}: secondary buffer changed")
        if bytes(
            machine.mem_read(rectangle_segment * 16 + rectangle_offset, len(records))
        ) != bytes(records):
            raise AssertionError(f"0x509d {name}: rectangle list changed")
        if bytes(
            machine.mem_read(state_segment * 16 + 0x5221, len(state))
        ) != bytes(state):
            raise AssertionError(f"0x509d {name}: render state changed")
        for register, value in initial.items():
            actual_register = machine.reg_read(REGISTERS[register])
            if actual_register != value:
                raise AssertionError(f"0x509d {name}: changed {register}")

        vectors.append(
            {
                "name": name,
                "dirty_copy_flags": flags,
                "rectangles": rectangles,
                "copied_rows": copied_rows,
                "copied_bytes": sum(width for _offset, width in copied_rows),
            }
        )

    return vectors


def resource_release_vectors() -> list[dict[str, object]]:
    table_segment = 0x3800
    cases = [
        ("clear_flags_skip_release", 0x0000, 0x0000, False),
        ("loaded_bit0_releases", 0x0001, 0x0001, True),
        ("loaded_bit1_releases", 0x0002, 0x0007, True),
        ("both_loaded_bits_release", 0x0003, 0x0013, True),
        ("unrelated_flag_skips_release", 0x8004, 0x0025, False),
        ("handle_index_wraps_to_sixteen_bits", 0x0101, 0x2001, True),
    ]
    vectors = []

    for case_index, (name, entry_flags, handle, expected_call) in enumerate(cases):
        entry_offset = (handle * 8) & 0xFFFF
        entry = struct.pack(
            "<HHI",
            (0x4100 + case_index * 0x31) & 0xFFFF,
            entry_flags,
            (0x12345678 + case_index * 0x11111111) & 0xFFFFFFFF,
        )
        initial = {
            "eax": 0xA1A10000 | handle,
            "ebx": 0xB2B22345,
            "ecx": 0xC3C33456,
            "edx": 0xD4D44567,
            "esi": 0xE5E55678,
            "edi": 0xF6F66789,
            "ebp": 0x9797789A,
            "sp": 0xFF00,
            "ds": 0x2400,
            "es": 0x2800,
            "fs": table_segment,
            "gs": 0x2C00,
        }
        calls = []

        def inspect_call(machine: Uc, address: int, _size: int) -> None:
            if address != 0x529C:
                return
            stack_pointer = machine.reg_read(UC_X86_REG_SP)
            stack = machine.mem_read(
                machine.reg_read(UC_X86_REG_SS) * 16 + stack_pointer, 4
            )
            return_offset, return_segment = struct.unpack("<HH", stack)
            calls.append(
                {
                    "handle": machine.reg_read(UC_X86_REG_AX),
                    "return_offset": return_offset,
                    "return_segment": return_segment,
                }
            )

        machine = execute(
            0x5288,
            0x529B,
            initial,
            [
                (table_segment, entry_offset, entry),
                (0, 0x529C, b"\xcb"),
            ],
            code_handler=inspect_call,
        )
        if bool(calls) != expected_call:
            raise AssertionError(
                f"0x5288 {name}: calls={calls}, expected_call={expected_call}"
            )
        if calls and calls != [
            {
                "handle": handle,
                "return_offset": 0x529A,
                "return_segment": 0,
            }
        ]:
            raise AssertionError(f"0x5288 {name}: bad call boundary {calls}")
        if bytes(
            machine.mem_read(table_segment * 16 + entry_offset, len(entry))
        ) != entry:
            raise AssertionError(f"0x5288 {name}: handle entry changed")
        for register, value in initial.items():
            actual_register = machine.reg_read(REGISTERS[register])
            if actual_register != value:
                raise AssertionError(f"0x5288 {name}: changed {register}")

        vectors.append(
            {
                "name": name,
                "handle": handle,
                "entry_offset": entry_offset,
                "entry_flags": entry_flags,
                "calls": calls,
            }
        )

    return vectors


def resource_free_inner_vectors() -> list[dict[str, object]]:
    table_segment = 0x3800
    state_segment = 0x2C00
    cases = [
        {
            "name": "last_resource_removes_without_move",
            "handle": 2,
            "resident": [2],
            "released_size": 0x30,
            "following_sizes": [],
            "terminator": 0xFFFF,
        },
        {
            "name": "first_resource_compacts_two_followers",
            "handle": 2,
            "resident": [2, 5, 7],
            "released_size": 0x20,
            "following_sizes": [0x30, 0x10],
            "terminator": 0xFFFF,
        },
        {
            "name": "middle_resource_preserves_predecessor",
            "handle": 4,
            "resident": [1, 4, 9],
            "released_size": 0x20,
            "following_sizes": [0x25],
            "terminator": 0xFFFF,
        },
        {
            "name": "nonparagraph_size_uses_floor_shift",
            "handle": 3,
            "resident": [3, 6],
            "released_size": 0x2F,
            "following_sizes": [0x31],
            "terminator": 0xFFFF,
        },
        {
            "name": "zero_sized_followers_skip_memmove",
            "handle": 5,
            "resident": [5, 8, 10],
            "released_size": 0x40,
            "following_sizes": [0, 0],
            "terminator": 0xFFFF,
        },
        {
            "name": "any_signed_negative_word_terminates",
            "handle": 6,
            "resident": [3, 6],
            "released_size": 0x10,
            "following_sizes": [],
            "terminator": 0x8000,
        },
    ]
    vectors = []

    for case_index, case in enumerate(cases):
        name = str(case["name"])
        handle = int(case["handle"])
        resident = [int(value) for value in case["resident"]]
        released_index = resident.index(handle)
        released_size = int(case["released_size"])
        following = resident[released_index + 1 :]
        following_sizes = [int(value) for value in case["following_sizes"]]
        if len(following) != len(following_sizes):
            raise AssertionError(f"0x529c {name}: bad generated follower sizes")
        terminator = int(case["terminator"])
        paragraphs = (released_size >> 4) & 0xFFFF
        released_segment = 0x5000 + case_index * 0x0100
        initial_free_bytes = (0x10203040 + case_index * 0x11111111) & 0xFFFFFFFF
        initial_pool_end = (0x6800 + case_index * 0x31) & 0xFFFF

        table = bytearray(
            (index * 17 + case_index * 29 + 5) & 0xFF
            for index in range(0x0A00)
        )

        def write_entry(
            entry_handle: int, segment: int, flags: int, size: int
        ) -> None:
            struct.pack_into(
                "<HHI",
                table,
                (entry_handle * 8) & 0xFFFF,
                segment & 0xFFFF,
                flags & 0xFFFF,
                size & 0xFFFFFFFF,
            )

        write_entry(handle, released_segment, 0xA503, released_size)
        for position, entry_handle in enumerate(resident):
            if entry_handle == handle:
                continue
            if position < released_index:
                segment = (released_segment - 0x20 * (released_index - position))
                size = 0x20
            else:
                follower_index = position - released_index - 1
                segment = released_segment + paragraphs + follower_index * 0x20
                size = following_sizes[follower_index]
            write_entry(entry_handle, segment, 0xB703, size)

        resident_words = [*resident, terminator]
        for position, entry_handle in enumerate(resident_words):
            struct.pack_into("<H", table, 0x0800 + position * 2, entry_handle)
        expected_table = bytearray(table)
        released_offset = (handle * 8) & 0xFFFF
        released_flags = struct.unpack_from(
            "<H", expected_table, released_offset + 2
        )[0]
        struct.pack_into(
            "<H", expected_table, released_offset + 2, released_flags & 0xFFFC
        )
        for entry_handle in following:
            entry_offset = (entry_handle * 8) & 0xFFFF
            segment = struct.unpack_from("<H", expected_table, entry_offset)[0]
            struct.pack_into(
                "<H", expected_table, entry_offset, (segment - paragraphs) & 0xFFFF
            )
        shifted_words = [*resident[:released_index], *following, terminator]
        for position, entry_handle in enumerate(shifted_words):
            struct.pack_into(
                "<H", expected_table, 0x0800 + position * 2, entry_handle
            )

        moved_bytes = sum(following_sizes) & 0xFFFFFFFF
        pool = bytearray(
            (index * 31 + case_index * 23 + 9) & 0xFF for index in range(0x1000)
        )
        expected_pool = bytearray(pool)
        source_linear_offset = paragraphs * 16
        if moved_bytes != 0:
            expected_pool[:moved_bytes] = pool[
                source_linear_offset : source_linear_offset + moved_bytes
            ]

        initial = {
            "eax": 0xA1A10000 | handle,
            "ebx": 0xB2B22345,
            "ecx": 0xC3C33456,
            "edx": 0xD4D44567,
            "esi": 0xE5E55678,
            "edi": 0xF6F66789,
            "ebp": 0x9797789A,
            "sp": 0xFF00,
            "ds": 0x2400,
            "es": 0x2800,
            "fs": table_segment,
            "gs": state_segment,
        }
        calls = []

        def inspect_memmove(machine: Uc, address: int, _size: int) -> None:
            if address != 0x5302:
                return
            byte_count = machine.reg_read(UC_X86_REG_EAX)
            source_segment = machine.reg_read(UC_X86_REG_DS)
            source_offset = machine.reg_read(UC_X86_REG_SI)
            destination_segment = machine.reg_read(UC_X86_REG_ES)
            destination_offset = machine.reg_read(UC_X86_REG_DI)
            calls.append(
                {
                    "byte_count": byte_count,
                    "source": [source_segment, source_offset],
                    "destination": [destination_segment, destination_offset],
                }
            )
            payload = bytes(
                machine.mem_read(
                    source_segment * 16 + source_offset, byte_count
                )
            )
            machine.mem_write(
                destination_segment * 16 + destination_offset, payload
            )

        machine = execute(
            0x529C,
            0x5313,
            initial,
            [
                (table_segment, 0, bytes(table)),
                (
                    state_segment,
                    0x0A46,
                    struct.pack("<I", initial_free_bytes),
                ),
                (
                    state_segment,
                    0x0A6A,
                    struct.pack("<H", initial_pool_end),
                ),
                (released_segment, 0, bytes(pool)),
                (0, 0x5302, b"\x90" * 5),
            ],
            code_handler=inspect_memmove,
        )

        expected_calls = []
        if moved_bytes != 0:
            expected_calls.append(
                {
                    "byte_count": moved_bytes,
                    "source": [
                        (released_segment + paragraphs) & 0xFFFF,
                        0,
                    ],
                    "destination": [released_segment, 0],
                }
            )
        if calls != expected_calls:
            raise AssertionError(
                f"0x529c {name}: calls={calls}, expected={expected_calls}"
            )
        if bytes(machine.mem_read(table_segment * 16, len(table))) != bytes(
            expected_table
        ):
            raise AssertionError(f"0x529c {name}: table/list mismatch")
        actual_free_bytes = struct.unpack(
            "<I", machine.mem_read(state_segment * 16 + 0x0A46, 4)
        )[0]
        expected_free_bytes = (initial_free_bytes + released_size) & 0xFFFFFFFF
        if actual_free_bytes != expected_free_bytes:
            raise AssertionError(
                f"0x529c {name}: free_bytes={actual_free_bytes:#x}, "
                f"expected={expected_free_bytes:#x}"
            )
        actual_pool_end = struct.unpack(
            "<H", machine.mem_read(state_segment * 16 + 0x0A6A, 2)
        )[0]
        expected_pool_end = (initial_pool_end - paragraphs) & 0xFFFF
        if actual_pool_end != expected_pool_end:
            raise AssertionError(
                f"0x529c {name}: pool_end={actual_pool_end:#x}, "
                f"expected={expected_pool_end:#x}"
            )
        if bytes(machine.mem_read(released_segment * 16, len(pool))) != bytes(
            expected_pool
        ):
            raise AssertionError(f"0x529c {name}: compacted pool mismatch")
        for register, value in initial.items():
            actual_register = machine.reg_read(REGISTERS[register])
            if actual_register != value:
                raise AssertionError(f"0x529c {name}: changed {register}")

        vectors.append(
            {
                "name": name,
                "handle": handle,
                "resident_before": resident_words,
                "resident_after": shifted_words,
                "released_size": released_size,
                "released_paragraphs": paragraphs,
                "moved_bytes": moved_bytes,
                "calls": calls,
            }
        )

    return vectors


def resource_handle_resolve_vectors() -> list[dict[str, object]]:
    table_segment = 0x3800
    cases = [
        ("clear_flags_returns_unloaded", 0x0000, 0x0000, False),
        ("unrelated_flag_returns_unloaded", 0x8004, 0x0013, False),
        ("loaded_bit0_resolves", 0x0001, 0x0007, True),
        ("loaded_bit1_resolves", 0x0002, 0x0025, True),
        ("both_loaded_bits_resolve", 0x0003, 0x0101, True),
        ("handle_index_wraps_to_sixteen_bits", 0x0003, 0x2006, True),
    ]
    vectors = []

    for case_index, (name, entry_flags, handle, loaded) in enumerate(cases):
        entry_offset = (handle * 8) & 0xFFFF
        resource_segment = (0x4100 + case_index * 0x0317) & 0xFFFF
        entry = struct.pack(
            "<HHI",
            resource_segment,
            entry_flags,
            (0x12345678 + case_index * 0x11111111) & 0xFFFFFFFF,
        )
        initial = {
            "eax": 0xA1A10000 | handle,
            "ebx": 0xB2B22345,
            "ecx": 0xC3C33456,
            "edx": 0xD4D44567,
            "esi": 0xE5E55678,
            "edi": 0xF6F66789,
            "ebp": 0x9797789A,
            "sp": 0xFF00,
            "ds": 0x2400,
            "es": 0x2800,
            "fs": table_segment,
            "gs": 0x2C00,
        }

        machine = execute(
            0x5320,
            0x533B,
            initial,
            [(table_segment, entry_offset, entry)],
        )

        expected = dict(initial)
        expected["eax"] = (initial["eax"] & 0xFFFF0000) | int(loaded)
        if loaded:
            expected["ds"] = resource_segment
            expected["esi"] = initial["esi"] & 0xFFFF0000
        for register, value in expected.items():
            actual_register = machine.reg_read(REGISTERS[register])
            if actual_register != value:
                raise AssertionError(
                    f"0x5320 {name}: {register}={actual_register:#x}, "
                    f"expected={value:#x}"
                )
        if bytes(
            machine.mem_read(table_segment * 16 + entry_offset, len(entry))
        ) != entry:
            raise AssertionError(f"0x5320 {name}: handle entry changed")
        if EXE[0x533B] != 0xCB:
            raise AssertionError("0x5320: expected far RET boundary")

        vectors.append(
            {
                "name": name,
                "handle": handle,
                "entry_offset": entry_offset,
                "entry_segment": resource_segment,
                "entry_flags": entry_flags,
                "result": {
                    "loaded": loaded,
                    "ax": int(loaded),
                    "ds": resource_segment if loaded else initial["ds"],
                    "si": 0 if loaded else initial["esi"] & 0xFFFF,
                },
            }
        )

    return vectors


def resource_get_field4_vectors() -> list[dict[str, object]]:
    table_segment = 0x4000
    cases = [
        ("zero_handle_zero_value", 0x0000, 0x00000000),
        ("first_entry_all_bits", 0x0001, 0xFFFFFFFF),
        ("largest_unwrapped_index", 0x1FFF, 0x80000001),
        ("index_wraps_to_zero", 0x2000, 0x12345678),
        ("index_wraps_to_entry_one", 0x4001, 0x89ABCDEF),
        ("high_handle_reads_final_dword", 0xFFFF, 0x55AA00FF),
        ("shift_result_sets_zero", 0x8000, 0xDEADBEEF),
        ("shift_result_sets_sign", 0x1000, 0x01020304),
    ]
    vectors = []

    for case_index, (name, handle, field_04) in enumerate(cases):
        entry_offset = (handle * 8) & 0xFFFF
        entry = struct.pack(
            "<HHI",
            (0x5100 + case_index * 0x101) & 0xFFFF,
            (0xA500 + case_index * 0x13) & 0xFFFF,
            field_04,
        )
        initial = {
            "eax": 0xA1A10000 | handle,
            "ebx": 0xB2B22345,
            "ecx": 0xC3C33456,
            "edx": 0xD4D44567,
            "esi": 0xE5E55678,
            "edi": 0xF6F66789,
            "ebp": 0x9797789A,
            "sp": 0xFF00,
            "ds": 0x2400,
            "es": 0x2800,
            "fs": table_segment,
            "gs": 0x2C00,
            "flags": 0x0AD7,
        }

        machine = execute(
            0x533C,
            0x5348,
            initial,
            [(table_segment, entry_offset, entry)],
        )

        expected = dict(initial)
        del expected["flags"]
        expected["eax"] = field_04
        for register, value in expected.items():
            actual_register = machine.reg_read(REGISTERS[register])
            if actual_register != value:
                raise AssertionError(
                    f"0x533c {name}: {register}={actual_register:#x}, "
                    f"expected={value:#x}"
                )
        if bytes(
            machine.mem_read(table_segment * 16 + entry_offset, len(entry))
        ) != entry:
            raise AssertionError(f"0x533c {name}: handle entry changed")

        shifted = (handle << 3) & 0xFFFF
        flags = machine.reg_read(UC_X86_REG_EFLAGS)
        expected_cf = bool(handle & 0x2000)
        expected_zf = shifted == 0
        expected_sf = bool(shifted & 0x8000)
        expected_pf = (shifted & 0xFF).bit_count() % 2 == 0
        actual_defined_flags = {
            "cf": bool(flags & 0x0001),
            "pf": bool(flags & 0x0004),
            "zf": bool(flags & 0x0040),
            "sf": bool(flags & 0x0080),
        }
        expected_defined_flags = {
            "cf": expected_cf,
            "pf": expected_pf,
            "zf": expected_zf,
            "sf": expected_sf,
        }
        if actual_defined_flags != expected_defined_flags:
            raise AssertionError(
                f"0x533c {name}: flags={actual_defined_flags}, "
                f"expected={expected_defined_flags}"
            )
        if EXE[0x5348] != 0xCB:
            raise AssertionError("0x533c: expected far RET boundary")

        vectors.append(
            {
                "name": name,
                "handle": handle,
                "entry_offset": entry_offset,
                "field_04": field_04,
                "eax": field_04,
                "defined_flags": actual_defined_flags,
            }
        )

    return vectors


def vm_special_slot_remove_vectors() -> list[dict[str, object]]:
    cases = [
        ("absent_owner", list(range(1, 17)), 0x7777, None),
        ("remove_first", [0x1111, *range(2, 17)], 0x1111, 0),
        ("remove_middle", [*range(1, 8), 0x2222, *range(9, 17)], 0x2222, 7),
        ("remove_last", [*range(1, 16), 0x3333], 0x3333, 15),
        (
            "remove_only_first_duplicate",
            [1, 2, 3, 0x4444, 5, 6, 7, 8, 0x4444, 10, 11, 12, 13, 14, 15, 16],
            0x4444,
            3,
        ),
        ("zero_owner_matches_empty_slot", [1, 2, 0, *range(4, 17)], 0, 2),
    ]
    vectors = []

    for case_index, (name, slots, owner, removed_index) in enumerate(cases):
        expected_slots = list(slots)
        if removed_index is not None:
            expected_slots[removed_index] = 0
        vectors.append(
            vm_special_slot_vector(
                0x5FD8,
                0x5FF5,
                name,
                case_index,
                owner,
                slots,
                expected_slots,
                removed_index is not None,
            )
        )

    return vectors


def vm_special_slot_insert_vectors() -> list[dict[str, object]]:
    cases = [
        ("duplicate_first", [0x1111, *range(2, 17)], 0x1111, None, True),
        (
            "duplicate_after_earlier_empty",
            [1, 0, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 0x2222],
            0x2222,
            None,
            True,
        ),
        ("insert_first_empty", [0, *range(2, 17)], 0x3333, 0, True),
        ("insert_middle_empty", [*range(1, 8), 0, *range(9, 17)], 0x4444, 7, True),
        ("insert_last_empty", [*range(1, 16), 0], 0x5555, 15, True),
        ("full_list_fails", list(range(1, 17)), 0x7777, None, False),
        ("zero_owner_matches_empty", [1, 2, 0, *range(4, 17)], 0, None, True),
    ]
    vectors = []

    for case_index, (name, slots, owner, inserted_index, success) in enumerate(cases):
        expected_slots = list(slots)
        if inserted_index is not None:
            expected_slots[inserted_index] = owner
        vectors.append(
            vm_special_slot_vector(
                0x5FF6,
                0x6022,
                name,
                case_index,
                owner,
                slots,
                expected_slots,
                success,
            )
        )

    return vectors


def vm_special_slot_vector(
    entry: int,
    return_address: int,
    name: str,
    case_index: int,
    owner: int,
    slots: list[int],
    expected_slots: list[int],
    success: bool,
) -> dict[str, object]:
    slot_offset = 0x6D3E
    stack_segment = 0x9000
    data_segment = 0x2400
    game_segment = 0x2C00
    slot_bytes = struct.pack("<16H", *slots)
    expected_slot_bytes = struct.pack("<16H", *expected_slots)
    ds_decoy = bytes((index * 17 + case_index * 29 + 3) & 0xFF for index in range(32))
    gs_decoy = bytes((index * 31 + case_index * 13 + 7) & 0xFF for index in range(32))
    initial = {
        "eax": 0xA1A10000 | owner,
        "ebx": 0xB2B22345,
        "ecx": 0xC3C33456,
        "edx": 0xD4D44567,
        "esi": 0xE5E55678,
        "edi": 0xF6F66789,
        "ebp": 0x9797789A,
        "sp": 0xFF00,
        "ds": data_segment,
        "es": 0x2800,
        "fs": 0x3800,
        "gs": game_segment,
        "ss": stack_segment,
        "flags": 0x0AD6,
    }
    machine = execute(
        entry,
        return_address,
        initial,
        [
            (stack_segment, slot_offset, slot_bytes),
            (data_segment, slot_offset, ds_decoy),
            (game_segment, slot_offset, gs_decoy),
        ],
    )

    for register, value in initial.items():
        if register == "flags":
            continue
        actual_register = machine.reg_read(REGISTERS[register])
        if actual_register != value:
            raise AssertionError(
                f"{entry:#x} {name}: {register}={actual_register:#x}, "
                f"expected={value:#x}"
            )
    actual_slot_bytes = bytes(
        machine.mem_read(stack_segment * 16 + slot_offset, len(slot_bytes))
    )
    if actual_slot_bytes != expected_slot_bytes:
        raise AssertionError(f"{entry:#x} {name}: SS slot list mismatch")
    if bytes(
        machine.mem_read(data_segment * 16 + slot_offset, len(ds_decoy))
    ) != ds_decoy:
        raise AssertionError(f"{entry:#x} {name}: DS decoy changed")
    if bytes(
        machine.mem_read(game_segment * 16 + slot_offset, len(gs_decoy))
    ) != gs_decoy:
        raise AssertionError(f"{entry:#x} {name}: GS decoy changed")
    actual_success = bool(machine.reg_read(UC_X86_REG_EFLAGS) & 1)
    if actual_success != success:
        raise AssertionError(
            f"{entry:#x} {name}: carry={actual_success}, expected={success}"
        )
    if EXE[return_address] != 0xC3:
        raise AssertionError(f"{entry:#x}: expected near RET boundary")

    return {
        "name": name,
        "owner": owner,
        "slots_before": slots,
        "slots_after": expected_slots,
        "success_carry": success,
        "storage_segment": "ss",
    }


def vm_field_offset_vectors() -> list[dict[str, object]]:
    table_segment = 0x3C00
    data_segment = 0x2400
    cases = [
        ("zero_selector_kind_bit0_zero", 0x0000, 0x0001, 0x00),
        ("selector_one_kind_bit1_positive", 0x0001, 0x0002, 0x7F),
        ("lowest_of_multiple_kind_bits", 0x0002, 0x00A0, 0x80),
        ("kind_bit15_negative_one", 0x0003, 0x8000, 0xFF),
        ("largest_row_before_wrap", 0x0FFF, 0x8000, 0x01),
        ("selector_shift_wraps_to_zero", 0x1000, 0x0004, 0xFE),
        ("selector_shift_discards_high_bits", 0xF801, 0x0040, 0x55),
        ("negative_byte_preserves_eax_high_word", 0x0007, 0x0010, 0xA5),
    ]
    vectors = []

    for case_index, (name, selector, kind_mask, table_byte) in enumerate(cases):
        bit_index = (kind_mask & -kind_mask).bit_length() - 1
        shifted_selector = (selector << 4) & 0xFFFF
        table_index = (shifted_selector + bit_index) & 0xFFFF
        table_offset = (0x6D60 + table_index) & 0xFFFF
        initial = {
            "eax": 0xA1A10000 | selector,
            "ebx": 0xB2B20000 | kind_mask,
            "ecx": 0xC3C33456,
            "edx": 0xD4D44567,
            "esi": 0xE5E55678,
            "edi": 0xF6F66789,
            "ebp": 0x9797789A,
            "sp": 0xFF00,
            "ds": data_segment,
            "es": 0x2800,
            "fs": 0x3800,
            "gs": table_segment,
            "ss": 0x9000,
            "flags": 0x0AD7,
        }
        ds_decoy = bytes([(table_byte + case_index + 0x31) & 0xFF])
        machine = execute(
            0x6023,
            0x6033,
            initial,
            [
                (table_segment, table_offset, bytes([table_byte])),
                (data_segment, table_offset, ds_decoy),
            ],
        )

        expected = dict(initial)
        del expected["flags"]
        signed_word = table_byte if table_byte < 0x80 else table_byte | 0xFF00
        expected["eax"] = (initial["eax"] & 0xFFFF0000) | signed_word
        for register, value in expected.items():
            actual_register = machine.reg_read(REGISTERS[register])
            if actual_register != value:
                raise AssertionError(
                    f"0x6023 {name}: {register}={actual_register:#x}, "
                    f"expected={value:#x}"
                )
        if machine.mem_read(table_segment * 16 + table_offset, 1)[0] != table_byte:
            raise AssertionError(f"0x6023 {name}: GS table byte changed")
        if bytes(machine.mem_read(data_segment * 16 + table_offset, 1)) != ds_decoy:
            raise AssertionError(f"0x6023 {name}: DS decoy changed")

        flags = machine.reg_read(UC_X86_REG_EFLAGS)
        add_result = table_index
        expected_flags = {
            "cf": False,
            "pf": (add_result & 0xFF).bit_count() % 2 == 0,
            "af": False,
            "zf": add_result == 0,
            "sf": bool(add_result & 0x8000),
            "of": False,
        }
        actual_flags = {
            "cf": bool(flags & 0x0001),
            "pf": bool(flags & 0x0004),
            "af": bool(flags & 0x0010),
            "zf": bool(flags & 0x0040),
            "sf": bool(flags & 0x0080),
            "of": bool(flags & 0x0800),
        }
        if actual_flags != expected_flags:
            raise AssertionError(
                f"0x6023 {name}: flags={actual_flags}, expected={expected_flags}"
            )
        if EXE[0x6033] != 0xC3:
            raise AssertionError("0x6023: expected near RET boundary")

        vectors.append(
            {
                "name": name,
                "selector": selector,
                "kind_mask": kind_mask,
                "lowest_set_bit": bit_index,
                "table_index": table_index,
                "table_byte": table_byte,
                "ax": signed_word,
                "defined_flags": actual_flags,
            }
        )

    return vectors


def vm_record_lookup_by_threshold_vectors() -> list[dict[str, object]]:
    game_segment = 0x2C00
    data_segment = 0x2400
    directory_segment = 0x4200
    decoy_segment = 0x4600
    decoy_offset = 0x0600
    cases = [
        ("below_first_uses_predecessor", 0x0200, 0x0005, 0xBEEF, [0x0010, 0x0030]),
        ("equal_first_uses_predecessor", 0x0300, 0x0010, 0xCAFE, [0x0010, 0x0030]),
        ("between_entries_returns_first", 0x0400, 0x0020, 0x1111, [0x0010, 0x0030]),
        ("equal_second_returns_first", 0x0500, 0x0030, 0x2222, [0x0010, 0x0030]),
        (
            "crosses_multiple_entries",
            0x0600,
            0x0090,
            0x3333,
            [0x0000, 0x0014, 0x0028, 0x004A, 0x0092],
        ),
        (
            "high_threshold_stops_on_ffff",
            0x0700,
            0xFFFF,
            0x4444,
            [0x1000, 0x8000, 0xFFF0, 0xFFFF],
        ),
        (
            "directory_offset_wraps",
            0xFFE0,
            0x0050,
            0x5555,
            [0x0010, 0x0030, 0x0050],
        ),
        ("final_subtract_is_zero", 0x0014, 0x0010, 0x7777, [0x0010, 0x0030]),
        (
            "final_subtract_overflows_signed",
            0x7FEC,
            0x0200,
            0x6666,
            [0x0100, 0x0200],
        ),
    ]
    vectors = []

    for case_index, (name, start, threshold, predecessor, entries) in enumerate(cases):
        stop_index = next(
            index for index, object_offset in enumerate(entries) if threshold <= object_offset
        )
        expected_result = predecessor if stop_index == 0 else entries[stop_index - 1]
        stopped_entry_offset = (start + stop_index * 20) & 0xFFFF
        final_si = (stopped_entry_offset - 20) & 0xFFFF
        pointer = struct.pack("<HH", start, directory_segment)
        decoy_pointer = struct.pack("<HH", decoy_offset, decoy_segment)
        predecessor_field = (start - 20 + 16) & 0xFFFF
        memory = [
            (game_segment, 0x672C, pointer),
            (data_segment, 0x672C, decoy_pointer),
            (directory_segment, predecessor_field, struct.pack("<H", predecessor)),
            (decoy_segment, (decoy_offset - 20 + 16) & 0xFFFF, b"\xad\xde"),
            (decoy_segment, (decoy_offset + 16) & 0xFFFF, b"\xff\xff"),
        ]
        directory_fields = [(predecessor_field, predecessor)]
        for index, object_offset in enumerate(entries):
            field_offset = (start + index * 20 + 16) & 0xFFFF
            memory.append(
                (directory_segment, field_offset, struct.pack("<H", object_offset))
            )
            directory_fields.append((field_offset, object_offset))

        initial = {
            "eax": 0xA1A10000 | threshold,
            "ebx": 0xB2B22345,
            "ecx": 0xC3C33456,
            "edx": 0xD4D44567,
            "esi": 0xE5E55678,
            "edi": 0xF6F66789,
            "ebp": 0x9797789A,
            "sp": 0xFF00,
            "ds": data_segment,
            "es": 0x2800,
            "fs": 0x3800,
            "gs": game_segment,
            "ss": 0x9000,
            "flags": 0x0AD7,
        }
        machine = execute(0x6034, 0x604D, initial, memory)

        expected = dict(initial)
        del expected["flags"]
        expected["eax"] = (initial["eax"] & 0xFFFF0000) | expected_result
        for register, value in expected.items():
            actual_register = machine.reg_read(REGISTERS[register])
            if actual_register != value:
                raise AssertionError(
                    f"0x6034 {name}: {register}={actual_register:#x}, "
                    f"expected={value:#x}"
                )

        if bytes(machine.mem_read(game_segment * 16 + 0x672C, 4)) != pointer:
            raise AssertionError(f"0x6034 {name}: GS directory pointer changed")
        if bytes(machine.mem_read(data_segment * 16 + 0x672C, 4)) != decoy_pointer:
            raise AssertionError(f"0x6034 {name}: DS decoy pointer changed")
        for field_offset, object_offset in directory_fields:
            actual = machine.mem_read(directory_segment * 16 + field_offset, 2)
            if bytes(actual) != struct.pack("<H", object_offset):
                raise AssertionError(f"0x6034 {name}: directory field changed")

        flags = machine.reg_read(UC_X86_REG_EFLAGS)
        expected_flags = {
            "cf": stopped_entry_offset < 20,
            "pf": (final_si & 0xFF).bit_count() % 2 == 0,
            "af": (stopped_entry_offset & 0xF) < (20 & 0xF),
            "zf": final_si == 0,
            "sf": bool(final_si & 0x8000),
            "of": bool(
                ((stopped_entry_offset ^ 20) & (stopped_entry_offset ^ final_si))
                & 0x8000
            ),
        }
        actual_flags = {
            "cf": bool(flags & 0x0001),
            "pf": bool(flags & 0x0004),
            "af": bool(flags & 0x0010),
            "zf": bool(flags & 0x0040),
            "sf": bool(flags & 0x0080),
            "of": bool(flags & 0x0800),
        }
        if actual_flags != expected_flags:
            raise AssertionError(
                f"0x6034 {name}: flags={actual_flags}, expected={expected_flags}"
            )
        if EXE[0x604D] != 0xC3:
            raise AssertionError("0x6034: expected near RET boundary")

        vectors.append(
            {
                "name": name,
                "threshold": threshold,
                "directory_offset": start,
                "predecessor": predecessor,
                "entries": entries,
                "stop_index": stop_index,
                "ax": expected_result,
                "defined_flags": actual_flags,
            }
        )

    return vectors


def active_object_list_vectors() -> list[dict[str, object]]:
    game_segment = 0x2C00
    data_segment = 0x2400
    directory_segment = 0x4200
    record_segment = 0x4600
    decoy_directory_segment = 0x4A00
    decoy_record_segment = 0x4E00
    record_pointer_offset = 0x3100
    cases = [
        (
            "first_entry_stops_scan",
            0x0200,
            [(0x0100, 0x0000, 0x0002), (0x0200, 0x0001, 0x0002)],
            [],
        ),
        (
            "filters_low_flag_byte_and_stops",
            0x0400,
            [
                (0x0100, 0x0001, 0x0202),
                (0x0200, 0x0001, 0x0000),
                (0x0300, 0x0001, 0x00FF),
                (0x0400, 0x0002, 0x0002),
                (0x0500, 0x0001, 0x0002),
            ],
            [0x0100, 0x0300],
        ),
        (
            "high_flag_byte_does_not_qualify",
            0x0800,
            [(0x1110, 0x0001, 0x0200), (0x2220, 0x0001, 0x0002), (0x3330, 0, 0)],
            [0x2220],
        ),
        (
            "directory_and_object_offsets_wrap",
            0xFFF0,
            [(0xFFFF, 0x0001, 0x0002), (0x1234, 0x8000, 0x0002)],
            [0xFFFF],
        ),
        (
            "terminating_kind_two",
            0x0C00,
            [(0x0100, 0x0001, 0x0002), (0x0200, 0x0002, 0x0002)],
            [0x0100],
        ),
    ]
    vectors = []

    for case_index, (name, directory_offset, entries, expected_objects) in enumerate(cases):
        directory_pointer = struct.pack("<HH", directory_offset, directory_segment)
        record_pointer = struct.pack(
            "<HH", record_pointer_offset, record_segment
        )
        decoy_directory_pointer = struct.pack(
            "<HH", 0x1800, decoy_directory_segment
        )
        decoy_record_pointer = struct.pack("<HH", 0x2200, decoy_record_segment)
        initial_output = bytes(
            (case_index * 41 + index * 13 + 7) & 0xFF for index in range(20)
        )
        expected_output = bytearray(initial_output)
        result_words = [*expected_objects, 0xFFFF]
        expected_output[: len(result_words) * 2] = struct.pack(
            f"<{len(result_words)}H", *result_words
        )
        memory = [
            (game_segment, 0x672C, directory_pointer),
            (game_segment, 0x6724, record_pointer),
            (data_segment, 0x672C, decoy_directory_pointer),
            (data_segment, 0x6724, decoy_record_pointer),
            (game_segment, 0x6A16, initial_output),
            (data_segment, 0x6A16, bytes([0xA5]) * len(initial_output)),
            (decoy_directory_segment, 0x1812, b"\x00\x00"),
        ]
        immutable_fields = []
        for index, (object_offset, entry_kind, flags) in enumerate(entries):
            entry_offset = (directory_offset + index * 20) & 0xFFFF
            object_field = (entry_offset + 0x10) & 0xFFFF
            kind_field = (entry_offset + 0x12) & 0xFFFF
            memory.extend(
                [
                    (
                        directory_segment,
                        object_field,
                        struct.pack("<H", object_offset),
                    ),
                    (directory_segment, kind_field, struct.pack("<H", entry_kind)),
                    (
                        record_segment,
                        (object_offset + 2) & 0xFFFF,
                        struct.pack("<H", flags),
                    ),
                    (
                        record_segment,
                        (record_pointer_offset + object_offset + 2) & 0xFFFF,
                        struct.pack("<H", flags ^ 0x0002),
                    ),
                    (
                        decoy_record_segment,
                        (object_offset + 2) & 0xFFFF,
                        struct.pack("<H", flags ^ 0x0002),
                    ),
                ]
            )
            immutable_fields.extend(
                [
                    (directory_segment, object_field, struct.pack("<H", object_offset)),
                    (directory_segment, kind_field, struct.pack("<H", entry_kind)),
                    (
                        record_segment,
                        (object_offset + 2) & 0xFFFF,
                        struct.pack("<H", flags),
                    ),
                ]
            )

        initial = {
            "eax": 0xA1A11234,
            "ebx": 0xB2B22345,
            "ecx": 0xC3C33456,
            "edx": 0xD4D44567,
            "esi": 0xE5E55678,
            "edi": 0xF6F66789,
            "ebp": 0x9797789A,
            "sp": 0xFF00,
            "ds": data_segment,
            "es": 0x2800,
            "fs": 0x3800,
            "gs": game_segment,
            "ss": 0x9000,
            "flags": 0x0AD7,
        }
        machine = execute(0x604E, 0x608E, initial, memory)

        for register, value in initial.items():
            if register == "flags":
                continue
            actual_register = machine.reg_read(REGISTERS[register])
            if actual_register != value:
                raise AssertionError(
                    f"0x604e {name}: {register}={actual_register:#x}, "
                    f"expected={value:#x}"
                )
        actual_output = bytes(
            machine.mem_read(game_segment * 16 + 0x6A16, len(initial_output))
        )
        if actual_output != bytes(expected_output):
            raise AssertionError(f"0x604e {name}: active-object output mismatch")
        if bytes(
            machine.mem_read(data_segment * 16 + 0x6A16, len(initial_output))
        ) != bytes([0xA5]) * len(initial_output):
            raise AssertionError(f"0x604e {name}: DS output decoy changed")
        for segment, offset, expected_bytes in immutable_fields:
            if bytes(machine.mem_read(segment * 16 + offset, len(expected_bytes))) != expected_bytes:
                raise AssertionError(f"0x604e {name}: input table changed")

        terminating_kind = next(kind for _, kind, _ in entries if kind != 1)
        flag_result = (terminating_kind - 1) & 0xFFFF
        flags = machine.reg_read(UC_X86_REG_EFLAGS)
        expected_flags = {
            "cf": terminating_kind < 1,
            "pf": (flag_result & 0xFF).bit_count() % 2 == 0,
            "af": (terminating_kind & 0xF) < 1,
            "zf": flag_result == 0,
            "sf": bool(flag_result & 0x8000),
            "of": bool(
                ((terminating_kind ^ 1) & (terminating_kind ^ flag_result))
                & 0x8000
            ),
        }
        actual_flags = {
            "cf": bool(flags & 0x0001),
            "pf": bool(flags & 0x0004),
            "af": bool(flags & 0x0010),
            "zf": bool(flags & 0x0040),
            "sf": bool(flags & 0x0080),
            "of": bool(flags & 0x0800),
        }
        if actual_flags != expected_flags:
            raise AssertionError(
                f"0x604e {name}: flags={actual_flags}, expected={expected_flags}"
            )
        if EXE[0x608E] != 0xC3:
            raise AssertionError("0x604e: expected near RET boundary")

        vectors.append(
            {
                "name": name,
                "directory_offset": directory_offset,
                "record_pointer_offset_ignored": record_pointer_offset,
                "entries": [
                    {"object_offset": obj, "entry_kind": kind, "flags": flags}
                    for obj, kind, flags in entries
                ],
                "active_objects": expected_objects,
                "terminator": 0xFFFF,
                "defined_flags": actual_flags,
            }
        )

    return vectors


def ship_3d_position_field_resolve_vectors() -> list[dict[str, object]]:
    game_segment = 0x2C00
    record_segment = 0x4400
    table = {
        (0x0B, 0x0008): 0x04,
        (0x0B, 0x0010): 0x06,
        (0x0B, 0x0200): 0x08,
        (0x0C, 0x0100): 0x02,
        (0x09, 0x0100): 0x04,
        (0x0A, 0x0100): 0x08,
        (0x11, 0x0002): 0x02,
    }
    cases = [
        ("direct_kind8", 0x0100, 0x7777, {0x0100: 0x0008}, 0x0500, 0x0104),
        ("direct_kind10", 0x0100, 0x7777, {0x0100: 0x0010}, 0x0500, 0x0106),
        ("direct_kind200", 0x0100, 0x7777, {0x0100: 0x0200}, 0x0500, 0x0108),
        (
            "parent_link_to_direct",
            0x0100,
            0x7777,
            {0x0100: 0x0002, 0x0102: 0x0300, 0x0300: 0x0010},
            0x0500,
            0x0306,
        ),
        (
            "parent_ffff_falls_back_to_arche",
            0x0100,
            0x7777,
            {0x0100: 0x0002, 0x0102: 0xFFFF, 0x0500: 0x0008},
            0x0500,
            0x0504,
        ),
        (
            "kind100_match",
            0x0100,
            0x7777,
            {0x0100: 0x0100, 0x0102: 0x7777},
            0x0500,
            0x0104,
        ),
        (
            "kind100_mismatch",
            0x0100,
            0x7777,
            {0x0100: 0x0100, 0x0102: 0x6666},
            0x0500,
            0x0108,
        ),
        ("direct_offset_wrap", 0xFFFC, 0x7777, {0xFFFC: 0x0008}, 0x0500, 0),
    ]
    vectors = []

    for name, record_offset, compare_word, words, arche, expected in cases:
        memory = [(game_segment, 0x6752, struct.pack("<H", arche))]
        table_offsets = []
        for (selector, kind), field_offset in table.items():
            column = (kind & -kind).bit_length() - 1
            offset = (0x6D60 + selector * 16 + column) & 0xFFFF
            table_offsets.append((offset, field_offset))
            memory.append((game_segment, offset, bytes([field_offset])))
            memory.append((record_segment, offset, bytes([field_offset ^ 0x3F])))
        for offset, value in words.items():
            memory.append((record_segment, offset, struct.pack("<H", value)))

        initial = {
            "eax": 0xA1A11234,
            "ebx": 0xB2B22345,
            "ecx": 0xC3C33456,
            "edx": 0xD4D40000 | compare_word,
            # The kind-0x100 path uses [EAX+ESI], not [AX+SI].
            "esi": record_offset,
            "edi": 0xF6F66789,
            "ebp": 0x9797789A,
            "sp": 0xFF00,
            "ds": record_segment,
            "es": 0x2800,
            "fs": 0x3800,
            "gs": game_segment,
            "ss": 0x9000,
            "flags": 0x0AD7,
        }
        machine = execute(0x61A6, 0x620F, initial, memory)

        result = machine.reg_read(UC_X86_REG_EAX)
        if result != expected:
            raise AssertionError(
                f"0x61a6 {name}: eax={result:#x}, expected={expected:#x}"
            )
        for register, value in initial.items():
            if register in {"eax", "flags"}:
                continue
            actual = machine.reg_read(REGISTERS[register])
            if actual != value:
                raise AssertionError(
                    f"0x61a6 {name}: {register}={actual:#x}, expected={value:#x}"
                )
        for offset, value in words.items():
            actual = machine.mem_read(record_segment * 16 + offset, 2)
            if actual != struct.pack("<H", value):
                raise AssertionError(f"0x61a6 {name}: record memory changed")
        for offset, field_offset in table_offsets:
            actual = machine.mem_read(game_segment * 16 + offset, 1)
            if actual != bytes([field_offset]):
                raise AssertionError(f"0x61a6 {name}: field table changed")
        if EXE[0x620F] != 0xC3:
            raise AssertionError("0x61a6: expected near RET boundary")

        vectors.append(
            {
                "name": name,
                "record_offset": record_offset,
                "kind100_compare_word": compare_word,
                "arche_record_offset": arche,
                "resolved_position_offset": expected,
                "eax": expected,
            }
        )

    return vectors


def ship_3d_position_distance_vectors() -> list[dict[str, object]]:
    game_segment = 0x2C00
    record_segment = 0x4400
    table = {
        (0x0B, 0x0040): 0x04,
        (0x0B, 0x0008): 0x04,
        (0x0B, 0x0010): 0x04,
        (0x0B, 0x0200): 0x04,
        (0x0E, 0x0040): 0x10,
        (0x0E, 0x0008): 0x10,
        (0x0C, 0x0100): 0x02,
        (0x09, 0x0100): 0x04,
        (0x0A, 0x0100): 0x08,
        (0x11, 0x0002): 0x02,
    }
    cases = [
        (
            "direct_kind40_three_four_five",
            0x0100,
            0x0200,
            0x9999,
            {0x0100: 0x0040, 0x0104: 100, 0x0106: 100,
             0x0200: 0x0040, 0x0204: 103, 0x0206: 104},
            0x0500,
        ),
        (
            "delegated_direct_kind_wrap_delta_8000",
            0x0100,
            0x0200,
            0x8888,
            {0x0100: 0x0008, 0x0104: 0x7FFF, 0x0106: 5,
             0x0200: 0x0010, 0x0204: 0xFFFF, 0x0206: 5},
            0x0500,
        ),
        (
            "parent_ffff_falls_back_to_arche",
            0x0100,
            0x0200,
            0x7777,
            {0x0100: 0x0002, 0x0102: 0xFFFF,
             0x0200: 0x0200, 0x0204: 13, 0x0206: 14,
             0x0500: 0x0008, 0x0504: 10, 0x0506: 10},
            0x0500,
        ),
        (
            "first_kind100_match",
            0x0100,
            0x0200,
            0x6666,
            {0x0100: 0x0100, 0x0102: 0x7777, 0x0104: 0, 0x0106: 0,
             0x0108: 50, 0x010A: 50, 0x0200: 0x0040,
             0x0204: 6, 0x0206: 8, 0x0210: 0x7777},
            0x0500,
        ),
        (
            "second_kind100_mismatch",
            0x0100,
            0x0200,
            0x5555,
            {0x0100: 0x0040, 0x0104: 4, 0x0106: 5, 0x0110: 0x1111,
             0x0200: 0x0100, 0x0202: 0x2222, 0x0204: 90, 0x0206: 90,
             0x0208: 7, 0x020A: 9},
            0x0500,
        ),
        (
            "inherited_compare_reaches_linked_kind100",
            0x0100,
            0x0200,
            0x5555,
            {0x0100: 0x0008, 0x0104: 1, 0x0106: 1,
             0x0200: 0x0002, 0x0202: 0x0300,
             0x0300: 0x0100, 0x0302: 0x5555,
             0x0304: 9, 0x0306: 16, 0x0308: 40, 0x030A: 40},
            0x0500,
        ),
    ]
    vectors = []

    for name, first_offset, second_offset, inherited, words, arche in cases:
        # The MZ header is 0x600 bytes. A far transfer addresses the loaded
        # image, while this oracle executes near routines at file offsets.
        memory = [
            (0, 0x2833, EXE[0x2E33:0x2E73]),
            (game_segment, 0x6752, struct.pack("<H", arche)),
        ]
        table_offsets = []
        for (selector, kind), field_offset in table.items():
            column = (kind & -kind).bit_length() - 1
            offset = (0x6D60 + selector * 16 + column) & 0xFFFF
            table_offsets.append((offset, field_offset))
            memory.append((game_segment, offset, bytes([field_offset])))
            memory.append((record_segment, offset, bytes([field_offset ^ 0x3F])))
        for offset, value in words.items():
            memory.append((record_segment, offset, struct.pack("<H", value)))

        initial = {
            "eax": 0xA1A11234,
            "ebx": 0xB2B22345,
            "ecx": 0xC3C33456,
            "edx": 0xD4D40000 | inherited,
            # 0x61F5 uses the 32-bit [EAX+ESI] address form, so callers must
            # supply a zero-extended SI even though the surrounding code is
            # otherwise 16-bit.
            "esi": first_offset,
            "edi": 0xF6F60000 | second_offset,
            "ebp": 0x9797789A,
            "sp": 0xFF00,
            "ds": record_segment,
            "es": 0x2800,
            "fs": 0x3800,
            "gs": game_segment,
            "ss": 0x9000,
            "flags": 0x0AD7,
        }
        resolved_fields: dict[str, int] = {}

        def capture_resolved_fields(
            machine: Uc, address: int, _size: int
        ) -> None:
            if address == 0x6173:
                resolved_fields["first"] = machine.reg_read(UC_X86_REG_SI)
                resolved_fields["second"] = machine.reg_read(UC_X86_REG_DI)

        machine = execute(
            0x60DD,
            0x61A5,
            initial,
            memory,
            code_handler=capture_resolved_fields,
        )

        result = machine.reg_read(UC_X86_REG_EAX)
        if set(resolved_fields) != {"first", "second"}:
            raise AssertionError(
                f"0x60dd {name}: coordinate fields were not resolved"
            )
        first_field = resolved_fields["first"]
        second_field = resolved_fields["second"]
        first_x = words[first_field]
        first_y = words[(first_field + 2) & 0xFFFF]
        second_x = words[second_field]
        second_y = words[(second_field + 2) & 0xFFFF]

        def signed_abs_delta(left: int, right: int) -> int:
            value = (left - right) & 0xFFFF
            if value & 0x8000:
                value = (-value) & 0xFFFF
            return value - 0x10000 if value & 0x8000 else value

        dx = signed_abs_delta(first_x, second_x)
        dy = signed_abs_delta(first_y, second_y)
        squared = (dx * dx + dy * dy) & 0xFFFFFFFF
        expected_eax = (squared & 0xFFFF0000) | math.isqrt(squared)
        if result != expected_eax:
            raise AssertionError(
                f"0x60dd {name}: eax={result:#x}, expected={expected_eax:#x}"
            )
        for register, value in initial.items():
            if register in {"eax", "flags"}:
                continue
            actual = machine.reg_read(REGISTERS[register])
            if actual != value:
                raise AssertionError(
                    f"0x60dd {name}: {register}={actual:#x}, expected={value:#x}"
                )
        for offset, value in words.items():
            actual = machine.mem_read(record_segment * 16 + offset, 2)
            if actual != struct.pack("<H", value):
                raise AssertionError(f"0x60dd {name}: record memory changed")
        for offset, field_offset in table_offsets:
            actual = machine.mem_read(game_segment * 16 + offset, 1)
            if actual != bytes([field_offset]):
                raise AssertionError(f"0x60dd {name}: field table changed")
        if machine.mem_read(0x2833, 0x40) != EXE[0x2E33:0x2E73]:
            raise AssertionError(f"0x60dd {name}: mirrored sqrt code changed")
        if EXE[0x61A5] != 0xC3:
            raise AssertionError("0x60dd: expected near RET boundary")

        vectors.append(
            {
                "name": name,
                "first_record_offset": first_offset,
                "second_record_offset": second_offset,
                "inherited_compare_word": inherited,
                "first_position_offset": first_field,
                "second_position_offset": second_field,
                "squared_distance": squared,
                "eax": expected_eax,
            }
        )

    return vectors


def ship_3d_object_table_bit_test_vectors() -> list[dict[str, object]]:
    game_segment = 0x2C00
    data_segment = 0x4400
    directory_segment = 0x4800
    cases = [
        ("index0_high_bit_set", 0, 0x001E, 0x0100, 0x80, 0x0200),
        ("index0_high_bit_clear", 0, 0x001E, 0x0100, 0x7F, 0x0200),
        ("index1_bit6_set", 1, 0x001E, 0x0100, 0x40, 0x0200),
        ("index7_low_bit_set", 7, 0x001E, 0x0100, 0x01, 0x0200),
        ("index8_next_byte_high_set", 8, 0x001E, 0x0100, 0x80, 0x0200),
        ("index15_next_byte_low_clear", 15, 0x001E, 0x0100, 0xFE, 0x0200),
        ("signed_field_offset_wrap", 0, 0xFFFE, 0x0001, 0x80, 0x0200),
        ("directory_stride_wrap", 1, 0x001E, 0x0100, 0x40, 0xFFF8),
    ]
    table_offset = 0x6DB1
    vectors = []

    for name, index, field_offset, bitset_base, value, directory_base in cases:
        target = 0x7000
        directory_words = []
        memory = [
            (
                game_segment,
                0x672C,
                struct.pack("<HH", directory_base, directory_segment),
            ),
            (data_segment, 0x672C, struct.pack("<HH", 0x1234, 0x5678)),
            (game_segment, table_offset, bytes([field_offset & 0xFF])),
            (data_segment, table_offset, bytes([(field_offset ^ 0x3F) & 0xFF])),
        ]
        for entry_index in range(index + 1):
            object_offset = (
                target if entry_index == index else 0x1000 + entry_index * 0x14
            )
            offset = (directory_base + entry_index * 20 + 0x10) & 0xFFFF
            directory_words.append((offset, object_offset))
            memory.append(
                (directory_segment, offset, struct.pack("<H", object_offset))
            )

        byte_offset = (
            bitset_base
            + (field_offset if field_offset < 0x8000 else field_offset - 0x10000)
            + (index >> 3)
        ) & 0xFFFF
        memory.append((data_segment, byte_offset, bytes([value])))
        memory.append((game_segment, byte_offset, bytes([value ^ 0xFF])))

        initial = {
            "eax": 0xA1A10000 | target,
            "ebx": 0xB2B22345,
            "ecx": 0xC3C33456,
            "edx": 0xD4D44567,
            "esi": 0xE5E50000 | bitset_base,
            "edi": 0xF6F66789,
            "ebp": 0x9797789A,
            "sp": 0xFF00,
            "ds": data_segment,
            "es": 0x2800,
            "fs": 0x3800,
            "gs": game_segment,
            "ss": 0x9000,
            "flags": 0x0AD7,
        }
        machine = execute(0x6210, 0x624A, initial, memory)

        for register, expected in initial.items():
            if register == "flags":
                continue
            actual = machine.reg_read(REGISTERS[register])
            if actual != expected:
                raise AssertionError(
                    f"0x6210 {name}: {register}={actual:#x}, expected={expected:#x}"
                )

        shift_count = (index & 7) + 1
        carry = (value >> (8 - shift_count)) & 1
        shifted = (value << shift_count) & 0xFF
        flags = machine.reg_read(UC_X86_REG_EFLAGS)
        expected_flags = {
            "carry": carry,
            "zero": int(shifted == 0),
            "sign": (shifted >> 7) & 1,
            "parity": int((shifted.bit_count() & 1) == 0),
        }
        actual_flags = {
            "carry": flags & 1,
            "zero": (flags >> 6) & 1,
            "sign": (flags >> 7) & 1,
            "parity": (flags >> 2) & 1,
        }
        if shift_count == 1:
            expected_flags["overflow"] = ((shifted >> 7) & 1) ^ carry
            actual_flags["overflow"] = (flags >> 11) & 1
        if actual_flags != expected_flags:
            raise AssertionError(
                f"0x6210 {name}: flags={actual_flags}, expected={expected_flags}"
            )

        for offset, object_offset in directory_words:
            actual = machine.mem_read(directory_segment * 16 + offset, 2)
            if actual != struct.pack("<H", object_offset):
                raise AssertionError(f"0x6210 {name}: directory memory changed")
        if machine.mem_read(game_segment * 16 + table_offset, 1) != bytes(
            [field_offset & 0xFF]
        ):
            raise AssertionError(f"0x6210 {name}: field table changed")
        if machine.mem_read(data_segment * 16 + byte_offset, 1) != bytes([value]):
            raise AssertionError(f"0x6210 {name}: bitset memory changed")
        if EXE[0x624A] != 0xC3:
            raise AssertionError("0x6210: expected near RET boundary")

        vectors.append(
            {
                "name": name,
                "object_offset": target,
                "directory_index": index,
                "directory_offset": directory_base,
                "field_offset": field_offset,
                "bitset_base": bitset_base,
                "bitset_byte_offset": byte_offset,
                "bitset_byte": value,
                "shift_count": shift_count,
                "defined_flags": expected_flags,
            }
        )

    return vectors


def ship_3d_nav_source_list_build_vectors() -> list[dict[str, object]]:
    game_segment = 0x2C00
    data_segment = 0x4400
    directory_segment = 0x4800
    object_segment = 0x4C00
    cases = [
        {
            "name": "no_children",
            "target": 0x5000,
            "entries": [(0x1000, 0x0002, 0x6000, 0x0002, 1)],
            "sentinel_kind": 0,
            "output_base": 0x0100,
            "expected": [],
            "directory_base": 0x0200,
        },
        {
            "name": "one_child",
            "target": 0x5000,
            "entries": [(0x1000, 0x0002, 0x5000, 0x0002, 1)],
            "sentinel_kind": 2,
            "output_base": 0x0100,
            "expected": [0x1000],
            "directory_base": 0x0200,
        },
        {
            "name": "two_siblings",
            "target": 0x5000,
            "entries": [
                (0x1000, 0x0002, 0x5000, 0x0002, 1),
                (0x1100, 0x0004, 0x5000, 0x0004, 1),
            ],
            "sentinel_kind": 0xFFFF,
            "output_base": 0x0100,
            "expected": [0x1000, 0x1100],
            "directory_base": 0x0200,
        },
        {
            "name": "depth_first_child_before_sibling",
            "target": 0x5000,
            "entries": [
                (0x1000, 0x0002, 0x5000, 0x0002, 1),
                (0x1100, 0x0004, 0x1000, 0x0004, 1),
                (0x1200, 0x0008, 0x5000, 0x0006, 1),
            ],
            "sentinel_kind": 7,
            "output_base": 0x0100,
            "expected": [0x1000, 0x1100, 0x1200],
            "directory_base": 0x0200,
        },
        {
            "name": "zero_field_offset_is_skipped",
            "target": 0x5000,
            "entries": [
                (0x1000, 0x0020, 0x5000, 0x0000, 1),
                (0x1100, 0x0002, 0x5000, 0x0002, 1),
            ],
            "sentinel_kind": 0,
            "output_base": 0x0100,
            "expected": [0x1100],
            "directory_base": 0x0200,
        },
        {
            "name": "next_inactive_entry_stops_scan",
            "target": 0x5000,
            "entries": [
                (0x1000, 0x0002, 0x5000, 0x0002, 1),
                (0x1100, 0x0004, 0x5000, 0x0004, 0),
            ],
            "sentinel_kind": 1,
            "output_base": 0x0100,
            "expected": [0x1000],
            "directory_base": 0x0200,
        },
        {
            "name": "output_cursor_wrap",
            "target": 0x5000,
            "entries": [
                (0x1000, 0x0002, 0x5000, 0x0002, 1),
                (0x1100, 0x0004, 0x5000, 0x0004, 1),
            ],
            "sentinel_kind": 0,
            "output_base": 0xFFFC,
            "expected": [0x1000, 0x1100],
            "directory_base": 0x0200,
        },
        {
            "name": "directory_and_object_field_wrap",
            "target": 0x5000,
            "entries": [(0xFFFE, 0x0002, 0x5000, 0x0002, 1)],
            "sentinel_kind": 2,
            "output_base": 0x0100,
            "expected": [0xFFFE],
            "directory_base": 0xFFF8,
        },
    ]
    vectors = []

    for case in cases:
        name = str(case["name"])
        target = int(case["target"])
        entries = list(case["entries"])
        sentinel_kind = int(case["sentinel_kind"])
        output_base = int(case["output_base"])
        expected_output = list(case["expected"])
        directory_base = int(case["directory_base"])
        memory = [
            # A direct far-call wrapper stops only after all recursive RETF
            # frames have unwound to the outer caller.
            (0, 0x5C00, b"\x9a\x4b\x62\x00\x00\x90"),
            (
                game_segment,
                0x672C,
                struct.pack("<HH", directory_base, directory_segment),
            ),
            (data_segment, 0x672C, struct.pack("<HH", 0x1234, 0x5678)),
        ]
        immutable_fields = []
        table_fields = set()

        for index, (
            object_offset,
            kind,
            parent,
            field_offset,
            entry_kind,
        ) in enumerate(entries):
            directory_offset = (directory_base + index * 20) & 0xFFFF
            for offset, value in (
                ((directory_offset + 0x10) & 0xFFFF, object_offset),
                ((directory_offset + 0x12) & 0xFFFF, entry_kind),
            ):
                encoded = struct.pack("<H", value)
                memory.append((directory_segment, offset, encoded))
                immutable_fields.append((directory_segment, offset, encoded))

            kind_bytes = struct.pack("<H", kind)
            memory.append((object_segment, object_offset, kind_bytes))
            memory.append(
                (data_segment, object_offset, struct.pack("<H", kind ^ 0x5555))
            )
            immutable_fields.append((object_segment, object_offset, kind_bytes))
            if field_offset != 0:
                parent_offset = (object_offset + field_offset) & 0xFFFF
                parent_bytes = struct.pack("<H", parent)
                memory.append((object_segment, parent_offset, parent_bytes))
                immutable_fields.append((object_segment, parent_offset, parent_bytes))

            column = (kind & -kind).bit_length() - 1
            table_offset = (0x6D60 + 0x11 * 16 + column) & 0xFFFF
            table_fields.add((table_offset, field_offset & 0xFF))

        stop_index = next(
            (index for index in range(1, len(entries)) if int(entries[index][4]) != 1),
            len(entries),
        )
        stop_kind = (
            int(entries[stop_index][4]) if stop_index < len(entries) else sentinel_kind
        )
        stop_offset = (directory_base + stop_index * 20 + 0x12) & 0xFFFF
        stop_bytes = struct.pack("<H", stop_kind)
        memory.append((directory_segment, stop_offset, stop_bytes))
        immutable_fields.append((directory_segment, stop_offset, stop_bytes))

        for table_offset, field_offset in table_fields:
            memory.append((game_segment, table_offset, bytes([field_offset])))
            memory.append((data_segment, table_offset, bytes([field_offset ^ 0x3F])))
            immutable_fields.append((game_segment, table_offset, bytes([field_offset])))

        output_words = len(expected_output) + 3
        for index in range(-1, output_words):
            offset = (output_base + index * 2) & 0xFFFF
            value = (0xA500 + index) & 0xFFFF
            memory.append((0x9000, offset, struct.pack("<H", value)))
            memory.append((data_segment, offset, struct.pack("<H", value ^ 0xFFFF)))

        initial = {
            "eax": 0xA1A11234,
            "ebx": 0xB2B22345,
            "ecx": 0xC3C33456,
            "edx": 0xD4D44567,
            "esi": 0xE5E55678,
            "edi": 0xF6F60000 | target,
            "ebp": 0x97970000 | output_base,
            "sp": 0xFF00,
            "ds": data_segment,
            "es": object_segment,
            "fs": 0x3800,
            "gs": game_segment,
            "ss": 0x9000,
            "flags": 0x0AD7,
        }
        entries_and_terminators = []

        def capture_recursion(_machine: Uc, address: int, _size: int) -> None:
            if address in {0x624B, 0x6289}:
                entries_and_terminators.append(address)

        machine = execute(
            0x5C00,
            0x5C05,
            initial,
            memory,
            code_handler=capture_recursion,
        )

        expected_registers = dict(initial)
        del expected_registers["flags"]
        final_output_offset = (output_base + len(expected_output) * 2) & 0xFFFF
        expected_registers["ebp"] = (initial["ebp"] & 0xFFFF0000) | final_output_offset
        for register, expected in expected_registers.items():
            actual = machine.reg_read(REGISTERS[register])
            if actual != expected:
                raise AssertionError(
                    f"0x624b {name}: {register}={actual:#x}, expected={expected:#x}"
                )

        actual_output = [
            struct.unpack(
                "<H",
                machine.mem_read(0x9000 * 16 + ((output_base + index * 2) & 0xFFFF), 2),
            )[0]
            for index in range(len(expected_output) + 1)
        ]
        if actual_output != expected_output + [0xFFFF]:
            raise AssertionError(
                f"0x624b {name}: output={actual_output}, "
                f"expected={expected_output + [0xFFFF]}, "
                f"trace={entries_and_terminators}"
            )
        for index in range(len(expected_output) + 1):
            offset = (output_base + index * 2) & 0xFFFF
            expected_decoy = ((0xA500 + index) & 0xFFFF) ^ 0xFFFF
            actual_decoy = struct.unpack(
                "<H", machine.mem_read(data_segment * 16 + offset, 2)
            )[0]
            if actual_decoy != expected_decoy:
                raise AssertionError(f"0x624b {name}: DS output decoy changed")
        for segment, offset, expected_bytes in immutable_fields:
            actual = bytes(machine.mem_read(segment * 16 + offset, len(expected_bytes)))
            if actual != expected_bytes:
                raise AssertionError(f"0x624b {name}: input memory changed")

        result = (stop_kind - 1) & 0xFFFF
        flags = machine.reg_read(UC_X86_REG_EFLAGS)
        expected_flags = {
            "cf": stop_kind < 1,
            "pf": (result & 0xFF).bit_count() % 2 == 0,
            "af": (stop_kind & 0xF) < 1,
            "zf": result == 0,
            "sf": bool(result & 0x8000),
            "of": bool(((stop_kind ^ 1) & (stop_kind ^ result)) & 0x8000),
        }
        actual_flags = {
            "cf": bool(flags & 0x0001),
            "pf": bool(flags & 0x0004),
            "af": bool(flags & 0x0010),
            "zf": bool(flags & 0x0040),
            "sf": bool(flags & 0x0080),
            "of": bool(flags & 0x0800),
        }
        if actual_flags != expected_flags:
            raise AssertionError(
                f"0x624b {name}: flags={actual_flags}, expected={expected_flags}"
            )
        if EXE[0x6292] != 0xCB:
            raise AssertionError("0x624b: expected far RET boundary")

        vectors.append(
            {
                "name": name,
                "target_offset": target,
                "directory_offset": directory_base,
                "processed_entry_count": stop_index,
                "output_base": output_base,
                "output_offsets": expected_output,
                "final_output_offset": final_output_offset,
                "terminator": 0xFFFF,
                "defined_flags": expected_flags,
            }
        )

    return vectors


def vm_token_special_vectors() -> list[dict[str, object]]:
    data_segment = 0x4400
    cases = [
        ("immediate_with_extra_byte", 0x1000, 0xBEEF, 0, True),
        ("immediate_without_extra_byte", 0x1100, 0xBEEF, 0, False),
        ("aligned_match_after_six_bytes", 0x1200, 0x1234, 6, True),
        ("unaligned_match_after_five_bytes", 0x1300, 0x1234, 5, False),
        ("scan_cursor_wraps_to_one", 0xFFFD, 0xA1B2, 4, True),
        ("match_word_crosses_offset_end", 0xFFFF, 0xCAFE, 0, False),
        ("post_match_add_wraps", 0xFFFE, 0x1357, 0, False),
        ("optional_increment_wraps", 0xFFFD, 0x2468, 0, True),
        ("optional_increment_signed_overflow", 0x7FFD, 0x00FF, 0, True),
    ]
    vectors = []

    for case_index, (name, start, terminator, scan_count, trailing_equal) in enumerate(
        cases
    ):
        bytes_by_linear_offset = {}
        for step in range(scan_count + 1):
            cursor = (start + step) & 0xFFFF
            bytes_by_linear_offset.setdefault(cursor, 0x55)
            bytes_by_linear_offset.setdefault(cursor + 1, 0x55)

        match_offset = (start + scan_count) & 0xFFFF
        bytes_by_linear_offset[match_offset] = terminator & 0xFF
        bytes_by_linear_offset[match_offset + 1] = terminator >> 8
        trailing_offset = (match_offset + 2) & 0xFFFF
        trailing = terminator & 0xFF
        if not trailing_equal:
            trailing = (trailing + 1 + case_index) & 0xFF
        bytes_by_linear_offset[trailing_offset] = trailing

        for step in range(scan_count):
            cursor = (start + step) & 0xFFFF
            scanned_word = (
                bytes_by_linear_offset[cursor]
                | (bytes_by_linear_offset[cursor + 1] << 8)
            )
            if scanned_word == terminator:
                raise AssertionError(f"0x6293 {name}: accidental early match")

        memory = [
            (data_segment, offset, bytes([value]))
            for offset, value in sorted(bytes_by_linear_offset.items())
        ]
        decoy = struct.pack("<H", terminator) + bytes([terminator & 0xFF])
        memory.extend(
            [
                (0x4800, start, decoy),
                (0x4C00, start, decoy),
            ]
        )
        initial = {
            "eax": 0xA1A10000 | terminator,
            "ebx": 0xB2B22345,
            "ecx": 0xC3C33456,
            "edx": 0xD4D44567,
            "esi": 0xE5E50000 | start,
            "edi": 0xF6F66789,
            "ebp": 0x9797789A,
            "sp": 0xFF00,
            "ds": data_segment,
            "es": 0x4800,
            "fs": 0x3800,
            "gs": 0x4C00,
            "ss": 0x9000,
            "flags": 0x0AD7,
        }

        machine = execute(0x6293, 0x62A2, initial, memory)

        post_match_offset = (match_offset + 2) & 0xFFFF
        final_offset = (
            (post_match_offset + 1) & 0xFFFF
            if trailing_equal
            else post_match_offset
        )
        expected_registers = dict(initial)
        del expected_registers["flags"]
        expected_registers["esi"] = (
            (initial["esi"] & 0xFFFF0000) | final_offset
        )
        for register, expected in expected_registers.items():
            actual = machine.reg_read(REGISTERS[register])
            if actual != expected:
                raise AssertionError(
                    f"0x6293 {name}: {register}={actual:#x}, expected={expected:#x}"
                )

        if trailing_equal:
            result = final_offset
            expected_flags = {
                "cf": False,
                "pf": (result & 0xFF).bit_count() % 2 == 0,
                "af": (post_match_offset & 0x0F) == 0x0F,
                "zf": result == 0,
                "sf": bool(result & 0x8000),
                "of": post_match_offset == 0x7FFF,
            }
        else:
            left = terminator & 0xFF
            result = (left - trailing) & 0xFF
            expected_flags = {
                "cf": left < trailing,
                "pf": result.bit_count() % 2 == 0,
                "af": (left & 0x0F) < (trailing & 0x0F),
                "zf": result == 0,
                "sf": bool(result & 0x80),
                "of": bool(((left ^ trailing) & (left ^ result)) & 0x80),
            }
        flags = machine.reg_read(UC_X86_REG_EFLAGS)
        actual_flags = {
            "cf": bool(flags & 0x0001),
            "pf": bool(flags & 0x0004),
            "af": bool(flags & 0x0010),
            "zf": bool(flags & 0x0040),
            "sf": bool(flags & 0x0080),
            "of": bool(flags & 0x0800),
        }
        if actual_flags != expected_flags:
            raise AssertionError(
                f"0x6293 {name}: flags={actual_flags}, expected={expected_flags}"
            )

        for offset, expected in bytes_by_linear_offset.items():
            actual = machine.mem_read(data_segment * 16 + offset, 1)[0]
            if actual != expected:
                raise AssertionError(f"0x6293 {name}: input memory changed")
        if machine.mem_read(0x4800 * 16 + start, 3) != decoy:
            raise AssertionError(f"0x6293 {name}: ES decoy changed")
        if machine.mem_read(0x4C00 * 16 + start, 3) != decoy:
            raise AssertionError(f"0x6293 {name}: GS decoy changed")
        if EXE[0x62A2] != 0xC3:
            raise AssertionError("0x6293: expected near RET boundary")

        vectors.append(
            {
                "name": name,
                "terminator": terminator,
                "start_offset": start,
                "scan_byte_count": scan_count,
                "match_offset": match_offset,
                "trailing_byte": trailing,
                "extra_byte_consumed": trailing_equal,
                "final_offset": final_offset,
                "defined_flags": expected_flags,
            }
        )

    return vectors


def vm_condition_5_vectors() -> list[dict[str, object]]:
    game_segment = 0x2C00
    script_segment = 0x4400
    record_segment = 0x4800
    stack_segment = 0x5000
    record_offset = 0x0800
    history_offset = 0x2000
    field_offset = 0x0006
    cases = [
        {
            "name": "no_conditions_succeeds",
            "control": 0x00,
            "detail": 0x00,
            "script": b"\x5a",
            "success": True,
        },
        {
            "name": "random_gate_zero_succeeds",
            "control": 0x02,
            "detail": 0x00,
            "prng": 0,
            "script": b"\x5a",
            "success": True,
        },
        {
            "name": "random_gate_nonzero_short_circuits",
            "control": 0x32,
            "detail": 0x00,
            "prng": 1,
            "script": b"\xff\xff\x34\x12\x00\x00",
            "success": False,
        },
        {
            "name": "field_equality_succeeds",
            "control": 0x04,
            "detail": 0x01,
            "record_word": 0x1234,
            "script": struct.pack("<H", 0x1234),
            "success": True,
        },
        {
            "name": "field_equality_fails",
            "control": 0x04,
            "detail": 0x01,
            "record_word": 0x1234,
            "script": struct.pack("<H", 0x5678),
            "success": False,
        },
        {
            "name": "field_signed_greater_succeeds",
            "control": 0x04,
            "detail": 0x00,
            "record_word": 0x0000,
            "script": struct.pack("<H", 0xFFFF),
            "success": True,
        },
        {
            "name": "field_signed_greater_fails",
            "control": 0x04,
            "detail": 0x00,
            "record_word": 0xFFFF,
            "script": struct.pack("<H", 0x0000),
            "success": False,
        },
        {
            "name": "field_inverted_order_succeeds",
            "control": 0x84,
            "detail": 0x00,
            "record_word": 0x0000,
            "script": struct.pack("<H", 0xFFFF),
            "success": True,
        },
        {
            "name": "history_list_accepts_recent_words",
            "control": 0x40,
            "detail": 0x00,
            "script": b"\xaa\xff\xff" + struct.pack("<3H", 0x1111, 0x2222, 0),
            "history": [0x2222, 0x1111, 3, 4, 5, 6, 7, 8],
            "ring_index": 4,
            "success": True,
        },
        {
            "name": "history_list_rejects_missing_recent_word",
            "control": 0x40,
            "detail": 0x00,
            "script": b"\xaa\xff\xff" + struct.pack("<3H", 0x1111, 0x2222, 0),
            "history": [0x9999, 0x1111, 3, 4, 5, 6, 7, 8],
            "ring_index": 4,
            "success": False,
        },
        {
            "name": "duplicate_history_slots_satisfy_required_count",
            "control": 0x40,
            "detail": 0x02,
            "script": b"\xbb\xff\xff" + struct.pack("<2H", 0x3333, 0),
            "history": [0x3333, 0x3333, 3, 4, 5, 6, 7, 8],
            "success": True,
        },
        {
            "name": "required_history_hits_zero_sentinel",
            "control": 0x40,
            "detail": 0x03,
            "script": b"\xbb\xff\xff" + struct.pack("<2H", 0x1111, 0),
            "history": [0x1111, 2, 3, 4, 5, 6, 7, 8],
            "success": False,
        },
        {
            "name": "text_word_mode_is_set",
            "control": 0x20,
            "detail": 0x00,
            "script": b"\x5a",
            "text_mode": 1,
            "success": True,
        },
        {
            "name": "presentation_words_copy_to_stack_segment",
            "control": 0x10,
            "detail": 0x00,
            "script": b"\xaa\xff\xff" + struct.pack("<3H", 0x1234, 0x5678, 0),
            "yield_flag": 1,
            "output": [0x1234, 0x5678, 0],
            "success": True,
        },
        {
            "name": "combined_cursor_and_side_effect_paths",
            "control": 0x74,
            "detail": 0x01,
            "record_word": 0x2222,
            "script": (
                struct.pack("<H", 0x2222)
                + b"\x99\xff\xff\xff"
                + struct.pack("<H", 0x3333)
                + b"\x88\xff\xff\xff"
                + struct.pack("<2H", 0x4444, 0)
            ),
            "history": [0x3333, 2, 3, 4, 5, 6, 7, 8],
            "text_mode": 1,
            "yield_flag": 1,
            "output": [0x4444, 0],
            "success": True,
        },
    ]
    vectors = []

    for case_index, case in enumerate(cases):
        name = str(case["name"])
        control = int(case["control"])
        detail = int(case["detail"])
        script = bytes(case["script"])
        script_offset = 0x1000 + case_index * 0x0100
        history = list(case.get("history", [1, 2, 3, 4, 5, 6, 7, 8]))
        ring_index = int(case.get("ring_index", 0))
        record_word = int(case.get("record_word", 0xA55A))
        prng_result = int(case.get("prng", 0))
        table_index = ((((detail >> 1) & 7) + 1) * 16 + 1) & 0xFFFF
        table_offset = (0x6D60 + table_index) & 0xFFFF
        history_bytes = struct.pack("<8H", *history)
        initial_stack_output = bytes([0xA5]) * 32
        initial_game_output = bytes([0xC3]) * 32
        initial_data_output = bytes([0xD4]) * 32
        memory = [
            (0, 0x27E2, b"\xb8" + struct.pack("<H", prng_result) + b"\xcb"),
            (script_segment, script_offset, script),
            (
                record_segment,
                record_offset + field_offset,
                struct.pack("<H", record_word),
            ),
            (record_segment, history_offset, history_bytes),
            (game_segment, 0x6744, struct.pack("<H", ring_index)),
            (
                game_segment,
                0x6746,
                struct.pack("<HH", history_offset, record_segment),
            ),
            (game_segment, table_offset, bytes([field_offset])),
            (script_segment, table_offset, bytes([field_offset ^ 0x3F])),
            (game_segment, 0x67B9, b"\x00"),
            (game_segment, 0x67B4, b"\x00"),
            (stack_segment, 0x67F8, initial_stack_output),
            (game_segment, 0x67F8, initial_game_output),
            (script_segment, 0x67F8, initial_data_output),
        ]
        initial = {
            "eax": 0xA1A11234,
            "ebx": 0xB2B22345,
            "ecx": 0xC3C30000 | (detail << 8) | control,
            "edx": 0xD4D44567,
            "esi": 0xE5E50000 | script_offset,
            "edi": 0xF6F60000 | record_offset,
            "ebp": 0x9797789A,
            "sp": 0xFF00,
            "ds": script_segment,
            "es": record_segment,
            "fs": 0x3800,
            "gs": game_segment,
            "ss": stack_segment,
            "flags": 0x0AD7,
        }
        prng_calls = []

        def capture_prng(_machine: Uc, address: int, _size: int) -> None:
            if address == 0x27E2:
                prng_calls.append(address)

        machine = execute(
            0x6339,
            0x6432,
            initial,
            memory,
            code_handler=capture_prng,
        )

        expected_prng_calls = 1 if control & 0x02 else 0
        if len(prng_calls) != expected_prng_calls:
            raise AssertionError(
                f"0x6339 {name}: prng calls={len(prng_calls)}, "
                f"expected={expected_prng_calls}"
            )
        flags = machine.reg_read(UC_X86_REG_EFLAGS)
        success = bool(flags & 0x0001)
        if success != bool(case["success"]):
            raise AssertionError(
                f"0x6339 {name}: success={success}, expected={case['success']}"
            )

        for register in (
            "ecx",
            "esi",
            "edi",
            "ebp",
            "sp",
            "ds",
            "es",
            "fs",
            "gs",
            "ss",
        ):
            actual = machine.reg_read(REGISTERS[register])
            if actual != initial[register]:
                raise AssertionError(f"0x6339 {name}: changed {register}")

        expected_text_mode = int(case.get("text_mode", 0))
        expected_yield_flag = int(case.get("yield_flag", 0))
        if machine.mem_read(game_segment * 16 + 0x67B9, 1)[0] != expected_text_mode:
            raise AssertionError(f"0x6339 {name}: text-word mode mismatch")
        if machine.mem_read(game_segment * 16 + 0x67B4, 1)[0] != expected_yield_flag:
            raise AssertionError(f"0x6339 {name}: yield flag mismatch")

        expected_output_words = list(case.get("output", []))
        expected_stack_output = bytearray(initial_stack_output)
        for index, value in enumerate(expected_output_words):
            struct.pack_into("<H", expected_stack_output, index * 2, value)
        actual_stack_output = bytes(
            machine.mem_read(stack_segment * 16 + 0x67F8, 32)
        )
        if actual_stack_output != bytes(expected_stack_output):
            raise AssertionError(f"0x6339 {name}: stack output mismatch")
        if (
            bytes(machine.mem_read(game_segment * 16 + 0x67F8, 32))
            != initial_game_output
        ):
            raise AssertionError(f"0x6339 {name}: GS output decoy changed")
        if (
            bytes(machine.mem_read(script_segment * 16 + 0x67F8, 32))
            != initial_data_output
        ):
            raise AssertionError(f"0x6339 {name}: DS output decoy changed")

        if (
            bytes(
                machine.mem_read(script_segment * 16 + script_offset, len(script))
            )
            != script
        ):
            raise AssertionError(f"0x6339 {name}: script changed")
        if (
            bytes(machine.mem_read(record_segment * 16 + history_offset, 16))
            != history_bytes
        ):
            raise AssertionError(f"0x6339 {name}: history changed")
        if machine.mem_read(
            record_segment * 16 + record_offset + field_offset, 2
        ) != struct.pack("<H", record_word):
            raise AssertionError(f"0x6339 {name}: record changed")
        if EXE[0x6432] != 0xC3:
            raise AssertionError("0x6339: expected near RET boundary")

        vectors.append(
            {
                "name": name,
                "control": control,
                "detail": detail,
                "prng_result": prng_result if control & 0x02 else None,
                "script_offset": script_offset,
                "record_word": record_word if control & 0x04 else None,
                "history_words": history if control & 0x40 else None,
                "text_word_list_mode": expected_text_mode,
                "yield_flag": expected_yield_flag,
                "presentation_words": expected_output_words,
                "success_carry": success,
            }
        )

    return vectors


def dic_word_lookup_vectors() -> list[dict[str, object]]:
    game_segment = 0x2C00
    dic_segment = 0x4400
    directory_segment = 0x4800
    cases = [
        (
            "first_active_entry_matches",
            0x1000,
            3,
            0x2000,
            b"HELLO\0",
            [(b"HELLO\0", 0x1111, 1), (b"", 0xEEEE, 0)],
            0x1111,
            True,
            1,
        ),
        (
            "second_active_entry_matches",
            0x1000,
            0,
            0x2000,
            b"BETA\0",
            [(b"ALPHA\0", 0x1111, 1), (b"BETA\0", 0x2222, 1), (b"", 0xEEEE, 0)],
            0x2222,
            True,
            2,
        ),
        (
            "first_entry_inactive",
            0x1000,
            0,
            0x2000,
            b"ANY\0",
            [(b"ANY\0", 0x3333, 0)],
            0x3333,
            False,
            0,
        ),
        (
            "active_miss_returns_inactive_object",
            0x1000,
            0,
            0x2000,
            b"MISSING\0",
            [(b"OTHER\0", 0x1111, 1), (b"", 0x4444, 7)],
            0x4444,
            False,
            1,
        ),
        (
            "prefix_is_not_a_match",
            0x1000,
            0,
            0x2000,
            b"ABC\0",
            [(b"ABCD\0", 0x1111, 1), (b"", 0x5555, 0)],
            0x5555,
            False,
            1,
        ),
        (
            "high_bytes_compare_unsigned",
            0x1000,
            0,
            0x2000,
            b"\x80\xfe\0",
            [(b"\x80\xfe\0", 0x6666, 1), (b"", 0xEEEE, 0)],
            0x6666,
            True,
            1,
        ),
        (
            "dictionary_offset_wraps",
            0xFFFE,
            4,
            0x2000,
            b"WRAP\0",
            [(b"WRAP\0", 0x7777, 1), (b"", 0xEEEE, 0)],
            0x7777,
            True,
            1,
        ),
        (
            "directory_stride_wraps",
            0x1000,
            0,
            0xFFF8,
            b"TARGET\0",
            [(b"OTHER\0", 0x1111, 1), (b"", 0x8888, 0)],
            0x8888,
            False,
            1,
        ),
    ]
    vectors = []

    for (
        name,
        dic_base,
        dictionary_offset,
        directory_base,
        word,
        entries,
        expected_object,
        expected_match,
        expected_compare_calls,
    ) in cases:
        word_offset = (dic_base + dictionary_offset) & 0xFFFF
        memory = [
            # Mirror the executable's far-called string_compare at 01CE:02C4.
            (0, 0x1FA4, EXE[0x25A4:0x25BA]),
            (game_segment, 0x6728, struct.pack("<HH", dic_base, dic_segment)),
            (
                game_segment,
                0x672C,
                struct.pack("<HH", directory_base, directory_segment),
            ),
            (0x2400, 0x6728, struct.pack("<HH", 0xAAAA, 0xBBBB)),
            (0x2400, 0x672C, struct.pack("<HH", 0xCCCC, 0xDDDD)),
        ]
        immutable = []
        for index, value in enumerate(word):
            offset = (word_offset + index) & 0xFFFF
            encoded = bytes([value])
            memory.append((dic_segment, offset, encoded))
            immutable.append((dic_segment, offset, encoded))

        for index, (entry_name, object_offset, entry_kind) in enumerate(entries):
            entry_offset = (directory_base + index * 20) & 0xFFFF
            name_bytes = entry_name.ljust(16, b"\xa5")[:16]
            for byte_index, value in enumerate(name_bytes):
                offset = (entry_offset + byte_index) & 0xFFFF
                encoded = bytes([value])
                memory.append((directory_segment, offset, encoded))
                immutable.append((directory_segment, offset, encoded))
            for relative, value in ((0x10, object_offset), (0x12, entry_kind)):
                offset = (entry_offset + relative) & 0xFFFF
                encoded = struct.pack("<H", value)
                memory.append((directory_segment, offset, encoded))
                immutable.append((directory_segment, offset, encoded))

        initial = {
            "eax": 0xA1A10000 | dictionary_offset,
            "ebx": 0xB2B22345,
            "ecx": 0xC3C33456,
            "edx": 0xD4D44567,
            "esi": 0xE5E55678,
            "edi": 0xF6F66789,
            "ebp": 0x9797789A,
            "sp": 0xFF00,
            "ds": 0x2400,
            "es": 0x2800,
            "fs": 0x3800,
            "gs": game_segment,
            "ss": 0x9000,
            "flags": 0x0AD7,
        }
        compare_calls = []

        def capture_compare(_machine: Uc, address: int, _size: int) -> None:
            if address == 0x1FA4:
                compare_calls.append(address)

        machine = execute(
            0x6433,
            0x6461,
            initial,
            memory,
            code_handler=capture_compare,
        )

        result = machine.reg_read(UC_X86_REG_EAX)
        expected_eax = (initial["eax"] & 0xFFFF0000) | expected_object
        if result != expected_eax:
            raise AssertionError(
                f"0x6433 {name}: eax={result:#x}, expected={expected_eax:#x}"
            )
        flags = machine.reg_read(UC_X86_REG_EFLAGS)
        matched = bool(flags & 0x0001)
        if matched != expected_match:
            raise AssertionError(
                f"0x6433 {name}: matched={matched}, expected={expected_match}"
            )
        if len(compare_calls) != expected_compare_calls:
            raise AssertionError(
                f"0x6433 {name}: compare calls={len(compare_calls)}, "
                f"expected={expected_compare_calls}"
            )

        for register, expected in initial.items():
            if register in {"eax", "flags"}:
                continue
            actual = machine.reg_read(REGISTERS[register])
            if actual != expected:
                raise AssertionError(f"0x6433 {name}: changed {register}")
        for segment, offset, expected in immutable:
            actual = bytes(machine.mem_read(segment * 16 + offset, len(expected)))
            if actual != expected:
                raise AssertionError(f"0x6433 {name}: input memory changed")
        if EXE[0x6461] != 0xC3:
            raise AssertionError("0x6433: expected near RET boundary")

        vectors.append(
            {
                "name": name,
                "dictionary_base": dic_base,
                "dictionary_offset": dictionary_offset,
                "word_offset": word_offset,
                "directory_offset": directory_base,
                "active_compare_calls": expected_compare_calls,
                "object_offset": expected_object,
                "matched_carry": expected_match,
            }
        )

    return vectors


def vm_branch_vectors() -> list[dict[str, object]]:
    game_segment = 0x2C00
    data_segment = 0x4400
    stack_segment = 0x9000
    cases = [
        ("pop_first_word", 0x0002, 0x1234, 0x01),
        ("pop_second_word", 0x0004, 0x5678, 0xFF),
        ("top_wraps_below_zero", 0x0000, 0x9ABC, 0x80),
        ("odd_top_remains_byte_granular", 0x0003, 0xDEF0, 0x7F),
        ("odd_top_wraps", 0x0001, 0x1357, 0x55),
        ("signed_overflow", 0x8000, 0x2468, 0xAA),
        ("stack_effective_offset_wraps", 0x97E2, 0xBEEF, 0x02),
    ]
    vectors = []

    for name, top, target, query_mode in cases:
        new_top = (top - 2) & 0xFFFF
        target_offset = (0x6820 + new_top) & 0xFFFF
        stack_top_decoy = top ^ 0xFFFF
        stack_query_decoy = query_mode ^ 0xFF
        game_target_decoy = target ^ 0xFFFF
        data_target_decoy = target ^ 0x5A5A
        target_bytes = struct.pack("<H", target)
        game_target_bytes = struct.pack("<H", game_target_decoy)
        data_target_bytes = struct.pack("<H", data_target_decoy)
        memory = [
            (game_segment, 0x6884, struct.pack("<H", top)),
            (game_segment, 0x67AD, bytes([query_mode])),
            (stack_segment, target_offset, target_bytes),
            (game_segment, target_offset, game_target_bytes),
            (data_segment, target_offset, data_target_bytes),
            (stack_segment, 0x6884, struct.pack("<H", stack_top_decoy)),
            (stack_segment, 0x67AD, bytes([stack_query_decoy])),
        ]
        initial = {
            "eax": 0xA1A1BEEF,
            "ebx": 0xB2B22345,
            "ecx": 0xC3C33456,
            "edx": 0xD4D44567,
            "esi": 0xE5E55678,
            "edi": 0xF6F66789,
            "ebp": 0x9797789A,
            "sp": 0xFF00,
            "ds": data_segment,
            "es": 0x4800,
            "fs": 0x4C00,
            "gs": game_segment,
            "ss": stack_segment,
            "flags": 0x0AD7,
        }
        pre_clear_state = []

        def capture_pre_clear(machine: Uc, address: int, _size: int) -> None:
            if address == 0x6473:
                pre_clear_state.append(
                    (
                        machine.reg_read(UC_X86_REG_ESI) & 0xFFFF,
                        machine.mem_read(game_segment * 16 + 0x67AD, 1)[0],
                    )
                )

        machine = execute(
            0x6462,
            0x647A,
            initial,
            memory,
            code_handler=capture_pre_clear,
        )

        if pre_clear_state != [(target, query_mode)]:
            raise AssertionError(
                f"0x6462 {name}: pre-clear state={pre_clear_state}, "
                f"expected={[(target, query_mode)]}"
            )

        expected_registers = dict(initial)
        del expected_registers["flags"]
        expected_registers["eax"] = (initial["eax"] & 0xFFFF0000) | new_top
        expected_registers["esi"] = (initial["esi"] & 0xFFFF0000) | target
        for register, expected in expected_registers.items():
            actual = machine.reg_read(REGISTERS[register])
            if actual != expected:
                raise AssertionError(
                    f"0x6462 {name}: {register}={actual:#x}, expected={expected:#x}"
                )

        actual_top = struct.unpack(
            "<H", machine.mem_read(game_segment * 16 + 0x6884, 2)
        )[0]
        if actual_top != new_top:
            raise AssertionError(
                f"0x6462 {name}: top={actual_top:#x}, expected={new_top:#x}"
            )
        actual_query = machine.mem_read(game_segment * 16 + 0x67AD, 1)[0]
        if actual_query != 0:
            raise AssertionError(f"0x6462 {name}: query mode was not cleared")
        if machine.mem_read(stack_segment * 16 + target_offset, 2) != target_bytes:
            raise AssertionError(f"0x6462 {name}: branch target changed")
        if (
            machine.mem_read(game_segment * 16 + target_offset, 2)
            != game_target_bytes
        ):
            raise AssertionError(f"0x6462 {name}: GS stack decoy changed")
        if (
            machine.mem_read(data_segment * 16 + target_offset, 2)
            != data_target_bytes
        ):
            raise AssertionError(f"0x6462 {name}: DS stack decoy changed")
        actual_stack_top = struct.unpack(
            "<H", machine.mem_read(stack_segment * 16 + 0x6884, 2)
        )[0]
        if actual_stack_top != stack_top_decoy:
            raise AssertionError(f"0x6462 {name}: SS top decoy changed")
        actual_stack_query = machine.mem_read(stack_segment * 16 + 0x67AD, 1)[0]
        if actual_stack_query != stack_query_decoy:
            raise AssertionError(f"0x6462 {name}: SS query decoy changed")

        flags = machine.reg_read(UC_X86_REG_EFLAGS)
        expected_flags = {
            "cf": top < 2,
            "pf": (new_top & 0xFF).bit_count() % 2 == 0,
            "af": (top & 0x0F) < 2,
            "zf": new_top == 0,
            "sf": bool(new_top & 0x8000),
            "of": bool(((top ^ 2) & (top ^ new_top)) & 0x8000),
        }
        actual_flags = {
            "cf": bool(flags & 0x0001),
            "pf": bool(flags & 0x0004),
            "af": bool(flags & 0x0010),
            "zf": bool(flags & 0x0040),
            "sf": bool(flags & 0x0080),
            "of": bool(flags & 0x0800),
        }
        if actual_flags != expected_flags:
            raise AssertionError(
                f"0x6462 {name}: flags={actual_flags}, expected={expected_flags}"
            )
        if EXE[0x647A] != 0xC3:
            raise AssertionError("0x6462: expected near RET boundary")

        vectors.append(
            {
                "name": name,
                "initial_top_byte_offset": top,
                "final_top_byte_offset": new_top,
                "target_effective_offset": target_offset,
                "target": target,
                "query_mode_before": query_mode,
                "defined_flags": expected_flags,
            }
        )

    return vectors


def scan_zero_word_vectors() -> list[dict[str, object]]:
    data_segment = 0x4400
    game_segment = 0x2C00
    stack_segment = 0x9000
    cases = [
        ("immediate_zero", 0x1000, [0x0000], 0, 0x0000),
        ("immediate_minus_one", 0x1100, [0xFFFF], 0, 0xFFFF),
        ("immediate_signed_minimum", 0x1200, [0x8000], 0, 0x8000),
        ("one_positive_then_zero", 0x1300, [0x0001, 0x0000], 1, 0x0000),
        (
            "three_positive_then_negative",
            0x1400,
            [0x0001, 0x7FFF, 0x0002, 0x8000],
            3,
            0x8000,
        ),
        ("count_sets_auxiliary_flag", 0x1500, [1] * 15 + [0], 15, 0),
        ("cursor_wraps", 0xFFFC, [0x0001, 0x7FFF, 0x0000], 2, 0x0000),
        ("unaligned_cursor", 0xFFFD, [0x0001, 0xFFFF], 1, 0xFFFF),
        ("count_sets_overflow_flag", 0x0000, None, 0x7FFF, 0x0000),
        ("loop_counter_exhausts", 0x0000, None, 0xFFFF, 0x0001),
    ]
    vectors = []

    for name, start, words, expected_count, expected_ax in cases:
        memory = [
            (game_segment, 0x27CF, struct.pack("<H", 0xA55A)),
            (stack_segment, 0x27CF, struct.pack("<H", 0x5AA5)),
        ]
        immutable = []
        instruction_count = 20000
        if words is None:
            if expected_count == 0xFFFF:
                input_bytes = b"\x01\x00" * 0x8000
                instruction_count = 350000
            else:
                input_bytes = (
                    b"\x01\x00" * expected_count + struct.pack("<H", expected_ax)
                )
                instruction_count = 180000
            memory.append((data_segment, 0, input_bytes))
            immutable.append((data_segment, 0, input_bytes))
        else:
            for index, word in enumerate(words):
                offset = (start + index * 2) & 0xFFFF
                encoded = struct.pack("<H", word)
                memory.append((data_segment, offset, encoded))
                immutable.append((data_segment, offset, encoded))

            data_count_decoy = struct.pack("<H", 0xC33C)
            memory.append((data_segment, 0x27CF, data_count_decoy))
            immutable.append((data_segment, 0x27CF, data_count_decoy))

        initial = {
            "eax": 0xA1A1BEEF,
            "ebx": 0xB2B22345,
            "ecx": 0xC3C33456,
            "edx": 0xD4D44567,
            "esi": 0xE5E50000 | start,
            "edi": 0xF6F66789,
            "ebp": 0x9797789A,
            "sp": 0xFF00,
            "ds": data_segment,
            "es": 0x4800,
            "fs": 0x4C00,
            "gs": game_segment,
            "ss": stack_segment,
            "flags": 0x0AD7,
        }

        machine = execute(
            0x647B,
            0x6493,
            initial,
            memory,
            instruction_count=instruction_count,
        )

        expected_registers = dict(initial)
        del expected_registers["flags"]
        expected_registers["eax"] = (
            initial["eax"] & 0xFFFF0000
        ) | expected_ax
        for register, expected in expected_registers.items():
            actual = machine.reg_read(REGISTERS[register])
            if actual != expected:
                raise AssertionError(
                    f"0x647b {name}: {register}={actual:#x}, expected={expected:#x}"
                )

        actual_count = struct.unpack(
            "<H", machine.mem_read(game_segment * 16 + 0x27CF, 2)
        )[0]
        if actual_count != expected_count:
            raise AssertionError(
                f"0x647b {name}: count={actual_count:#x}, "
                f"expected={expected_count:#x}"
            )
        stack_decoy = struct.unpack(
            "<H", machine.mem_read(stack_segment * 16 + 0x27CF, 2)
        )[0]
        if stack_decoy != 0x5AA5:
            raise AssertionError(f"0x647b {name}: SS count decoy changed")
        for segment, offset, expected in immutable:
            actual = bytes(machine.mem_read(segment * 16 + offset, len(expected)))
            if actual != expected:
                raise AssertionError(f"0x647b {name}: input memory changed")

        flags = machine.reg_read(UC_X86_REG_EFLAGS)
        expected_flags = {
            "cf": expected_count != 0xFFFF,
            "pf": (expected_count & 0xFF).bit_count() % 2 == 0,
            "af": (expected_count & 0x0F) == 0x0F,
            "zf": expected_count == 0,
            "sf": bool(expected_count & 0x8000),
            "of": expected_count == 0x7FFF,
        }
        actual_flags = {
            "cf": bool(flags & 0x0001),
            "pf": bool(flags & 0x0004),
            "af": bool(flags & 0x0010),
            "zf": bool(flags & 0x0040),
            "sf": bool(flags & 0x0080),
            "of": bool(flags & 0x0800),
        }
        if actual_flags != expected_flags:
            raise AssertionError(
                f"0x647b {name}: flags={actual_flags}, expected={expected_flags}"
            )
        if EXE[0x6493] != 0xC3:
            raise AssertionError("0x647b: expected near RET boundary")

        vectors.append(
            {
                "name": name,
                "start_offset": start,
                "count": expected_count,
                "final_ax": expected_ax,
                "defined_flags": expected_flags,
            }
        )

    return vectors


def vm_conditional_branch_vectors(
    entry: int, return_address: int, flag_offset: int
) -> list[dict[str, object]]:
    game_segment = 0x2C00
    data_segment = 0x4400
    stack_segment = 0x9000
    cases = [
        ("zero_branches", 0x00),
        ("unrelated_bit_branches", 0xFE),
        ("bit_zero_set_continues", 0x01),
        ("all_bits_set_continues", 0xFF),
    ]
    vectors = []

    for index, (name, flag_value) in enumerate(cases):
        branch_taken = (flag_value & 1) == 0
        top = 4
        new_top = 2
        target = 0x7100 + index * 0x111
        target_offset = 0x6820 + new_top
        query_mode = 0xA5
        target_bytes = struct.pack("<H", target)
        game_target_decoy = struct.pack("<H", target ^ 0xFFFF)
        data_target_decoy = struct.pack("<H", target ^ 0x5A5A)
        memory = [
            (game_segment, flag_offset, bytes([flag_value])),
            (data_segment, flag_offset, b"\x5a"),
            (stack_segment, flag_offset, b"\xa5"),
            (game_segment, 0x6884, struct.pack("<H", top)),
            (game_segment, 0x67AD, bytes([query_mode])),
            (stack_segment, target_offset, target_bytes),
            (game_segment, target_offset, game_target_decoy),
            (data_segment, target_offset, data_target_decoy),
        ]
        initial = {
            "eax": 0xA1A1BEEF,
            "ebx": 0xB2B22345,
            "ecx": 0xC3C33456,
            "edx": 0xD4D44567,
            "esi": 0xE5E55678,
            "edi": 0xF6F66789,
            "ebp": 0x9797789A,
            "sp": 0xFF00,
            "ds": data_segment,
            "es": 0x4800,
            "fs": 0x4C00,
            "gs": game_segment,
            "ss": stack_segment,
            "flags": 0x0AD7,
        }
        branch_calls = []

        def capture_branch(_machine: Uc, address: int, _size: int) -> None:
            if address == 0x6462:
                branch_calls.append(address)

        machine = execute(
            entry,
            return_address,
            initial,
            memory,
            code_handler=capture_branch,
        )

        expected_calls = [0x6462] if branch_taken else []
        if branch_calls != expected_calls:
            raise AssertionError(
                f"{entry:#x} {name}: branch calls={branch_calls}, "
                f"expected={expected_calls}"
            )
        expected_registers = dict(initial)
        del expected_registers["flags"]
        if branch_taken:
            expected_registers["eax"] = (
                initial["eax"] & 0xFFFF0000
            ) | new_top
            expected_registers["esi"] = (
                initial["esi"] & 0xFFFF0000
            ) | target
        for register, expected in expected_registers.items():
            actual = machine.reg_read(REGISTERS[register])
            if actual != expected:
                raise AssertionError(
                    f"{entry:#x} {name}: {register}={actual:#x}, "
                    f"expected={expected:#x}"
                )

        actual_top = struct.unpack(
            "<H", machine.mem_read(game_segment * 16 + 0x6884, 2)
        )[0]
        expected_top = new_top if branch_taken else top
        if actual_top != expected_top:
            raise AssertionError(
                f"{entry:#x} {name}: top={actual_top:#x}, "
                f"expected={expected_top:#x}"
            )
        actual_query = machine.mem_read(game_segment * 16 + 0x67AD, 1)[0]
        expected_query = 0 if branch_taken else query_mode
        if actual_query != expected_query:
            raise AssertionError(
                f"{entry:#x} {name}: query={actual_query:#x}, "
                f"expected={expected_query:#x}"
            )
        if machine.mem_read(stack_segment * 16 + target_offset, 2) != target_bytes:
            raise AssertionError(f"{entry:#x} {name}: branch target changed")
        if machine.mem_read(game_segment * 16 + flag_offset, 1)[0] != flag_value:
            raise AssertionError(f"{entry:#x} {name}: tested flag changed")
        if machine.mem_read(data_segment * 16 + flag_offset, 1) != b"\x5a":
            raise AssertionError(f"{entry:#x} {name}: DS flag decoy changed")
        if machine.mem_read(stack_segment * 16 + flag_offset, 1) != b"\xa5":
            raise AssertionError(f"{entry:#x} {name}: SS flag decoy changed")
        if (
            machine.mem_read(game_segment * 16 + target_offset, 2)
            != game_target_decoy
        ):
            raise AssertionError(f"{entry:#x} {name}: GS target decoy changed")
        if (
            machine.mem_read(data_segment * 16 + target_offset, 2)
            != data_target_decoy
        ):
            raise AssertionError(f"{entry:#x} {name}: DS target decoy changed")

        flags = machine.reg_read(UC_X86_REG_EFLAGS)
        if branch_taken:
            result = new_top
            expected_flags = {
                "cf": False,
                "pf": (result & 0xFF).bit_count() % 2 == 0,
                "af": False,
                "zf": result == 0,
                "sf": bool(result & 0x8000),
                "of": False,
            }
            actual_flags = {
                "cf": bool(flags & 0x0001),
                "pf": bool(flags & 0x0004),
                "af": bool(flags & 0x0010),
                "zf": bool(flags & 0x0040),
                "sf": bool(flags & 0x0080),
                "of": bool(flags & 0x0800),
            }
        else:
            result = flag_value & 1
            expected_flags = {
                "cf": False,
                "pf": result.bit_count() % 2 == 0,
                "zf": result == 0,
                "sf": False,
                "of": False,
            }
            actual_flags = {
                "cf": bool(flags & 0x0001),
                "pf": bool(flags & 0x0004),
                "zf": bool(flags & 0x0040),
                "sf": bool(flags & 0x0080),
                "of": bool(flags & 0x0800),
            }
        if actual_flags != expected_flags:
            raise AssertionError(
                f"{entry:#x} {name}: flags={actual_flags}, "
                f"expected={expected_flags}"
            )
        if EXE[return_address] != 0xC3:
            raise AssertionError(f"{entry:#x}: expected near RET boundary")

        vectors.append(
            {
                "name": name,
                "flag_offset": flag_offset,
                "flag_value": flag_value,
                "branch_taken": branch_taken,
                "final_stack_top": expected_top,
                "final_script_offset": target if branch_taken else initial["esi"] & 0xFFFF,
                "defined_flags": expected_flags,
            }
        )

    return vectors


def vm_script_profile_request_vectors() -> list[dict[str, object]]:
    data_segment = 0x4400
    game_segment = 0x2C00
    stack_segment = 0x9000
    cases = [
        ("zero_becomes_minus_one", 0x1000, 0x00, 0x0AD6),
        ("one_becomes_zero", 0x1100, 0x01, 0x0AD7),
        ("signed_maximum", 0x1200, 0x7F, 0x0246),
        ("signed_minimum", 0x1300, 0x80, 0x0247),
        ("minus_one_becomes_minus_two", 0x1400, 0xFF, 0x08D6),
        ("cursor_wraps", 0xFFFF, 0x42, 0x08D7),
    ]
    vectors = []

    for name, start, operand, initial_flags in cases:
        signed_operand = operand if operand < 0x80 else operand - 0x100
        before_decrement = signed_operand & 0xFFFF
        result = (before_decrement - 1) & 0xFFFF
        memory = [
            (data_segment, start, bytes([operand])),
            (0x4800, start, b"\x5a"),
            (game_segment, start, b"\xa5"),
            (game_segment, 0x6780, struct.pack("<H", 0xBEEF)),
            (data_segment, 0x6780, struct.pack("<H", 0xA55A)),
            (stack_segment, 0x6780, struct.pack("<H", 0x5AA5)),
        ]
        initial = {
            "eax": 0xA1A1BEEF,
            "ebx": 0xB2B22345,
            "ecx": 0xC3C33456,
            "edx": 0xD4D44567,
            "esi": 0xE5E50000 | start,
            "edi": 0xF6F66789,
            "ebp": 0x9797789A,
            "sp": 0xFF00,
            "ds": data_segment,
            "es": 0x4800,
            "fs": 0x4C00,
            "gs": game_segment,
            "ss": stack_segment,
            "flags": initial_flags,
        }

        machine = execute(0x64B8, 0x64BF, initial, memory)

        expected_registers = dict(initial)
        del expected_registers["flags"]
        expected_registers["eax"] = (initial["eax"] & 0xFFFF0000) | result
        expected_registers["esi"] = (
            initial["esi"] & 0xFFFF0000
        ) | ((start + 1) & 0xFFFF)
        for register, expected in expected_registers.items():
            actual = machine.reg_read(REGISTERS[register])
            if actual != expected:
                raise AssertionError(
                    f"0x64b8 {name}: {register}={actual:#x}, expected={expected:#x}"
                )

        actual_request = struct.unpack(
            "<H", machine.mem_read(game_segment * 16 + 0x6780, 2)
        )[0]
        if actual_request != result:
            raise AssertionError(
                f"0x64b8 {name}: request={actual_request:#x}, expected={result:#x}"
            )
        if machine.mem_read(data_segment * 16 + start, 1)[0] != operand:
            raise AssertionError(f"0x64b8 {name}: script byte changed")
        if machine.mem_read(0x4800 * 16 + start, 1) != b"\x5a":
            raise AssertionError(f"0x64b8 {name}: ES script decoy changed")
        if machine.mem_read(game_segment * 16 + start, 1) != b"\xa5":
            raise AssertionError(f"0x64b8 {name}: GS script decoy changed")
        data_decoy = struct.unpack(
            "<H", machine.mem_read(data_segment * 16 + 0x6780, 2)
        )[0]
        stack_decoy = struct.unpack(
            "<H", machine.mem_read(stack_segment * 16 + 0x6780, 2)
        )[0]
        if data_decoy != 0xA55A or stack_decoy != 0x5AA5:
            raise AssertionError(f"0x64b8 {name}: output decoy changed")

        flags = machine.reg_read(UC_X86_REG_EFLAGS)
        expected_flags = {
            "cf": bool(initial_flags & 0x0001),
            "pf": (result & 0xFF).bit_count() % 2 == 0,
            "af": (before_decrement & 0x0F) == 0,
            "zf": result == 0,
            "sf": bool(result & 0x8000),
            "of": before_decrement == 0x8000,
        }
        actual_flags = {
            "cf": bool(flags & 0x0001),
            "pf": bool(flags & 0x0004),
            "af": bool(flags & 0x0010),
            "zf": bool(flags & 0x0040),
            "sf": bool(flags & 0x0080),
            "of": bool(flags & 0x0800),
        }
        if actual_flags != expected_flags:
            raise AssertionError(
                f"0x64b8 {name}: flags={actual_flags}, expected={expected_flags}"
            )
        if EXE[0x64BF] != 0xC3:
            raise AssertionError("0x64b8: expected near RET boundary")

        vectors.append(
            {
                "name": name,
                "start_offset": start,
                "operand_byte": operand,
                "stored_request": result,
                "final_script_offset": (start + 1) & 0xFFFF,
                "defined_flags": expected_flags,
            }
        )

    return vectors


def vm_clear_state_vectors() -> list[dict[str, object]]:
    game_segment = 0x2C00
    data_segment = 0x4400
    stack_segment = 0x9000
    cases = [
        ("already_clear", 0x00, 0x0000),
        ("all_bits_set", 0xFF, 0xFFFF),
        ("mixed_values", 0x80, 0x1234),
        ("low_bits", 0x01, 0x0001),
    ]
    vectors = []

    for name, resume_state, resume_value in cases:
        memory = [
            (game_segment, 0x67B1, bytes([resume_state])),
            (game_segment, 0x6764, struct.pack("<H", resume_value)),
            (data_segment, 0x67B1, b"\x5a"),
            (data_segment, 0x6764, struct.pack("<H", 0xA55A)),
            (stack_segment, 0x67B1, b"\xa5"),
            (stack_segment, 0x6764, struct.pack("<H", 0x5AA5)),
        ]
        initial = {
            "eax": 0xA1A1BEEF,
            "ebx": 0xB2B22345,
            "ecx": 0xC3C33456,
            "edx": 0xD4D44567,
            "esi": 0xE5E55678,
            "edi": 0xF6F66789,
            "ebp": 0x9797789A,
            "sp": 0xFF00,
            "ds": data_segment,
            "es": 0x4800,
            "fs": 0x4C00,
            "gs": game_segment,
            "ss": stack_segment,
            "flags": 0x0AD7,
        }
        before_second_store = []

        def capture_store_order(machine: Uc, address: int, _size: int) -> None:
            if address == 0x64C6:
                before_second_store.append(
                    (
                        machine.mem_read(game_segment * 16 + 0x67B1, 1)[0],
                        struct.unpack(
                            "<H", machine.mem_read(game_segment * 16 + 0x6764, 2)
                        )[0],
                    )
                )

        machine = execute(
            0x64C0,
            0x64CD,
            initial,
            memory,
            code_handler=capture_store_order,
        )

        if before_second_store != [(0, resume_value)]:
            raise AssertionError(
                f"0x64c0 {name}: store order={before_second_store}, "
                f"expected={[(0, resume_value)]}"
            )
        for register, expected in initial.items():
            if register == "flags":
                continue
            actual = machine.reg_read(REGISTERS[register])
            if actual != expected:
                raise AssertionError(f"0x64c0 {name}: changed {register}")
        if machine.reg_read(UC_X86_REG_EFLAGS) != initial["flags"]:
            raise AssertionError(f"0x64c0 {name}: changed flags")
        if machine.mem_read(game_segment * 16 + 0x67B1, 1)[0] != 0:
            raise AssertionError(f"0x64c0 {name}: resume state not cleared")
        if struct.unpack(
            "<H", machine.mem_read(game_segment * 16 + 0x6764, 2)
        )[0] != 0:
            raise AssertionError(f"0x64c0 {name}: resume value not cleared")
        if machine.mem_read(data_segment * 16 + 0x67B1, 1) != b"\x5a":
            raise AssertionError(f"0x64c0 {name}: DS state decoy changed")
        if struct.unpack(
            "<H", machine.mem_read(data_segment * 16 + 0x6764, 2)
        )[0] != 0xA55A:
            raise AssertionError(f"0x64c0 {name}: DS value decoy changed")
        if machine.mem_read(stack_segment * 16 + 0x67B1, 1) != b"\xa5":
            raise AssertionError(f"0x64c0 {name}: SS state decoy changed")
        if struct.unpack(
            "<H", machine.mem_read(stack_segment * 16 + 0x6764, 2)
        )[0] != 0x5AA5:
            raise AssertionError(f"0x64c0 {name}: SS value decoy changed")
        if EXE[0x64CD] != 0xC3:
            raise AssertionError("0x64c0: expected near RET boundary")

        vectors.append(
            {
                "name": name,
                "resume_state_before": resume_state,
                "resume_value_before": resume_value,
                "resume_state_after": 0,
                "resume_value_after": 0,
                "flags_preserved": True,
            }
        )

    return vectors


def vm_record_string_copy_vectors() -> list[dict[str, object]]:
    data_segment = 0x4400
    game_segment = 0x2C00
    stack_segment = 0x9000
    cases = [
        ("first_slot_empty", 0x1000, 0x01, b"\x00", 0xA5),
        ("second_slot_one_character", 0x1100, 0x02, b"A\x00", 0x5A),
        ("slot_byte_zero_selects_minus_one", 0x1200, 0x00, b"NEG\x00", 0xCC),
        ("decrement_precedes_sign_extension", 0x1300, 0x80, b"MAX\x00", 0x33),
        ("signed_minimum_slot", 0x1400, 0x81, b"MIN\x00", 0x77),
        ("raw_high_bytes", 0x1500, 0xFF, b"\x80\xfe\x00", 0x42),
        ("source_cursor_wraps", 0xFFFD, 0x01, b"X\x00", 0x55),
        ("copy_is_not_slot_bounded", 0x1600, 0x01, b"ABCDEFGHIJKLMNOPQ\x00", 0x99),
        ("source_cursor_signed_overflow", 0x7FFD, 0x01, b"\x00", 0x11),
    ]
    vectors = []

    for name, start, slot_byte, string_bytes, pad_byte in cases:
        decremented = (slot_byte - 1) & 0xFF
        signed_slot = decremented if decremented < 0x80 else decremented - 0x100
        slot_shift = (signed_slot * 16) & 0xFFFF
        destination_start = (0x6CDE + slot_shift) & 0xFFFF
        script = bytes([slot_byte]) + string_bytes + bytes([pad_byte])
        final_source = (start + len(script)) & 0xFFFF
        final_destination = (destination_start + len(string_bytes)) & 0xFFFF
        memory = []
        immutable = []

        for index, value in enumerate(script):
            offset = (start + index) & 0xFFFF
            encoded = bytes([value])
            memory.append((data_segment, offset, encoded))
            immutable.append((data_segment, offset, encoded))
            memory.append((game_segment, offset, b"\xa5"))

        output_offsets = [
            (destination_start - 1) & 0xFFFF,
            *[
                (destination_start + index) & 0xFFFF
                for index in range(len(string_bytes) + 1)
            ],
        ]
        stack_before = {}
        game_decoys = {}
        data_decoys = {}
        for index, offset in enumerate(output_offsets):
            stack_value = (0x30 + index) & 0xFF
            game_value = (0x80 + index) & 0xFF
            data_value = (0xC0 + index) & 0xFF
            stack_before[offset] = stack_value
            game_decoys[offset] = game_value
            data_decoys[offset] = data_value
            memory.append((stack_segment, offset, bytes([stack_value])))
            memory.append((game_segment, offset, bytes([game_value])))
            memory.append((data_segment, offset, bytes([data_value])))

        initial = {
            "eax": 0xA1A1BEEF,
            "ebx": 0xB2B22345,
            "ecx": 0xC3C33456,
            "edx": 0xD4D44567,
            "esi": 0xE5E50000 | start,
            "edi": 0xF6F66789,
            "ebp": 0x9797789A,
            "sp": 0xFF00,
            "ds": data_segment,
            "es": 0x4800,
            "fs": 0x4C00,
            "gs": game_segment,
            "ss": stack_segment,
            "flags": 0x0AD7,
        }

        machine = execute(0x64CE, 0x64E4, initial, memory)

        expected_registers = dict(initial)
        del expected_registers["flags"]
        expected_registers["eax"] = (
            initial["eax"] & 0xFFFF0000
        ) | (slot_shift & 0xFF00)
        expected_registers["esi"] = (
            initial["esi"] & 0xFFFF0000
        ) | final_source
        expected_registers["ebp"] = (
            initial["ebp"] & 0xFFFF0000
        ) | final_destination
        for register, expected in expected_registers.items():
            actual = machine.reg_read(REGISTERS[register])
            if actual != expected:
                raise AssertionError(
                    f"0x64ce {name}: {register}={actual:#x}, expected={expected:#x}"
                )

        actual_output = bytes(
            machine.mem_read(stack_segment * 16 + destination_start, len(string_bytes))
        )
        if actual_output != string_bytes:
            raise AssertionError(
                f"0x64ce {name}: output={actual_output!r}, expected={string_bytes!r}"
            )
        before_offset = (destination_start - 1) & 0xFFFF
        after_offset = (destination_start + len(string_bytes)) & 0xFFFF
        if (
            machine.mem_read(stack_segment * 16 + before_offset, 1)[0]
            != stack_before[before_offset]
        ):
            raise AssertionError(f"0x64ce {name}: byte before output changed")
        if (
            machine.mem_read(stack_segment * 16 + after_offset, 1)[0]
            != stack_before[after_offset]
        ):
            raise AssertionError(f"0x64ce {name}: pad leaked into output")
        for offset in output_offsets:
            if (
                machine.mem_read(game_segment * 16 + offset, 1)[0]
                != game_decoys[offset]
            ):
                raise AssertionError(f"0x64ce {name}: GS output decoy changed")
            if (
                machine.mem_read(data_segment * 16 + offset, 1)[0]
                != data_decoys[offset]
            ):
                raise AssertionError(f"0x64ce {name}: DS output decoy changed")
        for segment, offset, expected in immutable:
            actual = bytes(machine.mem_read(segment * 16 + offset, len(expected)))
            if actual != expected:
                raise AssertionError(f"0x64ce {name}: script input changed")

        before_final_increment = (final_source - 1) & 0xFFFF
        flags = machine.reg_read(UC_X86_REG_EFLAGS)
        expected_flags = {
            "cf": False,
            "pf": (final_source & 0xFF).bit_count() % 2 == 0,
            "af": (before_final_increment & 0x0F) == 0x0F,
            "zf": final_source == 0,
            "sf": bool(final_source & 0x8000),
            "of": before_final_increment == 0x7FFF,
        }
        actual_flags = {
            "cf": bool(flags & 0x0001),
            "pf": bool(flags & 0x0004),
            "af": bool(flags & 0x0010),
            "zf": bool(flags & 0x0040),
            "sf": bool(flags & 0x0080),
            "of": bool(flags & 0x0800),
        }
        if actual_flags != expected_flags:
            raise AssertionError(
                f"0x64ce {name}: flags={actual_flags}, expected={expected_flags}"
            )
        if EXE[0x64E4] != 0xC3:
            raise AssertionError("0x64ce: expected near RET boundary")

        vectors.append(
            {
                "name": name,
                "start_offset": start,
                "slot_byte": slot_byte,
                "signed_slot_after_decrement": signed_slot,
                "destination_start": destination_start,
                "copied_byte_count": len(string_bytes),
                "final_source_offset": final_source,
                "final_destination_offset": final_destination,
                "defined_flags": expected_flags,
            }
        )

    return vectors


def vm_tagged_word_compare_vectors() -> list[dict[str, object]]:
    data_segment = 0x4400
    game_segment = 0x2C00
    stack_segment = 0x9000
    cases = [
        ("greater_tag_passes", 0x1000, 0x00F1, 1, 0, True),
        ("greater_tag_equal_branches", 0x1100, 0x00F1, 7, 7, False),
        ("greater_tag_less_branches", 0x1200, 0x00F1, -2, -1, False),
        ("greater_tag_signed_overflow_passes", 0x1300, 0x00F1, 0x7FFF, -1, True),
        ("greater_tag_signed_extremes_branch", 0x1400, 0x00F1, -0x8000, 0x7FFF, False),
        ("less_tag_passes", 0x1500, 0x00F2, -1, 0, True),
        ("less_tag_equal_branches", 0x1600, 0x00F2, -7, -7, False),
        ("less_tag_greater_branches", 0x1700, 0x00F2, 2, 1, False),
        ("less_tag_signed_overflow_passes", 0x1800, 0x00F2, -0x8000, 1, True),
        ("default_tag_equal_passes", 0x1900, 0x0000, -123, -123, True),
        ("default_tag_mismatch_branches", 0x1A00, 0x00F3, 0x1234, 0x1235, False),
        ("only_low_tag_byte_is_used", 0x1B00, 0x12F1, 9, 8, True),
        ("unaligned_cursor", 0x1C01, 0x34F2, -2, -1, True),
        ("second_word_crosses_segment_end", 0xFFFD, 0x00F3, 0x4567, 0x4567, True),
    ]
    vectors = []

    for index, (name, start, tag_word, value, compare, pass_result) in enumerate(cases):
        value_word = value & 0xFFFF
        compare_word = compare & 0xFFFF
        tag = tag_word & 0xFF
        branch_target = (0x7100 + index * 0x101) & 0xFFFF
        script = struct.pack("<HH", tag_word, value_word)
        memory = [
            (game_segment, 0x0AA6, struct.pack("<H", compare_word)),
            (data_segment, 0x0AA6, struct.pack("<H", compare_word ^ 0x8000)),
            (stack_segment, 0x0AA6, struct.pack("<H", compare_word ^ 0x5A5A)),
            (game_segment, 0x6884, struct.pack("<H", 4)),
            (game_segment, 0x67AD, b"\xa5"),
            (stack_segment, 0x6822, struct.pack("<H", branch_target)),
            (game_segment, 0x6822, struct.pack("<H", branch_target ^ 0xFFFF)),
        ]
        immutable = []
        for byte_index, byte in enumerate(script):
            # A word at DS:FFFF reads its high byte at the next linear address;
            # only the post-LODSW SI register wraps to offset 0001.
            offset = start + byte_index
            encoded = bytes([byte])
            memory.append((data_segment, offset, encoded))
            immutable.append((data_segment, offset, encoded))
            memory.append((0x4800, offset, b"\x5a"))
            memory.append((game_segment, offset, b"\xa5"))

        initial = {
            "eax": 0xA1A1BEEF,
            "ebx": 0xB2B22345,
            "ecx": 0xC3C33456,
            "edx": 0xD4D44567,
            "esi": 0xE5E50000 | start,
            "edi": 0xF6F66789,
            "ebp": 0x9797789A,
            "sp": 0xFF00,
            "ds": data_segment,
            "es": 0x4800,
            "fs": 0x4C00,
            "gs": game_segment,
            "ss": stack_segment,
            "flags": 0x0AD7,
        }
        branch_calls = []

        def capture_branch(_machine: Uc, address: int, _size: int) -> None:
            if address == 0x6462:
                branch_calls.append(address)

        machine = execute(
            0x64E5,
            0x650F,
            initial,
            memory,
            code_handler=capture_branch,
        )

        expected_calls = [] if pass_result else [0x6462]
        if branch_calls != expected_calls:
            raise AssertionError(
                f"0x64e5 {name}: branch calls={branch_calls}, "
                f"expected={expected_calls}"
            )
        expected_registers = dict(initial)
        del expected_registers["flags"]
        expected_registers["edx"] = (initial["edx"] & 0xFFFFFF00) | tag
        if pass_result:
            expected_registers["eax"] = (
                initial["eax"] & 0xFFFF0000
            ) | value_word
            expected_registers["esi"] = (
                initial["esi"] & 0xFFFF0000
            ) | ((start + 4) & 0xFFFF)
        else:
            expected_registers["eax"] = (initial["eax"] & 0xFFFF0000) | 2
            expected_registers["esi"] = (
                initial["esi"] & 0xFFFF0000
            ) | branch_target
        for register, expected in expected_registers.items():
            actual = machine.reg_read(REGISTERS[register])
            if actual != expected:
                raise AssertionError(
                    f"0x64e5 {name}: {register}={actual:#x}, expected={expected:#x}"
                )

        actual_top = struct.unpack(
            "<H", machine.mem_read(game_segment * 16 + 0x6884, 2)
        )[0]
        expected_top = 4 if pass_result else 2
        if actual_top != expected_top:
            raise AssertionError(
                f"0x64e5 {name}: top={actual_top:#x}, expected={expected_top:#x}"
            )
        actual_query = machine.mem_read(game_segment * 16 + 0x67AD, 1)[0]
        expected_query = 0xA5 if pass_result else 0
        if actual_query != expected_query:
            raise AssertionError(
                f"0x64e5 {name}: query={actual_query:#x}, expected={expected_query:#x}"
            )
        for segment, offset, expected in immutable:
            actual = bytes(machine.mem_read(segment * 16 + offset, len(expected)))
            if actual != expected:
                raise AssertionError(f"0x64e5 {name}: script input changed")
        for byte_index in range(len(script)):
            offset = start + byte_index
            if machine.mem_read(0x4800 * 16 + offset, 1) != b"\x5a":
                raise AssertionError(f"0x64e5 {name}: ES script decoy changed")
            if machine.mem_read(game_segment * 16 + offset, 1) != b"\xa5":
                raise AssertionError(f"0x64e5 {name}: GS script decoy changed")

        flags = machine.reg_read(UC_X86_REG_EFLAGS)
        if pass_result:
            result = (value_word - compare_word) & 0xFFFF
            expected_flags = {
                "cf": value_word < compare_word,
                "pf": (result & 0xFF).bit_count() % 2 == 0,
                "af": (value_word & 0x0F) < (compare_word & 0x0F),
                "zf": result == 0,
                "sf": bool(result & 0x8000),
                "of": bool(
                    ((value_word ^ compare_word) & (value_word ^ result)) & 0x8000
                ),
            }
        else:
            expected_flags = {
                "cf": False,
                "pf": False,
                "af": False,
                "zf": False,
                "sf": False,
                "of": False,
            }
        actual_flags = {
            "cf": bool(flags & 0x0001),
            "pf": bool(flags & 0x0004),
            "af": bool(flags & 0x0010),
            "zf": bool(flags & 0x0040),
            "sf": bool(flags & 0x0080),
            "of": bool(flags & 0x0800),
        }
        if actual_flags != expected_flags:
            raise AssertionError(
                f"0x64e5 {name}: flags={actual_flags}, expected={expected_flags}"
            )
        if EXE[0x650F] != 0xC3:
            raise AssertionError("0x64e5: expected near RET boundary")

        vectors.append(
            {
                "name": name,
                "start_offset": start,
                "tag_word": tag_word,
                "effective_tag": tag,
                "value": value,
                "compare": compare,
                "comparison_passed": pass_result,
                "branch_taken": not pass_result,
                "final_script_offset": (
                    (start + 4) & 0xFFFF if pass_result else branch_target
                ),
                "defined_flags": expected_flags,
            }
        )

    return vectors


def vm_tagged_byte_pair_compare_vectors() -> list[dict[str, object]]:
    data_segment = 0x4400
    game_segment = 0x2C00
    stack_segment = 0x9000
    cases = [
        ("f1_high_greater_passes", 0x1000, 0xF1, -100, 1, 100, 0, True, "high"),
        ("f1_high_less_branches", 0x1100, 0xF1, 100, -2, -100, -1, False, "high"),
        ("f1_equal_high_low_greater_passes", 0x1200, 0xF1, 6, -7, 5, -7, True, "low"),
        ("f1_equal_pair_branches", 0x1300, 0xF1, 5, -7, 5, -7, False, "low"),
        ("f1_equal_high_low_less_branches", 0x1400, 0xF1, 4, -7, 5, -7, False, "low"),
        ("f1_signed_high_overflow_passes", 0x1500, 0xF1, 0, 127, 0, -1, True, "high"),
        ("f1_signed_low_overflow_passes", 0x1600, 0xF1, 127, -1, -1, -1, True, "low"),
        ("f2_high_less_passes", 0x1700, 0xF2, 100, -2, -100, -1, True, "high"),
        ("f2_high_greater_branches", 0x1800, 0xF2, -100, 1, 100, 0, False, "high"),
        ("f2_equal_high_low_less_passes", 0x1900, 0xF2, 4, 7, 5, 7, True, "low"),
        ("f2_equal_pair_branches", 0x1A00, 0xF2, 5, 7, 5, 7, False, "low"),
        ("f2_equal_high_low_greater_branches", 0x1B00, 0xF2, 6, 7, 5, 7, False, "low"),
        ("f2_signed_high_overflow_passes", 0x1C00, 0xF2, 0, -128, 0, 1, True, "high"),
        ("default_equal_pair_passes", 0x1D00, 0x00, -123, 45, -123, 45, True, "low"),
        ("default_high_mismatch_branches", 0x1E00, 0xF3, 9, 10, 9, 11, False, "high"),
        ("default_low_mismatch_branches", 0x1F00, 0xF3, 8, 10, 9, 10, False, "low"),
        ("unaligned_cursor", 0x2001, 0xF1, -2, -3, -3, -3, True, "low"),
        ("padding_word_crosses_segment_end", 0xFFFC, 0x7E, -5, 6, -5, 6, True, "low"),
    ]
    vectors = []

    for index, case in enumerate(cases):
        (
            name,
            start,
            tag,
            pair_low,
            pair_high,
            compare_low,
            compare_high,
            pass_result,
            decision_byte,
        ) = case
        pair_word = ((pair_high & 0xFF) << 8) | (pair_low & 0xFF)
        padding_word = (0xA500 + index * 0x31) & 0xFFFF
        branch_target = (0x7100 + index * 0x101) & 0xFFFF
        script = bytes([tag, pair_low & 0xFF, pair_high & 0xFF]) + struct.pack(
            "<H", padding_word
        )
        memory = [
            (game_segment, 0x0AA8, bytes([compare_low & 0xFF])),
            (game_segment, 0x0AAA, bytes([compare_high & 0xFF])),
            (data_segment, 0x0AA8, bytes([(compare_low ^ 0x80) & 0xFF])),
            (data_segment, 0x0AAA, bytes([(compare_high ^ 0x80) & 0xFF])),
            (stack_segment, 0x0AA8, bytes([(compare_low ^ 0x5A) & 0xFF])),
            (stack_segment, 0x0AAA, bytes([(compare_high ^ 0x5A) & 0xFF])),
            (game_segment, 0x6884, struct.pack("<H", 4)),
            (game_segment, 0x67AD, b"\xa5"),
            (stack_segment, 0x6822, struct.pack("<H", branch_target)),
            (game_segment, 0x6822, struct.pack("<H", branch_target ^ 0xFFFF)),
        ]
        immutable = []
        for byte_index, byte in enumerate(script):
            # A word at DS:FFFF reads its high byte at the next linear address;
            # only the post-LODSW SI register wraps to offset 0001.
            offset = start + byte_index
            encoded = bytes([byte])
            memory.append((data_segment, offset, encoded))
            immutable.append((data_segment, offset, encoded))
            memory.append((0x4800, offset, b"\x5a"))
            memory.append((game_segment, offset, b"\xa5"))

        initial = {
            "eax": 0xA1A1BEEF,
            "ebx": 0xB2B22345,
            "ecx": 0xC3C33456,
            "edx": 0xD4D44567,
            "esi": 0xE5E50000 | start,
            "edi": 0xF6F66789,
            "ebp": 0x9797789A,
            "sp": 0xFF00,
            "ds": data_segment,
            "es": 0x4800,
            "fs": 0x4C00,
            "gs": game_segment,
            "ss": stack_segment,
            "flags": 0x0AD7,
        }
        branch_calls = []

        def capture_branch(_machine: Uc, address: int, _size: int) -> None:
            if address == 0x6462:
                branch_calls.append(address)

        machine = execute(
            0x6510,
            0x6558,
            initial,
            memory,
            code_handler=capture_branch,
        )

        expected_calls = [] if pass_result else [0x6462]
        if branch_calls != expected_calls:
            raise AssertionError(
                f"0x6510 {name}: branch calls={branch_calls}, expected={expected_calls}"
            )
        expected_registers = dict(initial)
        del expected_registers["flags"]
        expected_registers["ebx"] = (initial["ebx"] & 0xFFFF0000) | pair_word
        expected_registers["edx"] = (initial["edx"] & 0xFFFFFF00) | tag
        if pass_result:
            expected_registers["eax"] = (initial["eax"] & 0xFFFF0000) | padding_word
            expected_registers["esi"] = (initial["esi"] & 0xFFFF0000) | (
                (start + 5) & 0xFFFF
            )
        else:
            expected_registers["eax"] = (initial["eax"] & 0xFFFF0000) | 2
            expected_registers["esi"] = (initial["esi"] & 0xFFFF0000) | branch_target
        for register, expected in expected_registers.items():
            actual = machine.reg_read(REGISTERS[register])
            if actual != expected:
                raise AssertionError(
                    f"0x6510 {name}: {register}={actual:#x}, expected={expected:#x}"
                )

        actual_top = struct.unpack(
            "<H", machine.mem_read(game_segment * 16 + 0x6884, 2)
        )[0]
        expected_top = 4 if pass_result else 2
        if actual_top != expected_top:
            raise AssertionError(
                f"0x6510 {name}: top={actual_top:#x}, expected={expected_top:#x}"
            )
        actual_query = machine.mem_read(game_segment * 16 + 0x67AD, 1)[0]
        expected_query = 0xA5 if pass_result else 0
        if actual_query != expected_query:
            raise AssertionError(
                f"0x6510 {name}: query={actual_query:#x}, expected={expected_query:#x}"
            )
        for segment, offset, expected in immutable:
            actual = bytes(machine.mem_read(segment * 16 + offset, len(expected)))
            if actual != expected:
                raise AssertionError(f"0x6510 {name}: script input changed")
        for byte_index in range(len(script)):
            offset = start + byte_index
            if machine.mem_read(0x4800 * 16 + offset, 1) != b"\x5a":
                raise AssertionError(f"0x6510 {name}: ES script decoy changed")
            if machine.mem_read(game_segment * 16 + offset, 1) != b"\xa5":
                raise AssertionError(f"0x6510 {name}: GS script decoy changed")

        if pass_result:
            if decision_byte == "high":
                left = pair_high & 0xFF
                right = compare_high & 0xFF
            else:
                left = pair_low & 0xFF
                right = compare_low & 0xFF
            result = (left - right) & 0xFF
            expected_flags = {
                "cf": left < right,
                "pf": result.bit_count() % 2 == 0,
                "af": (left & 0x0F) < (right & 0x0F),
                "zf": result == 0,
                "sf": bool(result & 0x80),
                "of": bool(((left ^ right) & (left ^ result)) & 0x80),
            }
        else:
            expected_flags = {
                "cf": False,
                "pf": False,
                "af": False,
                "zf": False,
                "sf": False,
                "of": False,
            }
        flags = machine.reg_read(UC_X86_REG_EFLAGS)
        actual_flags = {
            "cf": bool(flags & 0x0001),
            "pf": bool(flags & 0x0004),
            "af": bool(flags & 0x0010),
            "zf": bool(flags & 0x0040),
            "sf": bool(flags & 0x0080),
            "of": bool(flags & 0x0800),
        }
        if actual_flags != expected_flags:
            raise AssertionError(
                f"0x6510 {name}: flags={actual_flags}, expected={expected_flags}"
            )
        if EXE[0x6558] != 0xC3:
            raise AssertionError("0x6510: expected near RET boundary")

        vectors.append(
            {
                "name": name,
                "start_offset": start,
                "tag": tag,
                "pair_low": pair_low,
                "pair_high": pair_high,
                "compare_low": compare_low,
                "compare_high": compare_high,
                "padding_word": padding_word,
                "comparison_byte": decision_byte,
                "comparison_passed": pass_result,
                "branch_taken": not pass_result,
                "final_script_offset": (
                    (start + 5) & 0xFFFF if pass_result else branch_target
                ),
                "defined_flags": expected_flags,
            }
        )

    return vectors


def vm_branch_stack_push_vectors() -> list[dict[str, object]]:
    data_segment = 0x4400
    game_segment = 0x2C00
    stack_segment = 0x9000
    cases = [
        ("push_first_word", 0x1000, 0x0002, 0x1234, 0x00),
        ("push_second_word", 0x1100, 0x0004, 0x5678, 0xFF),
        ("odd_top_is_byte_granular", 0x1201, 0x0003, 0x9ABC, 0x7F),
        ("top_add_wraps_to_zero", 0x1300, 0xFFFE, 0xDEF0, 0x80),
        ("top_add_wraps_to_one", 0x1400, 0xFFFF, 0x1357, 0x55),
        ("top_add_signed_overflow", 0x1500, 0x7FFF, 0x2468, 0xAA),
        ("stack_effective_offset_wraps", 0x1600, 0x97E0, 0xBEEF, 0x02),
        ("stack_word_crosses_segment_end", 0x1700, 0x97DF, 0xA55A, 0x01),
        ("script_word_crosses_segment_end", 0xFFFF, 0x0006, 0x5AA5, 0x33),
    ]
    vectors = []

    for name, start, top, target, query_mode in cases:
        new_top = (top + 2) & 0xFFFF
        target_offset = (0x6820 + top) & 0xFFFF
        stack_sentinel = target ^ 0xFFFF
        game_target_decoy = target ^ 0x5A5A
        data_target_decoy = target ^ 0xA5A5
        stack_top_decoy = top ^ 0xFFFF
        stack_query_decoy = query_mode ^ 0xFF
        target_bytes = struct.pack("<H", target)
        stack_sentinel_bytes = struct.pack("<H", stack_sentinel)
        game_target_bytes = struct.pack("<H", game_target_decoy)
        data_target_bytes = struct.pack("<H", data_target_decoy)
        script = struct.pack("<H", target)
        memory = [
            (game_segment, 0x6884, struct.pack("<H", top)),
            (game_segment, 0x67AD, bytes([query_mode])),
            (stack_segment, target_offset, stack_sentinel_bytes),
            (game_segment, target_offset, game_target_bytes),
            (data_segment, target_offset, data_target_bytes),
            (stack_segment, 0x6884, struct.pack("<H", stack_top_decoy)),
            (stack_segment, 0x67AD, bytes([stack_query_decoy])),
        ]
        immutable = []
        for byte_index, byte in enumerate(script):
            # A LODSW at DS:FFFF reads the next linear byte before SI wraps.
            offset = start + byte_index
            encoded = bytes([byte])
            memory.append((data_segment, offset, encoded))
            immutable.append((data_segment, offset, encoded))
            memory.append((0x4800, offset, b"\x5a"))
            memory.append((game_segment, offset, b"\xa5"))

        initial = {
            "eax": 0xA1A1BEEF,
            "ebx": 0xB2B22345,
            "ecx": 0xC3C33456,
            "edx": 0xD4D44567,
            "esi": 0xE5E50000 | start,
            "edi": 0xF6F66789,
            "ebp": 0x9797789A,
            "sp": 0xFF00,
            "ds": data_segment,
            "es": 0x4800,
            "fs": 0x4C00,
            "gs": game_segment,
            "ss": stack_segment,
            "flags": 0x0AD7,
        }
        phases = []

        def capture_phases(machine: Uc, address: int, _size: int) -> None:
            if address not in (0x655F, 0x656C, 0x656D):
                return
            phases.append(
                (
                    address,
                    machine.mem_read(game_segment * 16 + 0x67AD, 1)[0],
                    struct.unpack(
                        "<H", machine.mem_read(game_segment * 16 + 0x6884, 2)
                    )[0],
                    struct.unpack(
                        "<H", machine.mem_read(stack_segment * 16 + target_offset, 2)
                    )[0],
                    machine.reg_read(UC_X86_REG_AX),
                    machine.reg_read(UC_X86_REG_BP),
                    machine.reg_read(UC_X86_REG_SI),
                )
            )

        machine = execute(
            0x6559,
            0x6571,
            initial,
            memory,
            code_handler=capture_phases,
        )

        final_script = (start + 2) & 0xFFFF
        expected_phases = [
            (
                0x655F,
                1,
                top,
                stack_sentinel,
                initial["eax"] & 0xFFFF,
                initial["ebp"] & 0xFFFF,
                start,
            ),
            (0x656C, 1, new_top, stack_sentinel, new_top, top, start),
            (0x656D, 1, new_top, stack_sentinel, target, top, final_script),
        ]
        if phases != expected_phases:
            raise AssertionError(
                f"0x6559 {name}: phases={phases}, expected={expected_phases}"
            )

        expected_registers = dict(initial)
        del expected_registers["flags"]
        expected_registers["eax"] = (initial["eax"] & 0xFFFF0000) | target
        expected_registers["esi"] = (
            initial["esi"] & 0xFFFF0000
        ) | final_script
        expected_registers["ebp"] = (initial["ebp"] & 0xFFFF0000) | top
        for register, expected in expected_registers.items():
            actual = machine.reg_read(REGISTERS[register])
            if actual != expected:
                raise AssertionError(
                    f"0x6559 {name}: {register}={actual:#x}, expected={expected:#x}"
                )

        actual_top = struct.unpack(
            "<H", machine.mem_read(game_segment * 16 + 0x6884, 2)
        )[0]
        if actual_top != new_top:
            raise AssertionError(
                f"0x6559 {name}: top={actual_top:#x}, expected={new_top:#x}"
            )
        actual_query = machine.mem_read(game_segment * 16 + 0x67AD, 1)[0]
        if actual_query != 1:
            raise AssertionError(f"0x6559 {name}: query mode was not set")
        actual_target = bytes(
            machine.mem_read(stack_segment * 16 + target_offset, 2)
        )
        if actual_target != target_bytes:
            raise AssertionError(
                f"0x6559 {name}: stack={actual_target.hex()}, "
                f"expected={target_bytes.hex()}"
            )
        if (
            machine.mem_read(game_segment * 16 + target_offset, 2)
            != game_target_bytes
        ):
            raise AssertionError(f"0x6559 {name}: GS stack decoy changed")
        if (
            machine.mem_read(data_segment * 16 + target_offset, 2)
            != data_target_bytes
        ):
            raise AssertionError(f"0x6559 {name}: DS stack decoy changed")
        actual_stack_top = struct.unpack(
            "<H", machine.mem_read(stack_segment * 16 + 0x6884, 2)
        )[0]
        if actual_stack_top != stack_top_decoy:
            raise AssertionError(f"0x6559 {name}: SS top decoy changed")
        actual_stack_query = machine.mem_read(stack_segment * 16 + 0x67AD, 1)[0]
        if actual_stack_query != stack_query_decoy:
            raise AssertionError(f"0x6559 {name}: SS query decoy changed")
        for segment, offset, expected in immutable:
            actual = bytes(machine.mem_read(segment * 16 + offset, len(expected)))
            if actual != expected:
                raise AssertionError(f"0x6559 {name}: script input changed")
        for byte_index in range(len(script)):
            offset = start + byte_index
            if machine.mem_read(0x4800 * 16 + offset, 1) != b"\x5a":
                raise AssertionError(f"0x6559 {name}: ES script decoy changed")
            if machine.mem_read(game_segment * 16 + offset, 1) != b"\xa5":
                raise AssertionError(f"0x6559 {name}: GS script decoy changed")

        flags = machine.reg_read(UC_X86_REG_EFLAGS)
        expected_flags = {
            "cf": top > 0xFFFD,
            "pf": (new_top & 0xFF).bit_count() % 2 == 0,
            "af": (top & 0x0F) + 2 > 0x0F,
            "zf": new_top == 0,
            "sf": bool(new_top & 0x8000),
            "of": bool((~(top ^ 2) & (top ^ new_top)) & 0x8000),
        }
        actual_flags = {
            "cf": bool(flags & 0x0001),
            "pf": bool(flags & 0x0004),
            "af": bool(flags & 0x0010),
            "zf": bool(flags & 0x0040),
            "sf": bool(flags & 0x0080),
            "of": bool(flags & 0x0800),
        }
        if actual_flags != expected_flags:
            raise AssertionError(
                f"0x6559 {name}: flags={actual_flags}, expected={expected_flags}"
            )
        if EXE[0x6571] != 0xC3:
            raise AssertionError("0x6559: expected near RET boundary")

        vectors.append(
            {
                "name": name,
                "start_offset": start,
                "initial_top_byte_offset": top,
                "final_top_byte_offset": new_top,
                "stack_effective_offset": target_offset,
                "target": target,
                "query_mode_before": query_mode,
                "final_script_offset": final_script,
                "defined_flags": expected_flags,
            }
        )

    return vectors


def vm_branch_stack_pop_vectors() -> list[dict[str, object]]:
    game_segment = 0x2C00
    data_segment = 0x4400
    stack_segment = 0x9000
    cases = [
        ("empty_stack_is_unchanged", 0x0002, 0x00),
        ("pop_first_word", 0x0004, 0xFF),
        ("odd_top_is_byte_granular", 0x0003, 0x7F),
        ("top_wraps_below_zero", 0x0000, 0x80),
        ("odd_top_wraps", 0x0001, 0x55),
        ("signed_overflow", 0x8000, 0xAA),
        ("maximum_top", 0xFFFF, 0x02),
    ]
    vectors = []

    for name, top, query_mode in cases:
        final_top = top if top == 2 else (top - 2) & 0xFFFF
        data_top_decoy = top ^ 0xFFFF
        stack_top_decoy = top ^ 0x5A5A
        data_query_decoy = query_mode ^ 0xFF
        stack_query_decoy = query_mode ^ 0xA5
        memory = [
            (game_segment, 0x6884, struct.pack("<H", top)),
            (game_segment, 0x67AD, bytes([query_mode])),
            (data_segment, 0x6884, struct.pack("<H", data_top_decoy)),
            (stack_segment, 0x6884, struct.pack("<H", stack_top_decoy)),
            (data_segment, 0x67AD, bytes([data_query_decoy])),
            (stack_segment, 0x67AD, bytes([stack_query_decoy])),
        ]
        initial = {
            "eax": 0xA1A1BEEF,
            "ebx": 0xB2B22345,
            "ecx": 0xC3C33456,
            "edx": 0xD4D44567,
            "esi": 0xE5E55678,
            "edi": 0xF6F66789,
            "ebp": 0x9797789A,
            "sp": 0xFF00,
            "ds": data_segment,
            "es": 0x4800,
            "fs": 0x4C00,
            "gs": game_segment,
            "ss": stack_segment,
            "flags": 0x0AD7,
        }
        phases = []

        def capture_phases(machine: Uc, address: int, _size: int) -> None:
            if address not in (0x6578, 0x6581):
                return
            phases.append(
                (
                    address,
                    machine.mem_read(game_segment * 16 + 0x67AD, 1)[0],
                    struct.unpack(
                        "<H", machine.mem_read(game_segment * 16 + 0x6884, 2)
                    )[0],
                    machine.reg_read(UC_X86_REG_AX),
                )
            )

        machine = execute(
            0x6572,
            0x6587,
            initial,
            memory,
            code_handler=capture_phases,
        )

        expected_phases = [
            (0x6578, 0, top, initial["eax"] & 0xFFFF),
        ]
        if top != 2:
            expected_phases.append((0x6581, 0, top, top))
        if phases != expected_phases:
            raise AssertionError(
                f"0x6572 {name}: phases={phases}, expected={expected_phases}"
            )

        expected_registers = dict(initial)
        del expected_registers["flags"]
        expected_registers["eax"] = (initial["eax"] & 0xFFFF0000) | top
        for register, expected in expected_registers.items():
            actual = machine.reg_read(REGISTERS[register])
            if actual != expected:
                raise AssertionError(
                    f"0x6572 {name}: {register}={actual:#x}, expected={expected:#x}"
                )

        actual_top = struct.unpack(
            "<H", machine.mem_read(game_segment * 16 + 0x6884, 2)
        )[0]
        if actual_top != final_top:
            raise AssertionError(
                f"0x6572 {name}: top={actual_top:#x}, expected={final_top:#x}"
            )
        actual_query = machine.mem_read(game_segment * 16 + 0x67AD, 1)[0]
        if actual_query != 0:
            raise AssertionError(f"0x6572 {name}: query mode was not cleared")
        actual_data_top = struct.unpack(
            "<H", machine.mem_read(data_segment * 16 + 0x6884, 2)
        )[0]
        if actual_data_top != data_top_decoy:
            raise AssertionError(f"0x6572 {name}: DS top decoy changed")
        actual_stack_top = struct.unpack(
            "<H", machine.mem_read(stack_segment * 16 + 0x6884, 2)
        )[0]
        if actual_stack_top != stack_top_decoy:
            raise AssertionError(f"0x6572 {name}: SS top decoy changed")
        if machine.mem_read(data_segment * 16 + 0x67AD, 1)[0] != data_query_decoy:
            raise AssertionError(f"0x6572 {name}: DS query decoy changed")
        if (
            machine.mem_read(stack_segment * 16 + 0x67AD, 1)[0]
            != stack_query_decoy
        ):
            raise AssertionError(f"0x6572 {name}: SS query decoy changed")

        result = (top - 2) & 0xFFFF
        flags = machine.reg_read(UC_X86_REG_EFLAGS)
        expected_flags = {
            "cf": top < 2,
            "pf": (result & 0xFF).bit_count() % 2 == 0,
            "af": (top & 0x0F) < 2,
            "zf": result == 0,
            "sf": bool(result & 0x8000),
            "of": bool(((top ^ 2) & (top ^ result)) & 0x8000),
        }
        actual_flags = {
            "cf": bool(flags & 0x0001),
            "pf": bool(flags & 0x0004),
            "af": bool(flags & 0x0010),
            "zf": bool(flags & 0x0040),
            "sf": bool(flags & 0x0080),
            "of": bool(flags & 0x0800),
        }
        if actual_flags != expected_flags:
            raise AssertionError(
                f"0x6572 {name}: flags={actual_flags}, expected={expected_flags}"
            )
        if EXE[0x6587] != 0xC3:
            raise AssertionError("0x6572: expected near RET boundary")

        vectors.append(
            {
                "name": name,
                "initial_top_byte_offset": top,
                "final_top_byte_offset": final_top,
                "top_write_performed": top != 2,
                "query_mode_before": query_mode,
                "defined_flags": expected_flags,
            }
        )

    return vectors


def vm_random_branch_vectors() -> list[dict[str, object]]:
    data_segment = 0x4400
    game_segment = 0x2C00
    stack_segment = 0x9000
    cases = [
        ("modulus_one_zero_continues", 0x1000, 1, 0, 4, 0x7100, 0xA5),
        ("modulus_zero_zero_continues", 0x1101, 0, 0, 6, 0x7211, 0x5A),
        ("result_one_branches", 0x1200, 5, 1, 4, 0x7322, 0xFF),
        ("maximum_result_branches", 0x1300, 0xFFFF, 0xFFFF, 2, 0x7433, 0x80),
        ("high_bit_result_branches", 0x1400, 7, 0x8000, 0x8000, 0x7544, 0x7F),
        ("odd_branch_stack_top", 0x1500, 3, 2, 3, 0x7655, 0x33),
        ("script_word_crosses_segment_end", 0xFFFF, 9, 0, 8, 0x7766, 0x01),
    ]
    vectors = []

    for name, start, modulus, prng_result, top, target, query_mode in cases:
        branch_taken = prng_result != 0
        final_script = (start + 2) & 0xFFFF
        final_top = (top - 2) & 0xFFFF if branch_taken else top
        target_offset = (0x6820 + ((top - 2) & 0xFFFF)) & 0xFFFF
        target_bytes = struct.pack("<H", target)
        game_target_bytes = struct.pack("<H", target ^ 0xFFFF)
        data_target_bytes = struct.pack("<H", target ^ 0x5A5A)
        script = struct.pack("<H", modulus)
        data_top_decoy = top ^ 0xA5A5
        stack_top_decoy = top ^ 0x5A5A
        data_query_decoy = query_mode ^ 0xFF
        stack_query_decoy = query_mode ^ 0xA5
        memory = [
            (0, 0x27E2, b"\xb8" + struct.pack("<H", prng_result) + b"\xcb"),
            (game_segment, 0x6884, struct.pack("<H", top)),
            (game_segment, 0x67AD, bytes([query_mode])),
            (stack_segment, target_offset, target_bytes),
            (game_segment, target_offset, game_target_bytes),
            (data_segment, target_offset, data_target_bytes),
            (data_segment, 0x6884, struct.pack("<H", data_top_decoy)),
            (stack_segment, 0x6884, struct.pack("<H", stack_top_decoy)),
            (data_segment, 0x67AD, bytes([data_query_decoy])),
            (stack_segment, 0x67AD, bytes([stack_query_decoy])),
        ]
        immutable = []
        for byte_index, byte in enumerate(script):
            # A LODSW at DS:FFFF reads the next linear byte before SI wraps.
            offset = start + byte_index
            encoded = bytes([byte])
            memory.append((data_segment, offset, encoded))
            immutable.append((data_segment, offset, encoded))
            memory.append((0x4800, offset, b"\x5a"))
            memory.append((game_segment, offset, b"\xa5"))

        initial = {
            "eax": 0xA1A1BEEF,
            "ebx": 0xB2B22345,
            "ecx": 0xC3C33456,
            "edx": 0xD4D44567,
            "esi": 0xE5E50000 | start,
            "edi": 0xF6F66789,
            "ebp": 0x9797789A,
            "sp": 0xFF00,
            "ds": data_segment,
            "es": 0x4800,
            "fs": 0x4C00,
            "gs": game_segment,
            "ss": stack_segment,
            "flags": 0x0AD7,
        }
        prng_calls = []
        branch_calls = []

        def capture_calls(machine: Uc, address: int, _size: int) -> None:
            if address == 0x27E2:
                prng_calls.append(
                    (
                        machine.reg_read(UC_X86_REG_AX),
                        machine.reg_read(UC_X86_REG_SI),
                        machine.reg_read(UC_X86_REG_SP),
                        machine.reg_read(UC_X86_REG_CS),
                    )
                )
            elif address == 0x6462:
                branch_calls.append(
                    (
                        machine.reg_read(UC_X86_REG_AX),
                        machine.reg_read(UC_X86_REG_SI),
                        machine.reg_read(UC_X86_REG_SP),
                        machine.reg_read(UC_X86_REG_CS),
                    )
                )

        machine = execute(
            0x6588,
            0x6595,
            initial,
            memory,
            code_handler=capture_calls,
        )

        expected_prng_calls = [(modulus, final_script, 0xFEFC, 0x01CE)]
        if prng_calls != expected_prng_calls:
            raise AssertionError(
                f"0x6588 {name}: PRNG calls={prng_calls}, "
                f"expected={expected_prng_calls}"
            )
        expected_branch_calls = (
            [(prng_result, final_script, 0xFEFE, 0)] if branch_taken else []
        )
        if branch_calls != expected_branch_calls:
            raise AssertionError(
                f"0x6588 {name}: branch calls={branch_calls}, "
                f"expected={expected_branch_calls}"
            )

        expected_registers = dict(initial)
        del expected_registers["flags"]
        if branch_taken:
            expected_registers["eax"] = (
                initial["eax"] & 0xFFFF0000
            ) | final_top
            expected_registers["esi"] = (
                initial["esi"] & 0xFFFF0000
            ) | target
        else:
            expected_registers["eax"] = initial["eax"] & 0xFFFF0000
            expected_registers["esi"] = (
                initial["esi"] & 0xFFFF0000
            ) | final_script
        for register, expected in expected_registers.items():
            actual = machine.reg_read(REGISTERS[register])
            if actual != expected:
                raise AssertionError(
                    f"0x6588 {name}: {register}={actual:#x}, expected={expected:#x}"
                )
        if machine.reg_read(UC_X86_REG_CS) != 0:
            raise AssertionError(f"0x6588 {name}: far call did not restore CS")

        actual_top = struct.unpack(
            "<H", machine.mem_read(game_segment * 16 + 0x6884, 2)
        )[0]
        if actual_top != final_top:
            raise AssertionError(
                f"0x6588 {name}: top={actual_top:#x}, expected={final_top:#x}"
            )
        actual_query = machine.mem_read(game_segment * 16 + 0x67AD, 1)[0]
        expected_query = 0 if branch_taken else query_mode
        if actual_query != expected_query:
            raise AssertionError(
                f"0x6588 {name}: query={actual_query:#x}, expected={expected_query:#x}"
            )
        if machine.mem_read(stack_segment * 16 + target_offset, 2) != target_bytes:
            raise AssertionError(f"0x6588 {name}: branch target changed")
        if (
            machine.mem_read(game_segment * 16 + target_offset, 2)
            != game_target_bytes
        ):
            raise AssertionError(f"0x6588 {name}: GS stack decoy changed")
        if (
            machine.mem_read(data_segment * 16 + target_offset, 2)
            != data_target_bytes
        ):
            raise AssertionError(f"0x6588 {name}: DS stack decoy changed")
        if struct.unpack(
            "<H", machine.mem_read(data_segment * 16 + 0x6884, 2)
        )[0] != data_top_decoy:
            raise AssertionError(f"0x6588 {name}: DS top decoy changed")
        if struct.unpack(
            "<H", machine.mem_read(stack_segment * 16 + 0x6884, 2)
        )[0] != stack_top_decoy:
            raise AssertionError(f"0x6588 {name}: SS top decoy changed")
        if machine.mem_read(data_segment * 16 + 0x67AD, 1)[0] != data_query_decoy:
            raise AssertionError(f"0x6588 {name}: DS query decoy changed")
        if (
            machine.mem_read(stack_segment * 16 + 0x67AD, 1)[0]
            != stack_query_decoy
        ):
            raise AssertionError(f"0x6588 {name}: SS query decoy changed")
        for segment, offset, expected in immutable:
            actual = bytes(machine.mem_read(segment * 16 + offset, len(expected)))
            if actual != expected:
                raise AssertionError(f"0x6588 {name}: script input changed")
        for byte_index in range(len(script)):
            offset = start + byte_index
            if machine.mem_read(0x4800 * 16 + offset, 1) != b"\x5a":
                raise AssertionError(f"0x6588 {name}: ES script decoy changed")
            if machine.mem_read(game_segment * 16 + offset, 1) != b"\xa5":
                raise AssertionError(f"0x6588 {name}: GS script decoy changed")

        flags = machine.reg_read(UC_X86_REG_EFLAGS)
        if branch_taken:
            result = final_top
            expected_flags = {
                "cf": top < 2,
                "pf": (result & 0xFF).bit_count() % 2 == 0,
                "af": (top & 0x0F) < 2,
                "zf": result == 0,
                "sf": bool(result & 0x8000),
                "of": bool(((top ^ 2) & (top ^ result)) & 0x8000),
            }
        else:
            expected_flags = {
                "cf": False,
                "pf": True,
                "zf": True,
                "sf": False,
                "of": False,
            }
        flag_masks = {
            "cf": 0x0001,
            "pf": 0x0004,
            "af": 0x0010,
            "zf": 0x0040,
            "sf": 0x0080,
            "of": 0x0800,
        }
        actual_flags = {
            flag: bool(flags & flag_masks[flag]) for flag in expected_flags
        }
        if actual_flags != expected_flags:
            raise AssertionError(
                f"0x6588 {name}: flags={actual_flags}, expected={expected_flags}"
            )
        if EXE[0x6595] != 0xC3:
            raise AssertionError("0x6588: expected near RET boundary")

        vectors.append(
            {
                "name": name,
                "start_offset": start,
                "modulus": modulus,
                "prng_result": prng_result,
                "branch_taken": branch_taken,
                "initial_top_byte_offset": top,
                "final_top_byte_offset": final_top,
                "query_mode_before": query_mode,
                "final_script_offset": target if branch_taken else final_script,
                "defined_flags": expected_flags,
            }
        )

    return vectors


def vm_conditional_block_vectors() -> list[dict[str, object]]:
    data_segment = 0x4400
    game_segment = 0x2C00
    stack_segment = 0x9000
    cases = [
        {
            "name": "scan_immediate_zero_with_pad",
            "start": 0x1000,
            "scan_flags": 1,
            "script": b"\x00\x00\x00",
            "scan_final": 0x1003,
            "scan_pad": 0,
            "scan_steps": 1,
        },
        {
            "name": "scan_to_later_zero_without_pad",
            "start": 0x1101,
            "scan_flags": 0xFF,
            "script": b"\x12\x34\x56\x00\x00\x7f",
            "scan_final": 0x1106,
            "scan_pad": 0x7F,
            "scan_steps": 4,
        },
        {
            "name": "default_equal_continues",
            "start": 0x1200,
            "scan_flags": 0,
            "target": 0x1234,
            "block_match": 0x1234,
            "continue": True,
        },
        {
            "name": "default_mismatch_branches",
            "start": 0x1300,
            "scan_flags": 0,
            "target": 0x1234,
            "block_match": 0x1235,
            "top": 4,
            "branch_target": 0x7133,
            "continue": False,
        },
        {
            "name": "zero_match_always_branches",
            "start": 0x1400,
            "scan_flags": 0,
            "target": 0x4321,
            "block_match": 0,
            "top": 0,
            "branch_target": 0x7244,
            "continue": False,
        },
        {
            "name": "inverted_equal_branches",
            "start": 0x1500,
            "scan_flags": 0,
            "inverted": True,
            "target": 0x2345,
            "block_match": 0x2345,
            "top": 0x8000,
            "branch_target": 0x7355,
            "continue": False,
        },
        {
            "name": "inverted_mismatch_continues",
            "start": 0x1600,
            "scan_flags": 0,
            "inverted": True,
            "target": 0x2345,
            "block_match": 0x2346,
            "continue": True,
        },
        {
            "name": "resume_value_equal_continues",
            "start": 0x1700,
            "scan_flags": 0,
            "resume_state": 2,
            "target": 0x3456,
            "block_match": 0xA55A,
            "resume_value": 0x3456,
            "continue": True,
        },
        {
            "name": "resume_value_mismatch_branches",
            "start": 0x1800,
            "scan_flags": 0,
            "resume_state": 0xFF,
            "target": 0x3456,
            "block_match": 0x3456,
            "resume_value": 0x3457,
            "top": 3,
            "branch_target": 0x7466,
            "continue": False,
        },
        {
            "name": "only_scan_bit_zero_is_used",
            "start": 0x1900,
            "scan_flags": 0xFE,
            "target": 0x4567,
            "block_match": 0x4567,
            "continue": True,
        },
        {
            "name": "target_word_crosses_segment_end",
            "start": 0xFFFF,
            "scan_flags": 0,
            "target": 0x5678,
            "block_match": 0x5678,
            "continue": True,
        },
        {
            "name": "inverted_target_crosses_segment_end",
            "start": 0xFFFE,
            "scan_flags": 0,
            "inverted": True,
            "target": 0x6789,
            "block_match": 0x678A,
            "continue": True,
        },
    ]
    vectors = []

    for case_index, case in enumerate(cases):
        name = str(case["name"])
        start = int(case["start"])
        scan_flags = int(case["scan_flags"])
        scan_path = bool(scan_flags & 1)
        resume_state = int(case.get("resume_state", 0))
        block_match = int(case.get("block_match", 0x1357))
        resume_value = int(case.get("resume_value", 0x2468))
        inverted = bool(case.get("inverted", False))
        target = int(case.get("target", 0))
        top = int(case.get("top", 4))
        branch_target = int(case.get("branch_target", 0x7600 + case_index * 0x31))
        query_mode = (0x80 + case_index * 7) & 0xFF

        if scan_path:
            script = bytes(case["script"])
            operand_cursor = start
            continue_path = True
            branch_taken = False
            final_script = int(case["scan_final"])
            selected_offset = None
            selected_match = None
        else:
            script = (b"\xa1" if inverted else b"") + struct.pack("<H", target)
            operand_cursor = (start + int(inverted)) & 0xFFFF
            continue_path = bool(case["continue"])
            branch_taken = not continue_path
            final_script = (operand_cursor + 2) & 0xFFFF
            selected_offset = 0x6764 if resume_state & 2 else 0x6762
            selected_match = resume_value if resume_state & 2 else block_match

        final_top = (top - 2) & 0xFFFF if branch_taken else top
        stack_target_offset = (0x6820 + ((top - 2) & 0xFFFF)) & 0xFFFF
        branch_target_bytes = struct.pack("<H", branch_target)
        game_stack_decoy = struct.pack("<H", branch_target ^ 0xFFFF)
        data_stack_decoy = struct.pack("<H", branch_target ^ 0x5A5A)
        memory = [
            (game_segment, 0x67B2, bytes([scan_flags])),
            (game_segment, 0x67B1, bytes([resume_state])),
            (stack_segment, 0x6762, struct.pack("<H", block_match)),
            (stack_segment, 0x6764, struct.pack("<H", resume_value)),
            (game_segment, 0x6762, struct.pack("<H", block_match ^ 0xFFFF)),
            (game_segment, 0x6764, struct.pack("<H", resume_value ^ 0xFFFF)),
            (data_segment, 0x6762, struct.pack("<H", block_match ^ 0xA5A5)),
            (data_segment, 0x6764, struct.pack("<H", resume_value ^ 0xA5A5)),
            (data_segment, 0x67B2, bytes([scan_flags ^ 0xFF])),
            (stack_segment, 0x67B2, bytes([scan_flags ^ 0xA5])),
            (data_segment, 0x67B1, bytes([resume_state ^ 0xFF])),
            (stack_segment, 0x67B1, bytes([resume_state ^ 0xA5])),
            (game_segment, 0x6884, struct.pack("<H", top)),
            (game_segment, 0x67AD, bytes([query_mode])),
            (stack_segment, stack_target_offset, branch_target_bytes),
            (game_segment, stack_target_offset, game_stack_decoy),
            (data_segment, stack_target_offset, data_stack_decoy),
        ]
        immutable = []
        for byte_index, byte in enumerate(script):
            offset = start + byte_index
            encoded = bytes([byte])
            memory.append((data_segment, offset, encoded))
            immutable.append((data_segment, offset, encoded))
            memory.append((0x4800, offset, b"\x5a"))
            memory.append((game_segment, offset, b"\xa5"))

        initial = {
            "eax": 0xA1A1BEEF,
            "ebx": 0xB2B22345,
            "ecx": 0xC3C33456,
            "edx": 0xD4D44567,
            "esi": 0xE5E50000 | start,
            "edi": 0xF6F66789,
            "ebp": 0x9797789A,
            "sp": 0xFF00,
            "ds": data_segment,
            "es": 0x4800,
            "fs": 0x4C00,
            "gs": game_segment,
            "ss": stack_segment,
            "flags": 0x0AD7,
        }
        token_calls = []
        branch_calls = []

        def capture_calls(machine: Uc, address: int, _size: int) -> None:
            if address == 0x6293:
                token_calls.append(
                    (
                        machine.reg_read(UC_X86_REG_AX),
                        machine.reg_read(UC_X86_REG_SI),
                        machine.reg_read(UC_X86_REG_SP),
                    )
                )
            elif address == 0x6462:
                branch_calls.append(
                    (
                        machine.reg_read(UC_X86_REG_AX),
                        machine.reg_read(UC_X86_REG_SI),
                        machine.reg_read(UC_X86_REG_SP),
                    )
                )

        machine = execute(
            0x6596,
            0x65DA,
            initial,
            memory,
            code_handler=capture_calls,
        )

        expected_token_calls = (
            [
                (0, (start + step) & 0xFFFF, 0xFEFC)
                for step in range(int(case["scan_steps"]))
            ]
            if scan_path
            else []
        )
        if token_calls != expected_token_calls:
            raise AssertionError(
                f"0x6596 {name}: token calls={token_calls}, "
                f"expected={expected_token_calls}"
            )
        expected_branch_calls = (
            [(target, final_script, 0xFEFC)] if branch_taken else []
        )
        if branch_calls != expected_branch_calls:
            raise AssertionError(
                f"0x6596 {name}: branch calls={branch_calls}, "
                f"expected={expected_branch_calls}"
            )

        expected_registers = dict(initial)
        del expected_registers["flags"]
        if scan_path:
            expected_registers["eax"] = initial["eax"] & 0xFFFF0000
            expected_registers["esi"] = (
                initial["esi"] & 0xFFFF0000
            ) | final_script
        else:
            expected_registers["edx"] = (
                initial["edx"] & 0xFFFFFF00
            ) | int(inverted)
            expected_registers["ebp"] = (
                initial["ebp"] & 0xFFFF0000
            ) | int(selected_offset)
            if branch_taken:
                expected_registers["eax"] = (
                    initial["eax"] & 0xFFFF0000
                ) | final_top
                expected_registers["esi"] = (
                    initial["esi"] & 0xFFFF0000
                ) | branch_target
            else:
                expected_registers["eax"] = (
                    initial["eax"] & 0xFFFF0000
                ) | target
                expected_registers["esi"] = (
                    initial["esi"] & 0xFFFF0000
                ) | final_script
        for register, expected in expected_registers.items():
            actual = machine.reg_read(REGISTERS[register])
            if actual != expected:
                raise AssertionError(
                    f"0x6596 {name}: {register}={actual:#x}, expected={expected:#x}"
                )

        actual_top = struct.unpack(
            "<H", machine.mem_read(game_segment * 16 + 0x6884, 2)
        )[0]
        if actual_top != final_top:
            raise AssertionError(
                f"0x6596 {name}: top={actual_top:#x}, expected={final_top:#x}"
            )
        actual_query = machine.mem_read(game_segment * 16 + 0x67AD, 1)[0]
        expected_query = 0 if branch_taken else query_mode
        if actual_query != expected_query:
            raise AssertionError(
                f"0x6596 {name}: query={actual_query:#x}, expected={expected_query:#x}"
            )
        if (
            machine.mem_read(stack_segment * 16 + stack_target_offset, 2)
            != branch_target_bytes
        ):
            raise AssertionError(f"0x6596 {name}: branch target changed")
        if (
            machine.mem_read(game_segment * 16 + stack_target_offset, 2)
            != game_stack_decoy
        ):
            raise AssertionError(f"0x6596 {name}: GS stack decoy changed")
        if (
            machine.mem_read(data_segment * 16 + stack_target_offset, 2)
            != data_stack_decoy
        ):
            raise AssertionError(f"0x6596 {name}: DS stack decoy changed")
        for offset, value, label in (
            (0x6762, block_match, "block match"),
            (0x6764, resume_value, "resume value"),
        ):
            actual = struct.unpack(
                "<H", machine.mem_read(stack_segment * 16 + offset, 2)
            )[0]
            if actual != value:
                raise AssertionError(f"0x6596 {name}: {label} changed")
        for segment, offset, expected in immutable:
            actual = bytes(machine.mem_read(segment * 16 + offset, len(expected)))
            if actual != expected:
                raise AssertionError(f"0x6596 {name}: script input changed")
        for byte_index in range(len(script)):
            offset = start + byte_index
            if machine.mem_read(0x4800 * 16 + offset, 1) != b"\x5a":
                raise AssertionError(f"0x6596 {name}: ES script decoy changed")
            if machine.mem_read(game_segment * 16 + offset, 1) != b"\xa5":
                raise AssertionError(f"0x6596 {name}: GS script decoy changed")

        if branch_taken:
            result = final_top
            expected_flags = {
                "cf": top < 2,
                "pf": (result & 0xFF).bit_count() % 2 == 0,
                "af": (top & 0x0F) < 2,
                "zf": result == 0,
                "sf": bool(result & 0x8000),
                "of": bool(((top ^ 2) & (top ^ result)) & 0x8000),
            }
        elif scan_path:
            scan_pad = int(case["scan_pad"])
            if scan_pad == 0:
                before_increment = (final_script - 1) & 0xFFFF
                expected_flags = {
                    "cf": False,
                    "pf": (final_script & 0xFF).bit_count() % 2 == 0,
                    "af": (before_increment & 0x0F) == 0x0F,
                    "zf": final_script == 0,
                    "sf": bool(final_script & 0x8000),
                    "of": before_increment == 0x7FFF,
                }
            else:
                result = (-scan_pad) & 0xFF
                expected_flags = {
                    "cf": True,
                    "pf": result.bit_count() % 2 == 0,
                    "af": (scan_pad & 0x0F) != 0,
                    "zf": False,
                    "sf": bool(result & 0x80),
                    "of": scan_pad == 0x80,
                }
        else:
            result = int(inverted)
            expected_flags = {
                "cf": False,
                "pf": (result & 0xFF).bit_count() % 2 == 0,
                "zf": result == 0,
                "sf": False,
                "of": False,
            }
        flags = machine.reg_read(UC_X86_REG_EFLAGS)
        flag_masks = {
            "cf": 0x0001,
            "pf": 0x0004,
            "af": 0x0010,
            "zf": 0x0040,
            "sf": 0x0080,
            "of": 0x0800,
        }
        actual_flags = {
            flag: bool(flags & flag_masks[flag]) for flag in expected_flags
        }
        if actual_flags != expected_flags:
            raise AssertionError(
                f"0x6596 {name}: flags={actual_flags}, expected={expected_flags}"
            )
        if EXE[0x65DA] != 0xC3:
            raise AssertionError("0x6596: expected near RET boundary")

        vectors.append(
            {
                "name": name,
                "scan_path": scan_path,
                "scan_flags": scan_flags,
                "resume_state": resume_state if not scan_path else None,
                "inverted": inverted if not scan_path else None,
                "target": target if not scan_path else None,
                "selected_match_offset": selected_offset,
                "selected_match": selected_match,
                "branch_taken": branch_taken,
                "final_script_offset": branch_target if branch_taken else final_script,
                "defined_flags": expected_flags,
            }
        )

    return vectors


def vm_script_jump_vectors() -> list[dict[str, object]]:
    data_segment = 0x4400
    game_segment = 0x2C00
    stack_segment = 0x9000
    cases = [
        ("ordinary_target", 0x1000, 0x2345, 0x01, 0x4567),
        ("zero_target", 0x1100, 0x0000, 0xFF, 0xFFFF),
        ("odd_target", 0x1200, 0x1357, 0x80, 0x8000),
        ("maximum_target", 0x1300, 0xFFFF, 0x7F, 0x0001),
        ("unaligned_operand", 0x1401, 0x6789, 0x55, 0xA55A),
        ("operand_crosses_segment_end", 0xFFFF, 0xBEEF, 0xAA, 0x5AA5),
    ]
    vectors = []

    for name, start, target, resume_state, resume_value in cases:
        script = struct.pack("<H", target)
        data_state_decoy = resume_state ^ 0xFF
        stack_state_decoy = resume_state ^ 0xA5
        data_value_decoy = resume_value ^ 0xFFFF
        stack_value_decoy = resume_value ^ 0x5A5A
        memory = [
            (game_segment, 0x67B1, bytes([resume_state])),
            (game_segment, 0x6764, struct.pack("<H", resume_value)),
            (data_segment, 0x67B1, bytes([data_state_decoy])),
            (stack_segment, 0x67B1, bytes([stack_state_decoy])),
            (data_segment, 0x6764, struct.pack("<H", data_value_decoy)),
            (stack_segment, 0x6764, struct.pack("<H", stack_value_decoy)),
        ]
        immutable = []
        for byte_index, byte in enumerate(script):
            # A word at DS:FFFF reads its high byte at the next linear address.
            offset = start + byte_index
            encoded = bytes([byte])
            memory.append((data_segment, offset, encoded))
            immutable.append((data_segment, offset, encoded))
            memory.append((0x4800, offset, b"\x5a"))
            memory.append((game_segment, offset, b"\xa5"))

        initial = {
            "eax": 0xA1A1BEEF,
            "ebx": 0xB2B22345,
            "ecx": 0xC3C33456,
            "edx": 0xD4D44567,
            "esi": 0xE5E50000 | start,
            "edi": 0xF6F66789,
            "ebp": 0x9797789A,
            "sp": 0xFF00,
            "ds": data_segment,
            "es": 0x4800,
            "fs": 0x4C00,
            "gs": game_segment,
            "ss": stack_segment,
            "flags": 0x0AD7,
        }
        phases = []

        def capture_phases(machine: Uc, address: int, _size: int) -> None:
            if address not in (0x65DD, 0x65E3):
                return
            phases.append(
                (
                    address,
                    machine.reg_read(UC_X86_REG_SI),
                    machine.mem_read(game_segment * 16 + 0x67B1, 1)[0],
                    struct.unpack(
                        "<H", machine.mem_read(game_segment * 16 + 0x6764, 2)
                    )[0],
                )
            )

        machine = execute(
            0x65DB,
            0x65EA,
            initial,
            memory,
            code_handler=capture_phases,
        )

        expected_phases = [
            (0x65DD, target, resume_state, resume_value),
            (0x65E3, target, 0, resume_value),
        ]
        if phases != expected_phases:
            raise AssertionError(
                f"0x65db {name}: phases={phases}, expected={expected_phases}"
            )

        expected_registers = dict(initial)
        del expected_registers["flags"]
        expected_registers["esi"] = (
            initial["esi"] & 0xFFFF0000
        ) | target
        for register, expected in expected_registers.items():
            actual = machine.reg_read(REGISTERS[register])
            if actual != expected:
                raise AssertionError(
                    f"0x65db {name}: {register}={actual:#x}, expected={expected:#x}"
                )

        if machine.mem_read(game_segment * 16 + 0x67B1, 1)[0] != 0:
            raise AssertionError(f"0x65db {name}: resume state was not cleared")
        actual_value = struct.unpack(
            "<H", machine.mem_read(game_segment * 16 + 0x6764, 2)
        )[0]
        if actual_value != 0:
            raise AssertionError(f"0x65db {name}: resume value was not cleared")
        if machine.mem_read(data_segment * 16 + 0x67B1, 1)[0] != data_state_decoy:
            raise AssertionError(f"0x65db {name}: DS state decoy changed")
        if (
            machine.mem_read(stack_segment * 16 + 0x67B1, 1)[0]
            != stack_state_decoy
        ):
            raise AssertionError(f"0x65db {name}: SS state decoy changed")
        if struct.unpack(
            "<H", machine.mem_read(data_segment * 16 + 0x6764, 2)
        )[0] != data_value_decoy:
            raise AssertionError(f"0x65db {name}: DS value decoy changed")
        if struct.unpack(
            "<H", machine.mem_read(stack_segment * 16 + 0x6764, 2)
        )[0] != stack_value_decoy:
            raise AssertionError(f"0x65db {name}: SS value decoy changed")
        for segment, offset, expected in immutable:
            actual = bytes(machine.mem_read(segment * 16 + offset, len(expected)))
            if actual != expected:
                raise AssertionError(f"0x65db {name}: script input changed")
        for byte_index in range(len(script)):
            offset = start + byte_index
            if machine.mem_read(0x4800 * 16 + offset, 1) != b"\x5a":
                raise AssertionError(f"0x65db {name}: ES script decoy changed")
            if machine.mem_read(game_segment * 16 + offset, 1) != b"\xa5":
                raise AssertionError(f"0x65db {name}: GS script decoy changed")

        flags = machine.reg_read(UC_X86_REG_EFLAGS)
        flag_masks = {
            "cf": 0x0001,
            "pf": 0x0004,
            "af": 0x0010,
            "zf": 0x0040,
            "sf": 0x0080,
            "of": 0x0800,
        }
        expected_flags = {
            flag: bool(initial["flags"] & mask) for flag, mask in flag_masks.items()
        }
        actual_flags = {
            flag: bool(flags & mask) for flag, mask in flag_masks.items()
        }
        if actual_flags != expected_flags:
            raise AssertionError(
                f"0x65db {name}: flags={actual_flags}, expected={expected_flags}"
            )
        if EXE[0x65EA] != 0xC3:
            raise AssertionError("0x65db: expected near RET boundary")

        vectors.append(
            {
                "name": name,
                "start_offset": start,
                "target_offset": target,
                "resume_state_before": resume_state,
                "resume_value_before": resume_value,
                "defined_flags": expected_flags,
            }
        )

    return vectors


def vm_cond_state_array_vectors() -> list[dict[str, object]]:
    data_segment = 0x4400
    game_segment = 0x2C00
    stack_segment = 0x9000
    cases = [
        {
            "name": "assign_first_word",
            "start": 0x1000,
            "index": 0x00,
            "query": 0x00,
            "state": 0xA55A,
            "operand": 0x1234,
            "top": 4,
            "target": 0x7100,
        },
        {
            "name": "assign_high_positive_index",
            "start": 0x1101,
            "index": 0x7F,
            "query": 0xFE,
            "state": 0x0000,
            "operand": 0xBEEF,
            "top": 6,
            "target": 0x7211,
        },
        {
            "name": "assign_signed_minus_one",
            "start": 0x1200,
            "index": 0xFF,
            "query": 0x80,
            "state": 0x1357,
            "operand": 0x8001,
            "top": 2,
            "target": 0x7322,
        },
        {
            "name": "assign_operand_crosses_segment_end",
            "start": 0xFFFE,
            "index": 0x80,
            "query": 0x02,
            "state": 0x2468,
            "operand": 0xCAFE,
            "top": 8,
            "target": 0x7433,
        },
        {
            "name": "query_zero_continues",
            "start": 0x1300,
            "index": 0x03,
            "query": 0x01,
            "state": 0x0000,
            "operand": 0x5AA5,
            "top": 4,
            "target": 0x7544,
        },
        {
            "name": "query_signed_negative_zero_continues",
            "start": 0x1401,
            "index": 0xFE,
            "query": 0xFF,
            "state": 0x0000,
            "operand": 0xA55A,
            "top": 6,
            "target": 0x7655,
        },
        {
            "name": "query_nonzero_branches",
            "start": 0x1500,
            "index": 0x05,
            "query": 0x03,
            "state": 0x8000,
            "operand": 0x9696,
            "top": 4,
            "target": 0x7766,
        },
        {
            "name": "query_signed_minimum_odd_stack_branches",
            "start": 0x1600,
            "index": 0x80,
            "query": 0xA5,
            "state": 0x0001,
            "operand": 0x6969,
            "top": 3,
            "target": 0x7877,
        },
    ]
    vectors = []

    for case in cases:
        name = str(case["name"])
        start = int(case["start"])
        index_byte = int(case["index"])
        query_mode = int(case["query"])
        state_before = int(case["state"])
        operand = int(case["operand"])
        top = int(case["top"])
        target = int(case["target"])
        signed_index = index_byte if index_byte < 0x80 else index_byte - 0x100
        scaled_index = (signed_index * 2) & 0xFFFF
        state_offset = (0x6ADE + scaled_index) & 0xFFFF
        query_path = bool(query_mode & 1)
        branch_taken = query_path and state_before != 0
        final_script = (start + (1 if query_path else 3)) & 0xFFFF
        final_top = (top - 2) & 0xFFFF if branch_taken else top
        branch_offset = (0x6820 + ((top - 2) & 0xFFFF)) & 0xFFFF
        script = bytes([index_byte]) + struct.pack("<H", operand)
        state_bytes = struct.pack("<H", state_before)
        state_after = state_before if query_path else operand
        data_state_decoy = struct.pack("<H", state_before ^ 0xFFFF)
        game_state_decoy = struct.pack("<H", state_before ^ 0x5A5A)
        data_query_decoy = query_mode ^ 0xFF
        stack_query_decoy = query_mode ^ 0xA5
        target_bytes = struct.pack("<H", target)
        memory = [
            (game_segment, 0x67AD, bytes([query_mode])),
            (data_segment, 0x67AD, bytes([data_query_decoy])),
            (stack_segment, 0x67AD, bytes([stack_query_decoy])),
            (game_segment, 0x6884, struct.pack("<H", top)),
            (stack_segment, state_offset, state_bytes),
            (data_segment, state_offset, data_state_decoy),
            (game_segment, state_offset, game_state_decoy),
            (stack_segment, branch_offset, target_bytes),
            (data_segment, branch_offset, struct.pack("<H", target ^ 0xFFFF)),
            (game_segment, branch_offset, struct.pack("<H", target ^ 0x5A5A)),
        ]
        immutable = []
        for byte_index, byte in enumerate(script):
            offset = start + byte_index
            encoded = bytes([byte])
            memory.append((data_segment, offset, encoded))
            immutable.append((data_segment, offset, encoded))
            memory.append((0x4800, offset, b"\x5a"))
            memory.append((game_segment, offset, b"\xa5"))

        initial = {
            "eax": 0xA1A1BEEF,
            "ebx": 0xB2B22345,
            "ecx": 0xC3C33456,
            "edx": 0xD4D44567,
            "esi": 0xE5E50000 | start,
            "edi": 0xF6F66789,
            "ebp": 0x9797789A,
            "sp": 0xFF00,
            "ds": data_segment,
            "es": 0x4800,
            "fs": 0x4C00,
            "gs": game_segment,
            "ss": stack_segment,
            "flags": 0x0AD7,
        }
        branch_calls = []
        phases = []

        def capture_path(machine: Uc, address: int, _size: int) -> None:
            if address == 0x65ED:
                phases.append(
                    (
                        "sign_extended",
                        machine.reg_read(UC_X86_REG_AX),
                        machine.reg_read(UC_X86_REG_SI),
                    )
                )
            elif address == 0x6462:
                branch_calls.append(
                    (
                        machine.reg_read(UC_X86_REG_AX),
                        machine.reg_read(UC_X86_REG_BP),
                        machine.reg_read(UC_X86_REG_SI),
                    )
                )

        machine = execute(
            0x65EB,
            0x660B,
            initial,
            memory,
            code_handler=capture_path,
        )

        sign_extended = signed_index & 0xFFFF
        expected_phases = [("sign_extended", sign_extended, (start + 1) & 0xFFFF)]
        if phases != expected_phases:
            raise AssertionError(
                f"0x65eb {name}: phases={phases}, expected={expected_phases}"
            )
        expected_branch_calls = (
            [(scaled_index, scaled_index, (start + 1) & 0xFFFF)]
            if branch_taken
            else []
        )
        if branch_calls != expected_branch_calls:
            raise AssertionError(
                f"0x65eb {name}: branch calls={branch_calls}, "
                f"expected={expected_branch_calls}"
            )

        expected_registers = dict(initial)
        del expected_registers["flags"]
        expected_ax = final_top if branch_taken else (scaled_index if query_path else operand)
        expected_registers["eax"] = (initial["eax"] & 0xFFFF0000) | expected_ax
        expected_registers["esi"] = (
            initial["esi"] & 0xFFFF0000
        ) | (target if branch_taken else final_script)
        expected_registers["ebp"] = (initial["ebp"] & 0xFFFF0000) | scaled_index
        for register, expected in expected_registers.items():
            actual = machine.reg_read(REGISTERS[register])
            if actual != expected:
                raise AssertionError(
                    f"0x65eb {name}: {register}={actual:#x}, expected={expected:#x}"
                )

        actual_state = struct.unpack(
            "<H", machine.mem_read(stack_segment * 16 + state_offset, 2)
        )[0]
        if actual_state != state_after:
            raise AssertionError(
                f"0x65eb {name}: state={actual_state:#x}, expected={state_after:#x}"
            )
        if machine.mem_read(data_segment * 16 + state_offset, 2) != data_state_decoy:
            raise AssertionError(f"0x65eb {name}: DS state decoy changed")
        if machine.mem_read(game_segment * 16 + state_offset, 2) != game_state_decoy:
            raise AssertionError(f"0x65eb {name}: GS state decoy changed")
        expected_query = 0 if branch_taken else query_mode
        actual_query = machine.mem_read(game_segment * 16 + 0x67AD, 1)[0]
        if actual_query != expected_query:
            raise AssertionError(
                f"0x65eb {name}: query={actual_query:#x}, expected={expected_query:#x}"
            )
        if machine.mem_read(data_segment * 16 + 0x67AD, 1)[0] != data_query_decoy:
            raise AssertionError(f"0x65eb {name}: DS query decoy changed")
        if machine.mem_read(stack_segment * 16 + 0x67AD, 1)[0] != stack_query_decoy:
            raise AssertionError(f"0x65eb {name}: SS query decoy changed")
        actual_top = struct.unpack(
            "<H", machine.mem_read(game_segment * 16 + 0x6884, 2)
        )[0]
        if actual_top != final_top:
            raise AssertionError(
                f"0x65eb {name}: top={actual_top:#x}, expected={final_top:#x}"
            )
        if machine.mem_read(stack_segment * 16 + branch_offset, 2) != target_bytes:
            raise AssertionError(f"0x65eb {name}: branch target changed")
        for segment, offset, expected in immutable:
            actual = bytes(machine.mem_read(segment * 16 + offset, len(expected)))
            if actual != expected:
                raise AssertionError(f"0x65eb {name}: script input changed")

        flags = machine.reg_read(UC_X86_REG_EFLAGS)
        if branch_taken:
            expected_flags = {
                "cf": top < 2,
                "pf": (final_top & 0xFF).bit_count() % 2 == 0,
                "af": (top & 0x0F) < 2,
                "zf": final_top == 0,
                "sf": bool(final_top & 0x8000),
                "of": bool(((top ^ 2) & (top ^ final_top)) & 0x8000),
            }
        else:
            tested = state_before if query_path else 0
            expected_flags = {
                "cf": False,
                "pf": (tested & 0xFF).bit_count() % 2 == 0,
                "zf": tested == 0,
                "sf": bool(tested & 0x8000),
                "of": False,
            }
        flag_masks = {
            "cf": 0x0001,
            "pf": 0x0004,
            "af": 0x0010,
            "zf": 0x0040,
            "sf": 0x0080,
            "of": 0x0800,
        }
        actual_flags = {
            flag: bool(flags & flag_masks[flag]) for flag in expected_flags
        }
        if actual_flags != expected_flags:
            raise AssertionError(
                f"0x65eb {name}: flags={actual_flags}, expected={expected_flags}"
            )
        if EXE[0x660B] != 0xC3:
            raise AssertionError("0x65eb: expected near RET boundary")

        vectors.append(
            {
                "name": name,
                "start_offset": start,
                "signed_index": signed_index,
                "state_offset": state_offset,
                "query_mode_before": query_mode,
                "state_before": state_before,
                "operand_word": None if query_path else operand,
                "branch_taken": branch_taken,
                "final_script_offset": target if branch_taken else final_script,
                "final_state": state_after,
                "defined_flags": expected_flags,
            }
        )

    return vectors


def vm_text_handler_vectors() -> list[dict[str, object]]:
    data_segment = 0x4400
    game_segment = 0x2C00
    line_segment = 0x5000
    dictionary_segment = 0x6000
    stack_segment = 0x9000
    line_base = 0x0100
    line_index = 0x0020
    line_offset = line_base + line_index
    cases = [
        {
            "name": "inactive_still_arms_skip_and_loop",
            "start": 0x1800,
            "selector": 0x7F,
            "b4": 0x18,
            "b5": 0x70,
            "loop_target": 0x3333,
            "words": [0x0100, 0x0200],
            "path": "inactive",
        },
        {
            "name": "display_active_gate",
            "start": 0x1900,
            "selector": 0x01,
            "b4": 0x00,
            "b5": 0x80,
            "words": [0x0100],
            "display_active": 1,
            "path": "display_gate",
        },
        {
            "name": "presentation_defer_gate",
            "start": 0x1A00,
            "selector": 0x02,
            "b4": 0x00,
            "b5": 0x80,
            "words": [0x0100],
            "defer": 0x80,
            "path": "defer_gate",
        },
        {
            "name": "already_shown_gate",
            "start": 0x1B00,
            "selector": 0x03,
            "b4": 0x00,
            "b5": 0x80,
            "words": [0x0100],
            "line_flags": 0x8000,
            "path": "shown",
        },
        {
            "name": "wrong_presentation_record_gate",
            "start": 0x1C00,
            "selector": 0x04,
            "b4": 0x00,
            "b5": 0x80,
            "words": [0x0100],
            "presentation_record": 0x00C5,
            "path": "wrong_record",
        },
        {
            "name": "random_condition_rejects",
            "start": 0x1D00,
            "selector": 0x05,
            "b4": 0x02,
            "b5": 0x80,
            "words": [0x0100, 0x0200],
            "prng_result": 1,
            "path": "random_reject",
        },
        {
            "name": "accepted_raw_word_list",
            "start": 0x1E00,
            "selector": 0xFF,
            "b4": 0x00,
            "b5": 0x80,
            "words": [0x0100, 0x0200],
            "path": "raw",
        },
        {
            "name": "accepted_preserved_with_extra_control",
            "start": 0x1F00,
            "selector": 0x80,
            "b4": 0x05,
            "b5": 0x80,
            "extra_control": 5,
            "condition_value": 10,
            "words": [0x0100],
            "path": "raw_extra",
        },
        {
            "name": "assembled_punctuation_spacing",
            "start": 0x2000,
            "selector": 0x06,
            "b4": 0x20,
            "b5": 0x80,
            "words": [0x0100, 0x0110, 0x0120],
            "dictionary": {
                0x0000: b"\0",
                0x0100: b"HELLO\0",
                0x0110: b",\0",
                0x0120: b"WORLD\0",
            },
            "output": b"HELLO, WORLD \r\0",
            "line_length": 13,
            "path": "assembled",
        },
        {
            "name": "assembled_wraps_before_next_word",
            "start": 0x2100,
            "selector": 0x07,
            "b4": 0x20,
            "b5": 0x80,
            "words": [0x0200, 0x0220],
            "dictionary": {
                0x0000: b"\0",
                0x0200: b"12345678901234567890\0",
                0x0220: b"abcdefghijklmnop\0",
            },
            "output": b"12345678901234567890 \rabcdefghijklmnop \r\0",
            "line_length": 17,
            "path": "assembled",
        },
        {
            "name": "assembled_stops_at_menu_separator",
            "start": 0x2200,
            "selector": 0x08,
            "b4": 0x20,
            "b5": 0x80,
            "words": [0x0300, 0xFFFF, 0x0320],
            "dictionary": {
                0x0000: b"\0",
                0x0300: b"CHOICE\0",
                0x0320: b"MENU\0",
                0xFFFF: b"\0",
            },
            "output": b"CHOICE \r\0",
            "line_length": 7,
            "path": "assembled",
        },
    ]
    vectors = []

    for case in cases:
        name = str(case["name"])
        start = int(case["start"])
        selector = int(case["selector"])
        b4 = int(case["b4"])
        b5 = int(case["b5"])
        control = b4 | (b5 << 8)
        path = str(case["path"])
        loop_target = case.get("loop_target")
        extra_control = case.get("extra_control")
        words = [int(word) for word in case["words"]]
        script = bytearray(struct.pack("<HBBB", line_index, selector, b4, b5))
        condition_offset = start + len(script)
        if loop_target is not None:
            script.extend(struct.pack("<H", int(loop_target)))
            condition_offset += 2
        if extra_control is not None:
            script.extend(struct.pack("<H", int(extra_control)))
        words_offset = condition_offset + (2 if extra_control is not None else 0)
        for word in words:
            script.extend(struct.pack("<H", word))
        script.extend(b"\0\0")
        final_script = (start + len(script)) & 0xFFFF

        display_active = int(case.get("display_active", 0))
        defer = int(case.get("defer", 0))
        line_flags = int(case.get("line_flags", 0x1234))
        presentation_record = int(case.get("presentation_record", 0x00C4))
        word_list_mode = 0xA4
        initial_skip = 0x55
        initial_resume_state = 0xA5
        initial_resume_value = 0x5678
        initial_loop_target = 0x9ABC
        initial_selector = 0x2468
        initial_yield = 0x40
        initial_request = 0xA0
        initial_hold = 0x67
        initial_menu_pending = 0x33
        initial_menu_end = 0x4444
        initial_menu_words = (0x5555, 0x6666)
        initial_reveal = 0x7777
        initial_mode_cf9 = 0x22
        initial_mode_cfa = 0x44
        initial_voice = 0x66

        memory = [
            (0, 0x27E2, b"\xb8" + struct.pack("<H", int(case.get("prng_result", 0))) + b"\xcb"),
            (game_segment, 0x6724, struct.pack("<HH", line_base, line_segment)),
            (game_segment, 0x6728, struct.pack("<HH", 0, dictionary_segment)),
            (game_segment, 0x6E91, b"\x3a"),
            (game_segment, 0x6D71, b"\x20"),
            (line_segment, line_offset + 2, struct.pack("<H", line_flags)),
            (line_segment, line_offset + 0x3A, struct.pack("<H", presentation_record)),
            (line_segment, line_offset + 0x20, struct.pack("<H", int(case.get("condition_value", 10)))),
            (game_segment, 0x67AB, bytes([initial_skip])),
            (game_segment, 0x67B1, bytes([initial_resume_state])),
            (game_segment, 0x6764, struct.pack("<H", initial_resume_value)),
            (game_segment, 0x6778, struct.pack("<H", initial_loop_target)),
            (game_segment, 0x677C, struct.pack("<H", 0xAAAA)),
            (game_segment, 0x5E64, bytes([display_active])),
            (game_segment, 0x67B0, bytes([defer])),
            (game_segment, 0x67B9, bytes([word_list_mode])),
            (game_segment, 0x1FAB, struct.pack("<H", initial_selector)),
            (game_segment, 0x0CF9, bytes([initial_mode_cf9])),
            (game_segment, 0x0CFA, bytes([initial_mode_cfa])),
            (game_segment, 0x0CFB, bytes([initial_voice])),
            (game_segment, 0x0E18, b"\xcc" * 128),
            (game_segment, 0x5E58, struct.pack("<H", initial_reveal)),
            (game_segment, 0x67B4, bytes([initial_yield])),
            (game_segment, 0x67BC, bytes([initial_hold])),
            (game_segment, 0x67AA, bytes([initial_request])),
            (game_segment, 0x1FB3, bytes([initial_menu_pending])),
            (game_segment, 0x27D3, struct.pack("<H", initial_menu_end)),
            (game_segment, 0x674A, struct.pack("<HH", *initial_menu_words)),
            (game_segment, 0x27CF, struct.pack("<H", 0x8888)),
            (data_segment, 0x67AB, b"\xde"),
            (stack_segment, 0x67AB, b"\xad"),
        ]
        immutable = []
        for byte_index, byte in enumerate(script):
            offset = start + byte_index
            encoded = bytes([byte])
            memory.append((data_segment, offset, encoded))
            immutable.append((data_segment, offset, encoded))
        dictionary = case.get("dictionary", {0: b"\0"})
        for offset, encoded in dictionary.items():
            memory.append((dictionary_segment, int(offset), bytes(encoded)))

        initial = {
            "eax": 0xA1A1BEEF,
            "ebx": 0xB2B22345,
            "ecx": 0xC3C33456,
            "edx": 0xD4D44567,
            "esi": 0xE5E50000 | start,
            "edi": 0xF6F66789,
            "ebp": 0x9797789A,
            "sp": 0xFF00,
            "ds": data_segment,
            "es": 0x4800,
            "fs": 0x4C00,
            "gs": game_segment,
            "ss": stack_segment,
            "flags": 0x0AD7,
        }
        calls = {"condition": [], "scan": [], "strlen": []}

        def capture_calls(machine: Uc, address: int, _size: int) -> None:
            if address == 0x6339:
                calls["condition"].append(
                    (
                        machine.reg_read(UC_X86_REG_CX),
                        machine.reg_read(UC_X86_REG_ES),
                        machine.reg_read(UC_X86_REG_DI),
                        machine.reg_read(UC_X86_REG_SI),
                    )
                )
            elif address == 0x647B:
                calls["scan"].append(machine.reg_read(UC_X86_REG_SI))
            elif address == 0x67A7:
                calls["strlen"].append(
                    (
                        machine.reg_read(UC_X86_REG_ES),
                        machine.reg_read(UC_X86_REG_DI),
                    )
                )

        machine = execute(
            0x660C,
            0x67A6,
            initial,
            memory,
            code_handler=capture_calls,
            instruction_count=50000,
        )

        condition_expected = path not in {
            "inactive",
            "display_gate",
            "defer_gate",
            "shown",
            "wrong_record",
        }
        expected_condition_calls = (
            [(control, line_segment, line_offset, condition_offset)]
            if condition_expected
            else []
        )
        if calls["condition"] != expected_condition_calls:
            raise AssertionError(
                f"0x660c {name}: condition calls={calls['condition']}, "
                f"expected={expected_condition_calls}"
            )
        raw_path = path in {"raw", "raw_extra"}
        expected_scan_calls = [words_offset] if raw_path else []
        if calls["scan"] != expected_scan_calls:
            raise AssertionError(
                f"0x660c {name}: scan calls={calls['scan']}, "
                f"expected={expected_scan_calls}"
            )
        expected_strlen = []
        if path == "assembled":
            separator = words.index(0xFFFF) if 0xFFFF in words else len(words)
            expected_strlen = [
                (dictionary_segment, words[index + 1] if index + 1 < len(words) else 0)
                for index in range(separator)
            ]
        if calls["strlen"] != expected_strlen:
            raise AssertionError(
                f"0x660c {name}: strlen calls={calls['strlen']}, "
                f"expected={expected_strlen}"
            )

        accepted = path in {"raw", "raw_extra", "assembled"}
        expected_b5 = b5
        if accepted and not (b4 & 1):
            expected_b5 &= 0x7F
        actual_b5 = machine.mem_read(data_segment * 16 + start + 4, 1)[0]
        if actual_b5 != expected_b5:
            raise AssertionError(
                f"0x660c {name}: b5={actual_b5:#x}, expected={expected_b5:#x}"
            )
        for segment, offset, expected in immutable:
            if offset == start + 4 and accepted and not (b4 & 1):
                continue
            actual = bytes(machine.mem_read(segment * 16 + offset, len(expected)))
            if actual != expected:
                raise AssertionError(f"0x660c {name}: script input changed")

        actual_flags_word = struct.unpack(
            "<H", machine.mem_read(line_segment * 16 + line_offset + 2, 2)
        )[0]
        expected_flags_word = line_flags | (0x8000 if accepted else 0)
        if actual_flags_word != expected_flags_word:
            raise AssertionError(
                f"0x660c {name}: line flags={actual_flags_word:#x}, "
                f"expected={expected_flags_word:#x}"
            )
        actual_selector_ptr = struct.unpack(
            "<H", machine.mem_read(game_segment * 16 + 0x677C, 2)
        )[0]
        if actual_selector_ptr != (start + 2) & 0xFFFF:
            raise AssertionError(f"0x660c {name}: selector pointer is wrong")
        actual_selector = struct.unpack(
            "<H", machine.mem_read(game_segment * 16 + 0x1FAB, 2)
        )[0]
        expected_selector = (
            selector if selector < 0x80 else selector | 0xFF00
        ) if accepted else initial_selector
        if actual_selector != expected_selector:
            raise AssertionError(
                f"0x660c {name}: selector={actual_selector:#x}, "
                f"expected={expected_selector:#x}"
            )

        expected_skip = (
            ((b5 >> 4) & 7) + 1 if b4 & 8 else initial_skip
        )
        if machine.mem_read(game_segment * 16 + 0x67AB, 1)[0] != expected_skip:
            raise AssertionError(f"0x660c {name}: skip count is wrong")
        expected_resume_state = 1 if b4 & 0x10 else initial_resume_state
        expected_resume_value = 0 if b4 & 0x10 else initial_resume_value
        expected_loop_target = int(loop_target) if loop_target is not None else initial_loop_target
        if machine.mem_read(game_segment * 16 + 0x67B1, 1)[0] != expected_resume_state:
            raise AssertionError(f"0x660c {name}: resume state is wrong")
        actual_resume_value = struct.unpack(
            "<H", machine.mem_read(game_segment * 16 + 0x6764, 2)
        )[0]
        if actual_resume_value != expected_resume_value:
            raise AssertionError(f"0x660c {name}: resume value is wrong")
        actual_loop_target = struct.unpack(
            "<H", machine.mem_read(game_segment * 16 + 0x6778, 2)
        )[0]
        if actual_loop_target != expected_loop_target:
            raise AssertionError(f"0x660c {name}: loop target is wrong")
        if machine.mem_read(data_segment * 16 + 0x67AB, 1) != b"\xde":
            raise AssertionError(f"0x660c {name}: DS global decoy changed")
        if machine.mem_read(stack_segment * 16 + 0x67AB, 1) != b"\xad":
            raise AssertionError(f"0x660c {name}: SS global decoy changed")

        if raw_path:
            expected_count = len(words)
            actual_count = struct.unpack(
                "<H", machine.mem_read(game_segment * 16 + 0x27CF, 2)
            )[0]
            if actual_count != expected_count:
                raise AssertionError(
                    f"0x660c {name}: word count={actual_count}, "
                    f"expected={expected_count}"
                )
            actual_menu_end = struct.unpack(
                "<H", machine.mem_read(game_segment * 16 + 0x27D3, 2)
            )[0]
            actual_menu_words = struct.unpack(
                "<HH", machine.mem_read(game_segment * 16 + 0x674A, 4)
            )
            if actual_menu_end != words_offset or actual_menu_words != (
                words_offset,
                data_segment,
            ):
                raise AssertionError(f"0x660c {name}: menu pointers are wrong")
            expected_raw = {
                0x5E64: 0,
                0x0CF9: 1,
                0x67AA: initial_request | 1,
                0x67B4: (initial_yield + 2) & 0xFF,
                0x67B0: 1,
                0x67BC: 0,
                0x1FB3: 1,
            }
            for offset, expected in expected_raw.items():
                actual = machine.mem_read(game_segment * 16 + offset, 1)[0]
                if actual != expected:
                    raise AssertionError(
                        f"0x660c {name}: {offset:#x}={actual:#x}, expected={expected:#x}"
                    )
        elif path == "assembled":
            expected_output = bytes(case["output"])
            actual_output = bytes(
                machine.mem_read(game_segment * 16 + 0x0E18, len(expected_output))
            )
            if actual_output != expected_output:
                raise AssertionError(
                    f"0x660c {name}: output={actual_output!r}, "
                    f"expected={expected_output!r}"
                )
            expected_assembled = {
                0x0CFB: 1,
                0x0CFA: 0,
                0x67B0: 0,
                0x67B9: 0,
                0x5E64: 1,
                0x67B4: (initial_yield + 2) & 0xFF,
                0x67BC: 0,
                0x67AA: initial_request | 1,
            }
            for offset, expected in expected_assembled.items():
                actual = machine.mem_read(game_segment * 16 + offset, 1)[0]
                if actual != expected:
                    raise AssertionError(
                        f"0x660c {name}: {offset:#x}={actual:#x}, expected={expected:#x}"
                    )
            actual_reveal = struct.unpack(
                "<H", machine.mem_read(game_segment * 16 + 0x5E58, 2)
            )[0]
            if actual_reveal != 0:
                raise AssertionError(f"0x660c {name}: reveal cursor was not cleared")

        expected_registers = dict(initial)
        del expected_registers["flags"]
        expected_registers["eax"] = initial["eax"] & 0xFFFF0000
        expected_registers["ecx"] = (initial["ecx"] & 0xFFFF0000) | control
        expected_registers["esi"] = (initial["esi"] & 0xFFFF0000) | final_script
        expected_registers["es"] = game_segment if path == "assembled" else line_segment
        if path in {"inactive", "display_gate", "defer_gate", "shown"}:
            expected_bx = initial["ebx"] & 0xFFFF
            expected_dx = initial["edx"] & 0xFFFF
        elif path == "wrong_record":
            expected_bx = 0x003A
            expected_dx = presentation_record
        elif path == "raw_extra":
            expected_bx = 0x003A
            expected_dx = int(case["condition_value"])
        elif path == "assembled":
            expected_bx = dictionary_segment
            expected_dx = int(case["line_length"])
        else:
            expected_bx = 0x003A
            expected_dx = 0x00C4
        expected_registers["ebx"] = (initial["ebx"] & 0xFFFF0000) | expected_bx
        expected_registers["edx"] = (initial["edx"] & 0xFFFF0000) | expected_dx
        for register, expected in expected_registers.items():
            actual = machine.reg_read(REGISTERS[register])
            if actual != expected:
                raise AssertionError(
                    f"0x660c {name}: {register}={actual:#x}, expected={expected:#x}"
                )

        flags = machine.reg_read(UC_X86_REG_EFLAGS)
        expected_flags = {
            "cf": False,
            "pf": True,
            "zf": True,
            "sf": False,
            "of": False,
        }
        flag_masks = {
            "cf": 0x0001,
            "pf": 0x0004,
            "zf": 0x0040,
            "sf": 0x0080,
            "of": 0x0800,
        }
        actual_flags = {
            flag: bool(flags & flag_masks[flag]) for flag in expected_flags
        }
        if actual_flags != expected_flags:
            raise AssertionError(
                f"0x660c {name}: flags={actual_flags}, expected={expected_flags}"
            )
        if EXE[0x67A6] != 0xC3:
            raise AssertionError("0x660c: expected near RET boundary")

        vectors.append(
            {
                "name": name,
                "path": path,
                "selector": selector if selector < 0x80 else selector - 0x100,
                "control_word": control,
                "word_list_offset": words_offset,
                "final_script_offset": final_script,
                "line_flags_before": line_flags,
                "line_flags_after": expected_flags_word,
                "token_b5_after": expected_b5,
                "condition_called": condition_expected,
                "defined_flags": expected_flags,
            }
        )

    return vectors


def strlen_b_vectors() -> list[dict[str, object]]:
    text_segment = 0x7000
    data_segment = 0x4400
    game_segment = 0x2C00
    cases = [
        ("empty", 0x1000, b"", True),
        ("one_byte", 0x1100, b"A", True),
        ("punctuation", 0x1201, b"Commander!", True),
        ("high_bytes", 0x1300, bytes([0x80, 0xFE, 0xFF]), True),
        ("wraps_segment_offset", 0xFFFD, b"WRAP", True),
        ("length_254", 0x2000, b"X" * 254, True),
        ("maximum_scannable_length", 0x0000, b"Y" * 0xFFFE, True),
        ("unterminated_scan_bound", 0x0000, b"Z" * 0x10000, False),
    ]
    vectors = []

    for name, start, payload, terminated in cases:
        memory = []
        encoded = payload + (b"\0" if terminated else b"")
        for byte_index, byte in enumerate(encoded):
            memory.append(
                (text_segment, (start + byte_index) & 0xFFFF, bytes([byte]))
            )
        if len(encoded) < 0x1000:
            for byte_index in range(len(encoded)):
                offset = (start + byte_index) & 0xFFFF
                memory.append((data_segment, offset, b"\x5a"))
                memory.append((game_segment, offset, b"\xa5"))

        initial = {
            "eax": 0xA1A1BEEF,
            "ebx": 0xB2B22345,
            "ecx": 0xC3C33456,
            "edx": 0xD4D44567,
            "esi": 0xE5E55678,
            "edi": 0xF6F60000 | start,
            "ebp": 0x9797789A,
            "sp": 0xFF00,
            "ds": data_segment,
            "es": text_segment,
            "fs": 0x4C00,
            "gs": game_segment,
            "ss": 0x9000,
            "flags": 0x0AD7,
        }
        machine = execute(
            0x67A7,
            0x67B9,
            initial,
            memory,
            instruction_count=100000,
        )

        result = len(payload) if terminated else 0xFFFE
        expected_registers = dict(initial)
        del expected_registers["flags"]
        expected_registers["eax"] = (initial["eax"] & 0xFFFF0000) | result
        for register, expected in expected_registers.items():
            actual = machine.reg_read(REGISTERS[register])
            if actual != expected:
                raise AssertionError(
                    f"0x67a7 {name}: {register}={actual:#x}, expected={expected:#x}"
                )

        for byte_index, byte in enumerate(encoded):
            offset = (start + byte_index) & 0xFFFF
            actual = machine.mem_read(text_segment * 16 + offset, 1)[0]
            if actual != byte:
                raise AssertionError(f"0x67a7 {name}: input string changed")
        if len(encoded) < 0x1000:
            for byte_index in range(len(encoded)):
                offset = (start + byte_index) & 0xFFFF
                if machine.mem_read(data_segment * 16 + offset, 1) != b"\x5a":
                    raise AssertionError(f"0x67a7 {name}: DS decoy changed")
                if machine.mem_read(game_segment * 16 + offset, 1) != b"\xa5":
                    raise AssertionError(f"0x67a7 {name}: GS decoy changed")

        before_sub = (result + 2) & 0xFFFF
        flags = machine.reg_read(UC_X86_REG_EFLAGS)
        expected_flags = {
            "cf": before_sub < 2,
            "pf": (result & 0xFF).bit_count() % 2 == 0,
            "af": (before_sub & 0x0F) < 2,
            "zf": result == 0,
            "sf": bool(result & 0x8000),
            "of": bool(((before_sub ^ 2) & (before_sub ^ result)) & 0x8000),
        }
        flag_masks = {
            "cf": 0x0001,
            "pf": 0x0004,
            "af": 0x0010,
            "zf": 0x0040,
            "sf": 0x0080,
            "of": 0x0800,
        }
        actual_flags = {
            flag: bool(flags & flag_masks[flag]) for flag in expected_flags
        }
        if actual_flags != expected_flags:
            raise AssertionError(
                f"0x67a7 {name}: flags={actual_flags}, expected={expected_flags}"
            )
        if EXE[0x67B9] != 0xC3:
            raise AssertionError("0x67a7: expected near RET boundary")

        vectors.append(
            {
                "name": name,
                "start_offset": start,
                "terminated": terminated,
                "payload_length": len(payload),
                "return_length": result,
                "defined_flags": expected_flags,
            }
        )

    return vectors


def vm_presentation_register_set_vectors() -> list[dict[str, object]]:
    data_segment = 0x4400
    game_segment = 0x2C00
    stack_segment = 0x9000
    cases = [
        ("inactive_zero", 0x2400, 0x0000, 0x00, 0xA55A),
        ("inactive_unrelated_bit", 0x2501, 0x1234, 0x02, 0x5AA5),
        ("active_bit_zero", 0x2600, 0xBEEF, 0x01, 0x0000),
        ("active_all_bits", 0x2701, 0x8001, 0xFF, 0xFFFF),
        ("active_zero_operand", 0x2800, 0x0000, 0x81, 0x1357),
        ("operand_crosses_segment_end", 0xFFFF, 0xCAFE, 0x03, 0x2468),
    ]
    vectors = []

    for name, start, operand, active, register_before in cases:
        script = struct.pack("<H", operand)
        active_path = bool(active & 1)
        register_after = operand if active_path else register_before
        data_active_decoy = active ^ 0xFF
        stack_active_decoy = active ^ 0xA5
        data_register_decoy = register_before ^ 0xFFFF
        stack_register_decoy = register_before ^ 0x5A5A
        memory = [
            (game_segment, 0x67AC, bytes([active])),
            (game_segment, 0x6770, struct.pack("<H", register_before)),
            (data_segment, 0x67AC, bytes([data_active_decoy])),
            (stack_segment, 0x67AC, bytes([stack_active_decoy])),
            (data_segment, 0x6770, struct.pack("<H", data_register_decoy)),
            (stack_segment, 0x6770, struct.pack("<H", stack_register_decoy)),
        ]
        immutable = []
        for byte_index, byte in enumerate(script):
            offset = start + byte_index
            encoded = bytes([byte])
            memory.append((data_segment, offset, encoded))
            immutable.append((data_segment, offset, encoded))
            memory.append((0x4800, offset, b"\x5a"))
            memory.append((game_segment, offset, b"\xa5"))

        initial = {
            "eax": 0xA1A1BEEF,
            "ebx": 0xB2B22345,
            "ecx": 0xC3C33456,
            "edx": 0xD4D44567,
            "esi": 0xE5E50000 | start,
            "edi": 0xF6F66789,
            "ebp": 0x9797789A,
            "sp": 0xFF00,
            "ds": data_segment,
            "es": 0x4800,
            "fs": 0x4C00,
            "gs": game_segment,
            "ss": stack_segment,
            "flags": 0x0AD7,
        }
        phases = []

        def capture_phases(machine: Uc, address: int, _size: int) -> None:
            if address not in (0x67BB, 0x67C3):
                return
            phases.append(
                (
                    address,
                    machine.reg_read(UC_X86_REG_AX),
                    machine.reg_read(UC_X86_REG_SI),
                    struct.unpack(
                        "<H", machine.mem_read(game_segment * 16 + 0x6770, 2)
                    )[0],
                )
            )

        machine = execute(
            0x67BA,
            0x67C7,
            initial,
            memory,
            code_handler=capture_phases,
        )

        final_script = (start + 2) & 0xFFFF
        expected_phases = [(0x67BB, operand, final_script, register_before)]
        if active_path:
            expected_phases.append(
                (0x67C3, operand, final_script, register_before)
            )
        if phases != expected_phases:
            raise AssertionError(
                f"0x67ba {name}: phases={phases}, expected={expected_phases}"
            )

        expected_registers = dict(initial)
        del expected_registers["flags"]
        expected_registers["eax"] = (initial["eax"] & 0xFFFF0000) | operand
        expected_registers["esi"] = (initial["esi"] & 0xFFFF0000) | final_script
        for register, expected in expected_registers.items():
            actual = machine.reg_read(REGISTERS[register])
            if actual != expected:
                raise AssertionError(
                    f"0x67ba {name}: {register}={actual:#x}, expected={expected:#x}"
                )

        actual_register = struct.unpack(
            "<H", machine.mem_read(game_segment * 16 + 0x6770, 2)
        )[0]
        if actual_register != register_after:
            raise AssertionError(
                f"0x67ba {name}: register={actual_register:#x}, "
                f"expected={register_after:#x}"
            )
        if machine.mem_read(data_segment * 16 + 0x67AC, 1)[0] != data_active_decoy:
            raise AssertionError(f"0x67ba {name}: DS active decoy changed")
        if machine.mem_read(stack_segment * 16 + 0x67AC, 1)[0] != stack_active_decoy:
            raise AssertionError(f"0x67ba {name}: SS active decoy changed")
        if struct.unpack(
            "<H", machine.mem_read(data_segment * 16 + 0x6770, 2)
        )[0] != data_register_decoy:
            raise AssertionError(f"0x67ba {name}: DS register decoy changed")
        if struct.unpack(
            "<H", machine.mem_read(stack_segment * 16 + 0x6770, 2)
        )[0] != stack_register_decoy:
            raise AssertionError(f"0x67ba {name}: SS register decoy changed")
        for segment, offset, expected in immutable:
            actual = bytes(machine.mem_read(segment * 16 + offset, len(expected)))
            if actual != expected:
                raise AssertionError(f"0x67ba {name}: script input changed")

        tested = active & 1
        flags = machine.reg_read(UC_X86_REG_EFLAGS)
        expected_flags = {
            "cf": False,
            "pf": tested == 0,
            "zf": tested == 0,
            "sf": False,
            "of": False,
        }
        flag_masks = {
            "cf": 0x0001,
            "pf": 0x0004,
            "zf": 0x0040,
            "sf": 0x0080,
            "of": 0x0800,
        }
        actual_flags = {
            flag: bool(flags & flag_masks[flag]) for flag in expected_flags
        }
        if actual_flags != expected_flags:
            raise AssertionError(
                f"0x67ba {name}: flags={actual_flags}, expected={expected_flags}"
            )
        if EXE[0x67C7] != 0xC3:
            raise AssertionError("0x67ba: expected near RET boundary")

        vectors.append(
            {
                "name": name,
                "start_offset": start,
                "operand": operand,
                "presentation_active": active,
                "store_performed": active_path,
                "register_before": register_before,
                "register_after": register_after,
                "final_script_offset": final_script,
                "defined_flags": expected_flags,
            }
        )

    return vectors


def vm_load_string_vectors() -> list[dict[str, object]]:
    data_segment = 0x4400
    game_segment = 0x2C00
    stack_segment = 0x9000
    cases = [
        {
            "name": "empty_no_request",
            "start": 0x2A00,
            "text": b"",
            "pad": 0xA5,
        },
        {
            "name": "ordinary_no_request",
            "start": 0x2B01,
            "text": b"RADIO",
            "pad": 0x5A,
        },
        {
            "name": "exact_finale",
            "start": 0x2C00,
            "text": b"fin.",
            "pad": 0xFF,
        },
        {
            "name": "finale_prefix",
            "start": 0x2D00,
            "text": b"fin.ale",
            "pad": 0x80,
        },
        {
            "name": "case_sensitive_not_finale",
            "start": 0x2E01,
            "text": b"Fin.",
            "pad": 0x7F,
        },
        {
            "name": "request_blocked_by_pending_bit",
            "start": 0x2F00,
            "text": b"ship",
            "pad": 0x33,
            "request": 0xA2,
            "ship": 1,
            "scene": 1,
        },
        {
            "name": "request_from_ship_flag",
            "start": 0x3000,
            "text": b"ship",
            "pad": 0x44,
            "request": 0xA1,
            "ship": 0x8001,
            "scene": 0,
        },
        {
            "name": "request_from_scene_gate",
            "start": 0x3101,
            "text": b"scene",
            "pad": 0x55,
            "request": 0x10,
            "ship": 0x8000,
            "scene": 0x81,
        },
        {
            "name": "unrelated_activity_bits_do_not_request",
            "start": 0x3200,
            "text": b"idle",
            "pad": 0x66,
            "request": 0x10,
            "ship": 0x8000,
            "scene": 0x80,
        },
        {
            "name": "copy_and_pad_wrap_segment",
            "start": 0xFFFC,
            "text": b"fin.",
            "pad": 0x77,
        },
    ]
    vectors = []

    for case in cases:
        name = str(case["name"])
        start = int(case["start"])
        text_bytes = bytes(case["text"])
        pad = int(case["pad"])
        script = text_bytes + b"\0" + bytes([pad])
        finale_before = 0x5A
        request_before = int(case.get("request", 0xA0))
        ship_flags = int(case.get("ship", 0))
        scene_gate = int(case.get("scene", 0))
        active_line_before = 0x2468
        presentation_gate_before = 0xA5
        actor_before = 0x1357
        dialog_gate_before = 0x5A
        is_finale = text_bytes.startswith(b"fin.")
        request_raised = not (request_before & 2) and bool(
            (ship_flags & 1) or (scene_gate & 1)
        )
        memory = [
            (game_segment, 0x67BD, bytes([finale_before])),
            (game_segment, 0x67AA, bytes([request_before])),
            (game_segment, 0x24F3, struct.pack("<H", ship_flags)),
            (game_segment, 0x274F, bytes([scene_gate])),
            (game_segment, 0x6788, struct.pack("<H", active_line_before)),
            (game_segment, 0x1FB2, bytes([presentation_gate_before])),
            (game_segment, 0x1FA3, struct.pack("<H", actor_before)),
            (game_segment, 0x0B3B, bytes([dialog_gate_before])),
            (stack_segment, 0x2120, b"\xcc" * 128),
            (data_segment, 0x2120, b"\x5a" * 128),
            (game_segment, 0x2120, b"\xa5" * 128),
        ]
        immutable = []
        for byte_index, byte in enumerate(script):
            offset = start + byte_index
            encoded = bytes([byte])
            memory.append((data_segment, offset, encoded))
            immutable.append((data_segment, offset, encoded))

        initial = {
            "eax": 0xA1A1BEEF,
            "ebx": 0xB2B22345,
            "ecx": 0xC3C33456,
            "edx": 0xD4D44567,
            "esi": 0xE5E50000 | start,
            "edi": 0xF6F66789,
            "ebp": 0x9797789A,
            "sp": 0xFF00,
            "ds": data_segment,
            "es": 0x4800,
            "fs": 0x4C00,
            "gs": game_segment,
            "ss": stack_segment,
            "flags": 0x0AD7,
        }
        phases = []

        def capture_phases(machine: Uc, address: int, _size: int) -> None:
            if address not in (0x67D5, 0x67F6, 0x680F, 0x682F):
                return
            phases.append(
                (
                    address,
                    machine.reg_read(UC_X86_REG_SI),
                    machine.mem_read(game_segment * 16 + 0x67BD, 1)[0],
                    machine.mem_read(game_segment * 16 + 0x67AA, 1)[0],
                )
            )

        machine = execute(
            0x67C8,
            0x682F,
            initial,
            memory,
            code_handler=capture_phases,
        )

        nul_end = (start + len(text_bytes) + 1) & 0xFFFF
        final_script = (nul_end + 1) & 0xFFFF
        expected_phases = [
            (0x67D5, final_script, finale_before, request_before),
            (
                0x67F6,
                final_script,
                1 if is_finale else finale_before,
                request_before,
            ),
        ]
        if request_raised:
            expected_phases.append(
                (
                    0x680F,
                    final_script,
                    1 if is_finale else finale_before,
                    request_before,
                )
            )
        if phases != expected_phases:
            raise AssertionError(
                f"0x67c8 {name}: phases={phases}, expected={expected_phases}"
            )

        expected_registers = dict(initial)
        del expected_registers["flags"]
        expected_registers["eax"] = initial["eax"] & 0xFFFFFF00
        expected_registers["esi"] = (initial["esi"] & 0xFFFF0000) | final_script
        expected_registers["ebp"] = (initial["ebp"] & 0xFFFF0000) | 0x2120
        for register, expected in expected_registers.items():
            actual = machine.reg_read(REGISTERS[register])
            if actual != expected:
                raise AssertionError(
                    f"0x67c8 {name}: {register}={actual:#x}, expected={expected:#x}"
                )

        expected_buffer = text_bytes + b"\0"
        actual_buffer = bytes(
            machine.mem_read(stack_segment * 16 + 0x2120, len(expected_buffer))
        )
        if actual_buffer != expected_buffer:
            raise AssertionError(
                f"0x67c8 {name}: buffer={actual_buffer!r}, expected={expected_buffer!r}"
            )
        if machine.mem_read(data_segment * 16 + 0x2120, 8) != b"\x5a" * 8:
            raise AssertionError(f"0x67c8 {name}: DS buffer decoy changed")
        if machine.mem_read(game_segment * 16 + 0x2120, 8) != b"\xa5" * 8:
            raise AssertionError(f"0x67c8 {name}: GS buffer decoy changed")
        for segment, offset, expected in immutable:
            actual = bytes(machine.mem_read(segment * 16 + offset, len(expected)))
            if actual != expected:
                raise AssertionError(f"0x67c8 {name}: script input changed")

        expected_finale = 1 if is_finale else finale_before
        actual_finale = machine.mem_read(game_segment * 16 + 0x67BD, 1)[0]
        if actual_finale != expected_finale:
            raise AssertionError(
                f"0x67c8 {name}: finale={actual_finale:#x}, expected={expected_finale:#x}"
            )
        expected_request = request_before | (2 if request_raised else 0)
        actual_request = machine.mem_read(game_segment * 16 + 0x67AA, 1)[0]
        if actual_request != expected_request:
            raise AssertionError(
                f"0x67c8 {name}: request={actual_request:#x}, expected={expected_request:#x}"
            )

        expected_side_effects = {
            0x6788: 7 if request_raised else active_line_before,
            0x1FB2: 0 if request_raised else presentation_gate_before,
            0x1FA3: 0xFFFF if request_raised else actor_before,
            0x0B3B: 0 if request_raised else dialog_gate_before,
        }
        for offset, expected in expected_side_effects.items():
            size = 2 if offset in (0x6788, 0x1FA3) else 1
            actual_bytes = machine.mem_read(game_segment * 16 + offset, size)
            actual = (
                struct.unpack("<H", actual_bytes)[0]
                if size == 2
                else actual_bytes[0]
            )
            if actual != expected:
                raise AssertionError(
                    f"0x67c8 {name}: {offset:#x}={actual:#x}, expected={expected:#x}"
                )

        flags = machine.reg_read(UC_X86_REG_EFLAGS)
        if request_raised:
            expected_flags = {
                "cf": False,
                "pf": (expected_request & 0xFF).bit_count() % 2 == 0,
                "zf": expected_request == 0,
                "sf": bool(expected_request & 0x80),
                "of": False,
            }
        elif request_before & 2:
            tested = request_before & 2
            expected_flags = {
                "cf": False,
                "pf": tested.bit_count() % 2 == 0,
                "zf": False,
                "sf": False,
                "of": False,
            }
        elif ship_flags & 1:
            tested = ship_flags & 1
            expected_flags = {
                "cf": False,
                "pf": tested.bit_count() % 2 == 0,
                "zf": False,
                "sf": False,
                "of": False,
            }
        else:
            tested = scene_gate & 1
            expected_flags = {
                "cf": False,
                "pf": tested.bit_count() % 2 == 0,
                "zf": tested == 0,
                "sf": False,
                "of": False,
            }
        flag_masks = {
            "cf": 0x0001,
            "pf": 0x0004,
            "zf": 0x0040,
            "sf": 0x0080,
            "of": 0x0800,
        }
        actual_flags = {
            flag: bool(flags & flag_masks[flag]) for flag in expected_flags
        }
        if actual_flags != expected_flags:
            raise AssertionError(
                f"0x67c8 {name}: flags={actual_flags}, expected={expected_flags}"
            )
        if EXE[0x682F] != 0xC3:
            raise AssertionError("0x67c8: expected near RET boundary")

        vectors.append(
            {
                "name": name,
                "start_offset": start,
                "text_hex": text_bytes.hex(),
                "pad_byte": pad,
                "final_script_offset": final_script,
                "finale_set": is_finale,
                "request_raised": request_raised,
                "defined_flags": expected_flags,
            }
        )

    return vectors


def vm_conditional_jump_vectors() -> list[dict[str, object]]:
    data_segment = 0x4400
    game_segment = 0x2C00
    stack_segment = 0x9000
    cases = [
        ("even_zero_jumps", 0x3400, 0x00, 0x0000),
        ("even_unrelated_low_bit_clear", 0x3501, 0x02, 0x1234),
        ("even_high_bit_jumps", 0x3600, 0x80, 0xFFFF),
        ("odd_one_arms_query", 0x3700, 0x01, 0x2468),
        ("odd_multiple_bits", 0x3801, 0x03, 0x8001),
        ("odd_all_bits", 0x3900, 0xFF, 0xBEEF),
        ("odd_high_bit", 0x3A00, 0x81, 0x0000),
        ("even_target_crosses_segment_end", 0xFFFE, 0x7E, 0xCAFE),
        ("odd_target_crosses_segment_end", 0xFFFE, 0x7F, 0x1357),
    ]
    vectors = []

    for name, start, flags_byte, target in cases:
        odd_path = bool(flags_byte & 1)
        script = bytes([flags_byte]) + struct.pack("<H", target)
        query_before = 0xA4
        root_before = 0x5AA5
        top_before = 0x6789
        data_query_decoy = query_before ^ 0xFF
        stack_query_decoy = query_before ^ 0xA5
        data_root_decoy = root_before ^ 0xFFFF
        stack_root_decoy = root_before ^ 0xA5A5
        data_top_decoy = top_before ^ 0xFFFF
        stack_top_decoy = top_before ^ 0x5A5A
        memory = [
            (game_segment, 0x67AD, bytes([query_before])),
            (game_segment, 0x6820, struct.pack("<H", root_before)),
            (game_segment, 0x6884, struct.pack("<H", top_before)),
            (data_segment, 0x67AD, bytes([data_query_decoy])),
            (stack_segment, 0x67AD, bytes([stack_query_decoy])),
            (data_segment, 0x6820, struct.pack("<H", data_root_decoy)),
            (stack_segment, 0x6820, struct.pack("<H", stack_root_decoy)),
            (data_segment, 0x6884, struct.pack("<H", data_top_decoy)),
            (stack_segment, 0x6884, struct.pack("<H", stack_top_decoy)),
        ]
        immutable = []
        for byte_index, byte in enumerate(script):
            offset = start + byte_index
            encoded = bytes([byte])
            memory.append((data_segment, offset, encoded))
            immutable.append((data_segment, offset, encoded))
            memory.append((0x4800, offset, b"\x5a"))
            memory.append((game_segment, offset, b"\xa5"))

        initial = {
            "eax": 0xA1A1BEEF,
            "ebx": 0xB2B22345,
            "ecx": 0xC3C33456,
            "edx": 0xD4D44567,
            "esi": 0xE5E50000 | start,
            "edi": 0xF6F66789,
            "ebp": 0x9797789A,
            "sp": 0xFF00,
            "ds": data_segment,
            "es": 0x4800,
            "fs": 0x4C00,
            "gs": game_segment,
            "ss": stack_segment,
            "flags": 0x0AD7,
        }
        phases = []

        def capture_phases(machine: Uc, address: int, _size: int) -> None:
            if address not in (0x6831, 0x6835, 0x6839, 0x683F, 0x6844):
                return
            phases.append(
                (
                    address,
                    machine.reg_read(UC_X86_REG_AX),
                    machine.reg_read(UC_X86_REG_SI),
                    machine.mem_read(game_segment * 16 + 0x67AD, 1)[0],
                    struct.unpack(
                        "<H", machine.mem_read(game_segment * 16 + 0x6820, 2)
                    )[0],
                    struct.unpack(
                        "<H", machine.mem_read(game_segment * 16 + 0x6884, 2)
                    )[0],
                )
            )

        machine = execute(
            0x6830,
            0x684B,
            initial,
            memory,
            code_handler=capture_phases,
        )

        operand_offset = (start + 1) & 0xFFFF
        advanced_offset = (start + 3) & 0xFFFF
        ax_after_flag = (initial["eax"] & 0xFF00) | flags_byte
        expected_phases = [
            (
                0x6831,
                ax_after_flag,
                operand_offset,
                query_before,
                root_before,
                top_before,
            )
        ]
        if odd_path:
            expected_phases.extend(
                [
                    (
                        0x6839,
                        ax_after_flag,
                        operand_offset,
                        query_before,
                        root_before,
                        top_before,
                    ),
                    (
                        0x683F,
                        ax_after_flag,
                        operand_offset,
                        1,
                        root_before,
                        top_before,
                    ),
                    (
                        0x6844,
                        target,
                        advanced_offset,
                        1,
                        target,
                        top_before,
                    ),
                ]
            )
        else:
            expected_phases.append(
                (
                    0x6835,
                    ax_after_flag,
                    operand_offset,
                    query_before,
                    root_before,
                    top_before,
                )
            )
        if phases != expected_phases:
            raise AssertionError(
                f"0x6830 {name}: phases={phases}, expected={expected_phases}"
            )

        expected_registers = dict(initial)
        del expected_registers["flags"]
        expected_ax = target if odd_path else ax_after_flag
        expected_si = advanced_offset if odd_path else target
        expected_registers["eax"] = (initial["eax"] & 0xFFFF0000) | expected_ax
        expected_registers["esi"] = (initial["esi"] & 0xFFFF0000) | expected_si
        for register, expected in expected_registers.items():
            actual = machine.reg_read(REGISTERS[register])
            if actual != expected:
                raise AssertionError(
                    f"0x6830 {name}: {register}={actual:#x}, expected={expected:#x}"
                )

        expected_query = 1 if odd_path else query_before
        expected_root = target if odd_path else root_before
        expected_top = 2 if odd_path else top_before
        actual_query = machine.mem_read(game_segment * 16 + 0x67AD, 1)[0]
        actual_root = struct.unpack(
            "<H", machine.mem_read(game_segment * 16 + 0x6820, 2)
        )[0]
        actual_top = struct.unpack(
            "<H", machine.mem_read(game_segment * 16 + 0x6884, 2)
        )[0]
        if (actual_query, actual_root, actual_top) != (
            expected_query,
            expected_root,
            expected_top,
        ):
            raise AssertionError(
                f"0x6830 {name}: state={(actual_query, actual_root, actual_top)}, "
                f"expected={(expected_query, expected_root, expected_top)}"
            )
        decoys = [
            (data_segment, 0x67AD, bytes([data_query_decoy])),
            (stack_segment, 0x67AD, bytes([stack_query_decoy])),
            (data_segment, 0x6820, struct.pack("<H", data_root_decoy)),
            (stack_segment, 0x6820, struct.pack("<H", stack_root_decoy)),
            (data_segment, 0x6884, struct.pack("<H", data_top_decoy)),
            (stack_segment, 0x6884, struct.pack("<H", stack_top_decoy)),
        ]
        for segment, offset, expected in decoys:
            actual = bytes(machine.mem_read(segment * 16 + offset, len(expected)))
            if actual != expected:
                raise AssertionError(f"0x6830 {name}: segment decoy changed")
        for segment, offset, expected in immutable:
            actual = bytes(machine.mem_read(segment * 16 + offset, len(expected)))
            if actual != expected:
                raise AssertionError(f"0x6830 {name}: script input changed")

        tested = flags_byte & 1
        flags = machine.reg_read(UC_X86_REG_EFLAGS)
        expected_flags = {
            "cf": False,
            "pf": tested == 0,
            "zf": tested == 0,
            "sf": False,
            "of": False,
        }
        flag_masks = {
            "cf": 0x0001,
            "pf": 0x0004,
            "zf": 0x0040,
            "sf": 0x0080,
            "of": 0x0800,
        }
        actual_flags = {
            flag: bool(flags & flag_masks[flag]) for flag in expected_flags
        }
        if actual_flags != expected_flags:
            raise AssertionError(
                f"0x6830 {name}: flags={actual_flags}, expected={expected_flags}"
            )
        if EXE[0x684B] != 0xC3:
            raise AssertionError("0x6830: expected near RET boundary")

        vectors.append(
            {
                "name": name,
                "start_offset": start,
                "flags_byte": flags_byte,
                "odd_path": odd_path,
                "target": target,
                "final_script_offset": expected_si,
                "query_after": expected_query,
                "root_after": expected_root,
                "top_after": expected_top,
                "defined_flags": expected_flags,
            }
        )

    return vectors


def vm_poke_byte_vectors() -> list[dict[str, object]]:
    data_segment = 0x4400
    extra_segment = 0x4800
    game_segment = 0x2C00
    stack_segment = 0x9000
    cases = [
        ("zero_value", 0x3400, 0x00, 0x2100),
        ("maximum_value_and_target", 0x3501, 0xFF, 0xFFFF),
        ("high_bit_value", 0x3600, 0x80, 0x2101),
        ("unaligned_script_and_target", 0x3701, 0x5A, 0x2223),
        ("target_is_value_byte", 0x3800, 0x6C, 0x3800),
        ("target_is_operand_low_byte", 0x3900, 0xA7, 0x3901),
        ("operand_word_crosses_segment_end", 0xFFFE, 0x7E, 0x2400),
        ("cursor_add_wraps_to_zero", 0xFFFD, 0x35, 0x2500),
        ("cursor_add_signed_overflow", 0x7FFD, 0xC3, 0x2600),
        ("cursor_add_auxiliary_carry", 0x000E, 0x19, 0x2700),
    ]
    vectors = []

    for name, start, value, target in cases:
        script = bytes([value]) + struct.pack("<H", target)
        script_offsets = [start + index for index in range(len(script))]
        script_by_offset = dict(zip(script_offsets, script, strict=True))
        target_before = script_by_offset.get(target, value ^ 0xC3)
        es_target_decoy = target_before ^ 0x55
        gs_target_decoy = target_before ^ 0xAA
        ss_target_decoy = target_before ^ 0x3C
        memory = []

        for offset, byte in zip(script_offsets, script, strict=True):
            memory.append((data_segment, offset, bytes([byte])))
            memory.append((extra_segment, offset, b"\x5a"))
            memory.append((game_segment, offset, b"\xa5"))
            memory.append((stack_segment, offset, b"\x3c"))
        if target not in script_by_offset:
            memory.append((data_segment, target, bytes([target_before])))
        memory.extend(
            [
                (extra_segment, target, bytes([es_target_decoy])),
                (game_segment, target, bytes([gs_target_decoy])),
                (stack_segment, target, bytes([ss_target_decoy])),
            ]
        )

        initial = {
            "eax": 0xA1A1BEEF,
            "ebx": 0xB2B22345,
            "ecx": 0xC3C33456,
            "edx": 0xD4D44567,
            "esi": 0xE5E50000 | start,
            "edi": 0xF6F66789,
            "ebp": 0x9797789A,
            "sp": 0xFF00,
            "ds": data_segment,
            "es": extra_segment,
            "fs": 0x4C00,
            "gs": game_segment,
            "ss": stack_segment,
            "flags": 0x0AD7,
        }
        phases = []

        def capture_phases(machine: Uc, address: int, _size: int) -> None:
            if address not in (0x684D, 0x684F, 0x6851):
                return
            phases.append(
                (
                    address,
                    machine.reg_read(UC_X86_REG_AX),
                    machine.reg_read(UC_X86_REG_BX),
                    machine.reg_read(UC_X86_REG_SI),
                    machine.mem_read(data_segment * 16 + target, 1)[0],
                )
            )

        machine = execute(
            0x684C,
            0x6854,
            initial,
            memory,
            code_handler=capture_phases,
        )

        operand_offset = (start + 1) & 0xFFFF
        final_script_offset = (start + 3) & 0xFFFF
        ax_after_value = (initial["eax"] & 0xFF00) | value
        expected_phases = [
            (
                0x684D,
                ax_after_value,
                initial["ebx"] & 0xFFFF,
                operand_offset,
                target_before,
            ),
            (
                0x684F,
                ax_after_value,
                target,
                operand_offset,
                target_before,
            ),
            (
                0x6851,
                ax_after_value,
                target,
                operand_offset,
                value,
            ),
        ]
        if phases != expected_phases:
            raise AssertionError(
                f"0x684c {name}: phases={phases}, expected={expected_phases}"
            )

        expected_registers = dict(initial)
        del expected_registers["flags"]
        expected_registers["eax"] = (initial["eax"] & 0xFFFFFF00) | value
        expected_registers["ebx"] = (initial["ebx"] & 0xFFFF0000) | target
        expected_registers["esi"] = (
            initial["esi"] & 0xFFFF0000
        ) | final_script_offset
        for register, expected in expected_registers.items():
            actual = machine.reg_read(REGISTERS[register])
            if actual != expected:
                raise AssertionError(
                    f"0x684c {name}: {register}={actual:#x}, expected={expected:#x}"
                )

        actual_target = machine.mem_read(data_segment * 16 + target, 1)[0]
        if actual_target != value:
            raise AssertionError(
                f"0x684c {name}: target={actual_target:#x}, expected={value:#x}"
            )
        target_decoys = [
            (extra_segment, es_target_decoy),
            (game_segment, gs_target_decoy),
            (stack_segment, ss_target_decoy),
        ]
        for segment, expected in target_decoys:
            actual = machine.mem_read(segment * 16 + target, 1)[0]
            if actual != expected:
                raise AssertionError(f"0x684c {name}: target segment decoy changed")

        for offset, byte in zip(script_offsets, script, strict=True):
            expected = value if offset == target else byte
            actual = machine.mem_read(data_segment * 16 + offset, 1)[0]
            if actual != expected:
                raise AssertionError(
                    f"0x684c {name}: script byte {offset:#x}={actual:#x}, "
                    f"expected={expected:#x}"
                )
            if offset == target:
                continue
            decoys = [
                (extra_segment, 0x5A),
                (game_segment, 0xA5),
                (stack_segment, 0x3C),
            ]
            for segment, expected_decoy in decoys:
                actual_decoy = machine.mem_read(segment * 16 + offset, 1)[0]
                if actual_decoy != expected_decoy:
                    raise AssertionError(f"0x684c {name}: script decoy changed")

        before_add = operand_offset
        full_sum = before_add + 2
        expected_flags = {
            "cf": full_sum > 0xFFFF,
            "pf": (final_script_offset & 0xFF).bit_count() % 2 == 0,
            "af": (before_add & 0x0F) + 2 > 0x0F,
            "zf": final_script_offset == 0,
            "sf": bool(final_script_offset & 0x8000),
            "of": bool(
                (~(before_add ^ 2) & (before_add ^ final_script_offset)) & 0x8000
            ),
        }
        flags = machine.reg_read(UC_X86_REG_EFLAGS)
        actual_flags = {
            "cf": bool(flags & 0x0001),
            "pf": bool(flags & 0x0004),
            "af": bool(flags & 0x0010),
            "zf": bool(flags & 0x0040),
            "sf": bool(flags & 0x0080),
            "of": bool(flags & 0x0800),
        }
        if actual_flags != expected_flags:
            raise AssertionError(
                f"0x684c {name}: flags={actual_flags}, expected={expected_flags}"
            )
        if EXE[0x6854] != 0xC3:
            raise AssertionError("0x684c: expected near RET boundary")

        vectors.append(
            {
                "name": name,
                "start_offset": start,
                "value": value,
                "target_offset": target,
                "target_before": target_before,
                "final_script_offset": final_script_offset,
                "self_modifying": target in script_by_offset,
                "defined_flags": expected_flags,
            }
        )

    return vectors


def vm_yield_vectors(entry: int) -> list[dict[str, object]]:
    data_segment = 0x4400
    extra_segment = 0x4800
    game_segment = 0x2C00
    stack_segment = 0x9000
    cases = [
        ("clear_flag", 0x00, 0x0002),
        ("already_set", 0x01, 0x0AD7),
        ("maximum_byte", 0xFF, 0x0246),
        ("high_bit_only", 0x80, 0x0893),
        ("alternating_low", 0x5A, 0x0447),
        ("alternating_high", 0xA5, 0x0CD2),
    ]
    vectors = []

    for name, yield_before, initial_flags in cases:
        data_decoy = yield_before ^ 0x55
        extra_decoy = yield_before ^ 0x33
        stack_decoy = yield_before ^ 0xAA
        fs_decoy = yield_before ^ 0xC3
        initial = {
            "eax": 0xA1A1BEEF,
            "ebx": 0xB2B22345,
            "ecx": 0xC3C33456,
            "edx": 0xD4D44567,
            "esi": 0xE5E55678,
            "edi": 0xF6F66789,
            "ebp": 0x9797789A,
            "sp": 0xFF00,
            "ds": data_segment,
            "es": extra_segment,
            "fs": 0x4C00,
            "gs": game_segment,
            "ss": stack_segment,
            "flags": initial_flags,
        }
        phases = []

        def capture_entry(machine: Uc, address: int, _size: int) -> None:
            if address == entry:
                phases.append(
                    (
                        machine.mem_read(game_segment * 16 + 0x67B4, 1)[0],
                        machine.mem_read(data_segment * 16 + 0x67B4, 1)[0],
                        machine.mem_read(extra_segment * 16 + 0x67B4, 1)[0],
                        machine.mem_read(stack_segment * 16 + 0x67B4, 1)[0],
                    )
                )

        machine = execute(
            entry,
            entry + 6,
            initial,
            [
                (game_segment, 0x67B4, bytes([yield_before])),
                (data_segment, 0x67B4, bytes([data_decoy])),
                (extra_segment, 0x67B4, bytes([extra_decoy])),
                (stack_segment, 0x67B4, bytes([stack_decoy])),
                (initial["fs"], 0x67B4, bytes([fs_decoy])),
            ],
            code_handler=capture_entry,
        )

        expected_phase = [
            (yield_before, data_decoy, extra_decoy, stack_decoy)
        ]
        if phases != expected_phase:
            raise AssertionError(
                f"{entry:#06x} {name}: phases={phases}, expected={expected_phase}"
            )
        for register, expected in initial.items():
            if register == "flags":
                continue
            actual = machine.reg_read(REGISTERS[register])
            if actual != expected:
                raise AssertionError(
                    f"{entry:#06x} {name}: {register}={actual:#x}, "
                    f"expected={expected:#x}"
                )

        actual_yield = machine.mem_read(game_segment * 16 + 0x67B4, 1)[0]
        if actual_yield != 1:
            raise AssertionError(
                f"{entry:#06x} {name}: yield={actual_yield:#x}, expected=0x1"
            )
        decoys = [
            (data_segment, data_decoy),
            (extra_segment, extra_decoy),
            (stack_segment, stack_decoy),
            (initial["fs"], fs_decoy),
        ]
        for segment, expected in decoys:
            actual = machine.mem_read(segment * 16 + 0x67B4, 1)[0]
            if actual != expected:
                raise AssertionError(f"{entry:#06x} {name}: segment decoy changed")

        flag_mask = 0x0CD5
        actual_flags = machine.reg_read(UC_X86_REG_EFLAGS) & flag_mask
        expected_flags = initial_flags & flag_mask
        if actual_flags != expected_flags:
            raise AssertionError(
                f"{entry:#06x} {name}: flags={actual_flags:#x}, "
                f"expected={expected_flags:#x}"
            )
        if EXE[entry + 6] != 0xC3:
            raise AssertionError(f"{entry:#06x}: expected near RET boundary")

        vectors.append(
            {
                "name": name,
                "entry": entry,
                "yield_before": yield_before,
                "yield_after": actual_yield,
                "preserved_flags_mask": flag_mask,
                "preserved_flags": expected_flags,
            }
        )

    return vectors


def vm_shared_state_vectors() -> list[dict[str, object]]:
    data_segment = 0x4400
    extra_segment = 0x4800
    game_segment = 0x2C00
    record_segment = 0x5200
    stack_segment = 0x9000
    cases = [
        {"name": "query_ne_pass", "query": 1, "op": 0xF0, "current": 1, "rhs": 2, "pass": True},
        {"name": "query_ne_fail", "query": 3, "op": 0xF0, "current": 2, "rhs": 2, "pass": False},
        {"name": "query_le_signed_pass", "query": 1, "op": 0xF3, "current": 0x8000, "rhs": 0x7FFF, "pass": True},
        {"name": "query_le_signed_fail", "query": 1, "op": 0xF3, "current": 0x7FFF, "rhs": 0x8000, "pass": False},
        {"name": "query_ge_signed_pass", "query": 1, "op": 0xF4, "current": 0x7FFF, "rhs": 0xFFFF, "pass": True},
        {"name": "query_ge_signed_fail", "query": 1, "op": 0xF4, "current": 0x8000, "rhs": 0, "pass": False},
        {"name": "query_lt_signed_pass", "query": 1, "op": 0xF1, "current": 0xFFFF, "rhs": 0, "pass": True},
        {"name": "query_lt_signed_fail", "query": 1, "op": 0xF1, "current": 0, "rhs": 0xFFFF, "pass": False},
        {"name": "query_gt_signed_pass", "query": 1, "op": 0xF2, "current": 1, "rhs": 0xFFFF, "pass": True},
        {"name": "query_gt_signed_fail", "query": 1, "op": 0xF2, "current": 0xFFFF, "rhs": 1, "pass": False},
        {"name": "query_eq_c0_pass", "query": 1, "op": 0xF5, "mode": 0xC0, "current": 0x2468, "rhs_value": 0x2468, "pass": True},
        {"name": "query_eq_c2_fail", "query": 1, "op": 0xF5, "mode": 0xC2, "current": 0x1357, "rhs_value": 0x2468, "pass": False},
        {"name": "query_unknown_fails", "query": 1, "op": 0xE0, "current": 0x1111, "rhs": 0x1111, "pass": False},
        {"name": "set_add", "query": 0, "op": 0xF6, "current": 0x1234, "rhs": 0x1111},
        {"name": "set_add_wrap", "query": 2, "op": 0xF6, "current": 0xFFFF, "rhs": 1},
        {"name": "set_sub_c0_underflow", "query": 0, "op": 0xF7, "mode": 0xC0, "current": 0, "rhs_value": 1},
        {"name": "set_assign_c2", "query": 0, "op": 0xF5, "mode": 0xC2, "current": 0x1234, "rhs_value": 0xBEEF},
        {"name": "set_unknown_rewrites", "query": 0, "op": 0xE0, "current": 0x4567, "rhs": 0x1111},
        {"name": "segment_end_query_eq", "query": 1, "op": 0xF5, "current": 0xA55A, "rhs": 0xA55A, "pass": True, "start": 0xFFFA},
        {"name": "record_offsets_wrap", "query": 0, "op": 0xF6, "mode": 0xC0, "current": 2, "rhs_value": 3, "base": 0xFFF0, "field_offset": 0x0030, "rhs_offset": 0x0040},
    ]
    failure_tops = [2, 1, 0, 5, 0x8000, 0xFFFF]
    vectors = []

    def logic_flags_8(value: int) -> dict[str, bool]:
        result = value & 0xFF
        return {
            "cf": False,
            "pf": result.bit_count() % 2 == 0,
            "zf": result == 0,
            "sf": bool(result & 0x80),
            "of": False,
        }

    def add_flags_16(left: int, right: int) -> dict[str, bool]:
        full_result = left + right
        result = full_result & 0xFFFF
        return {
            "cf": full_result > 0xFFFF,
            "pf": (result & 0xFF).bit_count() % 2 == 0,
            "af": (left & 0x0F) + (right & 0x0F) > 0x0F,
            "zf": result == 0,
            "sf": bool(result & 0x8000),
            "of": bool((~(left ^ right) & (left ^ result)) & 0x8000),
        }

    def sub_flags_16(left: int, right: int) -> dict[str, bool]:
        result = (left - right) & 0xFFFF
        return {
            "cf": left < right,
            "pf": (result & 0xFF).bit_count() % 2 == 0,
            "af": (left & 0x0F) < (right & 0x0F),
            "zf": result == 0,
            "sf": bool(result & 0x8000),
            "of": bool(((left ^ right) & (left ^ result)) & 0x8000),
        }

    def cmp_flags_8(left: int, right: int) -> dict[str, bool]:
        result = (left - right) & 0xFF
        return {
            "cf": left < right,
            "pf": result.bit_count() % 2 == 0,
            "af": (left & 0x0F) < (right & 0x0F),
            "zf": result == 0,
            "sf": bool(result & 0x80),
            "of": bool(((left ^ right) & (left ^ result)) & 0x80),
        }

    failure_index = 0
    for case_index, case in enumerate(cases):
        name = str(case["name"])
        query_before = int(case["query"])
        query_path = bool(query_before & 1)
        op = int(case["op"])
        mode = int(case.get("mode", 0x11))
        current = int(case["current"])
        base_offset = int(case.get("base", 0x1000))
        field_offset = int(case.get("field_offset", 0x0200 + case_index * 4))
        rhs_operand = int(case.get("rhs_offset", case.get("rhs", 0x0600 + case_index * 4)))
        rhs = int(case.get("rhs_value", rhs_operand))
        start = int(case.get("start", 0x3000 + case_index * 0x20))
        field_effective = (base_offset + field_offset) & 0xFFFF
        rhs_effective = (base_offset + rhs_operand) & 0xFFFF
        script = struct.pack("<HBBH", field_offset, op, mode, rhs_operand)
        final_script = (start + len(script)) & 0xFFFF
        branch_failed = query_path and not bool(case.get("pass", False))
        if branch_failed:
            top_before = failure_tops[failure_index % len(failure_tops)]
            failure_index += 1
            top_after = (top_before - 2) & 0xFFFF
            branch_target = (0x5000 + case_index * 0x31) & 0xFFFF
            branch_stack_effective = (0x6820 + top_after) & 0xFFFF
        else:
            top_before = 0x2468
            top_after = top_before
            branch_target = 0x5AA5
            branch_stack_effective = 0x681E

        if query_path:
            field_after = current
        elif op == 0xF6:
            field_after = (current + rhs) & 0xFFFF
        elif op == 0xF7:
            field_after = (current - rhs) & 0xFFFF
        elif op == 0xF5:
            field_after = rhs
        else:
            field_after = current

        pointer = struct.pack("<HH", base_offset, record_segment)
        data_pointer_decoy = struct.pack("<HH", 0x2222, extra_segment)
        stack_pointer_decoy = struct.pack("<HH", 0x3333, stack_segment)
        data_query_decoy = query_before ^ 0x55
        stack_query_decoy = query_before ^ 0xAA
        data_top_decoy = top_before ^ 0xFFFF
        stack_top_decoy = top_before ^ 0x5A5A
        field_decoy = current ^ 0xFFFF
        memory = [
            (game_segment, 0x6724, pointer),
            (data_segment, 0x6724, data_pointer_decoy),
            (stack_segment, 0x6724, stack_pointer_decoy),
            (game_segment, 0x67AD, bytes([query_before])),
            (data_segment, 0x67AD, bytes([data_query_decoy])),
            (stack_segment, 0x67AD, bytes([stack_query_decoy])),
            (game_segment, 0x6884, struct.pack("<H", top_before)),
            (data_segment, 0x6884, struct.pack("<H", data_top_decoy)),
            (stack_segment, 0x6884, struct.pack("<H", stack_top_decoy)),
            (record_segment, field_effective, struct.pack("<H", current)),
            (data_segment, field_effective, struct.pack("<H", field_decoy)),
            (game_segment, field_effective, struct.pack("<H", field_decoy ^ 0xA5A5)),
            (stack_segment, field_effective, struct.pack("<H", field_decoy ^ 0x5A5A)),
            (stack_segment, branch_stack_effective, struct.pack("<H", branch_target)),
            (game_segment, branch_stack_effective, struct.pack("<H", branch_target ^ 0xFFFF)),
            (data_segment, branch_stack_effective, struct.pack("<H", branch_target ^ 0xA5A5)),
            (data_segment, start, script),
            (extra_segment, start, b"\x5a" * len(script)),
            (game_segment, start, b"\xa5" * len(script)),
        ]
        if mode in (0xC0, 0xC2):
            memory.extend(
                [
                    (record_segment, rhs_effective, struct.pack("<H", rhs)),
                    (data_segment, rhs_effective, struct.pack("<H", rhs ^ 0xFFFF)),
                    (game_segment, rhs_effective, struct.pack("<H", rhs ^ 0xA5A5)),
                    (stack_segment, rhs_effective, struct.pack("<H", rhs ^ 0x5A5A)),
                ]
            )

        initial = {
            "eax": 0xA1A1BEEF,
            "ebx": 0xB2B22345,
            "ecx": 0xC3C33456,
            "edx": 0xD4D44567,
            "esi": 0xE5E50000 | start,
            "edi": 0xF6F66789,
            "ebp": 0x9797789A,
            "sp": 0xFF00,
            "ds": data_segment,
            "es": extra_segment,
            "fs": 0x4C00,
            "gs": game_segment,
            "ss": stack_segment,
            "flags": 0x0AD7,
        }
        parse_phases = []
        terminal_phases = []

        def capture_phases(machine: Uc, address: int, _size: int) -> None:
            if address in (0x6869, 0x686E, 0x6875, 0x6886, 0x6889):
                parse_phases.append(
                    (
                        address,
                        machine.reg_read(UC_X86_REG_AX),
                        machine.reg_read(UC_X86_REG_BX),
                        machine.reg_read(UC_X86_REG_CX),
                        machine.reg_read(UC_X86_REG_DX),
                        machine.reg_read(UC_X86_REG_SI),
                        machine.reg_read(UC_X86_REG_DI),
                        machine.reg_read(UC_X86_REG_ES),
                    )
                )
            elif address == 0x6900:
                terminal_phases.append(
                    (
                        machine.reg_read(UC_X86_REG_AX),
                        machine.reg_read(UC_X86_REG_BX),
                        machine.reg_read(UC_X86_REG_CX),
                        machine.reg_read(UC_X86_REG_DX),
                        machine.reg_read(UC_X86_REG_SI),
                        machine.reg_read(UC_X86_REG_DI),
                        machine.reg_read(UC_X86_REG_ES),
                        struct.unpack(
                            "<H",
                            machine.mem_read(record_segment * 16 + field_effective, 2),
                        )[0],
                        machine.mem_read(game_segment * 16 + 0x67AD, 1)[0],
                        struct.unpack(
                            "<H", machine.mem_read(game_segment * 16 + 0x6884, 2)
                        )[0],
                    )
                )

        machine = execute(
            0x6863,
            0x6901,
            initial,
            memory,
            code_handler=capture_phases,
        )

        packed_ax = (op << 8) | mode
        expected_parse = [
            (0x6869, initial["eax"] & 0xFFFF, initial["ebx"] & 0xFFFF, initial["ecx"] & 0xFFFF, initial["edx"] & 0xFFFF, start, base_offset, record_segment),
            (0x686E, initial["eax"] & 0xFFFF, field_offset, current, initial["edx"] & 0xFFFF, start, base_offset, record_segment),
            (0x6875, packed_ax, field_offset, current, initial["edx"] & 0xFFFF, (start + 4) & 0xFFFF, base_offset, record_segment),
            (0x6886, packed_ax, field_offset, current, rhs, (start + 4) & 0xFFFF, base_offset, record_segment),
            (0x6889, packed_ax, field_offset, current, rhs, final_script, base_offset, record_segment),
        ]
        if parse_phases != expected_parse:
            raise AssertionError(
                f"0x6863 {name}: parse={parse_phases}, expected={expected_parse}"
            )

        if query_path and not branch_failed:
            expected_ax = (op << 8) | 1
            expected_si = final_script
            query_after = query_before
            expected_flags = logic_flags_8(1)
        elif branch_failed:
            expected_ax = top_after
            expected_si = branch_target
            query_after = 0
            expected_flags = sub_flags_16(top_before, 2)
        else:
            expected_ax = packed_ax
            expected_si = final_script
            query_after = query_before
            if op == 0xF6:
                expected_flags = add_flags_16(current, rhs)
            elif op == 0xF7:
                expected_flags = sub_flags_16(current, rhs)
            else:
                expected_flags = cmp_flags_8(op, 0xF5)

        expected_terminal = [
            (
                expected_ax,
                field_offset,
                field_after,
                rhs,
                expected_si,
                base_offset,
                record_segment,
                field_after,
                query_after,
                top_after,
            )
        ]
        if terminal_phases != expected_terminal:
            raise AssertionError(
                f"0x6863 {name}: terminal={terminal_phases}, "
                f"expected={expected_terminal}"
            )

        expected_registers = dict(initial)
        del expected_registers["flags"]
        expected_registers["eax"] = (initial["eax"] & 0xFFFF0000) | expected_ax
        expected_registers["ebx"] = (initial["ebx"] & 0xFFFF0000) | field_offset
        expected_registers["ecx"] = (initial["ecx"] & 0xFFFF0000) | field_after
        expected_registers["edx"] = (initial["edx"] & 0xFFFF0000) | rhs
        expected_registers["esi"] = (initial["esi"] & 0xFFFF0000) | expected_si
        expected_registers["es"] = record_segment
        for register, expected in expected_registers.items():
            actual = machine.reg_read(REGISTERS[register])
            if actual != expected:
                raise AssertionError(
                    f"0x6863 {name}: {register}={actual:#x}, expected={expected:#x}"
                )

        actual_field = struct.unpack(
            "<H", machine.mem_read(record_segment * 16 + field_effective, 2)
        )[0]
        if actual_field != field_after:
            raise AssertionError(
                f"0x6863 {name}: field={actual_field:#x}, expected={field_after:#x}"
            )
        if bytes(machine.mem_read(data_segment * 16 + start, len(script))) != script:
            raise AssertionError(f"0x6863 {name}: script input changed")
        if machine.mem_read(extra_segment * 16 + start, len(script)) != b"\x5a" * len(script):
            raise AssertionError(f"0x6863 {name}: ES script decoy changed")
        if machine.mem_read(game_segment * 16 + start, len(script)) != b"\xa5" * len(script):
            raise AssertionError(f"0x6863 {name}: GS script decoy changed")
        decoys = [
            (data_segment, 0x6724, data_pointer_decoy),
            (stack_segment, 0x6724, stack_pointer_decoy),
            (data_segment, 0x67AD, bytes([data_query_decoy])),
            (stack_segment, 0x67AD, bytes([stack_query_decoy])),
            (data_segment, 0x6884, struct.pack("<H", data_top_decoy)),
            (stack_segment, 0x6884, struct.pack("<H", stack_top_decoy)),
            (data_segment, field_effective, struct.pack("<H", field_decoy)),
            (game_segment, field_effective, struct.pack("<H", field_decoy ^ 0xA5A5)),
            (stack_segment, field_effective, struct.pack("<H", field_decoy ^ 0x5A5A)),
        ]
        for segment, offset, expected in decoys:
            actual = bytes(machine.mem_read(segment * 16 + offset, len(expected)))
            if actual != expected:
                raise AssertionError(f"0x6863 {name}: segment decoy changed")

        flags = machine.reg_read(UC_X86_REG_EFLAGS)
        flag_masks = {"cf": 0x0001, "pf": 0x0004, "af": 0x0010, "zf": 0x0040, "sf": 0x0080, "of": 0x0800}
        actual_flags = {
            flag: bool(flags & flag_masks[flag]) for flag in expected_flags
        }
        if actual_flags != expected_flags:
            raise AssertionError(
                f"0x6863 {name}: flags={actual_flags}, expected={expected_flags}"
            )
        if EXE[0x6901] != 0xC3:
            raise AssertionError("0x6863: expected near RET boundary")

        vectors.append(
            {
                "name": name,
                "query_mode_before": query_before,
                "operation": op,
                "rhs_mode": mode,
                "current_before": current,
                "resolved_rhs": rhs,
                "field_after": field_after,
                "branch_failed": branch_failed,
                "final_script_offset": expected_si,
                "query_mode_after": query_after,
                "branch_stack_top_after": top_after,
                "defined_flags": expected_flags,
            }
        )

    return vectors


def vm_shared_bit_state_vectors() -> list[dict[str, object]]:
    data_segment = 0x4400
    extra_segment = 0x4800
    game_segment = 0x2C00
    record_segment = 0x5200
    stack_segment = 0x9000
    cases = [
        ("query_present_pass", 1, False, 0x0001, 0x0001),
        ("query_absent_fails", 1, False, 0x0000, 0x0001),
        ("query_inverted_present_fails", 3, True, 0x0001, 0x0001),
        ("query_inverted_absent_pass", 1, True, 0x0000, 0x0001),
        ("query_any_bit_pass", 1, False, 0x0002, 0x0003),
        ("query_zero_mask_fails", 1, False, 0xFFFF, 0x0000),
        ("query_inverted_zero_mask_pass", 1, True, 0xFFFF, 0x0000),
        ("query_inverted_segment_end", 1, True, 0x0000, 0x8000),
        ("set_or_bits", 0, False, 0x0001, 0x0002),
        ("set_or_no_change", 2, False, 0xFFFF, 0x00F0),
        ("set_or_zero", 0, False, 0x0000, 0x0000),
        ("clear_selected_bits", 0, True, 0xFFFF, 0x00F0),
        ("clear_all_bits", 2, True, 0xA55A, 0xFFFF),
        ("record_offset_wrap", 0, False, 0x0001, 0x0002),
    ]
    failure_tops = [2, 0, 1]
    vectors = []

    def logic_flags(value: int, sign_mask: int) -> dict[str, bool]:
        result = value & (0xFFFF if sign_mask == 0x8000 else 0xFF)
        return {
            "cf": False,
            "pf": (result & 0xFF).bit_count() % 2 == 0,
            "zf": result == 0,
            "sf": bool(result & sign_mask),
            "of": False,
        }

    def sub_flags_16(left: int, right: int) -> dict[str, bool]:
        result = (left - right) & 0xFFFF
        return {
            "cf": left < right,
            "pf": (result & 0xFF).bit_count() % 2 == 0,
            "af": (left & 0x0F) < (right & 0x0F),
            "zf": result == 0,
            "sf": bool(result & 0x8000),
            "of": bool(((left ^ right) & (left ^ result)) & 0x8000),
        }

    failure_index = 0
    for case_index, (name, query_before, inverted, field_before, mask) in enumerate(cases):
        start = 0xFFFB if name == "query_inverted_segment_end" else 0x3600 + case_index * 0x20
        base_offset = 0xFFF0 if name == "record_offset_wrap" else 0x1200
        field_offset = 0x0030 if name == "record_offset_wrap" else 0x0300 + case_index * 4
        field_effective = (base_offset + field_offset) & 0xFFFF
        prefix = b"\xa1" if inverted else b""
        script = prefix + struct.pack("<HH", field_offset, mask)
        operand_start = (start + len(prefix)) & 0xFFFF
        final_script = (start + len(script)) & 0xFFFF
        has_bits = bool(field_before & mask)
        branch_failed = bool(query_before & 1) and has_bits == inverted
        if branch_failed:
            top_before = failure_tops[failure_index % len(failure_tops)]
            failure_index += 1
            top_after = (top_before - 2) & 0xFFFF
            branch_target = (0x5400 + case_index * 0x41) & 0xFFFF
            branch_stack_effective = (0x6820 + top_after) & 0xFFFF
        else:
            top_before = 0x2468
            top_after = top_before
            branch_target = 0x5AA5
            branch_stack_effective = 0x681E

        if query_before & 1:
            field_after = field_before
        elif inverted:
            field_after = field_before & (~mask & 0xFFFF)
        else:
            field_after = field_before | mask

        pointer = struct.pack("<HH", base_offset, record_segment)
        data_pointer_decoy = struct.pack("<HH", 0x2222, extra_segment)
        stack_pointer_decoy = struct.pack("<HH", 0x3333, stack_segment)
        data_query_decoy = query_before ^ 0x55
        stack_query_decoy = query_before ^ 0xAA
        data_top_decoy = top_before ^ 0xFFFF
        stack_top_decoy = top_before ^ 0x5A5A
        field_decoy = field_before ^ 0xFFFF
        memory = [
            (game_segment, 0x6724, pointer),
            (data_segment, 0x6724, data_pointer_decoy),
            (stack_segment, 0x6724, stack_pointer_decoy),
            (game_segment, 0x67AD, bytes([query_before])),
            (data_segment, 0x67AD, bytes([data_query_decoy])),
            (stack_segment, 0x67AD, bytes([stack_query_decoy])),
            (game_segment, 0x6884, struct.pack("<H", top_before)),
            (data_segment, 0x6884, struct.pack("<H", data_top_decoy)),
            (stack_segment, 0x6884, struct.pack("<H", stack_top_decoy)),
            (record_segment, field_effective, struct.pack("<H", field_before)),
            (data_segment, field_effective, struct.pack("<H", field_decoy)),
            (game_segment, field_effective, struct.pack("<H", field_decoy ^ 0xA5A5)),
            (stack_segment, field_effective, struct.pack("<H", field_decoy ^ 0x5A5A)),
            (stack_segment, branch_stack_effective, struct.pack("<H", branch_target)),
            (game_segment, branch_stack_effective, struct.pack("<H", branch_target ^ 0xFFFF)),
            (data_segment, branch_stack_effective, struct.pack("<H", branch_target ^ 0xA5A5)),
            (data_segment, start, script),
            (extra_segment, start, b"\x5a" * len(script)),
            (game_segment, start, b"\xa5" * len(script)),
        ]
        initial = {
            "eax": 0xA1A1BEEF,
            "ebx": 0xB2B22345,
            "ecx": 0xC3C33456,
            "edx": 0xD4D44567,
            "esi": 0xE5E50000 | start,
            "edi": 0xF6F66789,
            "ebp": 0x9797789A,
            "sp": 0xFF00,
            "ds": data_segment,
            "es": extra_segment,
            "fs": 0x4C00,
            "gs": game_segment,
            "ss": stack_segment,
            "flags": 0x0AD7,
        }
        parse_phases = []
        terminal_phases = []

        def capture_phases(machine: Uc, address: int, _size: int) -> None:
            if address in (0x6908, 0x6913, 0x6914, 0x6917):
                parse_phases.append(
                    (
                        address,
                        machine.reg_read(UC_X86_REG_AX),
                        machine.reg_read(UC_X86_REG_BX),
                        machine.reg_read(UC_X86_REG_DX),
                        machine.reg_read(UC_X86_REG_SI),
                        machine.reg_read(UC_X86_REG_DI),
                        machine.reg_read(UC_X86_REG_ES),
                    )
                )
            elif address == 0x6944:
                terminal_phases.append(
                    (
                        machine.reg_read(UC_X86_REG_AX),
                        machine.reg_read(UC_X86_REG_BX),
                        machine.reg_read(UC_X86_REG_DX),
                        machine.reg_read(UC_X86_REG_SI),
                        machine.reg_read(UC_X86_REG_DI),
                        machine.reg_read(UC_X86_REG_ES),
                        struct.unpack(
                            "<H", machine.mem_read(record_segment * 16 + field_effective, 2)
                        )[0],
                        machine.mem_read(game_segment * 16 + 0x67AD, 1)[0],
                        struct.unpack(
                            "<H", machine.mem_read(game_segment * 16 + 0x6884, 2)
                        )[0],
                    )
                )

        machine = execute(
            0x6902,
            0x6945,
            initial,
            memory,
            code_handler=capture_phases,
        )

        probe_byte = 0xA1 if inverted else field_offset & 0xFF
        dx_with_inversion = (initial["edx"] & 0xFF00) | int(inverted)
        expected_parse = [
            (0x6908, initial["eax"] & 0xFFFF, initial["ebx"] & 0xFFFF, initial["edx"] & 0xFFFF, start, base_offset, record_segment),
            (0x6913, (initial["eax"] & 0xFF00) | probe_byte, initial["ebx"] & 0xFFFF, dx_with_inversion, operand_start, base_offset, record_segment),
            (0x6914, field_offset, initial["ebx"] & 0xFFFF, dx_with_inversion, (operand_start + 2) & 0xFFFF, base_offset, record_segment),
            (0x6917, mask, field_offset, dx_with_inversion, final_script, base_offset, record_segment),
        ]
        if parse_phases != expected_parse:
            raise AssertionError(
                f"0x6902 {name}: parse={parse_phases}, expected={expected_parse}"
            )

        if branch_failed:
            expected_ax = top_after
            expected_si = branch_target
            query_after = 0
            expected_flags = sub_flags_16(top_before, 2)
        elif query_before & 1:
            expected_ax = field_before & mask
            expected_si = final_script
            query_after = query_before
            expected_flags = logic_flags(int(inverted), 0x80)
        elif inverted:
            expected_ax = ~mask & 0xFFFF
            expected_si = final_script
            query_after = query_before
            expected_flags = logic_flags(field_after, 0x8000)
        else:
            expected_ax = mask
            expected_si = final_script
            query_after = query_before
            expected_flags = logic_flags(field_after, 0x8000)

        expected_terminal = [
            (
                expected_ax,
                field_offset,
                dx_with_inversion,
                expected_si,
                base_offset,
                record_segment,
                field_after,
                query_after,
                top_after,
            )
        ]
        if terminal_phases != expected_terminal:
            raise AssertionError(
                f"0x6902 {name}: terminal={terminal_phases}, "
                f"expected={expected_terminal}"
            )

        expected_registers = dict(initial)
        del expected_registers["flags"]
        expected_registers["eax"] = (initial["eax"] & 0xFFFF0000) | expected_ax
        expected_registers["ebx"] = (initial["ebx"] & 0xFFFF0000) | field_offset
        expected_registers["edx"] = (initial["edx"] & 0xFFFF0000) | dx_with_inversion
        expected_registers["esi"] = (initial["esi"] & 0xFFFF0000) | expected_si
        expected_registers["es"] = record_segment
        for register, expected in expected_registers.items():
            actual = machine.reg_read(REGISTERS[register])
            if actual != expected:
                raise AssertionError(
                    f"0x6902 {name}: {register}={actual:#x}, expected={expected:#x}"
                )

        actual_field = struct.unpack(
            "<H", machine.mem_read(record_segment * 16 + field_effective, 2)
        )[0]
        if actual_field != field_after:
            raise AssertionError(
                f"0x6902 {name}: field={actual_field:#x}, expected={field_after:#x}"
            )
        if bytes(machine.mem_read(data_segment * 16 + start, len(script))) != script:
            raise AssertionError(f"0x6902 {name}: script input changed")
        if machine.mem_read(extra_segment * 16 + start, len(script)) != b"\x5a" * len(script):
            raise AssertionError(f"0x6902 {name}: ES script decoy changed")
        if machine.mem_read(game_segment * 16 + start, len(script)) != b"\xa5" * len(script):
            raise AssertionError(f"0x6902 {name}: GS script decoy changed")
        decoys = [
            (data_segment, 0x6724, data_pointer_decoy),
            (stack_segment, 0x6724, stack_pointer_decoy),
            (data_segment, 0x67AD, bytes([data_query_decoy])),
            (stack_segment, 0x67AD, bytes([stack_query_decoy])),
            (data_segment, 0x6884, struct.pack("<H", data_top_decoy)),
            (stack_segment, 0x6884, struct.pack("<H", stack_top_decoy)),
            (data_segment, field_effective, struct.pack("<H", field_decoy)),
            (game_segment, field_effective, struct.pack("<H", field_decoy ^ 0xA5A5)),
            (stack_segment, field_effective, struct.pack("<H", field_decoy ^ 0x5A5A)),
        ]
        for segment, offset, expected in decoys:
            actual = bytes(machine.mem_read(segment * 16 + offset, len(expected)))
            if actual != expected:
                raise AssertionError(f"0x6902 {name}: segment decoy changed")

        flags = machine.reg_read(UC_X86_REG_EFLAGS)
        flag_masks = {"cf": 0x0001, "pf": 0x0004, "af": 0x0010, "zf": 0x0040, "sf": 0x0080, "of": 0x0800}
        actual_flags = {
            flag: bool(flags & flag_masks[flag]) for flag in expected_flags
        }
        if actual_flags != expected_flags:
            raise AssertionError(
                f"0x6902 {name}: flags={actual_flags}, expected={expected_flags}"
            )
        if EXE[0x6945] != 0xC3:
            raise AssertionError("0x6902: expected near RET boundary")

        vectors.append(
            {
                "name": name,
                "query_mode_before": query_before,
                "inverted": inverted,
                "field_before": field_before,
                "mask": mask,
                "field_after": field_after,
                "branch_failed": branch_failed,
                "final_script_offset": expected_si,
                "query_mode_after": query_after,
                "branch_stack_top_after": top_after,
                "defined_flags": expected_flags,
            }
        )

    return vectors


def vm_record_wildcard_vectors() -> list[dict[str, object]]:
    data_segment = 0x4400
    extra_segment = 0x4800
    game_segment = 0x2C00
    record_segment = 0x5200
    directory_segment = 0x5800
    stack_segment = 0x9000
    cases = [
        {"name": "query_equal_pass", "query": 1, "field": 0x1234, "value": 0x1234},
        {"name": "query_mismatch_fails", "query": 1, "field": 0x1111, "value": 0x2222},
        {"name": "query_inverted_equal_fails", "query": 3, "inverted": True, "field": 0x1234, "value": 0x1234},
        {"name": "query_inverted_mismatch_pass", "query": 1, "inverted": True, "field": 0x1111, "value": 0x2222},
        {"name": "query_wildcard_maps_to_ffff", "query": 1, "field": 0xFFFF, "value": 0x7777},
        {"name": "query_wildcard_ordinary_fails", "query": 1, "field": 0x1111, "value": 0x7777},
        {"name": "query_wildcard_itself_ffff", "query": 1, "field": 0xFFFF, "value": 0xFFFF, "wildcard": 0xFFFF},
        {"name": "query_inverted_segment_end", "query": 1, "inverted": True, "field": 1, "value": 2, "start": 0xFFFB},
        {"name": "query_plain_segment_end", "query": 1, "field": 0xA55A, "value": 0xA55A, "start": 0xFFFC},
        {"name": "set_direct", "query": 0, "opcode": 0xAD, "field": 0x1111, "value": 0x2222},
        {"name": "set_bc_publishes", "query": 2, "opcode": 0xBC, "field": 0x1111, "value": 0x3456},
        {"name": "set_replaces_ffff_remove_present", "query": 0, "opcode": 0xAF, "field": 0xFFFF, "value": 0x3333, "slots": "remove_present"},
        {"name": "set_ffff_remove_absent", "query": 0, "opcode": 0xB2, "field": 0xFFFF, "value": 0xFFFF, "slots": "remove_absent"},
        {"name": "set_wildcard_insert_existing", "query": 0, "opcode": 0xB3, "field": 0x1111, "value": 0x7777, "slots": "insert_existing"},
        {"name": "set_ffff_insert_free", "query": 0, "opcode": 0xBA, "field": 0x1111, "value": 0xFFFF, "slots": "insert_free"},
        {"name": "set_wildcard_full_skips_write", "query": 0, "opcode": 0xBC, "field": 0x1357, "value": 0x7777, "slots": "insert_full"},
        {"name": "set_record_offset_wrap", "query": 0, "opcode": 0xBB, "field": 1, "value": 2, "base": 0xFFF0},
    ]
    failure_tops = [2, 0, 1, 5]
    vectors = []

    def logic_flags_8(value: int) -> dict[str, bool]:
        result = value & 0xFF
        return {
            "cf": False,
            "pf": result.bit_count() % 2 == 0,
            "zf": result == 0,
            "sf": bool(result & 0x80),
            "of": False,
        }

    def sub_flags_16(left: int, right: int) -> dict[str, bool]:
        result = (left - right) & 0xFFFF
        return {
            "cf": left < right,
            "pf": (result & 0xFF).bit_count() % 2 == 0,
            "af": (left & 0x0F) < (right & 0x0F),
            "zf": result == 0,
            "sf": bool(result & 0x8000),
            "of": bool(((left ^ right) & (left ^ result)) & 0x8000),
        }

    def cmp_flags_16(left: int, right: int) -> dict[str, bool]:
        return sub_flags_16(left, right)

    failure_index = 0
    for case_index, case in enumerate(cases):
        name = str(case["name"])
        query_before = int(case["query"])
        query_path = bool(query_before & 1)
        inverted = bool(case.get("inverted", False))
        opcode = int(case.get("opcode", 0xAD))
        field_before = int(case["field"])
        requested_value = int(case["value"])
        wildcard = int(case.get("wildcard", 0x7777))
        base_offset = int(case.get("base", 0x1400))
        field_offset = 0x0300
        field_effective = (base_offset + field_offset) & 0xFFFF
        start = int(case.get("start", 0x3A00 + case_index * 0x20))
        prefix = b"\xa1" if query_path and inverted else b""
        script = prefix + struct.pack("<HH", field_offset, requested_value)
        final_script = (start + len(script)) & 0xFFFF
        transformed_value = (
            0xFFFF if query_path and requested_value == wildcard else requested_value
        )
        branch_failed = query_path and (
            (field_before == transformed_value) == inverted
        )
        if branch_failed:
            top_before = failure_tops[failure_index % len(failure_tops)]
            failure_index += 1
            top_after = (top_before - 2) & 0xFFFF
            branch_target = (0x5800 + case_index * 0x37) & 0xFFFF
            branch_stack_effective = (0x6820 + top_after) & 0xFFFF
        else:
            top_before = 0x2468
            top_after = top_before
            branch_target = 0x5AA5
            branch_stack_effective = 0x681E

        owner = 0x0200
        slot_mode = str(case.get("slots", "none"))
        slots_before = [0x3000 + index for index in range(16)]
        if slot_mode == "remove_present":
            slots_before[5] = owner
        elif slot_mode == "remove_absent":
            slots_before[3] = 0
        elif slot_mode == "insert_existing":
            slots_before[4] = owner
        elif slot_mode == "insert_free":
            slots_before[3] = 0
        elif slot_mode == "insert_full":
            pass
        else:
            slots_before[3] = 0
        slots_after = list(slots_before)

        helper_kind = "none"
        insert_success = False
        if query_path:
            field_after = field_before
        elif field_before == 0xFFFF:
            helper_kind = "remove"
            if owner in slots_after:
                slots_after[slots_after.index(owner)] = 0
            field_after = requested_value
        elif requested_value in (wildcard, 0xFFFF):
            helper_kind = "insert"
            if owner in slots_after:
                insert_success = True
            elif 0 in slots_after:
                insert_success = True
                slots_after[slots_after.index(0)] = owner
            field_after = 0xFFFF if insert_success else field_before
        else:
            field_after = requested_value

        branch_a_before = 0xA55A
        branch_a_after = requested_value if not query_path and opcode == 0xBC else branch_a_before
        pointer = struct.pack("<HH", base_offset, record_segment)
        directory_offset = 0x2000
        directory_pointer = struct.pack("<HH", directory_offset, directory_segment)
        directory = bytearray(0x3C)
        for entry_index, object_offset in enumerate((0x0100, owner, 0x0400)):
            struct.pack_into("<H", directory, entry_index * 0x14 + 0x10, object_offset)
        data_pointer_decoy = struct.pack("<HH", 0x2222, extra_segment)
        stack_pointer_decoy = struct.pack("<HH", 0x3333, stack_segment)
        data_query_decoy = query_before ^ 0x55
        stack_query_decoy = query_before ^ 0xAA
        data_top_decoy = top_before ^ 0xFFFF
        stack_top_decoy = top_before ^ 0x5A5A
        field_decoy = field_before ^ 0xFFFF
        slots_bytes = struct.pack("<16H", *slots_before)
        slots_after_bytes = struct.pack("<16H", *slots_after)
        memory = [
            (game_segment, 0x6724, pointer),
            (data_segment, 0x6724, data_pointer_decoy),
            (stack_segment, 0x6724, stack_pointer_decoy),
            (game_segment, 0x672C, directory_pointer),
            (data_segment, 0x672C, data_pointer_decoy),
            (stack_segment, 0x672C, stack_pointer_decoy),
            (directory_segment, directory_offset, bytes(directory)),
            (game_segment, 0x674E, struct.pack("<H", wildcard)),
            (data_segment, 0x674E, struct.pack("<H", wildcard ^ 0xFFFF)),
            (stack_segment, 0x674E, struct.pack("<H", wildcard ^ 0xA5A5)),
            (game_segment, 0x6782, struct.pack("<H", branch_a_before)),
            (data_segment, 0x6782, struct.pack("<H", branch_a_before ^ 0xFFFF)),
            (stack_segment, 0x6782, struct.pack("<H", branch_a_before ^ 0xA5A5)),
            (game_segment, 0x67AD, bytes([query_before])),
            (data_segment, 0x67AD, bytes([data_query_decoy])),
            (stack_segment, 0x67AD, bytes([stack_query_decoy])),
            (game_segment, 0x6884, struct.pack("<H", top_before)),
            (data_segment, 0x6884, struct.pack("<H", data_top_decoy)),
            (stack_segment, 0x6884, struct.pack("<H", stack_top_decoy)),
            (record_segment, field_effective, struct.pack("<H", field_before)),
            (data_segment, field_effective, struct.pack("<H", field_decoy)),
            (game_segment, field_effective, struct.pack("<H", field_decoy ^ 0xA5A5)),
            (stack_segment, field_effective, struct.pack("<H", field_decoy ^ 0x5A5A)),
            (stack_segment, 0x6D3E, slots_bytes),
            (game_segment, 0x6D3E, b"\xa5" * len(slots_bytes)),
            (data_segment, 0x6D3E, b"\x5a" * len(slots_bytes)),
            (stack_segment, branch_stack_effective, struct.pack("<H", branch_target)),
            (game_segment, branch_stack_effective, struct.pack("<H", branch_target ^ 0xFFFF)),
            (data_segment, branch_stack_effective, struct.pack("<H", branch_target ^ 0xA5A5)),
            (data_segment, start - 1, bytes([opcode])),
            (data_segment, start, script),
            (extra_segment, start - 1, b"\x5a" * (len(script) + 1)),
            (game_segment, start - 1, b"\xa5" * (len(script) + 1)),
        ]
        initial = {
            "eax": 0xA1A1BEEF,
            "ebx": 0xB2B22345,
            "ecx": 0xC3C33456,
            "edx": 0xD4D44567,
            "esi": 0xE5E50000 | start,
            "edi": 0xF6F66789,
            "ebp": 0x9797789A,
            "sp": 0xFF00,
            "ds": data_segment,
            "es": extra_segment,
            "fs": 0x4C00,
            "gs": game_segment,
            "ss": stack_segment,
            "flags": 0x0AD7,
        }
        base_phases = []
        helper_events = []
        terminal_phases = []

        def capture_phases(machine: Uc, address: int, _size: int) -> None:
            if address == 0x694C:
                base_phases.append(
                    (
                        machine.reg_read(UC_X86_REG_DI),
                        machine.reg_read(UC_X86_REG_ES),
                        machine.reg_read(UC_X86_REG_SI),
                    )
                )
            elif address in (0x6462, 0x6034, 0x5FD8, 0x5FF6):
                helper_events.append(
                    (
                        address,
                        machine.reg_read(UC_X86_REG_AX),
                        machine.reg_read(UC_X86_REG_SI),
                        machine.reg_read(UC_X86_REG_DS),
                    )
                )
            elif address == 0x69C5:
                terminal_phases.append(
                    (
                        machine.reg_read(UC_X86_REG_AX),
                        machine.reg_read(UC_X86_REG_BX),
                        machine.reg_read(UC_X86_REG_CX),
                        machine.reg_read(UC_X86_REG_DX),
                        machine.reg_read(UC_X86_REG_SI),
                        machine.reg_read(UC_X86_REG_DI),
                        machine.reg_read(UC_X86_REG_ES),
                        struct.unpack(
                            "<H", machine.mem_read(record_segment * 16 + field_effective, 2)
                        )[0],
                        struct.unpack(
                            "<H", machine.mem_read(game_segment * 16 + 0x6782, 2)
                        )[0],
                        machine.mem_read(game_segment * 16 + 0x67AD, 1)[0],
                        struct.unpack(
                            "<H", machine.mem_read(game_segment * 16 + 0x6884, 2)
                        )[0],
                    )
                )

        machine = execute(
            0x6946,
            0x69C6,
            initial,
            memory,
            code_handler=capture_phases,
        )

        if base_phases != [(base_offset, record_segment, start)]:
            raise AssertionError(
                f"0x6946 {name}: base={base_phases}, "
                f"expected={[(base_offset, record_segment, start)]}"
            )
        if branch_failed:
            expected_helpers = [(0x6462, transformed_value, final_script, data_segment)]
        elif helper_kind == "remove":
            expected_helpers = [
                (0x6034, field_offset, final_script, data_segment),
                (0x5FD8, owner, final_script, data_segment),
            ]
        elif helper_kind == "insert":
            expected_helpers = [
                (0x6034, field_offset, final_script, data_segment),
                (0x5FF6, owner, final_script, data_segment),
            ]
        else:
            expected_helpers = []
        if helper_events != expected_helpers:
            raise AssertionError(
                f"0x6946 {name}: helpers={helper_events}, expected={expected_helpers}"
            )

        if query_path:
            probe_byte = 0xA1 if inverted else field_offset & 0xFF
            expected_dx = (probe_byte << 8) | int(inverted)
            expected_cx = initial["ecx"] & 0xFFFF
            if branch_failed:
                expected_ax = top_after
                expected_si = branch_target
                query_after = 0
                expected_flags = sub_flags_16(top_before, 2)
            else:
                expected_ax = transformed_value
                expected_si = final_script
                query_after = query_before
                expected_flags = logic_flags_8(int(inverted))
        else:
            expected_dx = initial["edx"] & 0xFFFF
            expected_cx = (initial["ecx"] & 0xFF00) | opcode
            expected_si = final_script
            query_after = query_before
            if helper_kind == "insert":
                expected_ax = 0xFFFF if insert_success else owner
                expected_flags = {"cf": insert_success}
            else:
                expected_ax = requested_value
                if helper_kind == "remove":
                    expected_flags = {"cf": owner in slots_before}
                else:
                    expected_flags = cmp_flags_16(requested_value, 0xFFFF)

        expected_terminal = [
            (
                expected_ax,
                field_offset,
                expected_cx,
                expected_dx,
                expected_si,
                base_offset,
                record_segment,
                field_after,
                branch_a_after,
                query_after,
                top_after,
            )
        ]
        if terminal_phases != expected_terminal:
            raise AssertionError(
                f"0x6946 {name}: terminal={terminal_phases}, "
                f"expected={expected_terminal}"
            )

        expected_registers = dict(initial)
        del expected_registers["flags"]
        expected_registers["eax"] = (initial["eax"] & 0xFFFF0000) | expected_ax
        expected_registers["ebx"] = (initial["ebx"] & 0xFFFF0000) | field_offset
        expected_registers["ecx"] = (initial["ecx"] & 0xFFFF0000) | expected_cx
        expected_registers["edx"] = (initial["edx"] & 0xFFFF0000) | expected_dx
        expected_registers["esi"] = (initial["esi"] & 0xFFFF0000) | expected_si
        expected_registers["es"] = record_segment
        for register, expected in expected_registers.items():
            actual = machine.reg_read(REGISTERS[register])
            if actual != expected:
                raise AssertionError(
                    f"0x6946 {name}: {register}={actual:#x}, expected={expected:#x}"
                )

        actual_field = struct.unpack(
            "<H", machine.mem_read(record_segment * 16 + field_effective, 2)
        )[0]
        if actual_field != field_after:
            raise AssertionError(
                f"0x6946 {name}: field={actual_field:#x}, expected={field_after:#x}"
            )
        actual_slots = bytes(machine.mem_read(stack_segment * 16 + 0x6D3E, 32))
        if actual_slots != slots_after_bytes:
            raise AssertionError(
                f"0x6946 {name}: slots={actual_slots.hex()}, "
                f"expected={slots_after_bytes.hex()}"
            )
        if machine.mem_read(data_segment * 16 + 0x6D3E, 32) != b"\x5a" * 32:
            raise AssertionError(f"0x6946 {name}: DS slot decoy changed")
        if machine.mem_read(game_segment * 16 + 0x6D3E, 32) != b"\xa5" * 32:
            raise AssertionError(f"0x6946 {name}: GS slot decoy changed")
        source = bytes(machine.mem_read(data_segment * 16 + start - 1, len(script) + 1))
        if source != bytes([opcode]) + script:
            raise AssertionError(f"0x6946 {name}: script input changed")
        if machine.mem_read(extra_segment * 16 + start - 1, len(script) + 1) != b"\x5a" * (len(script) + 1):
            raise AssertionError(f"0x6946 {name}: ES script decoy changed")
        if machine.mem_read(game_segment * 16 + start - 1, len(script) + 1) != b"\xa5" * (len(script) + 1):
            raise AssertionError(f"0x6946 {name}: GS script decoy changed")

        flags = machine.reg_read(UC_X86_REG_EFLAGS)
        flag_masks = {"cf": 0x0001, "pf": 0x0004, "af": 0x0010, "zf": 0x0040, "sf": 0x0080, "of": 0x0800}
        actual_flags = {
            flag: bool(flags & flag_masks[flag]) for flag in expected_flags
        }
        if actual_flags != expected_flags:
            raise AssertionError(
                f"0x6946 {name}: flags={actual_flags}, expected={expected_flags}"
            )
        if EXE[0x69C6] != 0xC3:
            raise AssertionError("0x6946: expected near RET boundary")

        vectors.append(
            {
                "name": name,
                "query_mode_before": query_before,
                "inverted": inverted,
                "opcode": opcode,
                "field_before": field_before,
                "requested_value": requested_value,
                "wildcard_value": wildcard,
                "field_after": field_after,
                "helper_kind": helper_kind,
                "branch_failed": branch_failed,
                "final_script_offset": expected_si,
                "branch_a_after": branch_a_after,
                "query_mode_after": query_after,
                "branch_stack_top_after": top_after,
                "defined_flags": expected_flags,
            }
        )

    return vectors


def vm_cd_record_triple_vectors() -> list[dict[str, object]]:
    data_segment = 0x4400
    extra_segment = 0x4800
    game_segment = 0x2C00
    record_segment = 0x5200
    directory_segment = 0x5800
    stack_segment = 0x9000
    cases = [
        {"name": "query_exact_pass", "query": 1, "stored": (0xCD, 0x1111, 0x2222)},
        {"name": "query_kind_mismatch_fails", "query": 1, "stored": (0xC4, 0x1111, 0x2222)},
        {"name": "query_second_mismatch_fails", "query": 1, "stored": (0xCD, 0x3333, 0x2222)},
        {"name": "query_third_mismatch_fails", "query": 1, "stored": (0xCD, 0x1111, 0x4444)},
        {"name": "query_inverted_exact_fails", "query": 3, "inverted": True, "stored": (0xCD, 0x1111, 0x2222)},
        {"name": "query_inverted_mismatch_pass", "query": 1, "inverted": True, "stored": (0xC4, 0x1111, 0x2222)},
        {"name": "query_inverted_segment_end", "query": 1, "inverted": True, "stored": (0xC4, 0x1111, 0x2222), "start": 0xFFF9},
        {"name": "query_plain_segment_end", "query": 1, "stored": (0xCD, 0x1111, 0x2222), "start": 0xFFFA},
        {"name": "set_kind_gate", "query": 0, "kind": 0x0200},
        {"name": "set_ui_gate", "query": 0, "kind": 0x0400, "ui": 1},
        {"name": "set_request_gate", "query": 0, "kind": 0x0400, "request": 2},
        {"name": "set_c2_zero", "query": 0, "kind": 0x0400, "c2": 0},
        {"name": "set_c2_success", "query": 0, "kind": 0x0400, "request": 4, "c2": 1},
        {"name": "set_owner_remove_present", "query": 0, "kind": 0x0200, "wildcard": 0x0200, "slots": "remove_present"},
        {"name": "set_third_insert_existing", "query": 0, "kind": 0x0200, "third": 0x7777, "wildcard": 0x7777, "slots": "insert_existing"},
        {"name": "set_third_insert_free", "query": 0, "kind": 0x0400, "third": 0x7777, "wildcard": 0x7777, "slots": "insert_free", "c2": 0},
        {"name": "set_third_insert_full", "query": 0, "kind": 0x0400, "third": 0x7777, "wildcard": 0x7777, "slots": "insert_full"},
        {"name": "set_remove_then_insert", "query": 0, "kind": 0x0200, "third": 0x0200, "wildcard": 0x0200, "slots": "remove_present"},
        {"name": "set_negative_field_offset", "query": 0, "kind": 0x0200, "field_offset": -2},
        {"name": "set_script_segment_end", "query": 0, "kind": 0x0200, "start": 0xFFFA},
    ]
    failure_tops = [2, 0, 1, 5]
    vectors = []

    def logic_flags(value: int, sign_mask: int) -> dict[str, bool]:
        result = value & (0xFFFF if sign_mask == 0x8000 else 0xFF)
        return {
            "cf": False,
            "pf": (result & 0xFF).bit_count() % 2 == 0,
            "zf": result == 0,
            "sf": bool(result & sign_mask),
            "of": False,
        }

    def sub_flags_16(left: int, right: int) -> dict[str, bool]:
        result = (left - right) & 0xFFFF
        return {
            "cf": left < right,
            "pf": (result & 0xFF).bit_count() % 2 == 0,
            "af": (left & 0x0F) < (right & 0x0F),
            "zf": result == 0,
            "sf": bool(result & 0x8000),
            "of": bool(((left ^ right) & (left ^ result)) & 0x8000),
        }

    failure_index = 0
    for case_index, case in enumerate(cases):
        name = str(case["name"])
        query_before = int(case["query"])
        query_path = bool(query_before & 1)
        inverted = bool(case.get("inverted", False))
        start = int(case.get("start", 0x4200 + case_index * 0x20))
        base_offset = 0x1234
        first_record = 0x0800 if query_path else 0x0300
        second_record = 0x1111 if query_path else 0x0500
        third_record = 0x2222 if query_path else int(case.get("third", 0x0700))
        prefix = b"\xa1" if query_path and inverted else b""
        script = prefix + struct.pack("<HHH", first_record, second_record, third_record)
        final_script = (start + len(script)) & 0xFFFF
        owner = 0x0200
        wildcard = int(case.get("wildcard", 0x7777))
        kind = int(case.get("kind", 0x0200))
        field_offset = int(case.get("field_offset", 6))
        field_offset_word = field_offset & 0xFFFF
        field_target = (second_record + field_offset) & 0xFFFF
        field_before = 0xDEAD
        ui_before = int(case.get("ui", 0))
        request_before = int(case.get("request", 0))
        c2_result = int(case.get("c2", 0))
        c2_gate_before = 0xA5
        active_line_before = 0x1357

        if query_path:
            stored_kind, stored_second, stored_third = case["stored"]
            matches = (
                stored_kind == 0xCD
                and stored_second == second_record
                and stored_third == third_record
            )
            branch_failed = matches == inverted
        else:
            stored_kind = stored_second = stored_third = 0
            branch_failed = False
        if branch_failed:
            top_before = failure_tops[failure_index % len(failure_tops)]
            failure_index += 1
            top_after = (top_before - 2) & 0xFFFF
            branch_target = (0x5C00 + case_index * 0x29) & 0xFFFF
            branch_stack_effective = (0x6820 + top_after) & 0xFFFF
        else:
            top_before = 0x2468
            top_after = top_before
            branch_target = 0x5AA5
            branch_stack_effective = 0x681E

        slot_mode = str(case.get("slots", "none"))
        slots_before = [0x3000 + index for index in range(16)]
        if slot_mode == "remove_present":
            slots_before[5] = second_record
        elif slot_mode == "insert_existing":
            slots_before[4] = second_record
        elif slot_mode == "insert_free" or slot_mode == "none":
            slots_before[3] = 0
        elif slot_mode == "insert_full":
            pass
        slots_after = list(slots_before)
        remove_called = not query_path and owner == wildcard
        insert_called = not query_path and third_record == wildcard
        if remove_called and second_record in slots_after:
            slots_after[slots_after.index(second_record)] = 0
        insert_success = False
        if insert_called:
            if second_record in slots_after:
                insert_success = True
            elif 0 in slots_after:
                insert_success = True
                slots_after[slots_after.index(0)] = second_record

        if query_path:
            field_after = field_before
            c2_called = False
        elif insert_called and not insert_success:
            field_after = field_before
            c2_called = False
        else:
            field_after = 0xFFFF if insert_called else third_record
            c2_called = (
                (ui_before & 1) == 0
                and (request_before & 2) == 0
                and kind == 0x0400
            )
        if c2_called and c2_result != 0:
            c2_gate_after = 0
            request_after = request_before | 2
            active_line_after = 0x2B
        else:
            c2_gate_after = c2_gate_before
            request_after = request_before
            active_line_after = active_line_before

        directory_offset = 0x2000
        directory = bytearray(0x3C)
        for entry_index, object_offset in enumerate((0x0100, owner, 0x0400)):
            struct.pack_into("<H", directory, entry_index * 0x14 + 0x10, object_offset)
        pointer = struct.pack("<HH", base_offset, record_segment)
        directory_pointer = struct.pack("<HH", directory_offset, directory_segment)
        slots_bytes = struct.pack("<16H", *slots_before)
        slots_after_bytes = struct.pack("<16H", *slots_after)
        bit_index = (kind & -kind).bit_length() - 1
        field_table_offset = 0x6D60 + (0x11 << 4) + bit_index
        memory = [
            (game_segment, 0x6724, pointer),
            (data_segment, 0x6724, struct.pack("<HH", 0x2222, extra_segment)),
            (stack_segment, 0x6724, struct.pack("<HH", 0x3333, stack_segment)),
            (game_segment, 0x672C, directory_pointer),
            (directory_segment, directory_offset, bytes(directory)),
            (game_segment, 0x674E, struct.pack("<H", wildcard)),
            (game_segment, 0x67AD, bytes([query_before])),
            (game_segment, 0x6884, struct.pack("<H", top_before)),
            (game_segment, 0x2793, bytes([ui_before])),
            (game_segment, 0x67AA, bytes([request_before])),
            (game_segment, 0x1FB2, bytes([c2_gate_before])),
            (game_segment, 0x6788, struct.pack("<H", active_line_before)),
            (game_segment, field_table_offset, bytes([field_offset_word & 0xFF])),
            (data_segment, field_table_offset, b"\x55"),
            (stack_segment, 0x6D3E, slots_bytes),
            (data_segment, 0x6D3E, b"\x5a" * len(slots_bytes)),
            (game_segment, 0x6D3E, b"\xa5" * len(slots_bytes)),
            (stack_segment, branch_stack_effective, struct.pack("<H", branch_target)),
            (game_segment, branch_stack_effective, struct.pack("<H", branch_target ^ 0xFFFF)),
            (data_segment, start, script),
            (extra_segment, start, b"\x5a" * len(script)),
            (game_segment, start, b"\xa5" * len(script)),
            (0, 0x7409, b"\xb8" + struct.pack("<H", c2_result) + b"\xcb"),
        ]
        if query_path:
            memory.extend(
                [
                    (
                        record_segment,
                        first_record,
                        struct.pack("<HHH", stored_kind, stored_second, stored_third),
                    ),
                    (
                        record_segment,
                        (base_offset + first_record) & 0xFFFF,
                        b"\xa5" * 6,
                    ),
                ]
            )
        else:
            memory.extend(
                [
                    (record_segment, owner + 2, b"\x81"),
                    (record_segment, third_record + 2, b"\x42"),
                    (record_segment, second_record, struct.pack("<H", kind)),
                    (record_segment, second_record + 2, b"\x24"),
                    (record_segment, field_target, struct.pack("<H", field_before)),
                    (
                        record_segment,
                        (base_offset + second_record) & 0xFFFF,
                        b"\xa5" * 8,
                    ),
                ]
            )

        initial = {
            "eax": 0xA1A1BEEF,
            "ebx": 0xB2B22345,
            "ecx": 0xC3C33456,
            "edx": 0xD4D44567,
            "esi": 0xE5E50000 | start,
            "edi": 0x00006789,
            "ebp": 0x9797789A,
            "sp": 0xFF00,
            "ds": data_segment,
            "es": extra_segment,
            "fs": 0x4C00,
            "gs": game_segment,
            "ss": stack_segment,
            "flags": 0x0AD7,
        }
        base_phases = []
        read_phases = []
        helper_events = []
        c2_events = []
        terminal_phases = []

        def capture_phases(machine: Uc, address: int, _size: int) -> None:
            if address == 0x69CE:
                base_phases.append(
                    (
                        machine.reg_read(UC_X86_REG_DI),
                        machine.reg_read(UC_X86_REG_ES),
                        machine.reg_read(UC_X86_REG_SI),
                    )
                )
            elif address in (0x6A20, 0x6A25, 0x6A2C):
                read_phases.append(address)
            elif address in (0x6462, 0x6034, 0x6023, 0x5FD8, 0x5FF6):
                helper_events.append(
                    (
                        address,
                        machine.reg_read(UC_X86_REG_AX),
                        machine.reg_read(UC_X86_REG_BX),
                        machine.reg_read(UC_X86_REG_SI),
                    )
                )
            elif address == 0x7409:
                c2_events.append(
                    (
                        machine.reg_read(UC_X86_REG_AX),
                        machine.reg_read(UC_X86_REG_BX),
                        machine.reg_read(UC_X86_REG_SI),
                        machine.reg_read(UC_X86_REG_DI),
                        machine.reg_read(UC_X86_REG_ES),
                        machine.reg_read(UC_X86_REG_SP),
                    )
                )
            elif address == 0x6AA4:
                terminal_phases.append(
                    (
                        machine.reg_read(UC_X86_REG_EAX),
                        machine.reg_read(UC_X86_REG_BX),
                        machine.reg_read(UC_X86_REG_CX),
                        machine.reg_read(UC_X86_REG_DX),
                        machine.reg_read(UC_X86_REG_SI),
                        machine.reg_read(UC_X86_REG_BP),
                        machine.reg_read(UC_X86_REG_DI),
                        machine.reg_read(UC_X86_REG_ES),
                    )
                )

        machine = execute(
            0x69C7,
            0x6AA6,
            initial,
            memory,
            code_handler=capture_phases,
        )

        if base_phases != [(base_offset, record_segment, start)]:
            raise AssertionError(f"0x69c7 {name}: wrong initial far-base phase")
        if read_phases != ([] if query_path else [0x6A20, 0x6A25, 0x6A2C]):
            raise AssertionError(
                f"0x69c7 {name}: reads={read_phases}, expected set-path probes"
            )

        if query_path:
            if branch_failed:
                expected_helpers = [(0x6462, third_record, first_record, final_script)]
            else:
                expected_helpers = []
        else:
            expected_helpers = [
                (0x6034, first_record, initial["ebx"] & 0xFFFF, (start + 2) & 0xFFFF),
                (0x6023, 0x11, kind, final_script),
            ]
            if remove_called:
                expected_helpers.append((0x5FD8, second_record, kind, final_script))
            expected_helpers.append((0x6023, 0x11, kind, final_script))
            if insert_called:
                expected_helpers.append((0x5FF6, second_record, kind, final_script))
        if helper_events != expected_helpers:
            raise AssertionError(
                f"0x69c7 {name}: helpers={helper_events}, expected={expected_helpers}"
            )

        if c2_called:
            expected_c2 = [
                (
                    field_offset_word,
                    kind,
                    final_script,
                    (second_record + 4) & 0xFFFF,
                    record_segment,
                    0xFEF8,
                )
            ]
        else:
            expected_c2 = []
        if c2_events != expected_c2:
            raise AssertionError(
                f"0x69c7 {name}: c2={c2_events}, expected={expected_c2}"
            )

        if query_path:
            probe_byte = 0xA1 if inverted else first_record & 0xFF
            expected_dx = (probe_byte << 8) | int(inverted)
            expected_bx = first_record
            expected_cx = initial["ecx"] & 0xFFFF
            expected_bp = second_record
            terminal_di = base_offset
            if branch_failed:
                expected_ax16 = top_after
                expected_si = branch_target
                query_after = 0
                expected_flags = sub_flags_16(top_before, 2)
            else:
                expected_ax16 = third_record
                expected_si = final_script
                query_after = query_before
                expected_flags = logic_flags(int(inverted), 0x80)
            expected_eax = (initial["eax"] & 0xFFFF0000) | expected_ax16
        else:
            expected_dx = second_record
            expected_bx = kind
            expected_cx = 0xFFFF if insert_called else third_record
            expected_bp = (second_record + field_offset) & 0xFFFF
            expected_si = final_script
            query_after = query_before
            terminal_di = (second_record + 4) & 0xFFFF if c2_called else second_record
            signed_eax = field_offset_word
            if field_offset < 0:
                signed_eax |= 0xFFFF0000
            if c2_called:
                expected_eax = (signed_eax & 0xFFFF0000) | c2_result
            else:
                expected_eax = signed_eax
            if insert_called and not insert_success:
                expected_flags = {"cf": False}
            elif ui_before & 1:
                expected_flags = logic_flags(ui_before & 1, 0x80)
            elif request_before & 2:
                expected_flags = logic_flags(request_before & 2, 0x80)
            elif kind != 0x0400:
                expected_flags = sub_flags_16(kind, 0x0400)
            elif c2_result == 0:
                expected_flags = logic_flags(0, 0x8000)
            else:
                expected_flags = logic_flags(request_after, 0x80)

        expected_terminal = [
            (
                expected_eax,
                expected_bx,
                expected_cx,
                expected_dx,
                expected_si,
                expected_bp,
                terminal_di,
                record_segment,
            )
        ]
        if terminal_phases != expected_terminal:
            raise AssertionError(
                f"0x69c7 {name}: terminal={terminal_phases}, "
                f"expected={expected_terminal}"
            )

        expected_registers = dict(initial)
        del expected_registers["flags"]
        expected_registers["eax"] = expected_eax
        expected_registers["ebx"] = (initial["ebx"] & 0xFFFF0000) | expected_bx
        expected_registers["ecx"] = (initial["ecx"] & 0xFFFF0000) | expected_cx
        expected_registers["edx"] = (initial["edx"] & 0xFFFF0000) | expected_dx
        expected_registers["esi"] = (initial["esi"] & 0xFFFF0000) | expected_si
        expected_registers["ebp"] = (initial["ebp"] & 0xFFFF0000) | expected_bp
        for register, expected in expected_registers.items():
            actual = machine.reg_read(REGISTERS[register])
            if actual != expected:
                raise AssertionError(
                    f"0x69c7 {name}: {register}={actual:#x}, expected={expected:#x}"
                )

        if not query_path:
            actual_field = struct.unpack(
                "<H", machine.mem_read(record_segment * 16 + field_target, 2)
            )[0]
            if actual_field != field_after:
                raise AssertionError(
                    f"0x69c7 {name}: field={actual_field:#x}, expected={field_after:#x}"
                )
        actual_slots = bytes(machine.mem_read(stack_segment * 16 + 0x6D3E, 32))
        if actual_slots != slots_after_bytes:
            raise AssertionError(f"0x69c7 {name}: special-slot state mismatch")
        if bytes(machine.mem_read(data_segment * 16 + start, len(script))) != script:
            raise AssertionError(f"0x69c7 {name}: script input changed")

        actual_globals = (
            machine.mem_read(game_segment * 16 + 0x1FB2, 1)[0],
            machine.mem_read(game_segment * 16 + 0x67AA, 1)[0],
            struct.unpack(
                "<H", machine.mem_read(game_segment * 16 + 0x6788, 2)
            )[0],
            machine.mem_read(game_segment * 16 + 0x67AD, 1)[0],
            struct.unpack(
                "<H", machine.mem_read(game_segment * 16 + 0x6884, 2)
            )[0],
        )
        expected_globals = (
            c2_gate_after,
            request_after,
            active_line_after,
            query_after,
            top_after,
        )
        if actual_globals != expected_globals:
            raise AssertionError(
                f"0x69c7 {name}: globals={actual_globals}, expected={expected_globals}"
            )

        flags = machine.reg_read(UC_X86_REG_EFLAGS)
        flag_masks = {"cf": 0x0001, "pf": 0x0004, "af": 0x0010, "zf": 0x0040, "sf": 0x0080, "of": 0x0800}
        actual_flags = {
            flag: bool(flags & flag_masks[flag]) for flag in expected_flags
        }
        if actual_flags != expected_flags:
            raise AssertionError(
                f"0x69c7 {name}: flags={actual_flags}, expected={expected_flags}"
            )
        if EXE[0x6AA6] != 0xC3:
            raise AssertionError("0x69c7: expected near RET boundary")

        vectors.append(
            {
                "name": name,
                "query_mode_before": query_before,
                "inverted": inverted,
                "first_record": first_record,
                "second_record": second_record,
                "third_record": third_record,
                "kind": kind if not query_path else None,
                "field_offset": field_offset if not query_path else None,
                "field_after": field_after if not query_path else None,
                "remove_called": remove_called,
                "insert_called": insert_called,
                "insert_success": insert_success,
                "c2_called": c2_called,
                "c2_result": c2_result if c2_called else None,
                "branch_failed": branch_failed,
                "final_script_offset": expected_si,
                "defined_flags": expected_flags,
            }
        )

    return vectors


def vm_b7_record_bit_vectors() -> list[dict[str, object]]:
    data_segment = 0x4400
    extra_segment = 0x4800
    game_segment = 0x2C00
    record_segment = 0x5200
    stack_segment = 0x9000
    cases = [
        {
            "name": "query_high_bit_set_pass",
            "query": 1,
            "bit": 0,
            "field": 0x80,
        },
        {
            "name": "query_high_bit_clear_fails",
            "query": 1,
            "bit": 0,
            "field": 0x7F,
        },
        {
            "name": "query_inverted_set_fails",
            "query": 3,
            "inverted": True,
            "bit": 3,
            "field": 0x10,
        },
        {
            "name": "query_inverted_clear_pass",
            "query": 1,
            "inverted": True,
            "bit": 3,
            "field": 0xEF,
        },
        {
            "name": "query_low_bit_set_pass",
            "query": 1,
            "bit": 7,
            "field": 0x01,
        },
        {
            "name": "query_next_byte",
            "query": 1,
            "bit": 8,
            "field": 0x80,
        },
        {
            "name": "query_maximum_bit_index",
            "query": 1,
            "bit": 0xFF,
            "field": 0x01,
        },
        {
            "name": "query_inverted_segment_end",
            "query": 1,
            "inverted": True,
            "bit": 6,
            "field": 0x00,
            "start": 0xFFFC,
        },
        {
            "name": "set_high_bit",
            "query": 0,
            "bit": 0,
            "field": 0x01,
        },
        {
            "name": "set_low_bit_already_set",
            "query": 2,
            "bit": 7,
            "field": 0x81,
        },
        {
            "name": "set_middle_bit",
            "query": 0,
            "bit": 13,
            "field": 0x40,
        },
        {
            "name": "clear_middle_bit",
            "query": 0,
            "inverted": True,
            "bit": 2,
            "field": 0xFF,
        },
        {
            "name": "clear_absent_bit",
            "query": 2,
            "inverted": True,
            "bit": 6,
            "field": 0x02,
        },
        {
            "name": "record_offset_wrap",
            "query": 0,
            "bit": 8,
            "field": 0x00,
            "base": 0xFFF0,
            "offset": 0x000F,
        },
    ]
    failure_tops = [2, 0, 1, 5]
    vectors = []

    def logic_flags_8(value: int) -> dict[str, bool]:
        result = value & 0xFF
        return {
            "cf": False,
            "pf": result.bit_count() % 2 == 0,
            "zf": result == 0,
            "sf": bool(result & 0x80),
            "of": False,
        }

    def sub_flags_16(left: int, right: int) -> dict[str, bool]:
        result = (left - right) & 0xFFFF
        return {
            "cf": left < right,
            "pf": (result & 0xFF).bit_count() % 2 == 0,
            "af": (left & 0x0F) < (right & 0x0F),
            "zf": result == 0,
            "sf": bool(result & 0x8000),
            "of": bool(((left ^ right) & (left ^ result)) & 0x8000),
        }

    failure_index = 0
    for case_index, case in enumerate(cases):
        name = str(case["name"])
        query_before = int(case["query"])
        query_path = bool(query_before & 1)
        inverted = bool(case.get("inverted", False))
        bit_index = int(case["bit"])
        remainder = bit_index & 7
        byte_advance = bit_index >> 3
        mask = 0x80 >> remainder
        field_before = int(case["field"])
        base_offset = int(case.get("base", 0x1400))
        offset = int(case.get("offset", 0x0320 + case_index * 0x10))
        field_index = (offset + byte_advance) & 0xFFFF
        field_effective = (base_offset + field_index) & 0xFFFF
        start = int(case.get("start", 0x4600 + case_index * 0x20))
        prefix = b"\xa1" if inverted else b""
        script = prefix + struct.pack("<HB", offset, bit_index)
        final_script = (start + len(script)) & 0xFFFF
        is_set = bool(field_before & mask)
        branch_failed = query_path and is_set == inverted
        if branch_failed:
            top_before = failure_tops[failure_index % len(failure_tops)]
            failure_index += 1
            top_after = (top_before - 2) & 0xFFFF
            branch_target = (0x5A00 + case_index * 0x31) & 0xFFFF
            branch_stack_effective = (0x6820 + top_after) & 0xFFFF
        else:
            top_before = 0x2468
            top_after = top_before
            branch_target = 0x5AA5
            branch_stack_effective = 0x681E

        if query_path:
            field_after = field_before
        elif inverted:
            field_after = field_before & (~mask & 0xFF)
        else:
            field_after = field_before | mask

        pointer = struct.pack("<HH", base_offset, record_segment)
        data_pointer_decoy = struct.pack("<HH", 0x2222, extra_segment)
        stack_pointer_decoy = struct.pack("<HH", 0x3333, stack_segment)
        data_query_decoy = query_before ^ 0x55
        stack_query_decoy = query_before ^ 0xAA
        data_top_decoy = top_before ^ 0xFFFF
        field_decoy = field_before ^ 0xFF
        memory = [
            (game_segment, 0x6724, pointer),
            (data_segment, 0x6724, data_pointer_decoy),
            (stack_segment, 0x6724, stack_pointer_decoy),
            (game_segment, 0x67AD, bytes([query_before])),
            (data_segment, 0x67AD, bytes([data_query_decoy])),
            (stack_segment, 0x67AD, bytes([stack_query_decoy])),
            (game_segment, 0x6884, struct.pack("<H", top_before)),
            (data_segment, 0x6884, struct.pack("<H", data_top_decoy)),
            (record_segment, field_effective, bytes([field_before])),
            (data_segment, field_effective, bytes([field_decoy])),
            (extra_segment, field_effective, bytes([field_decoy ^ 0xA5])),
            (game_segment, field_effective, bytes([field_decoy ^ 0x5A])),
            (stack_segment, field_effective, bytes([field_decoy ^ 0x3C])),
            (stack_segment, branch_stack_effective, struct.pack("<H", branch_target)),
            (
                game_segment,
                branch_stack_effective,
                struct.pack("<H", branch_target ^ 0xFFFF),
            ),
            (
                data_segment,
                branch_stack_effective,
                struct.pack("<H", branch_target ^ 0xA5A5),
            ),
            (data_segment, start, script),
            (extra_segment, start, b"\x5a" * len(script)),
            (game_segment, start, b"\xa5" * len(script)),
        ]
        initial = {
            "eax": 0xA1A1BEEF,
            "ebx": 0xB2B22345,
            "ecx": 0xC3C33456,
            "edx": 0xD4D44567,
            "esi": 0xE5E50000 | start,
            "edi": 0xF6F66789,
            "ebp": 0x9797789A,
            "sp": 0xFF00,
            "ds": data_segment,
            "es": extra_segment,
            "fs": 0x4C00,
            "gs": game_segment,
            "ss": stack_segment,
            "flags": 0x0AD7,
        }
        base_phases = []
        parse_phases = []
        terminal_phases = []

        def capture_phases(machine: Uc, address: int, _size: int) -> None:
            if address == 0x6AAD:
                base_phases.append(
                    (
                        machine.reg_read(UC_X86_REG_DI),
                        machine.reg_read(UC_X86_REG_ES),
                        machine.reg_read(UC_X86_REG_SI),
                    )
                )
            elif address == 0x6AC8:
                parse_phases.append(
                    (
                        machine.reg_read(UC_X86_REG_AX),
                        machine.reg_read(UC_X86_REG_BX),
                        machine.reg_read(UC_X86_REG_CX),
                        machine.reg_read(UC_X86_REG_DX),
                        machine.reg_read(UC_X86_REG_SI),
                        machine.reg_read(UC_X86_REG_DI),
                        machine.reg_read(UC_X86_REG_ES),
                    )
                )
            elif address == 0x6B04:
                terminal_phases.append(
                    (
                        machine.reg_read(UC_X86_REG_AX),
                        machine.reg_read(UC_X86_REG_BX),
                        machine.reg_read(UC_X86_REG_CX),
                        machine.reg_read(UC_X86_REG_DX),
                        machine.reg_read(UC_X86_REG_SI),
                        machine.reg_read(UC_X86_REG_DI),
                        machine.reg_read(UC_X86_REG_ES),
                        machine.mem_read(record_segment * 16 + field_effective, 1)[0],
                        machine.mem_read(game_segment * 16 + 0x67AD, 1)[0],
                        struct.unpack(
                            "<H", machine.mem_read(game_segment * 16 + 0x6884, 2)
                        )[0],
                    )
                )

        machine = execute(
            0x6AA7,
            0x6B05,
            initial,
            memory,
            code_handler=capture_phases,
        )

        expected_base = [(base_offset, record_segment, start)]
        if base_phases != expected_base:
            raise AssertionError(
                f"0x6aa7 {name}: base={base_phases}, expected={expected_base}"
            )

        dx_with_inversion = (initial["edx"] & 0xFF00) | int(inverted)
        expected_parse = [
            (
                byte_advance,
                field_index,
                remainder,
                dx_with_inversion,
                final_script,
                base_offset,
                record_segment,
            )
        ]
        if parse_phases != expected_parse:
            raise AssertionError(
                f"0x6aa7 {name}: parse={parse_phases}, expected={expected_parse}"
            )

        if branch_failed:
            expected_ax = top_after
            expected_si = branch_target
            expected_cx = remainder
            query_after = 0
            expected_flags = sub_flags_16(top_before, 2)
        elif query_path:
            shifted = (field_before << remainder) & 0xFF
            expected_ax = (shifted << 1) & 0xFF
            expected_si = final_script
            expected_cx = remainder
            query_after = query_before
            expected_flags = logic_flags_8(int(inverted))
        else:
            expected_ax = (~mask & 0xFF) if inverted else mask
            expected_si = final_script
            expected_cx = 7 - remainder
            query_after = query_before
            expected_flags = logic_flags_8(field_after)

        expected_terminal = [
            (
                expected_ax,
                field_index,
                expected_cx,
                dx_with_inversion,
                expected_si,
                base_offset,
                record_segment,
                field_after,
                query_after,
                top_after,
            )
        ]
        if terminal_phases != expected_terminal:
            raise AssertionError(
                f"0x6aa7 {name}: terminal={terminal_phases}, "
                f"expected={expected_terminal}"
            )

        expected_registers = dict(initial)
        del expected_registers["flags"]
        expected_registers["eax"] = (initial["eax"] & 0xFFFF0000) | expected_ax
        expected_registers["ebx"] = (
            initial["ebx"] & 0xFFFF0000
        ) | field_index
        expected_registers["ecx"] = (
            initial["ecx"] & 0xFFFF0000
        ) | expected_cx
        expected_registers["edx"] = (
            initial["edx"] & 0xFFFF0000
        ) | dx_with_inversion
        expected_registers["esi"] = (
            initial["esi"] & 0xFFFF0000
        ) | expected_si
        expected_registers["es"] = record_segment
        for register, expected in expected_registers.items():
            actual = machine.reg_read(REGISTERS[register])
            if actual != expected:
                raise AssertionError(
                    f"0x6aa7 {name}: {register}={actual:#x}, expected={expected:#x}"
                )

        actual_field = machine.mem_read(record_segment * 16 + field_effective, 1)[0]
        if actual_field != field_after:
            raise AssertionError(
                f"0x6aa7 {name}: field={actual_field:#x}, expected={field_after:#x}"
            )
        if bytes(machine.mem_read(data_segment * 16 + start, len(script))) != script:
            raise AssertionError(f"0x6aa7 {name}: script input changed")
        if (
            machine.mem_read(extra_segment * 16 + start, len(script))
            != b"\x5a" * len(script)
        ):
            raise AssertionError(f"0x6aa7 {name}: ES script decoy changed")
        if (
            machine.mem_read(game_segment * 16 + start, len(script))
            != b"\xa5" * len(script)
        ):
            raise AssertionError(f"0x6aa7 {name}: GS script decoy changed")
        decoys = [
            (data_segment, 0x6724, data_pointer_decoy),
            (stack_segment, 0x6724, stack_pointer_decoy),
            (data_segment, 0x67AD, bytes([data_query_decoy])),
            (stack_segment, 0x67AD, bytes([stack_query_decoy])),
            (data_segment, 0x6884, struct.pack("<H", data_top_decoy)),
            (data_segment, field_effective, bytes([field_decoy])),
            (extra_segment, field_effective, bytes([field_decoy ^ 0xA5])),
            (game_segment, field_effective, bytes([field_decoy ^ 0x5A])),
            (stack_segment, field_effective, bytes([field_decoy ^ 0x3C])),
        ]
        for segment, decoy_offset, expected in decoys:
            actual = bytes(
                machine.mem_read(segment * 16 + decoy_offset, len(expected))
            )
            if actual != expected:
                raise AssertionError(f"0x6aa7 {name}: segment decoy changed")

        flags = machine.reg_read(UC_X86_REG_EFLAGS)
        flag_masks = {
            "cf": 0x0001,
            "pf": 0x0004,
            "af": 0x0010,
            "zf": 0x0040,
            "sf": 0x0080,
            "of": 0x0800,
        }
        actual_flags = {
            flag: bool(flags & flag_masks[flag]) for flag in expected_flags
        }
        if actual_flags != expected_flags:
            raise AssertionError(
                f"0x6aa7 {name}: flags={actual_flags}, expected={expected_flags}"
            )
        if EXE[0x6B05] != 0xC3:
            raise AssertionError("0x6aa7: expected near RET boundary")

        vectors.append(
            {
                "name": name,
                "query_mode_before": query_before,
                "inverted": inverted,
                "record_base_offset": base_offset,
                "record_offset": offset,
                "bit_index": bit_index,
                "mask": mask,
                "field_before": field_before,
                "field_after": field_after,
                "branch_failed": branch_failed,
                "final_script_offset": expected_si,
                "query_mode_after": query_after,
                "branch_stack_top_after": top_after,
                "defined_flags": expected_flags,
            }
        )

    return vectors


def vm_b8_record_pair_vectors() -> list[dict[str, object]]:
    data_segment = 0x4400
    extra_segment = 0x4800
    game_segment = 0x2C00
    record_segment = 0x5200
    directory_segment = 0x5800
    stack_segment = 0x9000
    cases = [
        {
            "name": "query_exact_pass",
            "query": 1,
            "stored": (0x1111, 0x2222),
        },
        {
            "name": "query_first_mismatch_fails",
            "query": 1,
            "stored": (0x3333, 0x2222),
        },
        {
            "name": "query_second_mismatch_fails",
            "query": 1,
            "stored": (0x1111, 0x4444),
        },
        {
            "name": "query_odd_mode_three_pass",
            "query": 3,
            "stored": (0x1111, 0x2222),
        },
        {
            "name": "query_script_segment_end",
            "query": 1,
            "stored": (0x1111, 0x2222),
            "start": 0xFFFA,
        },
        {
            "name": "set_clear_matching_link",
            "query": 0,
            "link_matches": True,
        },
        {
            "name": "set_preserve_mismatched_link",
            "query": 0,
            "link": 0x9999,
        },
        {
            "name": "set_even_nonzero_mode",
            "query": 2,
            "link_matches": True,
        },
        {
            "name": "set_effective_offset_wrap",
            "query": 0,
            "base": 0xFFF0,
            "offset": 0x0020,
            "link_matches": True,
        },
        {
            "name": "set_pair_first_word_at_ffff",
            "query": 0,
            "base": 0xFFF0,
            "offset": 0x000F,
        },
    ]
    failure_tops = [2, 0, 1, 5]
    vectors = []

    def sub_flags_16(left: int, right: int) -> dict[str, bool]:
        result = (left - right) & 0xFFFF
        return {
            "cf": left < right,
            "pf": (result & 0xFF).bit_count() % 2 == 0,
            "af": (left & 0x0F) < (right & 0x0F),
            "zf": result == 0,
            "sf": bool(result & 0x8000),
            "of": bool(((left ^ right) & (left ^ result)) & 0x8000),
        }

    failure_index = 0
    for case_index, case in enumerate(cases):
        name = str(case["name"])
        query_before = int(case["query"])
        query_path = bool(query_before & 1)
        base_offset = int(case.get("base", 0x1200))
        offset = int(case.get("offset", 0x0300 + case_index * 0x20))
        record_offset = (base_offset + offset) & 0xFFFF
        first = 0x1111
        second = 0x2222
        stored_first, stored_second = case.get("stored", (0xA5A5, 0x5A5A))
        start = int(case.get("start", 0x4A00 + case_index * 0x20))
        script = struct.pack("<HHH", offset, first, second)
        final_script = (start + len(script)) & 0xFFFF
        branch_failed = query_path and (
            stored_first != first or stored_second != second
        )
        if branch_failed:
            top_before = failure_tops[failure_index % len(failure_tops)]
            failure_index += 1
            top_after = (top_before - 2) & 0xFFFF
            branch_target = (0x5E00 + case_index * 0x2B) & 0xFFFF
            branch_stack_effective = (0x6820 + top_after) & 0xFFFF
        else:
            top_before = 0x2468
            top_after = top_before
            branch_target = 0x5AA5
            branch_stack_effective = 0x681E

        owner = 0x0777
        secondary_offset = 0x0900
        secondary_link_offset = secondary_offset + 0x16
        link_before = (
            owner if bool(case.get("link_matches", False)) else int(case.get("link", 0x8888))
        )
        relative_link_offset = (base_offset + secondary_link_offset) & 0xFFFF
        relative_link_decoy = link_before ^ 0xFFFF
        if query_path:
            pair_after = (stored_first, stored_second)
            link_after = link_before
            owner_called = False
        else:
            pair_after = (first, second)
            link_after = 0 if link_before == owner else link_before
            owner_called = True

        pointer = struct.pack("<HH", base_offset, record_segment)
        data_pointer_decoy = struct.pack("<HH", 0x2222, extra_segment)
        stack_pointer_decoy = struct.pack("<HH", 0x3333, stack_segment)
        data_query_decoy = query_before ^ 0x55
        stack_query_decoy = query_before ^ 0xAA
        data_top_decoy = top_before ^ 0xFFFF
        directory_offset = 0x2000
        directory_pointer = struct.pack("<HH", directory_offset, directory_segment)
        decoy_directory_pointer = struct.pack("<HH", 0x2400, extra_segment)
        memory = [
            (game_segment, 0x6724, pointer),
            (data_segment, 0x6724, data_pointer_decoy),
            (stack_segment, 0x6724, stack_pointer_decoy),
            (game_segment, 0x672C, directory_pointer),
            (data_segment, 0x672C, decoy_directory_pointer),
            (directory_segment, directory_offset - 4, struct.pack("<H", owner)),
            (directory_segment, directory_offset + 0x10, b"\xff\xff"),
            (game_segment, 0x6752, struct.pack("<H", secondary_offset)),
            (data_segment, 0x6752, struct.pack("<H", secondary_offset ^ 0xFFFF)),
            (stack_segment, 0x6752, struct.pack("<H", secondary_offset ^ 0x5A5A)),
            (game_segment, 0x67AD, bytes([query_before])),
            (data_segment, 0x67AD, bytes([data_query_decoy])),
            (stack_segment, 0x67AD, bytes([stack_query_decoy])),
            (game_segment, 0x6884, struct.pack("<H", top_before)),
            (data_segment, 0x6884, struct.pack("<H", data_top_decoy)),
            (record_segment, record_offset, struct.pack("<H", stored_first)),
            (
                record_segment,
                (record_offset + 2) & 0xFFFF,
                struct.pack("<H", stored_second),
            ),
            (record_segment, secondary_link_offset, struct.pack("<H", link_before)),
            (
                record_segment,
                relative_link_offset,
                struct.pack("<H", relative_link_decoy),
            ),
            (data_segment, record_offset, b"\xad\xde\xad\xde"),
            (game_segment, record_offset, b"\xa5\xa5\xa5\xa5"),
            (stack_segment, record_offset, b"\x5a\x5a\x5a\x5a"),
            (stack_segment, branch_stack_effective, struct.pack("<H", branch_target)),
            (
                game_segment,
                branch_stack_effective,
                struct.pack("<H", branch_target ^ 0xFFFF),
            ),
            (
                data_segment,
                branch_stack_effective,
                struct.pack("<H", branch_target ^ 0xA5A5),
            ),
        ]
        immutable_script = []
        for byte_index, byte in enumerate(script):
            script_offset = start + byte_index
            encoded = bytes([byte])
            memory.append((data_segment, script_offset, encoded))
            memory.append((extra_segment, script_offset, b"\x5a"))
            memory.append((game_segment, script_offset, b"\xa5"))
            immutable_script.append((script_offset, encoded))

        initial = {
            "eax": 0xA1A1BEEF,
            "ebx": 0xB2B22345,
            "ecx": 0xC3C33456,
            "edx": 0xD4D44567,
            "esi": 0xE5E50000 | start,
            "edi": 0xF6F66789,
            "ebp": 0x9797789A,
            "sp": 0xFF00,
            "ds": data_segment,
            "es": extra_segment,
            "fs": 0x4C00,
            "gs": game_segment,
            "ss": stack_segment,
            "flags": 0x0AD7,
        }
        base_phases = []
        parse_phases = []
        helper_events = []
        terminal_phases = []

        def capture_phases(machine: Uc, address: int, _size: int) -> None:
            if address == 0x6B0C:
                base_phases.append(
                    (
                        machine.reg_read(UC_X86_REG_DI),
                        machine.reg_read(UC_X86_REG_ES),
                        machine.reg_read(UC_X86_REG_SI),
                    )
                )
            elif address == 0x6B13:
                parse_phases.append(
                    (
                        machine.reg_read(UC_X86_REG_AX),
                        machine.reg_read(UC_X86_REG_BX),
                        machine.reg_read(UC_X86_REG_SI),
                        machine.reg_read(UC_X86_REG_DI),
                        machine.reg_read(UC_X86_REG_ES),
                    )
                )
            elif address in (0x6462, 0x6034):
                helper_events.append(
                    (
                        address,
                        machine.reg_read(UC_X86_REG_AX),
                        machine.reg_read(UC_X86_REG_BX),
                        machine.reg_read(UC_X86_REG_SI),
                        machine.reg_read(UC_X86_REG_DI),
                        machine.reg_read(UC_X86_REG_ES),
                    )
                )
            elif address == 0x6B4A:
                terminal_phases.append(
                    (
                        machine.reg_read(UC_X86_REG_AX),
                        machine.reg_read(UC_X86_REG_BX),
                        machine.reg_read(UC_X86_REG_SI),
                        machine.reg_read(UC_X86_REG_DI),
                        machine.reg_read(UC_X86_REG_ES),
                        struct.unpack(
                            "<H",
                            machine.mem_read(record_segment * 16 + record_offset, 2),
                        )[0],
                        struct.unpack(
                            "<H",
                            machine.mem_read(
                                record_segment * 16
                                + ((record_offset + 2) & 0xFFFF),
                                2,
                            ),
                        )[0],
                        struct.unpack(
                            "<H",
                            machine.mem_read(
                                record_segment * 16 + secondary_link_offset, 2
                            ),
                        )[0],
                        machine.mem_read(game_segment * 16 + 0x67AD, 1)[0],
                        struct.unpack(
                            "<H", machine.mem_read(game_segment * 16 + 0x6884, 2)
                        )[0],
                    )
                )

        machine = execute(
            0x6B06,
            0x6B4B,
            initial,
            memory,
            code_handler=capture_phases,
        )

        expected_base = [(base_offset, record_segment, start)]
        if base_phases != expected_base:
            raise AssertionError(
                f"0x6b06 {name}: base={base_phases}, expected={expected_base}"
            )
        expected_parse = [
            (second, first, final_script, record_offset, record_segment)
        ]
        if parse_phases != expected_parse:
            raise AssertionError(
                f"0x6b06 {name}: parse={parse_phases}, expected={expected_parse}"
            )

        if branch_failed:
            expected_ax = top_after
            expected_si = branch_target
            query_after = 0
            expected_helpers = [
                (0x6462, second, first, final_script, record_offset, record_segment)
            ]
            expected_flags = sub_flags_16(top_before, 2)
            terminal_di = record_offset
        elif query_path:
            expected_ax = second
            expected_si = final_script
            query_after = query_before
            expected_helpers = []
            expected_flags = sub_flags_16(second, stored_second)
            terminal_di = record_offset
        else:
            expected_ax = owner
            expected_si = final_script
            query_after = query_before
            expected_helpers = [
                (0x6034, record_offset, first, final_script, record_offset, record_segment)
            ]
            expected_flags = sub_flags_16(owner, link_before)
            terminal_di = secondary_offset

        if helper_events != expected_helpers:
            raise AssertionError(
                f"0x6b06 {name}: helpers={helper_events}, expected={expected_helpers}"
            )
        expected_terminal = [
            (
                expected_ax,
                first,
                expected_si,
                terminal_di,
                record_segment,
                pair_after[0],
                pair_after[1],
                link_after,
                query_after,
                top_after,
            )
        ]
        if terminal_phases != expected_terminal:
            raise AssertionError(
                f"0x6b06 {name}: terminal={terminal_phases}, "
                f"expected={expected_terminal}"
            )

        expected_registers = dict(initial)
        del expected_registers["flags"]
        expected_registers["eax"] = (
            initial["eax"] & 0xFFFF0000
        ) | expected_ax
        expected_registers["ebx"] = (initial["ebx"] & 0xFFFF0000) | first
        expected_registers["esi"] = (
            initial["esi"] & 0xFFFF0000
        ) | expected_si
        expected_registers["es"] = record_segment
        for register, expected in expected_registers.items():
            actual = machine.reg_read(REGISTERS[register])
            if actual != expected:
                raise AssertionError(
                    f"0x6b06 {name}: {register}={actual:#x}, expected={expected:#x}"
                )

        actual_pair = (
            struct.unpack(
                "<H", machine.mem_read(record_segment * 16 + record_offset, 2)
            )[0],
            struct.unpack(
                "<H",
                machine.mem_read(
                    record_segment * 16 + ((record_offset + 2) & 0xFFFF), 2
                ),
            )[0],
        )
        if actual_pair != pair_after:
            raise AssertionError(
                f"0x6b06 {name}: pair={actual_pair}, expected={pair_after}"
            )
        actual_link = struct.unpack(
            "<H",
            machine.mem_read(record_segment * 16 + secondary_link_offset, 2),
        )[0]
        if actual_link != link_after:
            raise AssertionError(
                f"0x6b06 {name}: link={actual_link:#x}, expected={link_after:#x}"
            )
        actual_relative_decoy = struct.unpack(
            "<H",
            machine.mem_read(record_segment * 16 + relative_link_offset, 2),
        )[0]
        if actual_relative_decoy != relative_link_decoy:
            raise AssertionError(f"0x6b06 {name}: relative link decoy changed")
        for script_offset, expected in immutable_script:
            actual = bytes(machine.mem_read(data_segment * 16 + script_offset, 1))
            if actual != expected:
                raise AssertionError(f"0x6b06 {name}: script input changed")
            if machine.mem_read(extra_segment * 16 + script_offset, 1) != b"\x5a":
                raise AssertionError(f"0x6b06 {name}: ES script decoy changed")
            if machine.mem_read(game_segment * 16 + script_offset, 1) != b"\xa5":
                raise AssertionError(f"0x6b06 {name}: GS script decoy changed")
        decoys = [
            (data_segment, 0x6724, data_pointer_decoy),
            (stack_segment, 0x6724, stack_pointer_decoy),
            (data_segment, 0x672C, decoy_directory_pointer),
            (data_segment, 0x6752, struct.pack("<H", secondary_offset ^ 0xFFFF)),
            (stack_segment, 0x6752, struct.pack("<H", secondary_offset ^ 0x5A5A)),
            (data_segment, 0x67AD, bytes([data_query_decoy])),
            (stack_segment, 0x67AD, bytes([stack_query_decoy])),
            (data_segment, 0x6884, struct.pack("<H", data_top_decoy)),
        ]
        for segment, decoy_offset, expected in decoys:
            actual = bytes(
                machine.mem_read(segment * 16 + decoy_offset, len(expected))
            )
            if actual != expected:
                raise AssertionError(f"0x6b06 {name}: segment decoy changed")

        flags = machine.reg_read(UC_X86_REG_EFLAGS)
        flag_masks = {
            "cf": 0x0001,
            "pf": 0x0004,
            "af": 0x0010,
            "zf": 0x0040,
            "sf": 0x0080,
            "of": 0x0800,
        }
        actual_flags = {
            flag: bool(flags & flag_masks[flag]) for flag in expected_flags
        }
        if actual_flags != expected_flags:
            raise AssertionError(
                f"0x6b06 {name}: flags={actual_flags}, expected={expected_flags}"
            )
        if EXE[0x6B4B] != 0xC3:
            raise AssertionError("0x6b06: expected near RET boundary")

        vectors.append(
            {
                "name": name,
                "query_mode_before": query_before,
                "record_base_offset": base_offset,
                "script_record_offset": offset,
                "effective_record_offset": record_offset,
                "requested_pair": [first, second],
                "pair_before": [stored_first, stored_second],
                "pair_after": list(pair_after),
                "owner_lookup_called": owner_called,
                "owner": owner,
                "secondary_link_before": link_before,
                "secondary_link_after": link_after,
                "branch_failed": branch_failed,
                "final_script_offset": expected_si,
                "query_mode_after": query_after,
                "branch_stack_top_after": top_after,
                "defined_flags": expected_flags,
            }
        )

    return vectors


def vm_c5_record_match_vectors() -> list[dict[str, object]]:
    data_segment = 0x4400
    extra_segment = 0x4800
    game_segment = 0x2C00
    record_segment = 0x5200
    stack_segment = 0x9000
    cases = [
        {
            "name": "query_exact_pass",
            "query": 1,
            "record_kind": 0x00C5,
            "record_value_matches": True,
        },
        {
            "name": "query_value_mismatch_fails",
            "query": 1,
            "record_kind": 0x00C5,
        },
        {
            "name": "query_kind_mismatch_fails",
            "query": 1,
            "record_kind": 0x00C6,
            "record_value_matches": True,
        },
        {
            "name": "query_inverted_exact_fails",
            "query": 3,
            "inverted": True,
            "record_kind": 0x00C5,
            "record_value_matches": True,
        },
        {
            "name": "query_inverted_value_mismatch_pass",
            "query": 1,
            "inverted": True,
            "record_kind": 0x00C5,
        },
        {
            "name": "query_inverted_kind_mismatch_pass",
            "query": 1,
            "inverted": True,
            "record_kind": 0x00C6,
            "record_value_matches": True,
        },
        {
            "name": "query_inverted_script_end",
            "query": 1,
            "inverted": True,
            "record_kind": 0x00C6,
            "record_value_matches": True,
            "start": 0xFFFB,
        },
        {
            "name": "query_record_word_at_ffff",
            "query": 1,
            "record_kind": 0x00C5,
            "record_value_matches": True,
            "record": 0xFFFF,
        },
        {"name": "set_success", "query": 0},
        {"name": "set_prefixed_success", "query": 2, "inverted": True},
        {"name": "set_inactive_fails", "query": 0, "active": 0},
        {"name": "set_other_active_bits_fail", "query": 0, "active": 0x82},
        {"name": "set_related_kind_fails", "query": 0, "related_kind": 0x0400},
        {"name": "set_occupied_destination_fails", "query": 0, "record_kind": 0x00C4},
    ]
    failure_tops = [2, 0, 1, 5]
    vectors = []

    def logic_flags(value: int, sign_mask: int) -> dict[str, bool]:
        result = value & (0xFFFF if sign_mask == 0x8000 else 0xFF)
        return {
            "cf": False,
            "pf": (result & 0xFF).bit_count() % 2 == 0,
            "zf": result == 0,
            "sf": bool(result & sign_mask),
            "of": False,
        }

    def sub_flags_16(left: int, right: int) -> dict[str, bool]:
        result = (left - right) & 0xFFFF
        return {
            "cf": left < right,
            "pf": (result & 0xFF).bit_count() % 2 == 0,
            "af": (left & 0x0F) < (right & 0x0F),
            "zf": result == 0,
            "sf": bool(result & 0x8000),
            "of": bool(((left ^ right) & (left ^ result)) & 0x8000),
        }

    failure_index = 0
    for case_index, case in enumerate(cases):
        name = str(case["name"])
        query_before = int(case["query"])
        query_path = bool(query_before & 1)
        inverted = bool(case.get("inverted", False))
        base_offset = 0x1234
        record_offset = int(case.get("record", 0x0800 + case_index * 0x20))
        operand = 0x3456 if query_path else 0x2000 + case_index * 0x20
        record_kind = int(case.get("record_kind", 0))
        record_value = (
            operand if bool(case.get("record_value_matches", False)) else 0x7777
        )
        record_tail = 0x9999
        related_kind = int(case.get("related_kind", 0x0200))
        active = int(case.get("active", 1))
        start = int(case.get("start", 0x5000 + case_index * 0x20))
        prefix = b"\xa1" if inverted else b""
        script = prefix + struct.pack("<HH", record_offset, operand)
        final_script = (start + len(script)) & 0xFFFF
        matches = record_value == operand and record_kind == 0x00C5
        if query_path:
            success = matches != inverted
        else:
            success = bool(active & 1) and related_kind == 0x0200 and record_kind == 0
        branch_failed = not success
        if branch_failed:
            top_before = failure_tops[failure_index % len(failure_tops)]
            failure_index += 1
            top_after = (top_before - 2) & 0xFFFF
            branch_target = (0x6200 + case_index * 0x25) & 0xFFFF
            branch_stack_effective = (0x6820 + top_after) & 0xFFFF
        else:
            top_before = 0x2468
            top_after = top_before
            branch_target = 0x5AA5
            branch_stack_effective = 0x681E

        if query_path or not success:
            record_after = (record_kind, record_value, record_tail)
        else:
            record_after = (0x00C5, operand, 0)

        pointer = struct.pack("<HH", base_offset, record_segment)
        data_pointer_decoy = struct.pack("<HH", 0x2222, extra_segment)
        stack_pointer_decoy = struct.pack("<HH", 0x3333, stack_segment)
        data_query_decoy = query_before ^ 0x55
        stack_query_decoy = query_before ^ 0xAA
        data_top_decoy = top_before ^ 0xFFFF
        relative_record = (base_offset + record_offset) & 0xFFFF
        relative_related = (base_offset + operand) & 0xFFFF
        record_decoy = b"\xad\xde\xad\xde\xad\xde"
        related_decoy = b"\x5a\xa5\x5a\xa5"
        memory = [
            (game_segment, 0x6724, pointer),
            (data_segment, 0x6724, data_pointer_decoy),
            (stack_segment, 0x6724, stack_pointer_decoy),
            (game_segment, 0x67AD, bytes([query_before])),
            (data_segment, 0x67AD, bytes([data_query_decoy])),
            (stack_segment, 0x67AD, bytes([stack_query_decoy])),
            (game_segment, 0x6884, struct.pack("<H", top_before)),
            (data_segment, 0x6884, struct.pack("<H", data_top_decoy)),
            (record_segment, record_offset, struct.pack("<H", record_kind)),
            (
                record_segment,
                (record_offset + 2) & 0xFFFF,
                struct.pack("<H", record_value),
            ),
            (
                record_segment,
                (record_offset + 4) & 0xFFFF,
                struct.pack("<H", record_tail),
            ),
            (record_segment, operand, struct.pack("<H", related_kind)),
            (record_segment, (operand + 2) & 0xFFFF, bytes([active])),
            (record_segment, relative_record, record_decoy),
            (record_segment, relative_related, related_decoy),
            (data_segment, record_offset, b"\x11\x22\x33\x44\x55\x66"),
            (game_segment, record_offset, b"\xa1\xa2\xa3\xa4\xa5\xa6"),
            (stack_segment, record_offset, b"\xb1\xb2\xb3\xb4\xb5\xb6"),
            (stack_segment, branch_stack_effective, struct.pack("<H", branch_target)),
            (
                game_segment,
                branch_stack_effective,
                struct.pack("<H", branch_target ^ 0xFFFF),
            ),
            (
                data_segment,
                branch_stack_effective,
                struct.pack("<H", branch_target ^ 0xA5A5),
            ),
        ]
        immutable_script = []
        for byte_index, byte in enumerate(script):
            script_offset = start + byte_index
            encoded = bytes([byte])
            memory.append((data_segment, script_offset, encoded))
            memory.append((extra_segment, script_offset, b"\x5a"))
            memory.append((game_segment, script_offset, b"\xa5"))
            immutable_script.append((script_offset, encoded))

        initial = {
            "eax": 0xA1A1BEEF,
            "ebx": 0xB2B22345,
            "ecx": 0xC3C33456,
            "edx": 0xD4D44567,
            "esi": 0xE5E50000 | start,
            "edi": 0xF6F66789,
            "ebp": 0x9797789A,
            "sp": 0xFF00,
            "ds": data_segment,
            "es": extra_segment,
            "fs": 0x4C00,
            "gs": game_segment,
            "ss": stack_segment,
            "flags": 0x0AD7,
        }
        base_phases = []
        parse_phases = []
        helper_events = []
        terminal_phases = []

        def capture_phases(machine: Uc, address: int, _size: int) -> None:
            if address == 0x6D1E:
                base_phases.append(
                    (
                        machine.reg_read(UC_X86_REG_DI),
                        machine.reg_read(UC_X86_REG_ES),
                        machine.reg_read(UC_X86_REG_SI),
                    )
                )
            elif address == 0x6D2D:
                parse_phases.append(
                    (
                        machine.reg_read(UC_X86_REG_AX),
                        machine.reg_read(UC_X86_REG_BP),
                        machine.reg_read(UC_X86_REG_DX),
                        machine.reg_read(UC_X86_REG_SI),
                        machine.reg_read(UC_X86_REG_DI),
                        machine.reg_read(UC_X86_REG_ES),
                    )
                )
            elif address == 0x6462:
                helper_events.append(
                    (
                        machine.reg_read(UC_X86_REG_AX),
                        machine.reg_read(UC_X86_REG_BX),
                        machine.reg_read(UC_X86_REG_BP),
                        machine.reg_read(UC_X86_REG_DX),
                        machine.reg_read(UC_X86_REG_SI),
                    )
                )
            elif address == 0x6D7E:
                terminal_phases.append(
                    (
                        machine.reg_read(UC_X86_REG_AX),
                        machine.reg_read(UC_X86_REG_BX),
                        machine.reg_read(UC_X86_REG_BP),
                        machine.reg_read(UC_X86_REG_DX),
                        machine.reg_read(UC_X86_REG_SI),
                        machine.reg_read(UC_X86_REG_DI),
                        machine.reg_read(UC_X86_REG_ES),
                        struct.unpack(
                            "<H",
                            machine.mem_read(record_segment * 16 + record_offset, 2),
                        )[0],
                        struct.unpack(
                            "<H",
                            machine.mem_read(
                                record_segment * 16
                                + ((record_offset + 2) & 0xFFFF),
                                2,
                            ),
                        )[0],
                        struct.unpack(
                            "<H",
                            machine.mem_read(
                                record_segment * 16
                                + ((record_offset + 4) & 0xFFFF),
                                2,
                            ),
                        )[0],
                        machine.mem_read(game_segment * 16 + 0x67AD, 1)[0],
                        struct.unpack(
                            "<H", machine.mem_read(game_segment * 16 + 0x6884, 2)
                        )[0],
                    )
                )

        machine = execute(
            0x6D18,
            0x6D7F,
            initial,
            memory,
            code_handler=capture_phases,
        )

        expected_base = [(base_offset, record_segment, start)]
        if base_phases != expected_base:
            raise AssertionError(
                f"0x6d18 {name}: base={base_phases}, expected={expected_base}"
            )
        dx_with_inversion = (initial["edx"] & 0xFF00) | int(inverted)
        expected_parse = [
            (
                operand,
                record_offset,
                dx_with_inversion,
                final_script,
                base_offset,
                record_segment,
            )
        ]
        if parse_phases != expected_parse:
            raise AssertionError(
                f"0x6d18 {name}: parse={parse_phases}, expected={expected_parse}"
            )

        if query_path:
            terminal_bx = initial["ebx"] & 0xFFFF
            if record_value != operand:
                decision_ax = operand
            else:
                decision_ax = record_kind
        else:
            terminal_bx = operand
            if (active & 1) == 0:
                decision_ax = operand
            elif related_kind != 0x0200:
                decision_ax = related_kind
            else:
                decision_ax = record_kind
        if branch_failed:
            expected_ax = top_after
            expected_si = branch_target
            query_after = 0
            expected_helpers = [
                (
                    decision_ax,
                    terminal_bx,
                    record_offset,
                    dx_with_inversion,
                    final_script,
                )
            ]
            expected_flags = sub_flags_16(top_before, 2)
        else:
            expected_ax = decision_ax
            expected_si = final_script
            query_after = query_before
            expected_helpers = []
            if query_path:
                expected_flags = logic_flags(int(inverted), 0x80)
            else:
                expected_flags = logic_flags(record_kind, 0x8000)

        if helper_events != expected_helpers:
            raise AssertionError(
                f"0x6d18 {name}: helpers={helper_events}, expected={expected_helpers}"
            )
        expected_terminal = [
            (
                expected_ax,
                terminal_bx,
                record_offset,
                dx_with_inversion,
                expected_si,
                base_offset,
                record_segment,
                record_after[0],
                record_after[1],
                record_after[2],
                query_after,
                top_after,
            )
        ]
        if terminal_phases != expected_terminal:
            raise AssertionError(
                f"0x6d18 {name}: terminal={terminal_phases}, "
                f"expected={expected_terminal}"
            )

        expected_registers = dict(initial)
        del expected_registers["flags"]
        expected_registers["eax"] = (
            initial["eax"] & 0xFFFF0000
        ) | expected_ax
        expected_registers["ebx"] = (
            initial["ebx"] & 0xFFFF0000
        ) | terminal_bx
        expected_registers["edx"] = (
            initial["edx"] & 0xFFFF0000
        ) | dx_with_inversion
        expected_registers["esi"] = (
            initial["esi"] & 0xFFFF0000
        ) | expected_si
        expected_registers["ebp"] = (
            initial["ebp"] & 0xFFFF0000
        ) | record_offset
        expected_registers["es"] = record_segment
        for register, expected in expected_registers.items():
            actual = machine.reg_read(REGISTERS[register])
            if actual != expected:
                raise AssertionError(
                    f"0x6d18 {name}: {register}={actual:#x}, expected={expected:#x}"
                )

        actual_record = (
            struct.unpack(
                "<H", machine.mem_read(record_segment * 16 + record_offset, 2)
            )[0],
            struct.unpack(
                "<H",
                machine.mem_read(
                    record_segment * 16 + ((record_offset + 2) & 0xFFFF), 2
                ),
            )[0],
            struct.unpack(
                "<H",
                machine.mem_read(
                    record_segment * 16 + ((record_offset + 4) & 0xFFFF), 2
                ),
            )[0],
        )
        if actual_record != record_after:
            raise AssertionError(
                f"0x6d18 {name}: record={actual_record}, expected={record_after}"
            )
        if bytes(machine.mem_read(record_segment * 16 + relative_record, 6)) != record_decoy:
            raise AssertionError(f"0x6d18 {name}: base-relative record decoy changed")
        if bytes(machine.mem_read(record_segment * 16 + relative_related, 4)) != related_decoy:
            raise AssertionError(f"0x6d18 {name}: base-relative related decoy changed")
        for script_offset, expected in immutable_script:
            actual = bytes(machine.mem_read(data_segment * 16 + script_offset, 1))
            if actual != expected:
                raise AssertionError(f"0x6d18 {name}: script input changed")
            if machine.mem_read(extra_segment * 16 + script_offset, 1) != b"\x5a":
                raise AssertionError(f"0x6d18 {name}: ES script decoy changed")
            if machine.mem_read(game_segment * 16 + script_offset, 1) != b"\xa5":
                raise AssertionError(f"0x6d18 {name}: GS script decoy changed")
        decoys = [
            (data_segment, 0x6724, data_pointer_decoy),
            (stack_segment, 0x6724, stack_pointer_decoy),
            (data_segment, 0x67AD, bytes([data_query_decoy])),
            (stack_segment, 0x67AD, bytes([stack_query_decoy])),
            (data_segment, 0x6884, struct.pack("<H", data_top_decoy)),
        ]
        for segment, decoy_offset, expected in decoys:
            actual = bytes(
                machine.mem_read(segment * 16 + decoy_offset, len(expected))
            )
            if actual != expected:
                raise AssertionError(f"0x6d18 {name}: segment decoy changed")

        flags = machine.reg_read(UC_X86_REG_EFLAGS)
        flag_masks = {
            "cf": 0x0001,
            "pf": 0x0004,
            "af": 0x0010,
            "zf": 0x0040,
            "sf": 0x0080,
            "of": 0x0800,
        }
        actual_flags = {
            flag: bool(flags & flag_masks[flag]) for flag in expected_flags
        }
        if actual_flags != expected_flags:
            raise AssertionError(
                f"0x6d18 {name}: flags={actual_flags}, expected={expected_flags}"
            )
        if EXE[0x6D7F] != 0xC3:
            raise AssertionError("0x6d18: expected near RET boundary")

        vectors.append(
            {
                "name": name,
                "query_mode_before": query_before,
                "inverted": inverted,
                "record_base_offset_ignored": base_offset,
                "record_offset": record_offset,
                "operand": operand,
                "record_before": [record_kind, record_value, record_tail],
                "related_kind": related_kind,
                "related_active_byte": active,
                "record_after": list(record_after),
                "branch_failed": branch_failed,
                "final_script_offset": expected_si,
                "query_mode_after": query_after,
                "branch_stack_top_after": top_after,
                "defined_flags": expected_flags,
            }
        )

    return vectors


def vm_c6_record_match_vectors() -> list[dict[str, object]]:
    data_segment = 0x4400
    extra_segment = 0x4800
    game_segment = 0x2C00
    record_segment = 0x5200
    stack_segment = 0x9000
    cases = [
        {
            "name": "query_exact_pass",
            "query": 1,
            "record_kind": 0x00C6,
            "record_value_matches": True,
        },
        {
            "name": "query_value_mismatch_fails",
            "query": 1,
            "record_kind": 0x00C6,
        },
        {
            "name": "query_kind_mismatch_fails",
            "query": 1,
            "record_kind": 0x00C5,
            "record_value_matches": True,
        },
        {
            "name": "query_inverted_exact_fails",
            "query": 3,
            "inverted": True,
            "record_kind": 0x00C6,
            "record_value_matches": True,
        },
        {
            "name": "query_inverted_value_mismatch_pass",
            "query": 1,
            "inverted": True,
            "record_kind": 0x00C6,
        },
        {
            "name": "query_inverted_kind_mismatch_pass",
            "query": 1,
            "inverted": True,
            "record_kind": 0x00C5,
            "record_value_matches": True,
        },
        {
            "name": "query_inverted_script_end",
            "query": 1,
            "inverted": True,
            "record_kind": 0x00C5,
            "record_value_matches": True,
            "start": 0xFFFB,
        },
        {
            "name": "query_record_word_at_ffff",
            "query": 1,
            "record_kind": 0x00C6,
            "record_value_matches": True,
            "record": 0xFFFF,
        },
        {"name": "set_overwrites_empty", "query": 0},
        {
            "name": "set_overwrites_existing",
            "query": 2,
            "record_kind": 0x00C5,
            "record_value": 0xAAAA,
            "record_tail": 0xBBBB,
        },
        {
            "name": "set_prefixed_overwrite",
            "query": 0,
            "inverted": True,
            "record_kind": 0x0400,
        },
    ]
    failure_tops = [2, 0, 1, 5]
    vectors = []

    def logic_flags_8(value: int) -> dict[str, bool]:
        result = value & 0xFF
        return {
            "cf": False,
            "pf": result.bit_count() % 2 == 0,
            "zf": result == 0,
            "sf": bool(result & 0x80),
            "of": False,
        }

    def sub_flags_16(left: int, right: int) -> dict[str, bool]:
        result = (left - right) & 0xFFFF
        return {
            "cf": left < right,
            "pf": (result & 0xFF).bit_count() % 2 == 0,
            "af": (left & 0x0F) < (right & 0x0F),
            "zf": result == 0,
            "sf": bool(result & 0x8000),
            "of": bool(((left ^ right) & (left ^ result)) & 0x8000),
        }

    failure_index = 0
    for case_index, case in enumerate(cases):
        name = str(case["name"])
        query_before = int(case["query"])
        query_path = bool(query_before & 1)
        inverted = bool(case.get("inverted", False))
        base_offset = 0x1234
        record_offset = int(case.get("record", 0x0A00 + case_index * 0x20))
        operand = 0x4567
        record_kind = int(case.get("record_kind", 0))
        record_value = int(
            case.get(
                "record_value",
                operand if bool(case.get("record_value_matches", False)) else 0x7777,
            )
        )
        record_tail = int(case.get("record_tail", 0x9999))
        start = int(case.get("start", 0x5400 + case_index * 0x20))
        prefix = b"\xa1" if inverted else b""
        script = prefix + struct.pack("<HH", record_offset, operand)
        final_script = (start + len(script)) & 0xFFFF
        matches = record_value == operand and record_kind == 0x00C6
        branch_failed = query_path and matches == inverted
        if branch_failed:
            top_before = failure_tops[failure_index % len(failure_tops)]
            failure_index += 1
            top_after = (top_before - 2) & 0xFFFF
            branch_target = (0x6600 + case_index * 0x23) & 0xFFFF
            branch_stack_effective = (0x6820 + top_after) & 0xFFFF
        else:
            top_before = 0x2468
            top_after = top_before
            branch_target = 0x5AA5
            branch_stack_effective = 0x681E

        if query_path:
            record_after = (record_kind, record_value, record_tail)
        else:
            record_after = (0x00C6, operand, 0)

        pointer = struct.pack("<HH", base_offset, record_segment)
        data_pointer_decoy = struct.pack("<HH", 0x2222, extra_segment)
        stack_pointer_decoy = struct.pack("<HH", 0x3333, stack_segment)
        data_query_decoy = query_before ^ 0x55
        stack_query_decoy = query_before ^ 0xAA
        data_top_decoy = top_before ^ 0xFFFF
        relative_record = (base_offset + record_offset) & 0xFFFF
        record_decoy = b"\xad\xde\xad\xde\xad\xde"
        memory = [
            (game_segment, 0x6724, pointer),
            (data_segment, 0x6724, data_pointer_decoy),
            (stack_segment, 0x6724, stack_pointer_decoy),
            (game_segment, 0x67AD, bytes([query_before])),
            (data_segment, 0x67AD, bytes([data_query_decoy])),
            (stack_segment, 0x67AD, bytes([stack_query_decoy])),
            (game_segment, 0x6884, struct.pack("<H", top_before)),
            (data_segment, 0x6884, struct.pack("<H", data_top_decoy)),
            (record_segment, record_offset, struct.pack("<H", record_kind)),
            (
                record_segment,
                (record_offset + 2) & 0xFFFF,
                struct.pack("<H", record_value),
            ),
            (
                record_segment,
                (record_offset + 4) & 0xFFFF,
                struct.pack("<H", record_tail),
            ),
            (record_segment, relative_record, record_decoy),
            (data_segment, record_offset, b"\x11\x22\x33\x44\x55\x66"),
            (game_segment, record_offset, b"\xa1\xa2\xa3\xa4\xa5\xa6"),
            (stack_segment, record_offset, b"\xb1\xb2\xb3\xb4\xb5\xb6"),
            (stack_segment, branch_stack_effective, struct.pack("<H", branch_target)),
            (
                game_segment,
                branch_stack_effective,
                struct.pack("<H", branch_target ^ 0xFFFF),
            ),
            (
                data_segment,
                branch_stack_effective,
                struct.pack("<H", branch_target ^ 0xA5A5),
            ),
        ]
        immutable_script = []
        for byte_index, byte in enumerate(script):
            script_offset = start + byte_index
            encoded = bytes([byte])
            memory.append((data_segment, script_offset, encoded))
            memory.append((extra_segment, script_offset, b"\x5a"))
            memory.append((game_segment, script_offset, b"\xa5"))
            immutable_script.append((script_offset, encoded))

        initial = {
            "eax": 0xA1A1BEEF,
            "ebx": 0xB2B22345,
            "ecx": 0xC3C33456,
            "edx": 0xD4D44567,
            "esi": 0xE5E50000 | start,
            "edi": 0xF6F66789,
            "ebp": 0x9797789A,
            "sp": 0xFF00,
            "ds": data_segment,
            "es": extra_segment,
            "fs": 0x4C00,
            "gs": game_segment,
            "ss": stack_segment,
            "flags": 0x0AD7,
        }
        base_phases = []
        parse_phases = []
        helper_events = []
        terminal_phases = []

        def capture_phases(machine: Uc, address: int, _size: int) -> None:
            if address == 0x6D86:
                base_phases.append(
                    (
                        machine.reg_read(UC_X86_REG_DI),
                        machine.reg_read(UC_X86_REG_ES),
                        machine.reg_read(UC_X86_REG_SI),
                    )
                )
            elif address == 0x6D95:
                parse_phases.append(
                    (
                        machine.reg_read(UC_X86_REG_AX),
                        machine.reg_read(UC_X86_REG_BP),
                        machine.reg_read(UC_X86_REG_DX),
                        machine.reg_read(UC_X86_REG_SI),
                        machine.reg_read(UC_X86_REG_DI),
                        machine.reg_read(UC_X86_REG_ES),
                    )
                )
            elif address == 0x6462:
                helper_events.append(
                    (
                        machine.reg_read(UC_X86_REG_AX),
                        machine.reg_read(UC_X86_REG_BP),
                        machine.reg_read(UC_X86_REG_DX),
                        machine.reg_read(UC_X86_REG_SI),
                    )
                )
            elif address == 0x6DCD:
                terminal_phases.append(
                    (
                        machine.reg_read(UC_X86_REG_AX),
                        machine.reg_read(UC_X86_REG_BP),
                        machine.reg_read(UC_X86_REG_DX),
                        machine.reg_read(UC_X86_REG_SI),
                        machine.reg_read(UC_X86_REG_DI),
                        machine.reg_read(UC_X86_REG_ES),
                        struct.unpack(
                            "<H",
                            machine.mem_read(record_segment * 16 + record_offset, 2),
                        )[0],
                        struct.unpack(
                            "<H",
                            machine.mem_read(
                                record_segment * 16
                                + ((record_offset + 2) & 0xFFFF),
                                2,
                            ),
                        )[0],
                        struct.unpack(
                            "<H",
                            machine.mem_read(
                                record_segment * 16
                                + ((record_offset + 4) & 0xFFFF),
                                2,
                            ),
                        )[0],
                        machine.mem_read(game_segment * 16 + 0x67AD, 1)[0],
                        struct.unpack(
                            "<H", machine.mem_read(game_segment * 16 + 0x6884, 2)
                        )[0],
                    )
                )

        machine = execute(
            0x6D80,
            0x6DCE,
            initial,
            memory,
            code_handler=capture_phases,
        )

        expected_base = [(base_offset, record_segment, start)]
        if base_phases != expected_base:
            raise AssertionError(
                f"0x6d80 {name}: base={base_phases}, expected={expected_base}"
            )
        dx_with_inversion = (initial["edx"] & 0xFF00) | int(inverted)
        expected_parse = [
            (
                operand,
                record_offset,
                dx_with_inversion,
                final_script,
                base_offset,
                record_segment,
            )
        ]
        if parse_phases != expected_parse:
            raise AssertionError(
                f"0x6d80 {name}: parse={parse_phases}, expected={expected_parse}"
            )

        if record_value != operand:
            decision_ax = operand
        elif query_path:
            decision_ax = record_kind
        else:
            decision_ax = operand
        if branch_failed:
            expected_ax = top_after
            expected_si = branch_target
            query_after = 0
            expected_helpers = [
                (decision_ax, record_offset, dx_with_inversion, final_script)
            ]
            expected_flags = sub_flags_16(top_before, 2)
        else:
            expected_ax = decision_ax
            expected_si = final_script
            query_after = query_before
            expected_helpers = []
            if query_path:
                expected_flags = logic_flags_8(int(inverted))
            else:
                expected_flags = logic_flags_8(query_before & 1)

        if helper_events != expected_helpers:
            raise AssertionError(
                f"0x6d80 {name}: helpers={helper_events}, expected={expected_helpers}"
            )
        expected_terminal = [
            (
                expected_ax,
                record_offset,
                dx_with_inversion,
                expected_si,
                base_offset,
                record_segment,
                record_after[0],
                record_after[1],
                record_after[2],
                query_after,
                top_after,
            )
        ]
        if terminal_phases != expected_terminal:
            raise AssertionError(
                f"0x6d80 {name}: terminal={terminal_phases}, "
                f"expected={expected_terminal}"
            )

        expected_registers = dict(initial)
        del expected_registers["flags"]
        expected_registers["eax"] = (
            initial["eax"] & 0xFFFF0000
        ) | expected_ax
        expected_registers["edx"] = (
            initial["edx"] & 0xFFFF0000
        ) | dx_with_inversion
        expected_registers["esi"] = (
            initial["esi"] & 0xFFFF0000
        ) | expected_si
        expected_registers["ebp"] = (
            initial["ebp"] & 0xFFFF0000
        ) | record_offset
        expected_registers["es"] = record_segment
        for register, expected in expected_registers.items():
            actual = machine.reg_read(REGISTERS[register])
            if actual != expected:
                raise AssertionError(
                    f"0x6d80 {name}: {register}={actual:#x}, expected={expected:#x}"
                )

        actual_record = (
            struct.unpack(
                "<H", machine.mem_read(record_segment * 16 + record_offset, 2)
            )[0],
            struct.unpack(
                "<H",
                machine.mem_read(
                    record_segment * 16 + ((record_offset + 2) & 0xFFFF), 2
                ),
            )[0],
            struct.unpack(
                "<H",
                machine.mem_read(
                    record_segment * 16 + ((record_offset + 4) & 0xFFFF), 2
                ),
            )[0],
        )
        if actual_record != record_after:
            raise AssertionError(
                f"0x6d80 {name}: record={actual_record}, expected={record_after}"
            )
        if bytes(machine.mem_read(record_segment * 16 + relative_record, 6)) != record_decoy:
            raise AssertionError(f"0x6d80 {name}: base-relative record decoy changed")
        for script_offset, expected in immutable_script:
            actual = bytes(machine.mem_read(data_segment * 16 + script_offset, 1))
            if actual != expected:
                raise AssertionError(f"0x6d80 {name}: script input changed")
            if machine.mem_read(extra_segment * 16 + script_offset, 1) != b"\x5a":
                raise AssertionError(f"0x6d80 {name}: ES script decoy changed")
            if machine.mem_read(game_segment * 16 + script_offset, 1) != b"\xa5":
                raise AssertionError(f"0x6d80 {name}: GS script decoy changed")
        decoys = [
            (data_segment, 0x6724, data_pointer_decoy),
            (stack_segment, 0x6724, stack_pointer_decoy),
            (data_segment, 0x67AD, bytes([data_query_decoy])),
            (stack_segment, 0x67AD, bytes([stack_query_decoy])),
            (data_segment, 0x6884, struct.pack("<H", data_top_decoy)),
        ]
        for segment, decoy_offset, expected in decoys:
            actual = bytes(
                machine.mem_read(segment * 16 + decoy_offset, len(expected))
            )
            if actual != expected:
                raise AssertionError(f"0x6d80 {name}: segment decoy changed")

        flags = machine.reg_read(UC_X86_REG_EFLAGS)
        flag_masks = {
            "cf": 0x0001,
            "pf": 0x0004,
            "af": 0x0010,
            "zf": 0x0040,
            "sf": 0x0080,
            "of": 0x0800,
        }
        actual_flags = {
            flag: bool(flags & flag_masks[flag]) for flag in expected_flags
        }
        if actual_flags != expected_flags:
            raise AssertionError(
                f"0x6d80 {name}: flags={actual_flags}, expected={expected_flags}"
            )
        if EXE[0x6DCE] != 0xC3:
            raise AssertionError("0x6d80: expected near RET boundary")

        vectors.append(
            {
                "name": name,
                "query_mode_before": query_before,
                "inverted": inverted,
                "record_base_offset_ignored": base_offset,
                "record_offset": record_offset,
                "operand": operand,
                "record_before": [record_kind, record_value, record_tail],
                "record_after": list(record_after),
                "branch_failed": branch_failed,
                "final_script_offset": expected_si,
                "query_mode_after": query_after,
                "branch_stack_top_after": top_after,
                "defined_flags": expected_flags,
            }
        )

    return vectors


def vm_c7_record_match_vectors() -> list[dict[str, object]]:
    data_segment = 0x4400
    extra_segment = 0x4800
    game_segment = 0x2C00
    record_segment = 0x5200
    stack_segment = 0x9000
    cases = [
        {
            "name": "query_exact_pass",
            "query": 1,
            "record_kind": 0x00C7,
            "record_value_matches": True,
        },
        {
            "name": "query_value_mismatch_fails",
            "query": 1,
            "record_kind": 0x00C7,
        },
        {
            "name": "query_kind_mismatch_fails",
            "query": 1,
            "record_kind": 0x00C6,
            "record_value_matches": True,
        },
        {
            "name": "query_inverted_exact_fails",
            "query": 3,
            "inverted": True,
            "record_kind": 0x00C7,
            "record_value_matches": True,
        },
        {
            "name": "query_inverted_value_mismatch_pass",
            "query": 1,
            "inverted": True,
            "record_kind": 0x00C7,
        },
        {
            "name": "query_inverted_kind_mismatch_pass",
            "query": 1,
            "inverted": True,
            "record_kind": 0x00C6,
            "record_value_matches": True,
        },
        {
            "name": "query_inverted_script_end",
            "query": 1,
            "inverted": True,
            "record_kind": 0x00C6,
            "record_value_matches": True,
            "start": 0xFFFB,
        },
        {
            "name": "query_record_word_at_ffff",
            "query": 1,
            "record_kind": 0x00C7,
            "record_value_matches": True,
            "record": 0xFFFF,
        },
        {"name": "set_empty_success", "query": 0},
        {"name": "set_c4_success", "query": 2, "record_kind": 0x00C4},
        {"name": "set_inactive_fails", "query": 0, "active": 0},
        {"name": "set_other_active_bits_fail", "query": 0, "active": 0x82},
        {"name": "set_occupied_c5_fails", "query": 0, "record_kind": 0x00C5},
        {
            "name": "set_related_kind_ignored",
            "query": 0,
            "related_kind": 0xBEEF,
        },
        {
            "name": "set_prefixed_c4_success",
            "query": 2,
            "inverted": True,
            "record_kind": 0x00C4,
        },
    ]
    failure_tops = [2, 0, 1, 5]
    vectors = []

    def logic_flags(value: int, sign_mask: int) -> dict[str, bool]:
        result = value & (0xFFFF if sign_mask == 0x8000 else 0xFF)
        return {
            "cf": False,
            "pf": (result & 0xFF).bit_count() % 2 == 0,
            "zf": result == 0,
            "sf": bool(result & sign_mask),
            "of": False,
        }

    def sub_flags_16(left: int, right: int) -> dict[str, bool]:
        result = (left - right) & 0xFFFF
        return {
            "cf": left < right,
            "pf": (result & 0xFF).bit_count() % 2 == 0,
            "af": (left & 0x0F) < (right & 0x0F),
            "zf": result == 0,
            "sf": bool(result & 0x8000),
            "of": bool(((left ^ right) & (left ^ result)) & 0x8000),
        }

    failure_index = 0
    for case_index, case in enumerate(cases):
        name = str(case["name"])
        query_before = int(case["query"])
        query_path = bool(query_before & 1)
        inverted = bool(case.get("inverted", False))
        base_offset = 0x1234
        record_offset = int(case.get("record", 0x0C00 + case_index * 0x20))
        operand = 0x3800 + case_index * 0x20
        record_kind = int(case.get("record_kind", 0))
        record_value = (
            operand if bool(case.get("record_value_matches", False)) else 0x7777
        )
        record_tail = 0x9999
        related_kind = int(case.get("related_kind", 0x0200))
        active = int(case.get("active", 1))
        start = int(case.get("start", 0x5800 + case_index * 0x20))
        prefix = b"\xa1" if inverted else b""
        script = prefix + struct.pack("<HH", record_offset, operand)
        final_script = (start + len(script)) & 0xFFFF
        matches = record_value == operand and record_kind == 0x00C7
        if query_path:
            success = matches != inverted
        else:
            success = bool(active & 1) and record_kind in (0, 0x00C4)
        branch_failed = not success
        if branch_failed:
            top_before = failure_tops[failure_index % len(failure_tops)]
            failure_index += 1
            top_after = (top_before - 2) & 0xFFFF
            branch_target = (0x6A00 + case_index * 0x1F) & 0xFFFF
            branch_stack_effective = (0x6820 + top_after) & 0xFFFF
        else:
            top_before = 0x2468
            top_after = top_before
            branch_target = 0x5AA5
            branch_stack_effective = 0x681E

        if query_path or not success:
            record_after = (record_kind, record_value, record_tail)
        else:
            record_after = (0x00C7, operand, 0)

        pointer = struct.pack("<HH", base_offset, record_segment)
        data_pointer_decoy = struct.pack("<HH", 0x2222, extra_segment)
        stack_pointer_decoy = struct.pack("<HH", 0x3333, stack_segment)
        data_query_decoy = query_before ^ 0x55
        stack_query_decoy = query_before ^ 0xAA
        data_top_decoy = top_before ^ 0xFFFF
        relative_record = (base_offset + record_offset) & 0xFFFF
        relative_related = (base_offset + operand) & 0xFFFF
        record_decoy = b"\xad\xde\xad\xde\xad\xde"
        related_decoy = b"\x5a\xa5\x5a\xa5"
        memory = [
            (game_segment, 0x6724, pointer),
            (data_segment, 0x6724, data_pointer_decoy),
            (stack_segment, 0x6724, stack_pointer_decoy),
            (game_segment, 0x67AD, bytes([query_before])),
            (data_segment, 0x67AD, bytes([data_query_decoy])),
            (stack_segment, 0x67AD, bytes([stack_query_decoy])),
            (game_segment, 0x6884, struct.pack("<H", top_before)),
            (data_segment, 0x6884, struct.pack("<H", data_top_decoy)),
            (record_segment, record_offset, struct.pack("<H", record_kind)),
            (
                record_segment,
                (record_offset + 2) & 0xFFFF,
                struct.pack("<H", record_value),
            ),
            (
                record_segment,
                (record_offset + 4) & 0xFFFF,
                struct.pack("<H", record_tail),
            ),
            (record_segment, operand, struct.pack("<H", related_kind)),
            (record_segment, (operand + 2) & 0xFFFF, bytes([active])),
            (record_segment, relative_record, record_decoy),
            (record_segment, relative_related, related_decoy),
            (data_segment, record_offset, b"\x11\x22\x33\x44\x55\x66"),
            (game_segment, record_offset, b"\xa1\xa2\xa3\xa4\xa5\xa6"),
            (stack_segment, record_offset, b"\xb1\xb2\xb3\xb4\xb5\xb6"),
            (stack_segment, branch_stack_effective, struct.pack("<H", branch_target)),
            (
                game_segment,
                branch_stack_effective,
                struct.pack("<H", branch_target ^ 0xFFFF),
            ),
            (
                data_segment,
                branch_stack_effective,
                struct.pack("<H", branch_target ^ 0xA5A5),
            ),
        ]
        immutable_script = []
        for byte_index, byte in enumerate(script):
            script_offset = start + byte_index
            encoded = bytes([byte])
            memory.append((data_segment, script_offset, encoded))
            memory.append((extra_segment, script_offset, b"\x5a"))
            memory.append((game_segment, script_offset, b"\xa5"))
            immutable_script.append((script_offset, encoded))

        initial = {
            "eax": 0xA1A1BEEF,
            "ebx": 0xB2B22345,
            "ecx": 0xC3C33456,
            "edx": 0xD4D44567,
            "esi": 0xE5E50000 | start,
            "edi": 0xF6F66789,
            "ebp": 0x9797789A,
            "sp": 0xFF00,
            "ds": data_segment,
            "es": extra_segment,
            "fs": 0x4C00,
            "gs": game_segment,
            "ss": stack_segment,
            "flags": 0x0AD7,
        }
        base_phases = []
        parse_phases = []
        helper_events = []
        terminal_phases = []

        def capture_phases(machine: Uc, address: int, _size: int) -> None:
            if address == 0x6DD5:
                base_phases.append(
                    (
                        machine.reg_read(UC_X86_REG_DI),
                        machine.reg_read(UC_X86_REG_ES),
                        machine.reg_read(UC_X86_REG_SI),
                    )
                )
            elif address == 0x6DE4:
                parse_phases.append(
                    (
                        machine.reg_read(UC_X86_REG_AX),
                        machine.reg_read(UC_X86_REG_BP),
                        machine.reg_read(UC_X86_REG_DX),
                        machine.reg_read(UC_X86_REG_SI),
                        machine.reg_read(UC_X86_REG_DI),
                        machine.reg_read(UC_X86_REG_ES),
                    )
                )
            elif address == 0x6462:
                helper_events.append(
                    (
                        machine.reg_read(UC_X86_REG_AX),
                        machine.reg_read(UC_X86_REG_BX),
                        machine.reg_read(UC_X86_REG_BP),
                        machine.reg_read(UC_X86_REG_DX),
                        machine.reg_read(UC_X86_REG_SI),
                    )
                )
            elif address == 0x6E32:
                terminal_phases.append(
                    (
                        machine.reg_read(UC_X86_REG_AX),
                        machine.reg_read(UC_X86_REG_BX),
                        machine.reg_read(UC_X86_REG_BP),
                        machine.reg_read(UC_X86_REG_DX),
                        machine.reg_read(UC_X86_REG_SI),
                        machine.reg_read(UC_X86_REG_DI),
                        machine.reg_read(UC_X86_REG_ES),
                        struct.unpack(
                            "<H",
                            machine.mem_read(record_segment * 16 + record_offset, 2),
                        )[0],
                        struct.unpack(
                            "<H",
                            machine.mem_read(
                                record_segment * 16
                                + ((record_offset + 2) & 0xFFFF),
                                2,
                            ),
                        )[0],
                        struct.unpack(
                            "<H",
                            machine.mem_read(
                                record_segment * 16
                                + ((record_offset + 4) & 0xFFFF),
                                2,
                            ),
                        )[0],
                        machine.mem_read(game_segment * 16 + 0x67AD, 1)[0],
                        struct.unpack(
                            "<H", machine.mem_read(game_segment * 16 + 0x6884, 2)
                        )[0],
                    )
                )

        machine = execute(
            0x6DCF,
            0x6E33,
            initial,
            memory,
            code_handler=capture_phases,
        )

        expected_base = [(base_offset, record_segment, start)]
        if base_phases != expected_base:
            raise AssertionError(
                f"0x6dcf {name}: base={base_phases}, expected={expected_base}"
            )
        dx_with_inversion = (initial["edx"] & 0xFF00) | int(inverted)
        expected_parse = [
            (
                operand,
                record_offset,
                dx_with_inversion,
                final_script,
                base_offset,
                record_segment,
            )
        ]
        if parse_phases != expected_parse:
            raise AssertionError(
                f"0x6dcf {name}: parse={parse_phases}, expected={expected_parse}"
            )

        if query_path:
            terminal_bx = initial["ebx"] & 0xFFFF
            decision_ax = operand if record_value != operand else record_kind
        else:
            terminal_bx = operand
            decision_ax = operand if (active & 1) == 0 else record_kind
        if branch_failed:
            expected_ax = top_after
            expected_si = branch_target
            query_after = 0
            expected_helpers = [
                (
                    decision_ax,
                    terminal_bx,
                    record_offset,
                    dx_with_inversion,
                    final_script,
                )
            ]
            expected_flags = sub_flags_16(top_before, 2)
        else:
            expected_ax = decision_ax
            expected_si = final_script
            query_after = query_before
            expected_helpers = []
            if query_path:
                expected_flags = logic_flags(int(inverted), 0x80)
            elif record_kind == 0x00C4:
                expected_flags = sub_flags_16(record_kind, 0x00C4)
            else:
                expected_flags = logic_flags(record_kind, 0x8000)

        if helper_events != expected_helpers:
            raise AssertionError(
                f"0x6dcf {name}: helpers={helper_events}, expected={expected_helpers}"
            )
        expected_terminal = [
            (
                expected_ax,
                terminal_bx,
                record_offset,
                dx_with_inversion,
                expected_si,
                base_offset,
                record_segment,
                record_after[0],
                record_after[1],
                record_after[2],
                query_after,
                top_after,
            )
        ]
        if terminal_phases != expected_terminal:
            raise AssertionError(
                f"0x6dcf {name}: terminal={terminal_phases}, "
                f"expected={expected_terminal}"
            )

        expected_registers = dict(initial)
        del expected_registers["flags"]
        expected_registers["eax"] = (
            initial["eax"] & 0xFFFF0000
        ) | expected_ax
        expected_registers["ebx"] = (
            initial["ebx"] & 0xFFFF0000
        ) | terminal_bx
        expected_registers["edx"] = (
            initial["edx"] & 0xFFFF0000
        ) | dx_with_inversion
        expected_registers["esi"] = (
            initial["esi"] & 0xFFFF0000
        ) | expected_si
        expected_registers["ebp"] = (
            initial["ebp"] & 0xFFFF0000
        ) | record_offset
        expected_registers["es"] = record_segment
        for register, expected in expected_registers.items():
            actual = machine.reg_read(REGISTERS[register])
            if actual != expected:
                raise AssertionError(
                    f"0x6dcf {name}: {register}={actual:#x}, expected={expected:#x}"
                )

        actual_record = (
            struct.unpack(
                "<H", machine.mem_read(record_segment * 16 + record_offset, 2)
            )[0],
            struct.unpack(
                "<H",
                machine.mem_read(
                    record_segment * 16 + ((record_offset + 2) & 0xFFFF), 2
                ),
            )[0],
            struct.unpack(
                "<H",
                machine.mem_read(
                    record_segment * 16 + ((record_offset + 4) & 0xFFFF), 2
                ),
            )[0],
        )
        if actual_record != record_after:
            raise AssertionError(
                f"0x6dcf {name}: record={actual_record}, expected={record_after}"
            )
        if bytes(machine.mem_read(record_segment * 16 + relative_record, 6)) != record_decoy:
            raise AssertionError(f"0x6dcf {name}: base-relative record decoy changed")
        if bytes(machine.mem_read(record_segment * 16 + relative_related, 4)) != related_decoy:
            raise AssertionError(f"0x6dcf {name}: base-relative related decoy changed")
        for script_offset, expected in immutable_script:
            actual = bytes(machine.mem_read(data_segment * 16 + script_offset, 1))
            if actual != expected:
                raise AssertionError(f"0x6dcf {name}: script input changed")
            if machine.mem_read(extra_segment * 16 + script_offset, 1) != b"\x5a":
                raise AssertionError(f"0x6dcf {name}: ES script decoy changed")
            if machine.mem_read(game_segment * 16 + script_offset, 1) != b"\xa5":
                raise AssertionError(f"0x6dcf {name}: GS script decoy changed")
        decoys = [
            (data_segment, 0x6724, data_pointer_decoy),
            (stack_segment, 0x6724, stack_pointer_decoy),
            (data_segment, 0x67AD, bytes([data_query_decoy])),
            (stack_segment, 0x67AD, bytes([stack_query_decoy])),
            (data_segment, 0x6884, struct.pack("<H", data_top_decoy)),
        ]
        for segment, decoy_offset, expected in decoys:
            actual = bytes(
                machine.mem_read(segment * 16 + decoy_offset, len(expected))
            )
            if actual != expected:
                raise AssertionError(f"0x6dcf {name}: segment decoy changed")

        flags = machine.reg_read(UC_X86_REG_EFLAGS)
        flag_masks = {
            "cf": 0x0001,
            "pf": 0x0004,
            "af": 0x0010,
            "zf": 0x0040,
            "sf": 0x0080,
            "of": 0x0800,
        }
        actual_flags = {
            flag: bool(flags & flag_masks[flag]) for flag in expected_flags
        }
        if actual_flags != expected_flags:
            raise AssertionError(
                f"0x6dcf {name}: flags={actual_flags}, expected={expected_flags}"
            )
        if EXE[0x6E33] != 0xC3:
            raise AssertionError("0x6dcf: expected near RET boundary")

        vectors.append(
            {
                "name": name,
                "query_mode_before": query_before,
                "inverted": inverted,
                "record_base_offset_ignored": base_offset,
                "record_offset": record_offset,
                "operand": operand,
                "record_before": [record_kind, record_value, record_tail],
                "related_kind_ignored": related_kind,
                "related_active_byte": active,
                "record_after": list(record_after),
                "branch_failed": branch_failed,
                "final_script_offset": expected_si,
                "query_mode_after": query_after,
                "branch_stack_top_after": top_after,
                "defined_flags": expected_flags,
            }
        )

    return vectors


def vm_c8_record_match_vectors() -> list[dict[str, object]]:
    data_segment = 0x4400
    extra_segment = 0x4800
    game_segment = 0x2C00
    record_segment = 0x5200
    stack_segment = 0x9000
    cases = [
        {"name": "query_exact_pass", "query": 1, "kind": 0xC8, "match": True},
        {"name": "query_value_mismatch_fails", "query": 1, "kind": 0xC8},
        {"name": "query_kind_mismatch_fails", "query": 1, "kind": 0xC7, "match": True},
        {"name": "query_inverted_exact_fails", "query": 3, "inverted": True, "kind": 0xC8, "match": True},
        {"name": "query_inverted_value_mismatch_pass", "query": 1, "inverted": True, "kind": 0xC8},
        {"name": "query_inverted_kind_mismatch_pass", "query": 1, "inverted": True, "kind": 0xC7, "match": True},
        {"name": "query_inverted_script_end", "query": 1, "inverted": True, "kind": 0xC7, "match": True, "start": 0xFFFB},
        {"name": "query_record_word_at_ffff", "query": 1, "kind": 0xC8, "match": True, "record": 0xFFFF},
        {"name": "set_empty_zero_operand", "query": 0, "operand": 0},
        {"name": "set_empty_ignores_operand", "query": 2, "operand": 0xBEEF},
        {"name": "set_prefixed_ignores_operand", "query": 0, "inverted": True, "operand": 0x4567},
        {"name": "set_occupied_c8_fails", "query": 0, "kind": 0xC8},
        {"name": "set_occupied_c4_fails", "query": 2, "kind": 0xC4},
    ]
    failure_tops = [2, 0, 1, 5]
    vectors = []

    def logic_flags(value: int, sign_mask: int) -> dict[str, bool]:
        result = value & (0xFFFF if sign_mask == 0x8000 else 0xFF)
        return {
            "cf": False,
            "pf": (result & 0xFF).bit_count() % 2 == 0,
            "zf": result == 0,
            "sf": bool(result & sign_mask),
            "of": False,
        }

    def sub_flags_16(left: int, right: int) -> dict[str, bool]:
        result = (left - right) & 0xFFFF
        return {
            "cf": left < right,
            "pf": (result & 0xFF).bit_count() % 2 == 0,
            "af": (left & 0x0F) < (right & 0x0F),
            "zf": result == 0,
            "sf": bool(result & 0x8000),
            "of": bool(((left ^ right) & (left ^ result)) & 0x8000),
        }

    failure_index = 0
    for case_index, case in enumerate(cases):
        name = str(case["name"])
        query_before = int(case["query"])
        query_path = bool(query_before & 1)
        inverted = bool(case.get("inverted", False))
        base_offset = 0x1234
        record_offset = int(case.get("record", 0x1000 + case_index * 0x20))
        operand = int(case.get("operand", 0x4A00 + case_index * 0x20))
        kind = int(case.get("kind", 0))
        value = operand if bool(case.get("match", False)) else 0x7777
        tail = 0x9999
        start = int(case.get("start", 0x6000 + case_index * 0x20))
        prefix = b"\xa1" if inverted else b""
        script = prefix + struct.pack("<HH", record_offset, operand)
        final_script = (start + len(script)) & 0xFFFF
        matches = kind == 0xC8 and value == operand
        branch_failed = (matches == inverted) if query_path else kind != 0
        if branch_failed:
            top_before = failure_tops[failure_index % len(failure_tops)]
            failure_index += 1
            top_after = (top_before - 2) & 0xFFFF
            branch_target = (0x6E00 + case_index * 0x1D) & 0xFFFF
            stack_entry = (0x6820 + top_after) & 0xFFFF
        else:
            top_before = 0x2468
            top_after = top_before
            branch_target = 0x5AA5
            stack_entry = 0x681E
        record_after = (kind, value, tail) if query_path or branch_failed else (0xC8, 0, 0)

        pointer = struct.pack("<HH", base_offset, record_segment)
        data_pointer_decoy = struct.pack("<HH", 0x2222, extra_segment)
        stack_pointer_decoy = struct.pack("<HH", 0x3333, stack_segment)
        data_query_decoy = query_before ^ 0x55
        stack_query_decoy = query_before ^ 0xAA
        data_top_decoy = top_before ^ 0xFFFF
        relative_record = (base_offset + record_offset) & 0xFFFF
        record_decoy = b"\xad\xde\xad\xde\xad\xde"
        memory = [
            (game_segment, 0x6724, pointer),
            (data_segment, 0x6724, data_pointer_decoy),
            (stack_segment, 0x6724, stack_pointer_decoy),
            (game_segment, 0x67AD, bytes([query_before])),
            (data_segment, 0x67AD, bytes([data_query_decoy])),
            (stack_segment, 0x67AD, bytes([stack_query_decoy])),
            (game_segment, 0x6884, struct.pack("<H", top_before)),
            (data_segment, 0x6884, struct.pack("<H", data_top_decoy)),
            (record_segment, record_offset, struct.pack("<H", kind)),
            (record_segment, (record_offset + 2) & 0xFFFF, struct.pack("<H", value)),
            (record_segment, (record_offset + 4) & 0xFFFF, struct.pack("<H", tail)),
            (record_segment, relative_record, record_decoy),
            (stack_segment, stack_entry, struct.pack("<H", branch_target)),
            (game_segment, stack_entry, struct.pack("<H", branch_target ^ 0xFFFF)),
            (data_segment, stack_entry, struct.pack("<H", branch_target ^ 0xA5A5)),
        ]
        immutable_script = []
        for byte_index, byte in enumerate(script):
            script_offset = start + byte_index
            encoded = bytes([byte])
            memory.extend(
                [
                    (data_segment, script_offset, encoded),
                    (extra_segment, script_offset, b"\x5a"),
                    (game_segment, script_offset, b"\xa5"),
                ]
            )
            immutable_script.append((script_offset, encoded))

        initial = {
            "eax": 0xA1A1BEEF,
            "ebx": 0xB2B22345,
            "ecx": 0xC3C33456,
            "edx": 0xD4D44567,
            "esi": 0xE5E50000 | start,
            "edi": 0xF6F66789,
            "ebp": 0x9797789A,
            "sp": 0xFF00,
            "ds": data_segment,
            "es": extra_segment,
            "fs": 0x4C00,
            "gs": game_segment,
            "ss": stack_segment,
            "flags": 0x0AD7,
        }
        phases = []
        helpers = []

        def capture(machine: Uc, address: int, _size: int) -> None:
            if address == 0x6462:
                helpers.append(
                    (
                        machine.reg_read(UC_X86_REG_AX),
                        machine.reg_read(UC_X86_REG_BX),
                        machine.reg_read(UC_X86_REG_BP),
                        machine.reg_read(UC_X86_REG_DX),
                        machine.reg_read(UC_X86_REG_SI),
                    )
                )
            elif address == 0x6FB7:
                phases.append(
                    (
                        machine.reg_read(UC_X86_REG_AX),
                        machine.reg_read(UC_X86_REG_BX),
                        machine.reg_read(UC_X86_REG_BP),
                        machine.reg_read(UC_X86_REG_DX),
                        machine.reg_read(UC_X86_REG_SI),
                        machine.reg_read(UC_X86_REG_DI),
                        machine.reg_read(UC_X86_REG_ES),
                        tuple(
                            struct.unpack(
                                "<H",
                                machine.mem_read(
                                    record_segment * 16 + ((record_offset + index) & 0xFFFF),
                                    2,
                                ),
                            )[0]
                            for index in (0, 2, 4)
                        ),
                        machine.mem_read(game_segment * 16 + 0x67AD, 1)[0],
                        struct.unpack("<H", machine.mem_read(game_segment * 16 + 0x6884, 2))[0],
                    )
                )

        machine = execute(0x6F62, 0x6FB8, initial, memory, code_handler=capture)
        dx_value = (initial["edx"] & 0xFF00) | int(inverted)
        terminal_bx = (initial["ebx"] & 0xFFFF) if query_path else kind
        decision_ax = operand if value != operand else kind
        if not query_path:
            decision_ax = operand
        if branch_failed:
            expected_ax = top_after
            expected_si = branch_target
            query_after = 0
            expected_helpers = [(decision_ax, terminal_bx, record_offset, dx_value, final_script)]
            expected_flags = sub_flags_16(top_before, 2)
        else:
            expected_ax = decision_ax
            expected_si = final_script
            query_after = query_before
            expected_helpers = []
            expected_flags = logic_flags(int(inverted), 0x80) if query_path else logic_flags(kind, 0x8000)
        if helpers != expected_helpers:
            raise AssertionError(f"0x6f62 {name}: helpers={helpers}, expected={expected_helpers}")
        expected_phase = [
            (
                expected_ax,
                terminal_bx,
                record_offset,
                dx_value,
                expected_si,
                base_offset,
                record_segment,
                record_after,
                query_after,
                top_after,
            )
        ]
        if phases != expected_phase:
            raise AssertionError(f"0x6f62 {name}: terminal={phases}, expected={expected_phase}")

        expected_registers = dict(initial)
        del expected_registers["flags"]
        expected_registers.update(
            {
                "eax": (initial["eax"] & 0xFFFF0000) | expected_ax,
                "ebx": (initial["ebx"] & 0xFFFF0000) | terminal_bx,
                "edx": (initial["edx"] & 0xFFFF0000) | dx_value,
                "esi": (initial["esi"] & 0xFFFF0000) | expected_si,
                "ebp": (initial["ebp"] & 0xFFFF0000) | record_offset,
                "es": record_segment,
            }
        )
        for register, expected in expected_registers.items():
            actual = machine.reg_read(REGISTERS[register])
            if actual != expected:
                raise AssertionError(f"0x6f62 {name}: {register}={actual:#x}, expected={expected:#x}")
        if bytes(machine.mem_read(record_segment * 16 + relative_record, 6)) != record_decoy:
            raise AssertionError(f"0x6f62 {name}: base-relative decoy changed")
        for script_offset, expected in immutable_script:
            if bytes(machine.mem_read(data_segment * 16 + script_offset, 1)) != expected:
                raise AssertionError(f"0x6f62 {name}: script changed")
            if machine.mem_read(extra_segment * 16 + script_offset, 1) != b"\x5a":
                raise AssertionError(f"0x6f62 {name}: ES script decoy changed")
            if machine.mem_read(game_segment * 16 + script_offset, 1) != b"\xa5":
                raise AssertionError(f"0x6f62 {name}: GS script decoy changed")
        for segment, offset, expected in [
            (data_segment, 0x6724, data_pointer_decoy),
            (stack_segment, 0x6724, stack_pointer_decoy),
            (data_segment, 0x67AD, bytes([data_query_decoy])),
            (stack_segment, 0x67AD, bytes([stack_query_decoy])),
            (data_segment, 0x6884, struct.pack("<H", data_top_decoy)),
        ]:
            if bytes(machine.mem_read(segment * 16 + offset, len(expected))) != expected:
                raise AssertionError(f"0x6f62 {name}: segment decoy changed")
        flags = machine.reg_read(UC_X86_REG_EFLAGS)
        masks = {"cf": 1, "pf": 4, "af": 0x10, "zf": 0x40, "sf": 0x80, "of": 0x800}
        actual_flags = {flag: bool(flags & masks[flag]) for flag in expected_flags}
        if actual_flags != expected_flags:
            raise AssertionError(f"0x6f62 {name}: flags={actual_flags}, expected={expected_flags}")
        if EXE[0x6FB8] != 0xC3:
            raise AssertionError("0x6f62: expected near RET boundary")
        vectors.append(
            {
                "name": name,
                "query_mode_before": query_before,
                "inverted": inverted,
                "record_base_offset_ignored": base_offset,
                "record_offset": record_offset,
                "operand": operand,
                "record_before": [kind, value, tail],
                "record_after": list(record_after),
                "branch_failed": branch_failed,
                "final_script_offset": expected_si,
                "query_mode_after": query_after,
                "branch_stack_top_after": top_after,
                "defined_flags": expected_flags,
            }
        )
    return vectors


def vm_c9_clear_record_vectors() -> list[dict[str, object]]:
    data_segment = 0x4400
    extra_segment = 0x4800
    game_segment = 0x2C00
    record_segment = 0x5200
    stack_segment = 0x9000
    cases = [
        {"name": "clear_non_c4", "kind": 0x00C8, "related": 0x3000},
        {"name": "clear_zero_kind", "kind": 0, "related": 0x3100},
        {"name": "clear_non_c4_script_end", "kind": 0x0400, "related": 0x3200, "start": 0xFFFF},
        {"name": "c4_positive_offset", "kind": 0x00C4, "related": 0x3300, "related_kind": 1, "field_offset": 6},
        {"name": "c4_negative_offset", "kind": 0x00C4, "related": 0x3400, "related_kind": 0x8000, "field_offset": -4},
        {"name": "c4_reciprocal_wrap", "kind": 0x00C4, "related": 2, "related_kind": 0x20, "field_offset": -4},
        {"name": "c4_zero_offset", "kind": 0x00C4, "related": 0x3500, "related_kind": 4, "field_offset": 0},
        {"name": "c4_reciprocal_alias_primary", "kind": 0x00C4, "record": 0x1800, "related": 0x17F8, "related_kind": 2, "field_offset": 8},
    ]
    vectors = []

    def logic_flags_16(value: int) -> dict[str, bool]:
        result = value & 0xFFFF
        return {
            "cf": False,
            "pf": (result & 0xFF).bit_count() % 2 == 0,
            "zf": result == 0,
            "sf": bool(result & 0x8000),
            "of": False,
        }

    def sub_flags_16(left: int, right: int) -> dict[str, bool]:
        result = (left - right) & 0xFFFF
        return {
            "cf": left < right,
            "pf": (result & 0xFF).bit_count() % 2 == 0,
            "af": (left & 0x0F) < (right & 0x0F),
            "zf": result == 0,
            "sf": bool(result & 0x8000),
            "of": bool(((left ^ right) & (left ^ result)) & 0x8000),
        }

    for case_index, case in enumerate(cases):
        name = str(case["name"])
        kind = int(case["kind"])
        related = int(case["related"])
        related_kind = int(case.get("related_kind", 0xBEEF))
        field_offset = int(case.get("field_offset", 0))
        record_offset = int(case.get("record", 0x1400 + case_index * 0x20))
        record_tail = 0xA55A
        reciprocal_offset = (related + field_offset) & 0xFFFF
        reciprocal_words = [0xAAAA, 0xBBBB, 0xCCCC]
        if field_offset == 0:
            reciprocal_words[0] = related_kind
        elif field_offset == -4:
            reciprocal_words[2] = related_kind
        if reciprocal_offset == record_offset:
            reciprocal_words = [kind, related, record_tail]
        reciprocal_before = tuple(reciprocal_words)
        start = int(case.get("start", 0x6800 + case_index * 0x20))
        script = struct.pack("<H", record_offset)
        final_script = (start + 2) & 0xFFFF
        base_offset = 0x1234
        sequence_before = 0xA5
        depth_before = 0x5A
        is_c4 = kind == 0x00C4
        sequence_after = 0 if is_c4 else sequence_before
        depth_after = 6 if is_c4 else depth_before

        pointer = struct.pack("<HH", base_offset, record_segment)
        data_pointer_decoy = struct.pack("<HH", 0x2222, extra_segment)
        stack_pointer_decoy = struct.pack("<HH", 0x3333, stack_segment)
        relative_record = (base_offset + record_offset) & 0xFFFF
        relative_decoy = b"\xad\xde\xad\xde\xad\xde"
        bit_index = (related_kind & -related_kind).bit_length() - 1
        field_table_offset = (0x6D60 + (0x13 << 4) + bit_index) & 0xFFFF
        memory = [
            (game_segment, 0x6724, pointer),
            (data_segment, 0x6724, data_pointer_decoy),
            (stack_segment, 0x6724, stack_pointer_decoy),
            (game_segment, 0x252A, bytes([sequence_before])),
            (game_segment, 0x2531, bytes([depth_before])),
            (data_segment, 0x252A, b"\x11"),
            (data_segment, 0x2531, b"\x22"),
            (stack_segment, 0x252A, b"\x33"),
            (stack_segment, 0x2531, b"\x44"),
            (record_segment, record_offset, struct.pack("<HHH", kind, related, record_tail)),
            (record_segment, related, struct.pack("<H", related_kind)),
            (record_segment, reciprocal_offset, struct.pack("<HHH", *reciprocal_before)),
            (record_segment, relative_record, relative_decoy),
            (game_segment, field_table_offset, bytes([field_offset & 0xFF])),
            (data_segment, field_table_offset, b"\x7f"),
        ]
        immutable_script = []
        for byte_index, byte in enumerate(script):
            script_offset = start + byte_index
            encoded = bytes([byte])
            memory.extend(
                [
                    (data_segment, script_offset, encoded),
                    (extra_segment, script_offset, b"\x5a"),
                    (game_segment, script_offset, b"\xa5"),
                ]
            )
            immutable_script.append((script_offset, encoded))
        initial = {
            "eax": 0xA1A1BEEF,
            "ebx": 0xB2B22345,
            "ecx": 0xC3C33456,
            "edx": 0xD4D44567,
            "esi": 0xE5E50000 | start,
            "edi": 0xF6F66789,
            "ebp": 0x9797789A,
            "sp": 0xFF00,
            "ds": data_segment,
            "es": extra_segment,
            "fs": 0x4C00,
            "gs": game_segment,
            "ss": stack_segment,
            "flags": 0x0AD7,
        }
        order = []
        helper_events = []
        terminal = []

        def read_record(machine: Uc) -> tuple[int, int, int]:
            return tuple(
                struct.unpack(
                    "<H",
                    machine.mem_read(
                        record_segment * 16 + ((record_offset + offset) & 0xFFFF),
                        2,
                    ),
                )[0]
                for offset in (0, 2, 4)
            )

        def capture(machine: Uc, address: int, _size: int) -> None:
            if address in (0x6FC7, 0x6FC8, 0x6FCB, 0x6FCC):
                order.append((address, read_record(machine)))
            elif address == 0x6023:
                helper_events.append(
                    (
                        machine.reg_read(UC_X86_REG_AX),
                        machine.reg_read(UC_X86_REG_BX),
                        machine.reg_read(UC_X86_REG_SI),
                        machine.reg_read(UC_X86_REG_DI),
                        machine.reg_read(UC_X86_REG_ES),
                    )
                )
            elif address == 0x6FF1:
                terminal.append(
                    (
                        machine.reg_read(UC_X86_REG_AX),
                        machine.reg_read(UC_X86_REG_BX),
                        machine.reg_read(UC_X86_REG_CX),
                        machine.reg_read(UC_X86_REG_SI),
                        machine.reg_read(UC_X86_REG_DI),
                        machine.reg_read(UC_X86_REG_ES),
                        read_record(machine),
                        machine.mem_read(game_segment * 16 + 0x252A, 1)[0],
                        machine.mem_read(game_segment * 16 + 0x2531, 1)[0],
                    )
                )

        machine = execute(0x6FB9, 0x6FF2, initial, memory, code_handler=capture)
        expected_order = [
            (0x6FC7, (kind, related, record_tail)),
            (0x6FC8, (0, related, record_tail)),
            (0x6FCB, (0, related, record_tail)),
            (0x6FCC, (0, 0, record_tail)),
        ]
        if order != expected_order:
            raise AssertionError(f"0x6fb9 {name}: order={order}, expected={expected_order}")
        expected_helpers = (
            [(0x13, related_kind, final_script, (record_offset + 6) & 0xFFFF, record_segment)]
            if is_c4
            else []
        )
        if helper_events != expected_helpers:
            raise AssertionError(f"0x6fb9 {name}: helpers={helper_events}, expected={expected_helpers}")
        terminal_di = (reciprocal_offset + 6) & 0xFFFF if is_c4 else (record_offset + 6) & 0xFFFF
        terminal_bx = related_kind if is_c4 else related
        expected_terminal = [
            (0, terminal_bx, kind, final_script, terminal_di, record_segment, (0, 0, 0), sequence_after, depth_after)
        ]
        if terminal != expected_terminal:
            raise AssertionError(f"0x6fb9 {name}: terminal={terminal}, expected={expected_terminal}")
        expected_registers = dict(initial)
        del expected_registers["flags"]
        expected_registers.update(
            {
                "eax": initial["eax"] & 0xFFFF0000,
                "ebx": (initial["ebx"] & 0xFFFF0000) | terminal_bx,
                "ecx": (initial["ecx"] & 0xFFFF0000) | kind,
                "esi": (initial["esi"] & 0xFFFF0000) | final_script,
                "es": record_segment,
            }
        )
        for register, expected in expected_registers.items():
            actual = machine.reg_read(REGISTERS[register])
            if actual != expected:
                raise AssertionError(f"0x6fb9 {name}: {register}={actual:#x}, expected={expected:#x}")
        reciprocal_after = tuple(
            struct.unpack(
                "<H",
                machine.mem_read(
                    record_segment * 16 + ((reciprocal_offset + offset) & 0xFFFF),
                    2,
                ),
            )[0]
            for offset in (0, 2, 4)
        )
        expected_reciprocal = (0, 0, 0) if is_c4 else reciprocal_before
        if reciprocal_after != expected_reciprocal:
            raise AssertionError(
                f"0x6fb9 {name}: reciprocal={reciprocal_after}, expected={expected_reciprocal}"
            )
        if bytes(machine.mem_read(record_segment * 16 + relative_record, 6)) != relative_decoy:
            raise AssertionError(f"0x6fb9 {name}: base-relative record changed")
        for script_offset, expected in immutable_script:
            if bytes(machine.mem_read(data_segment * 16 + script_offset, 1)) != expected:
                raise AssertionError(f"0x6fb9 {name}: script changed")
        for segment, offset, expected in [
            (data_segment, 0x6724, data_pointer_decoy),
            (stack_segment, 0x6724, stack_pointer_decoy),
            (data_segment, 0x252A, b"\x11"),
            (data_segment, 0x2531, b"\x22"),
            (stack_segment, 0x252A, b"\x33"),
            (stack_segment, 0x2531, b"\x44"),
            (data_segment, field_table_offset, b"\x7f"),
        ]:
            if bytes(machine.mem_read(segment * 16 + offset, len(expected))) != expected:
                raise AssertionError(f"0x6fb9 {name}: segment decoy changed")
        expected_flags = logic_flags_16(0) if is_c4 else sub_flags_16(kind, 0xC4)
        flags = machine.reg_read(UC_X86_REG_EFLAGS)
        masks = {"cf": 1, "pf": 4, "af": 0x10, "zf": 0x40, "sf": 0x80, "of": 0x800}
        actual_flags = {flag: bool(flags & masks[flag]) for flag in expected_flags}
        if actual_flags != expected_flags:
            raise AssertionError(f"0x6fb9 {name}: flags={actual_flags}, expected={expected_flags}")
        if EXE[0x6FF2] != 0xC3:
            raise AssertionError("0x6fb9: expected near RET boundary")
        vectors.append(
            {
                "name": name,
                "record_base_offset_ignored": base_offset,
                "record_offset": record_offset,
                "old_record": [kind, related, record_tail],
                "record_after": [0, 0, 0],
                "related_kind": related_kind,
                "reciprocal_offset": reciprocal_offset,
                "reciprocal_after": list(expected_reciprocal),
                "sequence_active_after": sequence_after,
                "depth_step_after": depth_after,
                "final_script_offset": final_script,
                "defined_flags": expected_flags,
            }
        )
    return vectors


def byte_parser_mark_b16_vectors(entry: int, opcode: int) -> list[dict[str, object]]:
    data_segment = 0x4400
    extra_segment = 0x4800
    game_segment = 0x2C00
    stack_segment = 0x9000
    return_address = 0x6F00
    expected_code = bytes.fromhex("65c606160b01c3")
    vectors = []

    if EXE[entry : entry + len(expected_code)] != expected_code:
        raise AssertionError(f"{entry:#x}: unexpected handler bytes")

    for name, initial_flag in (("already_set", 1), ("overwrite_marker", 0xA5)):
        initial = {
            "eax": 0xA1A1BEEF,
            "ebx": 0xB2B22345,
            "ecx": 0xC3C33456,
            "edx": 0xD4D44567,
            "esi": 0xE5E55678,
            "edi": 0xF6F66789,
            "ebp": 0x9797789A,
            "ds": data_segment,
            "es": extra_segment,
            "fs": 0x4C00,
            "gs": game_segment,
            "ss": stack_segment,
            "flags": 0x0AD7,
        }
        stack_sentinel = bytes.fromhex("5aa59669")
        machine = execute(
            entry,
            return_address,
            initial,
            [
                (game_segment, 0x0B16, bytes([initial_flag])),
                (data_segment, 0x0B16, b"\x5a"),
                (stack_segment, 0x0B16, b"\xa5"),
                (
                    stack_segment,
                    0xFF00,
                    struct.pack("<H", return_address) + stack_sentinel,
                ),
                (0, return_address, b"\xcc"),
            ],
        )

        if machine.mem_read(game_segment * 16 + 0x0B16, 1) != b"\x01":
            raise AssertionError(f"{entry:#x} {name}: GS flag was not set")
        if machine.mem_read(data_segment * 16 + 0x0B16, 1) != b"\x5a":
            raise AssertionError(f"{entry:#x} {name}: DS decoy changed")
        if machine.mem_read(stack_segment * 16 + 0x0B16, 1) != b"\xa5":
            raise AssertionError(f"{entry:#x} {name}: SS decoy changed")
        for register, expected in initial.items():
            actual = machine.reg_read(REGISTERS[register])
            if actual != expected:
                raise AssertionError(
                    f"{entry:#x} {name}: {register}={actual:#x}, expected={expected:#x}"
                )
        if machine.reg_read(UC_X86_REG_SP) != 0xFF02:
            raise AssertionError(f"{entry:#x} {name}: near RET did not consume return word")
        actual_sentinel = bytes(machine.mem_read(stack_segment * 16 + 0xFF02, 4))
        if actual_sentinel != stack_sentinel:
            raise AssertionError(f"{entry:#x} {name}: stack sentinel changed")

        vectors.append(
            {
                "name": name,
                "entry": f"0x{entry:06x}",
                "dispatch_opcode": f"0x{opcode:02x}",
                "flag_before": initial_flag,
                "flag_after": 1,
                "code_bytes": expected_code.hex(),
                "stack_pointer_before": 0xFF00,
                "stack_pointer_after": 0xFF02,
                "registers_and_flags_preserved": True,
            }
        )
    return vectors


def credit_presenter_b_cryo_vectors() -> list[dict[str, object]]:
    data_segment = 0x4400
    destination_segment = 0x4800
    game_segment = 0x2C00
    stack_segment = 0x9000
    return_address = 0x6F00
    cases = [
        ("empty", 0x6800, b"\x00"),
        ("single", 0x6820, b"X\x00"),
        ("cryo_text", 0x6840, b"CRYO presents\x00"),
        ("high_bytes", 0x6880, b"\x80\xff\x01\x00"),
        ("script_wrap", 0xFFFE, b"AB\x00"),
    ]
    vectors = []

    for name, start, payload in cases:
        active_before = 0xA5
        timer_before = 0x5AA5
        destination_before = bytes([0xCC]) * (len(payload) + 2)
        stack_sentinel = bytes.fromhex("5aa59669")
        memory = [
            (destination_segment, 0x0E18, destination_before),
            (game_segment, 0x0E18, bytes([0xA5]) * len(destination_before)),
            (data_segment, 0x0E18, bytes([0x5A]) * len(destination_before)),
            (game_segment, 0x5E64, bytes([active_before])),
            (game_segment, 0x5E58, struct.pack("<H", timer_before)),
            (destination_segment, 0x5E64, b"\x11"),
            (destination_segment, 0x5E58, struct.pack("<H", 0x2222)),
            (data_segment, 0x5E64, b"\x33"),
            (data_segment, 0x5E58, struct.pack("<H", 0x4444)),
            (
                stack_segment,
                0xFF00,
                struct.pack("<H", return_address) + stack_sentinel,
            ),
            (0, return_address, b"\xcc"),
        ]
        immutable_source = []
        for byte_index, byte in enumerate(payload):
            source_offset = (start + byte_index) & 0xFFFF
            encoded = bytes([byte])
            memory.append((data_segment, source_offset, encoded))
            immutable_source.append((source_offset, encoded))

        initial = {
            "eax": 0xA1A1BEEF,
            "ebx": 0xB2B22345,
            "ecx": 0xC3C33456,
            "edx": 0xD4D44567,
            "esi": 0xE5E50000 | start,
            "edi": 0xF6F66789,
            "ebp": 0x9797789A,
            "ds": data_segment,
            "es": destination_segment,
            "fs": 0x4C00,
            "gs": game_segment,
            "ss": stack_segment,
            "flags": 0x0AD7,
        }
        order = []

        def capture(machine: Uc, address: int, _size: int) -> None:
            if address in (0x761B, 0x7621, 0x7628):
                order.append(
                    (
                        address,
                        machine.mem_read(game_segment * 16 + 0x5E64, 1)[0],
                        struct.unpack(
                            "<H", machine.mem_read(game_segment * 16 + 0x5E58, 2)
                        )[0],
                    )
                )

        machine = execute(
            0x7612,
            return_address,
            initial,
            memory,
            code_handler=capture,
        )
        expected_order = [
            (0x761B, active_before, timer_before),
            (0x7621, 1, timer_before),
            (0x7628, 1, 0),
        ]
        if order != expected_order:
            raise AssertionError(f"0x7612 {name}: order={order}, expected={expected_order}")

        destination_after = bytes(
            machine.mem_read(destination_segment * 16 + 0x0E18, len(destination_before))
        )
        expected_destination = payload + destination_before[len(payload) :]
        if destination_after != expected_destination:
            raise AssertionError(
                f"0x7612 {name}: destination={destination_after!r}, "
                f"expected={expected_destination!r}"
            )
        for source_offset, expected in immutable_source:
            if machine.mem_read(data_segment * 16 + source_offset, 1) != expected:
                raise AssertionError(f"0x7612 {name}: source changed")
        for segment, offset, expected in [
            (game_segment, 0x0E18, bytes([0xA5]) * len(destination_before)),
            (data_segment, 0x0E18, bytes([0x5A]) * len(destination_before)),
            (destination_segment, 0x5E64, b"\x11"),
            (destination_segment, 0x5E58, struct.pack("<H", 0x2222)),
            (data_segment, 0x5E64, b"\x33"),
            (data_segment, 0x5E58, struct.pack("<H", 0x4444)),
        ]:
            if bytes(machine.mem_read(segment * 16 + offset, len(expected))) != expected:
                raise AssertionError(f"0x7612 {name}: segment decoy changed")

        expected_registers = dict(initial)
        del expected_registers["flags"]
        expected_registers.update(
            {
                "eax": (initial["eax"] & 0xFFFFFF00),
                "esi": (initial["esi"] & 0xFFFF0000)
                | ((start + len(payload)) & 0xFFFF),
                "edi": (initial["edi"] & 0xFFFF0000) | (0x0E18 + len(payload)),
            }
        )
        for register, expected in expected_registers.items():
            actual = machine.reg_read(REGISTERS[register])
            if actual != expected:
                raise AssertionError(
                    f"0x7612 {name}: {register}={actual:#x}, expected={expected:#x}"
                )
        expected_flags = {"cf": False, "pf": True, "zf": True, "sf": False, "of": False}
        flags = machine.reg_read(UC_X86_REG_EFLAGS)
        masks = {"cf": 1, "pf": 4, "zf": 0x40, "sf": 0x80, "of": 0x800}
        actual_flags = {flag: bool(flags & masks[flag]) for flag in expected_flags}
        if actual_flags != expected_flags:
            raise AssertionError(
                f"0x7612 {name}: flags={actual_flags}, expected={expected_flags}"
            )
        if machine.reg_read(UC_X86_REG_SP) != 0xFF02:
            raise AssertionError(f"0x7612 {name}: near RET did not consume return word")
        if bytes(machine.mem_read(stack_segment * 16 + 0xFF02, 4)) != stack_sentinel:
            raise AssertionError(f"0x7612 {name}: stack sentinel changed")
        if EXE[0x7628] != 0xC3:
            raise AssertionError("0x7612: expected near RET boundary")

        vectors.append(
            {
                "name": name,
                "source_offset": start,
                "input_hex": payload.hex(),
                "destination_offset": 0x0E18,
                "final_source_offset": (start + len(payload)) & 0xFFFF,
                "final_destination_offset": 0x0E18 + len(payload),
                "reveal_active_after": 1,
                "reveal_timer_after": 0,
                "registers_and_segments_verified": True,
                "defined_flags": expected_flags,
            }
        )
    return vectors


def byte_parser_copy_printable_vectors(
    entry: int, destination_offset: int, opcode: int
) -> list[dict[str, object]]:
    data_segment = 0x4400
    destination_segment = 0x4800
    game_segment = 0x2C00
    stack_segment = 0x9000
    return_address = 0x6F00
    cases = [
        ("zero_at_offset_zero", 0, b"\x00"),
        ("immediate_control", 0x6800, b"\x1f"),
        ("immediate_high", 0x6820, b"\x80"),
        ("max_printable", 0x6840, b"\x7f\x00"),
        ("text_then_control", 0x6860, b"ABC\x1f"),
        ("text_then_high", 0x6880, b"Z\xff"),
        ("script_wrap", 0xFFFE, b"AB\x00"),
        ("high_at_segment_end", 0xFFFF, b"\x80"),
    ]
    expected_tail = bytes.fromhex("ac0ac078073c207203aaebf44e26c60500c3")
    vectors = []

    if EXE[entry : entry + 1] != b"\xbf":
        raise AssertionError(f"{entry:#x}: expected MOV DI entry")
    if struct.unpack("<H", EXE[entry + 1 : entry + 3])[0] != destination_offset:
        raise AssertionError(f"{entry:#x}: unexpected destination immediate")
    if EXE[entry + 3 : entry + 21] != expected_tail:
        raise AssertionError(f"{entry:#x}: unexpected printable-copy body")

    for name, start, payload in cases:
        stop_index = next(
            index
            for index, byte in enumerate(payload)
            if byte < 0x20 or byte >= 0x80
        )
        copied = payload[:stop_index]
        stop_byte = payload[stop_index]
        destination_before = bytes([0xCC]) * (len(copied) + 3)
        stack_sentinel = bytes.fromhex("5aa59669")
        memory = [
            (destination_segment, destination_offset, destination_before),
            (game_segment, destination_offset, bytes([0xA5]) * len(destination_before)),
            (data_segment, destination_offset, bytes([0x5A]) * len(destination_before)),
            (
                stack_segment,
                0xFF00,
                struct.pack("<H", return_address) + stack_sentinel,
            ),
            (0, return_address, b"\xcc"),
        ]
        immutable_source = []
        for byte_index, byte in enumerate(payload):
            source_offset = (start + byte_index) & 0xFFFF
            encoded = bytes([byte])
            memory.append((data_segment, source_offset, encoded))
            immutable_source.append((source_offset, encoded))

        initial = {
            "eax": 0xA1A1BE55,
            "ebx": 0xB2B22345,
            "ecx": 0xC3C33456,
            "edx": 0xD4D44567,
            "esi": 0xE5E50000 | start,
            "edi": 0xF6F66789,
            "ebp": 0x9797789A,
            "ds": data_segment,
            "es": destination_segment,
            "fs": 0x4C00,
            "gs": game_segment,
            "ss": stack_segment,
            "flags": 0x0AD7,
        }
        machine = execute(entry, return_address, initial, memory)

        expected_destination = (
            copied + b"\x00" + destination_before[len(copied) + 1 :]
        )
        destination_after = bytes(
            machine.mem_read(
                destination_segment * 16 + destination_offset,
                len(destination_before),
            )
        )
        if destination_after != expected_destination:
            raise AssertionError(
                f"{entry:#x} {name}: destination={destination_after!r}, "
                f"expected={expected_destination!r}"
            )
        for source_offset, expected in immutable_source:
            if machine.mem_read(data_segment * 16 + source_offset, 1) != expected:
                raise AssertionError(f"{entry:#x} {name}: source changed")
        for segment, expected_byte in ((game_segment, 0xA5), (data_segment, 0x5A)):
            actual = bytes(
                machine.mem_read(
                    segment * 16 + destination_offset,
                    len(destination_before),
                )
            )
            expected = bytes([expected_byte]) * len(destination_before)
            if actual != expected:
                raise AssertionError(f"{entry:#x} {name}: segment decoy changed")

        final_source_offset = (start + stop_index) & 0xFFFF
        final_destination_offset = destination_offset + len(copied)
        expected_registers = dict(initial)
        del expected_registers["flags"]
        expected_registers.update(
            {
                "eax": (initial["eax"] & 0xFFFFFF00) | stop_byte,
                "esi": (initial["esi"] & 0xFFFF0000) | final_source_offset,
                "edi": (initial["edi"] & 0xFFFF0000) | final_destination_offset,
                "sp": 0xFF02,
            }
        )
        for register, expected in expected_registers.items():
            actual = machine.reg_read(REGISTERS[register])
            if actual != expected:
                raise AssertionError(
                    f"{entry:#x} {name}: {register}={actual:#x}, expected={expected:#x}"
                )

        dec_input = (final_source_offset + 1) & 0xFFFF
        dec_result = final_source_offset
        expected_flags = {
            "cf": stop_byte < 0x20,
            "pf": (dec_result & 0xFF).bit_count() % 2 == 0,
            "af": (dec_input & 0x0F) == 0,
            "zf": dec_result == 0,
            "sf": bool(dec_result & 0x8000),
            "of": dec_input == 0x8000,
        }
        flags = machine.reg_read(UC_X86_REG_EFLAGS)
        masks = {"cf": 1, "pf": 4, "af": 0x10, "zf": 0x40, "sf": 0x80, "of": 0x800}
        actual_flags = {flag: bool(flags & masks[flag]) for flag in expected_flags}
        if actual_flags != expected_flags:
            raise AssertionError(
                f"{entry:#x} {name}: flags={actual_flags}, expected={expected_flags}"
            )
        if machine.reg_read(UC_X86_REG_SP) != 0xFF02:
            raise AssertionError(f"{entry:#x} {name}: near RET did not consume return word")
        if bytes(machine.mem_read(stack_segment * 16 + 0xFF02, 4)) != stack_sentinel:
            raise AssertionError(f"{entry:#x} {name}: stack sentinel changed")

        vectors.append(
            {
                "name": name,
                "entry": f"0x{entry:06x}",
                "dispatch_opcode": f"0x{opcode:02x}",
                "source_offset": start,
                "input_hex": payload.hex(),
                "copied_hex": copied.hex(),
                "stopping_byte": stop_byte,
                "destination_offset": destination_offset,
                "final_source_offset": final_source_offset,
                "final_destination_offset": final_destination_offset,
                "defined_flags": expected_flags,
            }
        )
    return vectors


def byte_parser_snd_bank_name_load_vectors() -> list[dict[str, object]]:
    entry = 0x763E
    data_segment = 0x4400
    destination_segment = 0x4800
    game_segment = 0x2C00
    stack_segment = 0x9000
    return_address = 0x6F00
    cases = [
        ("call_empty", 0x6800, b"\x00", 0x0000),
        ("skip_empty", 0x6820, b"\x00", 0x0001),
        ("call_text", 0x6840, b"BANK.SND\x1f", 0xA500),
        ("skip_text", 0x6880, b"SON.SND\x80", 0x5A01),
        ("call_max_printable", 0x68C0, b"\x7f\x00", 0xFFFE),
        ("skip_high", 0x68E0, b"\xff", 0x0003),
        ("call_script_wrap", 0xFFFE, b"AB\x00", 0x0000),
        ("skip_high_at_end", 0xFFFF, b"\x80", 0x8001),
    ]
    vectors = []

    for name, start, payload, ui_state in cases:
        stop_index = next(
            index
            for index, byte in enumerate(payload)
            if byte < 0x20 or byte >= 0x80
        )
        copied = payload[:stop_index]
        stop_byte = payload[stop_index]
        should_call = (ui_state & 1) == 0
        destination_before = bytes([0xCC]) * (len(copied) + 3)
        game_path = b"X:\\SOUND\\OLD.SND\x00"
        stack_sentinel = bytes.fromhex("5aa59669")
        memory = [
            # Runtime far address 0B1B:0855 is linear 0xBA05; file offset
            # 0xC005 includes the 0x600-byte executable header.
            (0, 0xBA05, EXE[0xC005:0xC1E6]),
            (destination_segment, 0x0D09, destination_before),
            (game_segment, 0x0D06, game_path),
            (data_segment, 0x0D06, bytes([0x5A]) * len(game_path)),
            (game_segment, 0x2793, struct.pack("<H", ui_state)),
            (data_segment, 0x2793, struct.pack("<H", ui_state ^ 0xFFFF)),
            (game_segment, 0x0ADE, b"\x00"),
            (
                stack_segment,
                0xFF00,
                struct.pack("<H", return_address) + stack_sentinel,
            ),
            (0, return_address, b"\xcc"),
        ]
        immutable_source = []
        for byte_index, byte in enumerate(payload):
            source_offset = (start + byte_index) & 0xFFFF
            encoded = bytes([byte])
            memory.append((data_segment, source_offset, encoded))
            immutable_source.append((source_offset, encoded))

        initial = {
            "eax": 0xA1A1BE55,
            "ebx": 0xB2B22345,
            "ecx": 0xC3C33456,
            "edx": 0xD4D44567,
            "esi": 0xE5E50000 | start,
            "edi": 0xF6F66789,
            "ebp": 0x9797789A,
            "sp": 0xFF00,
            "ds": data_segment,
            "es": destination_segment,
            "fs": 0x4C00,
            "gs": game_segment,
            "ss": stack_segment,
            "flags": 0x0AD7,
        }
        calls = []

        def capture(machine: Uc, address: int, _size: int) -> None:
            if address == 0xBA05:
                calls.append(
                    (
                        machine.reg_read(UC_X86_REG_AX),
                        machine.reg_read(UC_X86_REG_DS),
                        machine.reg_read(UC_X86_REG_SI),
                        machine.reg_read(UC_X86_REG_ES),
                        machine.reg_read(UC_X86_REG_DI),
                        machine.reg_read(UC_X86_REG_SP),
                        machine.reg_read(UC_X86_REG_CS),
                    )
                )

        machine = execute(
            entry,
            return_address,
            initial,
            memory,
            code_handler=capture,
        )
        expected_calls = (
            [
                (
                    1,
                    game_segment,
                    0x0D06,
                    destination_segment,
                    0x0D09 + len(copied),
                    0xFEF8,
                    0x0B1B,
                )
            ]
            if should_call
            else []
        )
        if calls != expected_calls:
            raise AssertionError(f"0x763e {name}: calls={calls}, expected={expected_calls}")

        expected_destination = (
            copied + b"\x00" + destination_before[len(copied) + 1 :]
        )
        destination_after = bytes(
            machine.mem_read(
                destination_segment * 16 + 0x0D09,
                len(destination_before),
            )
        )
        if destination_after != expected_destination:
            raise AssertionError(
                f"0x763e {name}: destination={destination_after!r}, "
                f"expected={expected_destination!r}"
            )
        for source_offset, expected in immutable_source:
            if machine.mem_read(data_segment * 16 + source_offset, 1) != expected:
                raise AssertionError(f"0x763e {name}: source changed")
        if bytes(machine.mem_read(game_segment * 16 + 0x0D06, len(game_path))) != game_path:
            raise AssertionError(f"0x763e {name}: game path changed")
        if bytes(machine.mem_read(data_segment * 16 + 0x0D06, len(game_path))) != bytes(
            [0x5A]
        ) * len(game_path):
            raise AssertionError(f"0x763e {name}: DS path decoy changed")
        if struct.unpack(
            "<H", machine.mem_read(game_segment * 16 + 0x2793, 2)
        )[0] != ui_state:
            raise AssertionError(f"0x763e {name}: UI state changed")

        final_source_offset = (start + stop_index) & 0xFFFF
        final_destination_offset = 0x0D09 + len(copied)
        expected_registers = dict(initial)
        del expected_registers["flags"]
        expected_registers.update(
            {
                "eax": (
                    (initial["eax"] & 0xFFFF0000) | 1
                    if should_call
                    else (initial["eax"] & 0xFFFFFF00) | stop_byte
                ),
                "esi": (initial["esi"] & 0xFFFF0000) | final_source_offset,
                "edi": (initial["edi"] & 0xFFFF0000) | final_destination_offset,
                "sp": 0xFF02,
            }
        )
        for register, expected in expected_registers.items():
            actual = machine.reg_read(REGISTERS[register])
            if actual != expected:
                raise AssertionError(
                    f"0x763e {name}: {register}={actual:#x}, expected={expected:#x}"
                )
        if machine.reg_read(UC_X86_REG_CS) != 0:
            raise AssertionError(f"0x763e {name}: far call did not restore CS")

        if should_call:
            expected_flags = {
                "cf": False,
                "pf": True,
                "zf": True,
                "sf": False,
                "of": False,
            }
        else:
            expected_flags = {
                "cf": False,
                "pf": False,
                "zf": False,
                "sf": False,
                "of": False,
            }
        flags = machine.reg_read(UC_X86_REG_EFLAGS)
        masks = {"cf": 1, "pf": 4, "zf": 0x40, "sf": 0x80, "of": 0x800}
        actual_flags = {flag: bool(flags & masks[flag]) for flag in expected_flags}
        if actual_flags != expected_flags:
            raise AssertionError(
                f"0x763e {name}: flags={actual_flags}, expected={expected_flags}"
            )
        if machine.reg_read(UC_X86_REG_SP) != 0xFF02:
            raise AssertionError(f"0x763e {name}: near RET did not consume return word")
        if bytes(machine.mem_read(stack_segment * 16 + 0xFF02, 4)) != stack_sentinel:
            raise AssertionError(f"0x763e {name}: stack sentinel changed")

        vectors.append(
            {
                "name": name,
                "dispatch_opcode": "0x11",
                "source_offset": start,
                "input_hex": payload.hex(),
                "copied_hex": copied.hex(),
                "stopping_byte": stop_byte,
                "final_source_offset": final_source_offset,
                "final_destination_offset": final_destination_offset,
                "ui_state": ui_state,
                "loader_called": should_call,
                "loader_mode": 1 if should_call else None,
                "loader_path_offset": 0x0D06 if should_call else None,
                "defined_flags": expected_flags,
            }
        )
    return vectors


def dlg_line_asset_table_fill_vectors() -> list[dict[str, object]]:
    entry = 0x7684
    dispatcher_entry = 0x74DD
    dispatcher_return = 0x74ED
    data_segment = 0x4400
    destination_segment = 0x4800
    game_segment = 0x2C00
    stack_segment = 0x9000
    direct_return = 0x6F00
    cases = [
        {
            "name": "dispatch_id_01_empty",
            "id": 0x01,
            "detail": b"\x00",
        },
        {
            "name": "dispatch_id_02_text_low_stop",
            "id": 0x02,
            "detail": b"TALK.HNM\x1f",
        },
        {
            "name": "dispatch_id_04_high_stop",
            "id": 0x04,
            "detail": b"A\x80",
        },
        {
            "name": "dispatch_shipped_id_ff",
            "id": 0xFF,
            "detail": b"MG_SCR1.HNM\x00",
        },
        {
            "name": "dispatch_id_80",
            "id": 0x80,
            "detail": b"\xff",
        },
        {
            "name": "dispatch_id_00",
            "id": 0x00,
            "detail": b"FD\x00",
        },
        {
            "name": "dispatch_source_wrap",
            "id": 0x03,
            "detail": b"A\x00",
            "start": 0xFFFD,
        },
        {
            "name": "dispatch_asset_destination_wrap",
            "id": 0x01,
            "detail": b"\x7f\x00",
            "asset_cursor": 0xFFFE,
            "detail_cursor": 0x5000,
        },
        {
            "name": "dispatch_detail_destination_wrap",
            "id": 0x02,
            "detail": b"AB\x00",
            "asset_cursor": 0x5000,
            "detail_cursor": 0xFFFF,
        },
        {
            "name": "direct_sf_clear_id_ff",
            "id": 0xFF,
            "detail": b"\x00",
            "direct": True,
            "flags": 0x0002,
        },
        {
            "name": "direct_sf_set_id_ff",
            "id": 0xFF,
            "detail": b"\x00",
            "direct": True,
            "flags": 0x0082,
        },
        {
            "name": "direct_sf_set_id_01",
            "id": 0x01,
            "detail": b"B\x00",
            "direct": True,
            "flags": 0x0883,
        },
    ]
    vectors = []

    def wrapped_bytes(machine: Uc, segment: int, offset: int, length: int) -> bytes:
        return bytes(
            machine.mem_read(segment * 16 + ((offset + index) & 0xFFFF), 1)[0]
            for index in range(length)
        )

    for case_index, case in enumerate(cases):
        name = str(case["name"])
        asset_id = int(case["id"])
        detail = bytes(case["detail"])
        direct = bool(case.get("direct", False))
        start = int(case.get("start", 0x6000 + case_index * 0x80))
        asset_cursor = int(case.get("asset_cursor", 0x3000 + case_index * 0x20))
        detail_cursor = int(case.get("detail_cursor", 0x4000 + case_index * 0x40))
        flags_before = int(case.get("flags", 0x0AD7))
        stop_index = next(
            index
            for index, byte in enumerate(detail)
            if byte < 0x20 or byte >= 0x80
        )
        copied = detail[:stop_index]
        stop_byte = detail[stop_index]
        stream = bytes([asset_id]) + detail if direct else bytes([0x07, asset_id]) + detail
        id_offset = start if direct else (start + 1) & 0xFFFF
        stop_offset = (id_offset + 1 + stop_index) & 0xFFFF
        final_asset_cursor = (asset_cursor + 4) & 0xFFFF
        final_detail_global = (detail_cursor + 0x1A) & 0xFFFF
        final_detail_cursor = (detail_cursor + len(copied)) & 0xFFFF

        signed_id = asset_id if asset_id < 0x80 else asset_id - 0x100
        sign_extended_id = signed_id & 0xFFFF
        handler_sf = bool(flags_before & 0x80) if direct else False
        if handler_sf:
            stored_id = sign_extended_id
        else:
            stored_id = (
                ((sign_extended_id - 1) << 4) + 0x0DD7
            ) & 0xFFFF

        asset_before = bytes([0xCC]) * 6
        detail_before = bytes([0xDD]) * (len(copied) + 3)
        stack_sentinel = bytes.fromhex("5aa59669")
        memory = [
            (game_segment, 0x1FAF, struct.pack("<H", asset_cursor)),
            (game_segment, 0x1FAD, struct.pack("<H", detail_cursor)),
            (data_segment, 0x1FAF, b"\x5a\xa5"),
            (data_segment, 0x1FAD, b"\x69\x96"),
            (destination_segment, 0x1FAF, b"\xc3\x3c"),
            (destination_segment, 0x1FAD, b"\x87\x78"),
            (stack_segment, 0x1FAF, b"\x0f\xf0"),
            (stack_segment, 0x1FAD, b"\x55\xaa"),
            (0, direct_return, b"\xcc"),
        ]
        for index, byte in enumerate(asset_before):
            memory.append(
                (destination_segment, (asset_cursor + index) & 0xFFFF, bytes([byte]))
            )
        for index, byte in enumerate(detail_before):
            memory.append(
                (destination_segment, (detail_cursor + index) & 0xFFFF, bytes([byte]))
            )
        immutable_source = []
        for index, byte in enumerate(stream):
            source_offset = (start + index) & 0xFFFF
            encoded = bytes([byte])
            memory.append((data_segment, source_offset, encoded))
            memory.append((game_segment, source_offset, b"\xa5"))
            immutable_source.append((source_offset, encoded))

        if direct:
            memory.append(
                (
                    stack_segment,
                    0xFF00,
                    struct.pack("<H", direct_return) + stack_sentinel,
                )
            )
            run_entry = entry
            stop_address = direct_return
            expected_sp = 0xFF02
            initial_eax = 0xA1A1BEEF
        else:
            memory.extend(
                [
                    (0, 0x218A, struct.pack("<H", entry)),
                    (stack_segment, 0xFEFC, b"\x13\x57"),
                    (stack_segment, 0xFF00, stack_sentinel),
                ]
            )
            run_entry = dispatcher_entry
            stop_address = dispatcher_return
            expected_sp = 0xFF00
            initial_eax = 0x0000BEEF

        initial = {
            "eax": initial_eax,
            "ebx": 0xB2B22345,
            "ecx": 0xC3C33456,
            "edx": 0xD4D44567,
            "esi": 0xE5E50000 | start,
            "edi": 0xF6F66789,
            "ebp": 0x9797789A,
            "sp": 0xFF00,
            "ds": data_segment,
            "es": destination_segment,
            "fs": 0x4C00,
            "gs": game_segment,
            "ss": stack_segment,
            "flags": flags_before,
        }
        phases = []

        def capture(machine: Uc, address: int, _size: int) -> None:
            if address not in (0x7684, 0x768B, 0x7694, 0x769D, 0x76A8, 0x76B9):
                return
            phases.append(
                (
                    address,
                    machine.reg_read(UC_X86_REG_AX),
                    machine.reg_read(UC_X86_REG_SI),
                    machine.reg_read(UC_X86_REG_DI),
                    bool(machine.reg_read(UC_X86_REG_EFLAGS) & 0x80),
                    struct.unpack(
                        "<H", machine.mem_read(game_segment * 16 + 0x1FAF, 2)
                    )[0],
                    struct.unpack(
                        "<H", machine.mem_read(game_segment * 16 + 0x1FAD, 2)
                    )[0],
                )
            )

        machine = execute(
            run_entry,
            stop_address,
            initial,
            memory,
            code_handler=capture,
        )
        handler_entries = [phase for phase in phases if phase[0] == 0x7684]
        if len(handler_entries) != 1 or handler_entries[0][4] != handler_sf:
            raise AssertionError(
                f"0x7684 {name}: handler entry SF={handler_entries}, "
                f"expected={handler_sf}"
            )
        cbw_phases = [phase for phase in phases if phase[0] == 0x768B]
        if (
            len(cbw_phases) != 1
            or cbw_phases[0][1] != sign_extended_id
            or cbw_phases[0][4] != handler_sf
        ):
            raise AssertionError(
                f"0x7684 {name}: CBW phase={cbw_phases}, "
                f"expected AX={sign_extended_id:#x}, SF={handler_sf}"
            )
        store_phases = [phase for phase in phases if phase[0] == 0x7694]
        if len(store_phases) != 1 or store_phases[0][1] != stored_id:
            raise AssertionError(
                f"0x7684 {name}: store phase={store_phases}, "
                f"expected AX={stored_id:#x}"
            )

        asset_after = wrapped_bytes(machine, destination_segment, asset_cursor, 6)
        expected_asset = struct.pack("<H", stored_id) + asset_before[2:]
        if asset_after != expected_asset:
            raise AssertionError(
                f"0x7684 {name}: asset={asset_after.hex()}, "
                f"expected={expected_asset.hex()}"
            )
        detail_after = wrapped_bytes(
            machine, destination_segment, detail_cursor, len(detail_before)
        )
        expected_detail = copied + b"\x00" + detail_before[len(copied) + 1 :]
        if detail_after != expected_detail:
            raise AssertionError(
                f"0x7684 {name}: detail={detail_after!r}, expected={expected_detail!r}"
            )
        actual_asset_cursor = struct.unpack(
            "<H", machine.mem_read(game_segment * 16 + 0x1FAF, 2)
        )[0]
        actual_detail_global = struct.unpack(
            "<H", machine.mem_read(game_segment * 16 + 0x1FAD, 2)
        )[0]
        if (
            actual_asset_cursor != final_asset_cursor
            or actual_detail_global != final_detail_global
        ):
            raise AssertionError(
                f"0x7684 {name}: cursors={(actual_asset_cursor, actual_detail_global)}, "
                f"expected={(final_asset_cursor, final_detail_global)}"
            )
        for segment, offset, expected in (
            (data_segment, 0x1FAF, b"\x5a\xa5"),
            (data_segment, 0x1FAD, b"\x69\x96"),
            (destination_segment, 0x1FAF, b"\xc3\x3c"),
            (destination_segment, 0x1FAD, b"\x87\x78"),
            (stack_segment, 0x1FAF, b"\x0f\xf0"),
            (stack_segment, 0x1FAD, b"\x55\xaa"),
        ):
            actual = bytes(machine.mem_read(segment * 16 + offset, len(expected)))
            if actual != expected:
                raise AssertionError(f"0x7684 {name}: cursor decoy changed")
        for source_offset, expected in immutable_source:
            actual = bytes(machine.mem_read(data_segment * 16 + source_offset, 1))
            if actual != expected:
                raise AssertionError(f"0x7684 {name}: source changed")

        expected_registers = dict(initial)
        del expected_registers["flags"]
        expected_registers.update(
            {
                "eax": (initial_eax & 0xFFFF0000) | (stored_id & 0xFF00) | stop_byte,
                "esi": (initial["esi"] & 0xFFFF0000) | stop_offset,
                "edi": (initial["edi"] & 0xFFFF0000) | final_detail_cursor,
                "sp": expected_sp,
            }
        )
        for register, expected in expected_registers.items():
            actual = machine.reg_read(REGISTERS[register])
            if actual != expected:
                raise AssertionError(
                    f"0x7684 {name}: {register}={actual:#x}, expected={expected:#x}"
                )

        dec_input = (stop_offset + 1) & 0xFFFF
        expected_flags = {
            "cf": stop_byte < 0x20,
            "pf": (stop_offset & 0xFF).bit_count() % 2 == 0,
            "af": (dec_input & 0x0F) == 0,
            "zf": stop_offset == 0,
            "sf": bool(stop_offset & 0x8000),
            "of": dec_input == 0x8000,
        }
        flags_after = machine.reg_read(UC_X86_REG_EFLAGS)
        flag_masks = {
            "cf": 0x0001,
            "pf": 0x0004,
            "af": 0x0010,
            "zf": 0x0040,
            "sf": 0x0080,
            "of": 0x0800,
        }
        actual_flags = {
            flag: bool(flags_after & mask) for flag, mask in flag_masks.items()
        }
        if actual_flags != expected_flags:
            raise AssertionError(
                f"0x7684 {name}: flags={actual_flags}, expected={expected_flags}"
            )
        if direct:
            if wrapped_bytes(machine, stack_segment, 0xFF02, 4) != stack_sentinel:
                raise AssertionError(f"0x7684 {name}: direct stack sentinel changed")
        else:
            if struct.unpack(
                "<H", machine.mem_read(stack_segment * 16 + 0xFEFE, 2)
            )[0] != dispatcher_return:
                raise AssertionError(f"0x7684 {name}: dispatcher return was not pushed")
            if bytes(machine.mem_read(stack_segment * 16 + 0xFF00, 4)) != stack_sentinel:
                raise AssertionError(f"0x7684 {name}: dispatcher stack sentinel changed")

        vectors.append(
            {
                "name": name,
                "entry_mode": "direct" if direct else "real_dispatcher",
                "asset_id": asset_id,
                "handler_entry_sf": handler_sf,
                "stored_id": stored_id,
                "copied_hex": copied.hex(),
                "stopping_byte": stop_byte,
                "asset_cursor_before": asset_cursor,
                "asset_cursor_after": actual_asset_cursor,
                "detail_cursor_before": detail_cursor,
                "detail_cursor_after": actual_detail_global,
                "final_source_offset": stop_offset,
                "final_destination_offset": final_detail_cursor,
                "defined_flags": expected_flags,
            }
        )
    return vectors


def index_lookup_1fd7_vectors() -> list[dict[str, object]]:
    entry = 0x76EA
    dispatcher_entry = 0x74DD
    dispatcher_return = 0x74ED
    direct_return = 0x6F00
    path_helper_entry = 0x23F2  # Runtime 01CE:0712.
    file_helper_entry = 0x2301  # Runtime 01CE:0621.
    data_segment = 0x4400
    destination_segment = 0x4800
    game_segment = 0x2C00
    back_buffer_segment = 0x4C00
    stack_segment = 0x9000
    back_buffer_offset = 0x5200
    cases = [
        {
            "name": "dispatch_ui_busy_empty",
            "id": 0x01,
            "detail": b"\x00",
            "ui_state": 0x0001,
            "ems_handle": 0x1234,
            "xms_handle": 0x2345,
            "call": None,
        },
        {
            "name": "dispatch_ui_busy_id_ff",
            "id": 0xFF,
            "detail": b"NEG\x80",
            "ui_state": 0xA503,
            "ems_handle": 0x1234,
            "xms_handle": -1,
            "call": None,
        },
        {
            "name": "dispatch_ems_path_call",
            "id": 0x02,
            "detail": b"FILE.BIN\x00",
            "ui_state": 0,
            "ems_handle": 0x1234,
            "xms_handle": -1,
            "call": "path",
        },
        {
            "name": "dispatch_ems_preferred_over_xms",
            "id": 0x03,
            "detail": b"A\x1f",
            "ui_state": 0,
            "ems_handle": 0,
            "xms_handle": 0x2345,
            "call": "path",
        },
        {
            "name": "dispatch_xms_file_call",
            "id": 0x04,
            "detail": b"ASSET.DAT\x00",
            "ui_state": 0,
            "ems_handle": -1,
            "xms_handle": 0x2345,
            "call": "file",
        },
        {
            "name": "dispatch_no_backend",
            "id": 0x00,
            "detail": b"\x7f\x00",
            "ui_state": 0,
            "ems_handle": -1,
            "xms_handle": -1,
            "call": None,
        },
        {
            "name": "dispatch_source_wrap_id_80",
            "id": 0x80,
            "detail": b"A\x00",
            "ui_state": 1,
            "ems_handle": -1,
            "xms_handle": -1,
            "call": None,
            "start": 0xFFFD,
        },
        {
            "name": "direct_sf_clear_id_ff",
            "id": 0xFF,
            "detail": b"\x00",
            "ui_state": 1,
            "ems_handle": -1,
            "xms_handle": -1,
            "call": None,
            "direct": True,
            "flags": 0x0002,
        },
        {
            "name": "direct_sf_set_id_ff",
            "id": 0xFF,
            "detail": b"\x00",
            "ui_state": 1,
            "ems_handle": -1,
            "xms_handle": -1,
            "call": None,
            "direct": True,
            "flags": 0x0082,
        },
        {
            "name": "direct_sf_set_id_01",
            "id": 0x01,
            "detail": b"B\x00",
            "ui_state": 1,
            "ems_handle": -1,
            "xms_handle": -1,
            "call": None,
            "direct": True,
            "flags": 0x0883,
        },
    ]
    expected_hash = "ac97646e8463df2225f218348fcdd1694f485651b279fa2fc29b56d8efff43b7"
    if hashlib.sha256(EXE[entry : entry + 106]).hexdigest() != expected_hash:
        raise AssertionError("0x76ea: recovered 106-byte body changed")

    vectors = []
    for case_index, case in enumerate(cases):
        name = str(case["name"])
        asset_id = int(case["id"])
        detail = bytes(case["detail"])
        ui_state = int(case["ui_state"])
        ems_handle = int(case["ems_handle"])
        xms_handle = int(case["xms_handle"])
        expected_call = case["call"]
        direct = bool(case.get("direct", False))
        start = int(case.get("start", 0x6200 + case_index * 0x80))
        flags_before = int(case.get("flags", 0x0AD7))
        stop_index = next(
            index
            for index, byte in enumerate(detail)
            if byte < 0x20 or byte >= 0x80
        )
        copied = detail[:stop_index]
        stop_byte = detail[stop_index]
        stream = bytes([asset_id]) + detail if direct else bytes([0x0B, asset_id]) + detail
        id_offset = start if direct else (start + 1) & 0xFFFF
        stop_offset = (id_offset + 1 + stop_index) & 0xFFFF

        signed_id = asset_id if asset_id < 0x80 else asset_id - 0x100
        sign_extended_id = signed_id & 0xFFFF
        handler_sf = bool(flags_before & 0x80) if direct else False
        if handler_sf:
            stored_id = sign_extended_id
        else:
            stored_id = (
                ((sign_extended_id - 1) << 4) + 0x0DD7
            ) & 0xFFFF

        index_before = b"\xcc\xcc"
        text_before = bytes([0xDD]) * (len(copied) + 3)
        game_path = b"fd\\SOURCE.DAT\x00"
        back_pointer = struct.pack("<HH", back_buffer_offset, back_buffer_segment)
        stack_sentinel = bytes.fromhex("5aa59669")
        memory = [
            # The helper bodies are separate recovered routines. A RETF at each
            # runtime entry isolates this caller while preserving far-call stack flow.
            (0, path_helper_entry, b"\xcb"),
            (0, file_helper_entry, b"\xcb"),
            (destination_segment, 0x1FD7, index_before),
            (destination_segment, 0x213A, text_before),
            (game_segment, 0x1FD7, b"\x5a\xa5"),
            (game_segment, 0x2137, game_path),
            (game_segment, 0x2793, struct.pack("<H", ui_state)),
            (game_segment, 0x0A58, struct.pack("<h", ems_handle)),
            (game_segment, 0x0A56, struct.pack("<h", xms_handle)),
            (game_segment, 0x5229, back_pointer),
            (data_segment, 0x1FD7, b"\x69\x96"),
            (data_segment, 0x2137, bytes([0xA5]) * len(game_path)),
            (data_segment, 0x2793, b"\x87\x78"),
            (data_segment, 0x0A58, b"\x34\x12"),
            (data_segment, 0x0A56, b"\x78\x56"),
            (data_segment, 0x5229, b"\x00\x11\x22\x33"),
            (back_buffer_segment, back_buffer_offset, b"\xde\xad\xbe\xef"),
            (0, direct_return, b"\xcc"),
        ]
        immutable_source = []
        for index, byte in enumerate(stream):
            source_offset = (start + index) & 0xFFFF
            encoded = bytes([byte])
            memory.append((data_segment, source_offset, encoded))
            immutable_source.append((source_offset, encoded))

        if direct:
            memory.append(
                (
                    stack_segment,
                    0xFF00,
                    struct.pack("<H", direct_return) + stack_sentinel,
                )
            )
            run_entry = entry
            stop_address = direct_return
            expected_sp = 0xFF02
            initial_eax = 0xA1A1BEEF
            expected_call_sp = 0xFEF4
        else:
            memory.extend(
                [
                    (0, 0x2192, struct.pack("<H", entry)),
                    (stack_segment, 0xFEFC, b"\x13\x57"),
                    (stack_segment, 0xFF00, stack_sentinel),
                ]
            )
            run_entry = dispatcher_entry
            stop_address = dispatcher_return
            expected_sp = 0xFF00
            initial_eax = 0x0000BEEF
            expected_call_sp = 0xFEF2

        initial = {
            "eax": initial_eax,
            "ebx": 0xB2B22345,
            "ecx": 0xC3C33456,
            "edx": 0xD4D44567,
            "esi": 0xE5E50000 | start,
            "edi": 0xF6F66789,
            "ebp": 0x9797789A,
            "sp": 0xFF00,
            "ds": data_segment,
            "es": destination_segment,
            "fs": 0x5000,
            "gs": game_segment,
            "ss": stack_segment,
            "flags": flags_before,
        }
        phases = []
        calls = []

        def capture(machine: Uc, address: int, _size: int) -> None:
            if address in (0x76EA, 0x76F2, 0x76FB):
                phases.append(
                    (
                        address,
                        machine.reg_read(UC_X86_REG_AX),
                        bool(machine.reg_read(UC_X86_REG_EFLAGS) & 0x80),
                    )
                )
            if address not in (path_helper_entry, file_helper_entry):
                return
            calls.append(
                {
                    "kind": "path" if address == path_helper_entry else "file",
                    "ax": machine.reg_read(UC_X86_REG_AX),
                    "ds": machine.reg_read(UC_X86_REG_DS),
                    "si": machine.reg_read(UC_X86_REG_SI),
                    "es": machine.reg_read(UC_X86_REG_ES),
                    "di": machine.reg_read(UC_X86_REG_DI),
                    "sp": machine.reg_read(UC_X86_REG_SP),
                    "cs": machine.reg_read(UC_X86_REG_CS),
                }
            )

        machine = execute(
            run_entry,
            stop_address,
            initial,
            memory,
            code_handler=capture,
        )
        handler_entries = [phase for phase in phases if phase[0] == 0x76EA]
        if len(handler_entries) != 1 or handler_entries[0][2] != handler_sf:
            raise AssertionError(
                f"0x76ea {name}: handler entry={handler_entries}, "
                f"expected SF={handler_sf}"
            )
        cbw_phases = [phase for phase in phases if phase[0] == 0x76F2]
        if (
            len(cbw_phases) != 1
            or cbw_phases[0][1] != sign_extended_id
            or cbw_phases[0][2] != handler_sf
        ):
            raise AssertionError(
                f"0x76ea {name}: CBW phase={cbw_phases}, "
                f"expected AX={sign_extended_id:#x}, SF={handler_sf}"
            )
        store_phases = [phase for phase in phases if phase[0] == 0x76FB]
        if len(store_phases) != 1 or store_phases[0][1] != stored_id:
            raise AssertionError(
                f"0x76ea {name}: store phase={store_phases}, "
                f"expected AX={stored_id:#x}"
            )

        if expected_call is None:
            expected_calls = []
        elif expected_call == "path":
            expected_calls = [
                {
                    "kind": "path",
                    "ax": game_segment,
                    "ds": game_segment,
                    "si": 0x2137,
                    "es": destination_segment,
                    "di": 0x213A + len(copied),
                    "sp": expected_call_sp,
                    "cs": 0x01CE,
                }
            ]
        else:
            expected_calls = [
                {
                    "kind": "file",
                    "ax": game_segment,
                    "ds": game_segment,
                    "si": 0x2137,
                    "es": back_buffer_segment,
                    "di": back_buffer_offset,
                    "sp": expected_call_sp,
                    "cs": 0x01CE,
                }
            ]
        if calls != expected_calls:
            raise AssertionError(
                f"0x76ea {name}: calls={calls}, expected={expected_calls}"
            )

        actual_index = bytes(
            machine.mem_read(destination_segment * 16 + 0x1FD7, 2)
        )
        if actual_index != struct.pack("<H", stored_id):
            raise AssertionError(
                f"0x76ea {name}: index={actual_index.hex()}, "
                f"expected={stored_id:#x}"
            )
        actual_text = bytes(
            machine.mem_read(
                destination_segment * 16 + 0x213A,
                len(text_before),
            )
        )
        expected_text = copied + b"\x00" + text_before[len(copied) + 1 :]
        if actual_text != expected_text:
            raise AssertionError(
                f"0x76ea {name}: text={actual_text!r}, expected={expected_text!r}"
            )
        for source_offset, expected in immutable_source:
            actual = bytes(machine.mem_read(data_segment * 16 + source_offset, 1))
            if actual != expected:
                raise AssertionError(f"0x76ea {name}: source changed")
        immutable_memory = [
            (game_segment, 0x1FD7, b"\x5a\xa5"),
            (game_segment, 0x2137, game_path),
            (game_segment, 0x2793, struct.pack("<H", ui_state)),
            (game_segment, 0x0A58, struct.pack("<h", ems_handle)),
            (game_segment, 0x0A56, struct.pack("<h", xms_handle)),
            (game_segment, 0x5229, back_pointer),
            (data_segment, 0x1FD7, b"\x69\x96"),
            (data_segment, 0x2137, bytes([0xA5]) * len(game_path)),
            (data_segment, 0x2793, b"\x87\x78"),
            (data_segment, 0x0A58, b"\x34\x12"),
            (data_segment, 0x0A56, b"\x78\x56"),
            (data_segment, 0x5229, b"\x00\x11\x22\x33"),
            (back_buffer_segment, back_buffer_offset, b"\xde\xad\xbe\xef"),
        ]
        for segment, offset, expected in immutable_memory:
            actual = bytes(machine.mem_read(segment * 16 + offset, len(expected)))
            if actual != expected:
                raise AssertionError(
                    f"0x76ea {name}: immutable {segment:#x}:{offset:#x} changed"
                )

        if expected_call is None:
            expected_eax = (
                (initial_eax & 0xFFFF0000) | (stored_id & 0xFF00) | stop_byte
            )
        else:
            expected_eax = 0
        expected_registers = dict(initial)
        del expected_registers["flags"]
        expected_registers.update(
            {
                "eax": expected_eax,
                "esi": (initial["esi"] & 0xFFFF0000) | stop_offset,
                "sp": expected_sp,
            }
        )
        for register, expected in expected_registers.items():
            actual = machine.reg_read(REGISTERS[register])
            if actual != expected:
                raise AssertionError(
                    f"0x76ea {name}: {register}={actual:#x}, expected={expected:#x}"
                )
        if machine.reg_read(UC_X86_REG_CS) != 0:
            raise AssertionError(f"0x76ea {name}: far call did not restore CS")

        if expected_call is not None or (ui_state & 1) != 0:
            expected_flags = {
                "cf": False,
                "pf": expected_call is not None,
                "zf": expected_call is not None,
                "sf": False,
                "of": False,
            }
        else:
            expected_flags = {
                "cf": False,
                "pf": True,
                "zf": True,
                "sf": False,
                "of": False,
            }
        flags_after = machine.reg_read(UC_X86_REG_EFLAGS)
        masks = {"cf": 1, "pf": 4, "zf": 0x40, "sf": 0x80, "of": 0x800}
        actual_flags = {
            flag: bool(flags_after & mask) for flag, mask in masks.items()
        }
        if actual_flags != expected_flags:
            raise AssertionError(
                f"0x76ea {name}: flags={actual_flags}, expected={expected_flags}"
            )
        if direct:
            if bytes(machine.mem_read(stack_segment * 16 + 0xFF02, 4)) != stack_sentinel:
                raise AssertionError(f"0x76ea {name}: direct stack sentinel changed")
        else:
            if struct.unpack(
                "<H", machine.mem_read(stack_segment * 16 + 0xFEFE, 2)
            )[0] != dispatcher_return:
                raise AssertionError(f"0x76ea {name}: dispatcher return was not pushed")
            if bytes(machine.mem_read(stack_segment * 16 + 0xFF00, 4)) != stack_sentinel:
                raise AssertionError(f"0x76ea {name}: dispatcher stack sentinel changed")

        vectors.append(
            {
                "name": name,
                "entry_mode": "direct" if direct else "real_dispatcher",
                "asset_id": asset_id,
                "handler_entry_sf": handler_sf,
                "stored_id": stored_id,
                "copied_hex": copied.hex(),
                "stopping_byte": stop_byte,
                "ui_state": ui_state,
                "ems_handle": ems_handle,
                "xms_handle": xms_handle,
                "helper_called": expected_call,
                "final_source_offset": stop_offset,
                "defined_flags": expected_flags,
            }
        )
    return vectors


def byte_parser_copy_131a_entry_vectors() -> list[dict[str, object]]:
    entry = 0x7754
    data_segment = 0x4400
    destination_segment = 0x4800
    game_segment = 0x2C00
    stack_segment = 0x9000
    return_address = 0x6F00
    cases = [
        ("immediate_nul", 0x6000, b"\x00", 0x3000, 0x00, 0x0002),
        ("immediate_high", 0x6020, b"\xff", 0x3020, 0x01, 0x0AD7),
        ("lower_printable", 0x6040, b"\x20\x1f", 0x3040, 0x0F, 0x0803),
        ("upper_printable", 0x6060, b"\x7f\x80", 0x3060, 0x7F, 0x00D6),
        ("ordinary_text", 0x6080, b"ENTRY\x00", 0x3080, 0x80, 0x0812),
        ("source_wrap", 0xFFFF, b"A\x00", 0x30A0, 0xFE, 0x00C3),
        ("destination_wrap", 0x60C0, b"AB\x00", 0xFFFE, 0x10, 0x0896),
        ("cursor_and_count_wrap", 0x60E0, b"\x00", 0xFFF8, 0xFF, 0x0047),
    ]
    expected_hash = "4c74b7b0c5779a3a71a6883e2408d5bfec92d98a20c26bf2cee47b5ff54d0108"
    if hashlib.sha256(EXE[entry : entry + 34]).hexdigest() != expected_hash:
        raise AssertionError("0x7754: recovered 34-byte body changed")

    def wrapped_bytes(machine: Uc, segment: int, offset: int, length: int) -> bytes:
        return bytes(
            machine.mem_read(segment * 16 + ((offset + index) & 0xFFFF), 1)[0]
            for index in range(length)
        )

    vectors = []
    for name, start, payload, cursor, count, flags_before in cases:
        stop_index = next(
            index
            for index, byte in enumerate(payload)
            if byte < 0x20 or byte >= 0x80
        )
        copied = payload[:stop_index]
        stop_byte = payload[stop_index]
        final_source = (start + stop_index) & 0xFFFF
        final_destination = (cursor + len(copied)) & 0xFFFF
        final_cursor = (cursor + 0x10) & 0xFFFF
        final_count = (count + 1) & 0xFF
        destination_before = bytes([0xCC]) * (len(copied) + 3)
        stack_sentinel = bytes.fromhex("5aa59669")
        memory = [
            (game_segment, 0x131A, struct.pack("<H", cursor)),
            (game_segment, 0x131E, bytes([count])),
            (data_segment, 0x131A, b"\x5a\xa5"),
            (data_segment, 0x131E, b"\x69"),
            (destination_segment, 0x131A, b"\x87\x78"),
            (destination_segment, 0x131E, b"\x96"),
            (
                stack_segment,
                0xFF00,
                struct.pack("<H", return_address) + stack_sentinel,
            ),
            (0, return_address, b"\xcc"),
        ]
        for index, byte in enumerate(destination_before):
            memory.append(
                (destination_segment, (cursor + index) & 0xFFFF, bytes([byte]))
            )
        immutable_source = []
        for index, byte in enumerate(payload):
            source_offset = (start + index) & 0xFFFF
            encoded = bytes([byte])
            memory.append((data_segment, source_offset, encoded))
            immutable_source.append((source_offset, encoded))

        initial = {
            "eax": 0xA1A1BE55,
            "ebx": 0xB2B22345,
            "ecx": 0xC3C33456,
            "edx": 0xD4D44567,
            "esi": 0xE5E50000 | start,
            "edi": 0xF6F66789,
            "ebp": 0x9797789A,
            "sp": 0xFF00,
            "ds": data_segment,
            "es": destination_segment,
            "fs": 0x5000,
            "gs": game_segment,
            "ss": stack_segment,
            "flags": flags_before,
        }
        machine = execute(entry, return_address, initial, memory)

        actual_destination = wrapped_bytes(
            machine, destination_segment, cursor, len(destination_before)
        )
        expected_destination = (
            copied + b"\x00" + destination_before[len(copied) + 1 :]
        )
        if actual_destination != expected_destination:
            raise AssertionError(
                f"0x7754 {name}: destination={actual_destination!r}, "
                f"expected={expected_destination!r}"
            )
        actual_cursor = struct.unpack(
            "<H", machine.mem_read(game_segment * 16 + 0x131A, 2)
        )[0]
        actual_count = machine.mem_read(game_segment * 16 + 0x131E, 1)[0]
        if actual_cursor != final_cursor or actual_count != final_count:
            raise AssertionError(
                f"0x7754 {name}: globals={(actual_cursor, actual_count)}, "
                f"expected={(final_cursor, final_count)}"
            )
        for source_offset, expected in immutable_source:
            actual = bytes(machine.mem_read(data_segment * 16 + source_offset, 1))
            if actual != expected:
                raise AssertionError(f"0x7754 {name}: source changed")
        for segment, offset, expected in (
            (data_segment, 0x131A, b"\x5a\xa5"),
            (data_segment, 0x131E, b"\x69"),
            (destination_segment, 0x131A, b"\x87\x78"),
            (destination_segment, 0x131E, b"\x96"),
        ):
            actual = bytes(machine.mem_read(segment * 16 + offset, len(expected)))
            if actual != expected:
                raise AssertionError(f"0x7754 {name}: global decoy changed")

        expected_registers = dict(initial)
        del expected_registers["flags"]
        expected_registers.update(
            {
                "eax": (initial["eax"] & 0xFFFFFF00) | stop_byte,
                "esi": (initial["esi"] & 0xFFFF0000) | final_source,
                "edi": (initial["edi"] & 0xFFFF0000) | final_destination,
                "sp": 0xFF02,
            }
        )
        for register, expected in expected_registers.items():
            actual = machine.reg_read(REGISTERS[register])
            if actual != expected:
                raise AssertionError(
                    f"0x7754 {name}: {register}={actual:#x}, expected={expected:#x}"
                )

        expected_flags = {
            "cf": cursor > 0xFFEF,
            "pf": (final_count.bit_count() % 2) == 0,
            "af": (count & 0x0F) == 0x0F,
            "zf": final_count == 0,
            "sf": bool(final_count & 0x80),
            "of": count == 0x7F,
        }
        flags_after = machine.reg_read(UC_X86_REG_EFLAGS)
        masks = {"cf": 1, "pf": 4, "af": 0x10, "zf": 0x40, "sf": 0x80, "of": 0x800}
        actual_flags = {
            flag: bool(flags_after & mask) for flag, mask in masks.items()
        }
        if actual_flags != expected_flags:
            raise AssertionError(
                f"0x7754 {name}: flags={actual_flags}, expected={expected_flags}"
            )
        if bytes(machine.mem_read(stack_segment * 16 + 0xFF02, 4)) != stack_sentinel:
            raise AssertionError(f"0x7754 {name}: stack sentinel changed")

        vectors.append(
            {
                "name": name,
                "source_offset": start,
                "input_hex": payload.hex(),
                "copied_hex": copied.hex(),
                "stopping_byte": stop_byte,
                "destination_cursor_before": cursor,
                "destination_cursor_after": final_cursor,
                "entry_count_before": count,
                "entry_count_after": final_count,
                "final_source_offset": final_source,
                "final_destination_offset": final_destination,
                "defined_flags": expected_flags,
            }
        )
    return vectors


def byte_parser_stream_0f18_append_vectors() -> list[dict[str, object]]:
    entry = 0x7776
    data_segment = 0x4400
    destination_segment = 0x4800
    game_segment = 0x2C00
    stack_segment = 0x9000
    return_address = 0x6F00
    cases = [
        ("empty", 0x6200, 0x0000, b"\x00", 0x3200, 0x0002),
        ("ordinary", 0x6220, 0x1234, b"ABC\x00", 0x3220, 0x0AD7),
        ("high_bytes", 0x6240, 0xA55A, b"\x80\xff\x00", 0x3240, 0x0803),
        ("embedded_nul", 0x6260, 0xBEEF, b"A\x00Z", 0x3260, 0x00D6),
        ("unaligned", 0x6281, 0x0102, b"XY\x00", 0x3281, 0x0812),
        ("source_wrap", 0xFFFE, 0xCAFE, b"Q\x00", 0x32A0, 0x00C3),
        ("destination_wrap", 0x62C0, 0xBEEF, b"A\x00", 0xFFFE, 0x0896),
    ]
    expected_hash = "d93f743d34dbe42e419c9a1ca52aae856d5d941fc2b79b8bc2fe15c241b0bfdc"
    if hashlib.sha256(EXE[entry : entry + 18]).hexdigest() != expected_hash:
        raise AssertionError("0x7776: recovered 18-byte body changed")

    def wrapped_bytes(machine: Uc, segment: int, offset: int, length: int) -> bytes:
        return bytes(
            machine.mem_read(segment * 16 + ((offset + index) & 0xFFFF), 1)[0]
            for index in range(length)
        )

    vectors = []
    for name, start, leading_word, suffix, cursor, flags_before in cases:
        nul_index = suffix.index(0)
        copied_suffix = suffix[: nul_index + 1]
        stream = struct.pack("<H", leading_word) + suffix
        copied = struct.pack("<H", leading_word) + copied_suffix
        final_source = (start + len(copied)) & 0xFFFF
        final_destination = (cursor + len(copied)) & 0xFFFF
        destination_before = bytes([0xCC]) * (len(copied) + 2)
        stack_sentinel = bytes.fromhex("5aa59669")
        memory = [
            (game_segment, 0x0F18, struct.pack("<H", cursor)),
            (data_segment, 0x0F18, b"\x5a\xa5"),
            (destination_segment, 0x0F18, b"\x87\x78"),
            (
                stack_segment,
                0xFF00,
                struct.pack("<H", return_address) + stack_sentinel,
            ),
            (0, return_address, b"\xcc"),
        ]
        for index, byte in enumerate(destination_before):
            memory.append(
                (destination_segment, (cursor + index) & 0xFFFF, bytes([byte]))
            )
        immutable_source = []
        for index, byte in enumerate(stream):
            source_offset = (start + index) & 0xFFFF
            encoded = bytes([byte])
            memory.append((data_segment, source_offset, encoded))
            immutable_source.append((source_offset, encoded))

        initial = {
            "eax": 0xA1A1BE55,
            "ebx": 0xB2B22345,
            "ecx": 0xC3C33456,
            "edx": 0xD4D44567,
            "esi": 0xE5E50000 | start,
            "edi": 0xF6F66789,
            "ebp": 0x9797789A,
            "sp": 0xFF00,
            "ds": data_segment,
            "es": destination_segment,
            "fs": 0x5000,
            "gs": game_segment,
            "ss": stack_segment,
            "flags": flags_before,
        }
        machine = execute(entry, return_address, initial, memory)

        actual_destination = wrapped_bytes(
            machine, destination_segment, cursor, len(destination_before)
        )
        expected_destination = copied + destination_before[len(copied) :]
        if actual_destination != expected_destination:
            raise AssertionError(
                f"0x7776 {name}: destination={actual_destination.hex()}, "
                f"expected={expected_destination.hex()}"
            )
        actual_cursor = struct.unpack(
            "<H", machine.mem_read(game_segment * 16 + 0x0F18, 2)
        )[0]
        if actual_cursor != final_destination:
            raise AssertionError(
                f"0x7776 {name}: cursor={actual_cursor:#x}, "
                f"expected={final_destination:#x}"
            )
        for source_offset, expected in immutable_source:
            actual = bytes(machine.mem_read(data_segment * 16 + source_offset, 1))
            if actual != expected:
                raise AssertionError(f"0x7776 {name}: source changed")
        for segment, offset, expected in (
            (data_segment, 0x0F18, b"\x5a\xa5"),
            (destination_segment, 0x0F18, b"\x87\x78"),
        ):
            actual = bytes(machine.mem_read(segment * 16 + offset, len(expected)))
            if actual != expected:
                raise AssertionError(f"0x7776 {name}: cursor decoy changed")

        expected_registers = dict(initial)
        del expected_registers["flags"]
        expected_registers.update(
            {
                "eax": initial["eax"] & 0xFFFFFF00,
                "esi": (initial["esi"] & 0xFFFF0000) | final_source,
                "edi": (initial["edi"] & 0xFFFF0000) | final_destination,
                "sp": 0xFF02,
            }
        )
        for register, expected in expected_registers.items():
            actual = machine.reg_read(REGISTERS[register])
            if actual != expected:
                raise AssertionError(
                    f"0x7776 {name}: {register}={actual:#x}, expected={expected:#x}"
                )
        expected_flags = {
            "cf": False,
            "pf": True,
            "af": False,
            "zf": True,
            "sf": False,
            "of": False,
        }
        flags_after = machine.reg_read(UC_X86_REG_EFLAGS)
        masks = {"cf": 1, "pf": 4, "af": 0x10, "zf": 0x40, "sf": 0x80, "of": 0x800}
        actual_flags = {
            flag: bool(flags_after & mask) for flag, mask in masks.items()
        }
        if actual_flags != expected_flags:
            raise AssertionError(
                f"0x7776 {name}: flags={actual_flags}, expected={expected_flags}"
            )
        if bytes(machine.mem_read(stack_segment * 16 + 0xFF02, 4)) != stack_sentinel:
            raise AssertionError(f"0x7776 {name}: stack sentinel changed")

        vectors.append(
            {
                "name": name,
                "source_offset": start,
                "leading_word": leading_word,
                "suffix_hex": suffix.hex(),
                "copied_hex": copied.hex(),
                "destination_cursor_before": cursor,
                "destination_cursor_after": final_destination,
                "final_source_offset": final_source,
                "defined_flags": expected_flags,
            }
        )
    return vectors


def nav_choice_handler_vectors(entry: int) -> list[dict[str, object]]:
    if entry not in (0x8713, 0x8848):
        raise AssertionError(f"unsupported navigation choice handler {entry:#x}")
    has_loader = entry == 0x8848
    source_offset = 0x6756 if has_loader else 0x6754
    byte_count = 36 if has_loader else 25
    expected_hash = (
        "1042a534ceca566ad5030d96d5ed1b4173f4e95b8d6fdbef331e9ce0aee7cdc3"
        if has_loader
        else "0415e99bdfa96db2734d75f9db77377a62603f797a3526cd3650f7b9c96ce0df"
    )
    if hashlib.sha256(EXE[entry : entry + byte_count]).hexdigest() != expected_hash:
        raise AssertionError(f"{entry:#x}: recovered body changed")

    data_segment = 0x4400
    extra_segment = 0x4800
    game_segment = 0x2C00
    stack_segment = 0x9000
    return_address = 0x6F00
    loader_entry = 0xBA05  # Runtime 0B1B:0855.
    cases = [
        ("phase_zero", 0x00, 0x1111, 0x2222, 0x3333, 0x0002),
        ("phase_bit_one_clear", 0x02, 0x4444, 0x5555, 0x6666, 0x0AD7),
        ("phase_one", 0x01, 0x7777, 0x8888, 0x9999, 0x0803),
        ("phase_both_low_bits", 0x03, 0xABCD, 0x1357, 0x2468, 0x00D6),
        ("phase_all_bits", 0xFF, 0x0000, 0xFFFF, 0xA5A5, 0x0812),
        ("source_all_ones", 0x81, 0xFFFF, 0x0102, 0x0304, 0x00C3),
    ]
    vectors = []
    for name, phase, source_value, type_before, link_before, flags_before in cases:
        active = (phase & 1) != 0
        path = b"radio.snd\x00"
        stack_sentinel = bytes.fromhex("5aa59669")
        memory = [
            (0, return_address, b"\xcc"),
            (0, loader_entry, b"\xcb"),
            (data_segment, 0x2565, bytes([phase])),
            (data_segment, source_offset, struct.pack("<H", source_value)),
            (data_segment, 0x6768, struct.pack("<H", type_before)),
            (data_segment, 0x676A, struct.pack("<H", link_before)),
            (data_segment, 0x0D16, path),
            (game_segment, 0x2565, b"\x5a"),
            (game_segment, source_offset, b"\xa5\x5a"),
            (game_segment, 0x6768, b"\x69\x96"),
            (game_segment, 0x676A, b"\x87\x78"),
            (game_segment, 0x0D16, bytes([0xCC]) * len(path)),
            (extra_segment, 0x2565, b"\x3c"),
            (extra_segment, source_offset, b"\xc3\x3c"),
            (extra_segment, 0x6768, b"\xf0\x0f"),
            (extra_segment, 0x676A, b"\x55\xaa"),
            (extra_segment, 0x0D16, bytes([0xDD]) * len(path)),
            (
                stack_segment,
                0xFF00,
                struct.pack("<H", return_address) + stack_sentinel,
            ),
        ]
        initial = {
            "eax": 0xA1A1BEEF,
            "ebx": 0xB2B22345,
            "ecx": 0xC3C33456,
            "edx": 0xD4D44567,
            "esi": 0xE5E55678,
            "edi": 0xF6F66789,
            "ebp": 0x9797789A,
            "sp": 0xFF00,
            "ds": data_segment,
            "es": extra_segment,
            "fs": 0x5000,
            "gs": game_segment,
            "ss": stack_segment,
            "flags": flags_before,
        }
        calls = []
        phases = []

        def capture(machine: Uc, address: int, _size: int) -> None:
            if address == loader_entry:
                calls.append(
                    {
                        "ax": machine.reg_read(UC_X86_REG_AX),
                        "ds": machine.reg_read(UC_X86_REG_DS),
                        "si": machine.reg_read(UC_X86_REG_SI),
                        "sp": machine.reg_read(UC_X86_REG_SP),
                        "cs": machine.reg_read(UC_X86_REG_CS),
                        "phase": machine.mem_read(data_segment * 16 + 0x2565, 1)[0],
                        "type": struct.unpack(
                            "<H", machine.mem_read(data_segment * 16 + 0x6768, 2)
                        )[0],
                        "link": struct.unpack(
                            "<H", machine.mem_read(data_segment * 16 + 0x676A, 2)
                        )[0],
                    }
                )
            if not active:
                return
            phase_addresses = (
                (0x871A, 0x871D, 0x8720, 0x8726, 0x872B)
                if not has_loader
                else (0x884F, 0x8852, 0x8855, 0x885B, 0x8860)
            )
            if address in phase_addresses:
                phases.append(
                    (
                        address,
                        machine.mem_read(data_segment * 16 + 0x2565, 1)[0],
                        struct.unpack(
                            "<H", machine.mem_read(data_segment * 16 + 0x6768, 2)
                        )[0],
                        struct.unpack(
                            "<H", machine.mem_read(data_segment * 16 + 0x676A, 2)
                        )[0],
                    )
                )

        machine = execute(
            entry,
            return_address,
            initial,
            memory,
            code_handler=capture,
        )

        expected_phase_addresses = (
            (0x871A, 0x871D, 0x8720, 0x8726, 0x872B)
            if not has_loader
            else (0x884F, 0x8852, 0x8855, 0x885B, 0x8860)
        )
        if active:
            expected_phases = [
                (expected_phase_addresses[0], phase, type_before, link_before),
                (expected_phase_addresses[1], phase, type_before, link_before),
                (expected_phase_addresses[2], phase, type_before, source_value),
                (expected_phase_addresses[3], phase, 0x00C3, source_value),
                (expected_phase_addresses[4], 0, 0x00C3, source_value),
            ]
        else:
            expected_phases = []
        if phases != expected_phases:
            raise AssertionError(
                f"{entry:#x} {name}: phases={phases}, expected={expected_phases}"
            )

        expected_calls = []
        if has_loader and active:
            expected_calls.append(
                {
                    "ax": 1,
                    "ds": data_segment,
                    "si": 0x0D16,
                    "sp": 0xFEFC,
                    "cs": 0x0B1B,
                    "phase": 0,
                    "type": 0x00C3,
                    "link": source_value,
                }
            )
        if calls != expected_calls:
            raise AssertionError(
                f"{entry:#x} {name}: calls={calls}, expected={expected_calls}"
            )

        expected_phase = 0 if active else phase
        expected_type = 0x00C3 if active else type_before
        expected_link = source_value if active else link_before
        actual_phase = machine.mem_read(data_segment * 16 + 0x2565, 1)[0]
        actual_type = struct.unpack(
            "<H", machine.mem_read(data_segment * 16 + 0x6768, 2)
        )[0]
        actual_link = struct.unpack(
            "<H", machine.mem_read(data_segment * 16 + 0x676A, 2)
        )[0]
        if (actual_phase, actual_type, actual_link) != (
            expected_phase,
            expected_type,
            expected_link,
        ):
            raise AssertionError(
                f"{entry:#x} {name}: state={(actual_phase, actual_type, actual_link)}, "
                f"expected={(expected_phase, expected_type, expected_link)}"
            )
        for segment, offset, expected in (
            (data_segment, source_offset, struct.pack("<H", source_value)),
            (data_segment, 0x0D16, path),
            (game_segment, 0x2565, b"\x5a"),
            (game_segment, source_offset, b"\xa5\x5a"),
            (game_segment, 0x6768, b"\x69\x96"),
            (game_segment, 0x676A, b"\x87\x78"),
            (game_segment, 0x0D16, bytes([0xCC]) * len(path)),
            (extra_segment, 0x2565, b"\x3c"),
            (extra_segment, source_offset, b"\xc3\x3c"),
            (extra_segment, 0x6768, b"\xf0\x0f"),
            (extra_segment, 0x676A, b"\x55\xaa"),
            (extra_segment, 0x0D16, bytes([0xDD]) * len(path)),
        ):
            actual = bytes(machine.mem_read(segment * 16 + offset, len(expected)))
            if actual != expected:
                raise AssertionError(
                    f"{entry:#x} {name}: immutable {segment:#x}:{offset:#x} changed"
                )

        expected_registers = dict(initial)
        del expected_registers["flags"]
        expected_registers["sp"] = 0xFF02
        if active:
            expected_registers["eax"] = (
                (initial["eax"] & 0xFFFF0000) | (1 if has_loader else source_value)
            )
            if has_loader:
                expected_registers["esi"] = (
                    initial["esi"] & 0xFFFF0000
                ) | 0x0D16
        for register, expected in expected_registers.items():
            actual = machine.reg_read(REGISTERS[register])
            if actual != expected:
                raise AssertionError(
                    f"{entry:#x} {name}: {register}={actual:#x}, expected={expected:#x}"
                )
        if machine.reg_read(UC_X86_REG_CS) != 0:
            raise AssertionError(f"{entry:#x} {name}: far call did not restore CS")

        expected_flags = {
            "cf": False,
            "pf": not active,
            "zf": not active,
            "sf": False,
            "of": False,
        }
        flags_after = machine.reg_read(UC_X86_REG_EFLAGS)
        masks = {"cf": 1, "pf": 4, "zf": 0x40, "sf": 0x80, "of": 0x800}
        actual_flags = {
            flag: bool(flags_after & mask) for flag, mask in masks.items()
        }
        if actual_flags != expected_flags:
            raise AssertionError(
                f"{entry:#x} {name}: flags={actual_flags}, expected={expected_flags}"
            )
        if bytes(machine.mem_read(stack_segment * 16 + 0xFF02, 4)) != stack_sentinel:
            raise AssertionError(f"{entry:#x} {name}: stack sentinel changed")

        vectors.append(
            {
                "name": name,
                "phase_before": phase,
                "phase_bit_zero_set": active,
                "source_record": source_value,
                "deferred_type_before": type_before,
                "deferred_type_after": actual_type,
                "deferred_link_before": link_before,
                "deferred_link_after": actual_link,
                "phase_after": actual_phase,
                "loader_called": bool(calls),
                "loader_mode": 1 if calls else None,
                "loader_path_offset": 0x0D16 if calls else None,
                "defined_flags": expected_flags,
            }
        )
    return vectors


def back_buffer_copy_from_vectors() -> list[dict[str, object]]:
    entry = 0x933A
    expected_hash = "0f0d19e171bb60749bf5523468b18aa2a731b735356689d76ba4f520f8161fc3"
    if hashlib.sha256(EXE[entry : entry + 42]).hexdigest() != expected_hash:
        raise AssertionError("0x933a: recovered 42-byte body changed")

    game_segment = 0x2C00
    data_segment = 0x4000
    extra_segment = 0x4800
    source_segment = 0x5000
    destination_segment = 0x6000
    stack_segment = 0x9000
    return_address = 0x6F00
    cases = [
        ("zero_width", 0, 0, 0, 0, 0),
        ("single_origin", 0, 0, 1, 0, 0),
        ("partial_row", 37, 12, 17, 0, 0),
        ("full_row", 0, 110, 320, 0, 0),
        ("last_screen_row_tail", 311, 199, 9, 0, 0),
        ("offset_wrap", 251, 204, 12, 0, 0),
        ("max_byte_row", 5, 255, 7, 0, 0),
        ("high_row_machine_formula", 3, 0x0100, 5, 0, 0),
        ("offset_add_carry", 0x0200, 204, 3, 0, 0),
        ("offset_add_auxiliary_carry", 1, 0x0F00, 2, 0, 0),
        ("offset_add_signed_overflow", 0x0300, 100, 1, 0, 0),
        ("nonzero_pointer_offsets_ignored", 7, 4, 6, 0x1234, 0x4321),
    ]
    vectors = []

    for case_index, (
        name,
        x,
        y,
        width,
        source_pointer_offset,
        destination_pointer_offset,
    ) in enumerate(cases):
        source = bytes(
            (index * 37 + case_index * 29 + 11) & 0xFF
            for index in range(0x10000)
        )
        destination = bytes(
            (index * 13 + case_index * 17 + 7) & 0xFF
            for index in range(0x10000)
        )
        expected_destination = bytearray(destination)
        swapped_y = ((y & 0xFF) << 8) | (y >> 8)
        row_offset = (swapped_y + ((y << 6) & 0xFFFF)) & 0xFFFF
        offset = (row_offset + x) & 0xFFFF
        natural_offset = (y * 320 + x) & 0xFFFF
        for index in range(width):
            copy_offset = (offset + index) & 0xFFFF
            expected_destination[copy_offset] = source[copy_offset]

        source_pointer = struct.pack(
            "<HH", source_pointer_offset, source_segment
        )
        destination_pointer = struct.pack(
            "<HH", destination_pointer_offset, destination_segment
        )
        data_pointer_decoy = struct.pack("<HH", 0x1111, extra_segment)
        extra_pointer_decoy = struct.pack("<HH", 0x2222, data_segment)
        stack_sentinel = bytes.fromhex("5aa59669")
        flags_before = 0x0AD7
        initial = {
            "eax": 0xA1A1BEEF,
            "ebx": 0xB2B20000 | x,
            "ecx": 0xC3C30000 | y,
            "edx": 0xD4D40000 | width,
            "esi": 0xE5E55678,
            "edi": 0xF6F66789,
            "ebp": 0x9797789A,
            "sp": 0xFF00,
            "ds": data_segment,
            "es": extra_segment,
            "fs": 0x6800,
            "gs": game_segment,
            "ss": stack_segment,
            "flags": flags_before,
        }
        phases = []

        def capture(machine: Uc, address: int, _size: int) -> None:
            if address not in (0x9340, 0x9345, 0x934A, 0x935B, 0x935D):
                return
            if address == 0x935B and any(
                phase["address"] == address for phase in phases
            ):
                return
            phases.append(
                {
                    "address": address,
                    "ds": machine.reg_read(UC_X86_REG_DS),
                    "es": machine.reg_read(UC_X86_REG_ES),
                    "si": machine.reg_read(UC_X86_REG_SI),
                    "di": machine.reg_read(UC_X86_REG_DI),
                    "cx": machine.reg_read(UC_X86_REG_CX),
                }
            )

        machine = execute(
            entry,
            return_address,
            initial,
            [
                (0, return_address, b"\xcc"),
                (game_segment, 0x0ABC, source_pointer),
                (game_segment, 0x5229, destination_pointer),
                (data_segment, 0x0ABC, data_pointer_decoy),
                (data_segment, 0x5229, data_pointer_decoy),
                (extra_segment, 0x0ABC, extra_pointer_decoy),
                (extra_segment, 0x5229, extra_pointer_decoy),
                (source_segment, 0, source),
                (destination_segment, 0, destination),
                (
                    stack_segment,
                    0xFF00,
                    struct.pack("<H", return_address) + stack_sentinel,
                ),
            ],
            code_handler=capture,
            instruction_count=max(20000, width + 64),
        )

        expected_phases = [
            {
                "address": 0x9340,
                "ds": data_segment,
                "es": extra_segment,
                "si": initial["esi"] & 0xFFFF,
                "di": initial["edi"] & 0xFFFF,
                "cx": y,
            },
            {
                "address": 0x9345,
                "ds": data_segment,
                "es": destination_segment,
                "si": initial["esi"] & 0xFFFF,
                "di": destination_pointer_offset,
                "cx": y,
            },
            {
                "address": 0x934A,
                "ds": source_segment,
                "es": destination_segment,
                "si": source_pointer_offset,
                "di": destination_pointer_offset,
                "cx": y,
            },
            {
                "address": 0x935B,
                "ds": source_segment,
                "es": destination_segment,
                "si": offset,
                "di": offset,
                "cx": width,
            },
            {
                "address": 0x935D,
                "ds": source_segment,
                "es": destination_segment,
                "si": (offset + width) & 0xFFFF,
                "di": (offset + width) & 0xFFFF,
                "cx": 0,
            },
        ]
        if phases != expected_phases:
            raise AssertionError(
                f"0x933a {name}: phases={phases}, expected={expected_phases}"
            )

        actual_destination = bytes(
            machine.mem_read(destination_segment * 16, 0x10000)
        )
        if actual_destination != bytes(expected_destination):
            mismatch = next(
                index
                for index, (actual, expected) in enumerate(
                    zip(actual_destination, expected_destination)
                )
                if actual != expected
            )
            raise AssertionError(
                f"0x933a {name}: destination[{mismatch:#x}]="
                f"{actual_destination[mismatch]:#x}, "
                f"expected={expected_destination[mismatch]:#x}"
            )
        if bytes(machine.mem_read(source_segment * 16, 0x10000)) != source:
            raise AssertionError(f"0x933a {name}: source changed")
        for segment, pointer_offset, expected in (
            (game_segment, 0x0ABC, source_pointer),
            (game_segment, 0x5229, destination_pointer),
            (data_segment, 0x0ABC, data_pointer_decoy),
            (data_segment, 0x5229, data_pointer_decoy),
            (extra_segment, 0x0ABC, extra_pointer_decoy),
            (extra_segment, 0x5229, extra_pointer_decoy),
        ):
            actual = bytes(machine.mem_read(segment * 16 + pointer_offset, 4))
            if actual != expected:
                raise AssertionError(
                    f"0x933a {name}: pointer {segment:#x}:{pointer_offset:#x} changed"
                )

        expected_registers = dict(initial)
        del expected_registers["flags"]
        expected_registers["sp"] = 0xFF02
        for register, expected in expected_registers.items():
            actual = machine.reg_read(REGISTERS[register])
            if actual != expected:
                raise AssertionError(
                    f"0x933a {name}: {register}={actual:#x}, expected={expected:#x}"
                )
        if machine.reg_read(UC_X86_REG_CS) != 0:
            raise AssertionError(f"0x933a {name}: near return changed CS")

        full_sum = row_offset + x
        expected_flags = {
            "cf": full_sum > 0xFFFF,
            "pf": (offset & 0xFF).bit_count() % 2 == 0,
            "af": (row_offset & 0x0F) + (x & 0x0F) > 0x0F,
            "zf": offset == 0,
            "sf": bool(offset & 0x8000),
            "of": bool((~(row_offset ^ x) & (row_offset ^ offset)) & 0x8000),
        }
        flags_after = machine.reg_read(UC_X86_REG_EFLAGS)
        masks = {
            "cf": 0x0001,
            "pf": 0x0004,
            "af": 0x0010,
            "zf": 0x0040,
            "sf": 0x0080,
            "of": 0x0800,
        }
        actual_flags = {
            flag: bool(flags_after & mask) for flag, mask in masks.items()
        }
        if actual_flags != expected_flags:
            raise AssertionError(
                f"0x933a {name}: flags={actual_flags}, expected={expected_flags}"
            )
        if bytes(machine.mem_read(stack_segment * 16 + 0xFF02, 4)) != stack_sentinel:
            raise AssertionError(f"0x933a {name}: stack sentinel changed")

        vectors.append(
            {
                "name": name,
                "x": x,
                "y": y,
                "width": width,
                "source_pointer_offset": source_pointer_offset,
                "destination_pointer_offset": destination_pointer_offset,
                "machine_offset": offset,
                "natural_y_times_320_offset": natural_offset,
                "natural_offset_matches": offset == natural_offset,
                "copied_sha256": hashlib.sha256(
                    bytes(
                        source[(offset + index) & 0xFFFF]
                        for index in range(width)
                    )
                ).hexdigest(),
                "defined_flags": expected_flags,
            }
        )

    return vectors


def presentation_mode_bits_update_vectors() -> list[dict[str, object]]:
    entry = 0x9510
    expected_hash = "e392b2ee6954a3ddd813ed580b630c89d89ae7d30c5c2a6b645972a2e84425c4"
    if hashlib.sha256(EXE[entry : entry + 58]).hexdigest() != expected_hash:
        raise AssertionError("0x9510: recovered 58-byte body changed")

    data_segment = 0x4000
    extra_segment = 0x4800
    game_segment = 0x2C00
    stack_segment = 0x9000
    return_address = 0x6F00
    cases = [
        ("gate_set_clears_modes", 0xA5F2, -0x8000),
        ("gate_set_preserves_other_bits", 0x5A0E, 0x1234),
        ("signed_minimum", 0x80F1, -0x8000),
        ("negative_one", 0x0109, -1),
        ("zero", 0x0000, 0),
        ("below_lower_boundary", 0x4401, 21),
        ("lower_boundary", 0x2204, 22),
        ("first_band_start", 0x3308, 23),
        ("first_band_end", 0x1201, 67),
        ("second_band_start", 0x2404, 68),
        ("second_band_end", 0x3608, 112),
        ("third_band_start", 0x4801, 113),
        ("third_band_end", 0x5A04, 157),
        ("above_upper_boundary", 0x6C08, 158),
        ("signed_maximum", 0x7E01, 0x7FFF),
    ]
    vectors = []

    for name, state_before, frame in cases:
        masked_state = state_before & 0xFF0F
        gate_set = (masked_state & 2) != 0
        if gate_set or frame <= 22 or frame > 157:
            mode = 0 if gate_set else 0x10
        elif frame <= 67:
            mode = 0x20
        elif frame <= 112:
            mode = 0x40
        else:
            mode = 0x80
        expected_state = masked_state | mode
        frame_word = frame & 0xFFFF
        stack_sentinel = bytes.fromhex("5aa59669")
        initial = {
            "eax": 0xA1A1BEEF,
            "ebx": 0xB2B22345,
            "ecx": 0xC3C33456,
            "edx": 0xD4D44567,
            "esi": 0xE5E55678,
            "edi": 0xF6F66789,
            "ebp": 0x9797789A,
            "sp": 0xFF00,
            "ds": data_segment,
            "es": extra_segment,
            "fs": 0x5000,
            "gs": game_segment,
            "ss": stack_segment,
            "flags": 0x0ED7,
        }
        phases = []

        def capture(machine: Uc, address: int, _size: int) -> None:
            if address not in (0x951D, 0x953F, 0x9544):
                return
            phases.append(
                {
                    "address": address,
                    "ax": machine.reg_read(UC_X86_REG_AX),
                    "bx": machine.reg_read(UC_X86_REG_BX),
                    "dx": machine.reg_read(UC_X86_REG_DX),
                }
            )

        machine = execute(
            entry,
            return_address,
            initial,
            [
                (0, return_address, b"\xcc"),
                (data_segment, 0x2793, struct.pack("<H", state_before)),
                (data_segment, 0x2795, struct.pack("<H", frame_word)),
                (game_segment, 0x2793, b"\x5a\xa5"),
                (game_segment, 0x2795, b"\x69\x96"),
                (extra_segment, 0x2793, b"\x87\x78"),
                (extra_segment, 0x2795, b"\xc3\x3c"),
                (
                    stack_segment,
                    0xFF00,
                    struct.pack("<H", return_address) + stack_sentinel,
                ),
            ],
            code_handler=capture,
        )

        if gate_set:
            expected_phases = [
                {
                    "address": 0x9544,
                    "ax": masked_state,
                    "bx": initial["ebx"] & 0xFFFF,
                    "dx": initial["edx"] & 0xFFFF,
                }
            ]
        else:
            expected_phases = [
                {
                    "address": 0x951D,
                    "ax": masked_state,
                    "bx": initial["ebx"] & 0xFFFF,
                    "dx": initial["edx"] & 0xFFFF,
                },
                {
                    "address": 0x953F,
                    "ax": masked_state,
                    "bx": mode >> 4,
                    "dx": frame_word,
                },
                {
                    "address": 0x9544,
                    "ax": expected_state,
                    "bx": mode,
                    "dx": frame_word,
                },
            ]
        if phases != expected_phases:
            raise AssertionError(
                f"0x9510 {name}: phases={phases}, expected={expected_phases}"
            )

        actual_state = struct.unpack(
            "<H", machine.mem_read(data_segment * 16 + 0x2793, 2)
        )[0]
        actual_frame = struct.unpack(
            "<H", machine.mem_read(data_segment * 16 + 0x2795, 2)
        )[0]
        if actual_state != expected_state or actual_frame != frame_word:
            raise AssertionError(
                f"0x9510 {name}: state={actual_state:#x}, frame={actual_frame:#x}, "
                f"expected={expected_state:#x}/{frame_word:#x}"
            )
        for segment, offset, expected in (
            (game_segment, 0x2793, b"\x5a\xa5"),
            (game_segment, 0x2795, b"\x69\x96"),
            (extra_segment, 0x2793, b"\x87\x78"),
            (extra_segment, 0x2795, b"\xc3\x3c"),
        ):
            actual = bytes(machine.mem_read(segment * 16 + offset, len(expected)))
            if actual != expected:
                raise AssertionError(
                    f"0x9510 {name}: decoy {segment:#x}:{offset:#x} changed"
                )

        expected_registers = dict(initial)
        del expected_registers["flags"]
        expected_registers["eax"] = (
            initial["eax"] & 0xFFFF0000
        ) | expected_state
        expected_registers["sp"] = 0xFF02
        for register, expected in expected_registers.items():
            actual = machine.reg_read(REGISTERS[register])
            if actual != expected:
                raise AssertionError(
                    f"0x9510 {name}: {register}={actual:#x}, expected={expected:#x}"
                )
        if machine.reg_read(UC_X86_REG_CS) != 0:
            raise AssertionError(f"0x9510 {name}: near return changed CS")

        flag_result = 2 if gate_set else expected_state
        expected_flags = {
            "cf": False,
            "pf": (flag_result & 0xFF).bit_count() % 2 == 0,
            "zf": flag_result == 0,
            "sf": bool(flag_result & 0x8000),
            "of": False,
        }
        flags_after = machine.reg_read(UC_X86_REG_EFLAGS)
        masks = {"cf": 1, "pf": 4, "zf": 0x40, "sf": 0x80, "of": 0x800}
        actual_flags = {
            flag: bool(flags_after & mask) for flag, mask in masks.items()
        }
        if actual_flags != expected_flags:
            raise AssertionError(
                f"0x9510 {name}: flags={actual_flags}, expected={expected_flags}"
            )
        if bytes(machine.mem_read(stack_segment * 16 + 0xFF02, 4)) != stack_sentinel:
            raise AssertionError(f"0x9510 {name}: stack sentinel changed")

        vectors.append(
            {
                "name": name,
                "state_before": state_before,
                "signed_frame": frame,
                "bit_one_gate_set": gate_set,
                "selected_mode": mode,
                "state_after": actual_state,
                "ax_result": machine.reg_read(UC_X86_REG_AX),
                "defined_flags": expected_flags,
            }
        )

    return vectors


def matrix_table_clear_2a1b_vectors() -> list[dict[str, object]]:
    entry = 0x963F
    expected_hash = "60225baa9b9f1b75e86b7849f4a7b8b9dff1baf628d87ec419d1ed2e67568a32"
    if hashlib.sha256(EXE[entry : entry + 23]).hexdigest() != expected_hash:
        raise AssertionError("0x963f: recovered 23-byte body changed")

    data_segment = 0x4000
    extra_segment = 0x4800
    game_segment = 0x2C00
    stack_segment = 0x6800
    return_address = 0x6F00
    cases = [
        ("varied_words", [0x1234, 0xABCD, 0x0001, 0x8000, 0x00FF, 0xFF00], 0x11),
        ("already_zero", [0x0000] * 6, 0x27),
        ("all_ffff", [0xFFFF] * 6, 0x3D),
        ("alternating", [0xAAAA, 0x5555, 0xAAAA, 0x5555, 0xAAAA, 0x5555], 0x53),
        ("ascending", [0x0000, 0x1111, 0x2222, 0x3333, 0x4444, 0x5555], 0x69),
    ]
    vectors = []

    for name, first_words, seed in cases:
        table_before = bytearray(6 * 24)
        for record_index, first_word in enumerate(first_words):
            record_offset = record_index * 24
            struct.pack_into("<H", table_before, record_offset, first_word)
            for tail_offset in range(2, 24):
                table_before[record_offset + tail_offset] = (
                    seed + record_index * 0x25 + tail_offset * 0x0B
                ) & 0xFF

        expected_table = bytearray(table_before)
        for record_index in range(6):
            struct.pack_into("<H", expected_table, record_index * 24, 0)

        data_decoy = bytes((seed + index * 3) & 0xFF for index in range(6 * 24))
        extra_decoy = bytes((seed + 0x40 + index * 5) & 0xFF for index in range(6 * 24))
        game_decoy = bytes((seed + 0x80 + index * 7) & 0xFF for index in range(6 * 24))
        stack_sentinel = bytes.fromhex("5aa59669c33c")
        initial = {
            "eax": 0xA1A11234,
            "ebx": 0xB2B22345,
            "ecx": 0xC3C33456,
            "edx": 0xD4D44567,
            "esi": 0xE5E55678,
            "edi": 0xF6F66789,
            "ebp": 0x9797789A,
            "sp": 0xFF00,
            "ds": data_segment,
            "es": extra_segment,
            "fs": 0x5000,
            "gs": game_segment,
            "ss": stack_segment,
            "flags": 0x0ED7,
        }
        phases = []

        def capture(machine: Uc, address: int, _size: int) -> None:
            if address not in (0x9648, 0x9652):
                return
            phases.append(
                {
                    "address": address,
                    "bp": machine.reg_read(UC_X86_REG_BP),
                    "cx": machine.reg_read(UC_X86_REG_CX),
                }
            )

        machine = execute(
            entry,
            return_address,
            initial,
            [
                (0, return_address, b"\xcc"),
                (data_segment, 0x2A1B, data_decoy),
                (extra_segment, 0x2A1B, extra_decoy),
                (game_segment, 0x2A1B, game_decoy),
                (stack_segment, 0x2A1B, bytes(table_before)),
                (
                    stack_segment,
                    0xFF00,
                    struct.pack("<HH", return_address, 0) + stack_sentinel,
                ),
            ],
            code_handler=capture,
        )

        expected_phases = [
            {
                "address": 0x9648,
                "bp": 0x2A1B + record_index * 0x18,
                "cx": 6 - record_index,
            }
            for record_index in range(6)
        ]
        expected_phases.append({"address": 0x9652, "bp": 0x2AAB, "cx": 0})
        if phases != expected_phases:
            raise AssertionError(
                f"0x963f {name}: phases={phases}, expected={expected_phases}"
            )

        actual_table = bytes(
            machine.mem_read(stack_segment * 16 + 0x2A1B, len(expected_table))
        )
        if actual_table != expected_table:
            raise AssertionError(f"0x963f {name}: SS matrix table mismatch")
        for segment, expected in (
            (data_segment, data_decoy),
            (extra_segment, extra_decoy),
            (game_segment, game_decoy),
        ):
            actual = bytes(machine.mem_read(segment * 16 + 0x2A1B, len(expected)))
            if actual != expected:
                raise AssertionError(
                    f"0x963f {name}: decoy table in {segment:#x} changed"
                )

        expected_registers = dict(initial)
        del expected_registers["flags"]
        expected_registers["sp"] = 0xFF04
        for register, expected in expected_registers.items():
            actual = machine.reg_read(REGISTERS[register])
            if actual != expected:
                raise AssertionError(
                    f"0x963f {name}: {register}={actual:#x}, expected={expected:#x}"
                )
        if machine.reg_read(UC_X86_REG_CS) != 0:
            raise AssertionError(f"0x963f {name}: far return did not restore CS")

        expected_flags = {
            "cf": False,
            "pf": False,
            "af": False,
            "zf": False,
            "sf": False,
            "of": False,
        }
        flags_after = machine.reg_read(UC_X86_REG_EFLAGS)
        masks = {"cf": 1, "pf": 4, "af": 0x10, "zf": 0x40, "sf": 0x80, "of": 0x800}
        actual_flags = {
            flag: bool(flags_after & mask) for flag, mask in masks.items()
        }
        if actual_flags != expected_flags:
            raise AssertionError(
                f"0x963f {name}: flags={actual_flags}, expected={expected_flags}"
            )
        if bytes(machine.mem_read(stack_segment * 16 + 0xFF04, 6)) != stack_sentinel:
            raise AssertionError(f"0x963f {name}: stack sentinel changed")

        vectors.append(
            {
                "name": name,
                "first_words_before": first_words,
                "table_before_sha256": hashlib.sha256(table_before).hexdigest(),
                "table_after_sha256": hashlib.sha256(actual_table).hexdigest(),
                "store_offsets": [phase["bp"] for phase in phases[:-1]],
                "final_loop_bp": phases[-1]["bp"],
                "return_sp": machine.reg_read(UC_X86_REG_SP),
                "defined_flags": expected_flags,
            }
        )

    return vectors


def ship_3d_projection_matrix_build_vectors() -> list[dict[str, object]]:
    entry = 0x98B9
    expected_hash = "ee9cefae7bb3c3bcc0acfa72dd6f6f3731e166b91b2e15c3d3e62eee82653bb5"
    if hashlib.sha256(EXE[entry : entry + 343]).hexdigest() != expected_hash:
        raise AssertionError("0x98b9: recovered 343-byte body changed")

    def signed32(value: int) -> int:
        value &= 0xFFFFFFFF
        return value - 0x100000000 if value & 0x80000000 else value

    def multiply32(lhs: int, rhs: int) -> int:
        return signed32(lhs * rhs)

    def add32(lhs: int, rhs: int) -> int:
        return signed32(lhs + rhs)

    def subtract32(lhs: int, rhs: int) -> int:
        return signed32(lhs - rhs)

    def negate32(value: int) -> int:
        return signed32(-value)

    def shift15(value: int) -> int:
        return signed32(value) >> 15

    cases = [
        ("zero", 0, 1, 2, (0, 0), (0, 0), (0, 0)),
        ("identity", 3, 4, 5, (16384, 0), (16384, 0), (16384, 0)),
        ("quarter_turn_mix", 6, 7, 8, (0, 16384), (16384, 0), (0, -16384)),
        (
            "mixed_signs",
            17,
            63,
            121,
            (12345, -23456),
            (-16384, 8192),
            (24576, -12288),
        ),
        (
            "signed_extremes",
            31,
            95,
            159,
            (32767, -32768),
            (-32768, 32767),
            (32767, 32767),
        ),
        (
            "negative_extremes",
            32,
            96,
            160,
            (-32768, -32768),
            (-32768, -1),
            (-1, -32768),
        ),
        ("small_values", 10, 11, 12, (1, -1), (2, -3), (4, -5)),
        (
            "overflow_products",
            40,
            80,
            120,
            (30000, 30000),
            (28000, -29000),
            (-31000, 32000),
        ),
        (
            "angle_boundaries",
            180,
            179,
            178,
            (16384, 0),
            (-16384, 0),
            (0, 16384),
        ),
        ("repeated_angle", 90, 90, 90, (11585, 11585), (11585, 11585), (11585, 11585)),
        (
            "asymmetric_large",
            45,
            135,
            179,
            (-30001, 12345),
            (22222, -11111),
            (-23456, 31415),
        ),
        (
            "final_negative",
            71,
            143,
            29,
            (8192, -24576),
            (24576, 16384),
            (-24576, 8192),
        ),
    ]
    data_segment = 0x4000
    extra_segment = 0x4800
    game_segment = 0x2C00
    stack_segment = 0x6800
    return_address = 0x6F00
    store_addresses = [
        0x9951,
        0x9963,
        0x9986,
        0x99A9,
        0x99BB,
        0x99DE,
        0x99ED,
        0x99F3,
        0x9A01,
    ]
    vectors = []

    for name, angle_a, angle_b, angle_c, pair_a, pair_b, pair_c in cases:
        table = bytearray(181 * 4)
        for index in range(181):
            cosine = ((index * 193 + 17) & 0xFFFF) - 0x8000
            sine = ((index * 389 + 91) & 0xFFFF) - 0x8000
            struct.pack_into("<hh", table, index * 4, cosine, sine)
        for angle, pair in (
            (angle_a, pair_a),
            (angle_b, pair_b),
            (angle_c, pair_c),
        ):
            struct.pack_into("<hh", table, angle * 4, *pair)

        a_cos = signed32(pair_a[0] * 2)
        a_sin = signed32(pair_a[1] * 2)
        b_cos = signed32(pair_b[0] * 2)
        b_sin = signed32(pair_b[1] * 2)
        c_cos = signed32(pair_c[0] * 2)
        c_sin = signed32(pair_c[1] * 2)
        b_sin_c_sin = shift15(multiply32(b_sin, c_sin))
        c_sin_b_cos = shift15(multiply32(c_sin, b_cos))
        matrix = [
            shift15(
                add32(
                    multiply32(a_cos, b_cos),
                    multiply32(b_sin_c_sin, a_sin),
                )
            ),
            shift15(negate32(multiply32(c_cos, a_sin))),
            shift15(
                subtract32(
                    multiply32(c_sin_b_cos, a_sin),
                    multiply32(a_cos, b_sin),
                )
            ),
            shift15(
                subtract32(
                    multiply32(b_sin_c_sin, a_cos),
                    multiply32(a_sin, b_cos),
                )
            ),
            negate32(shift15(multiply32(c_cos, a_cos))),
            shift15(
                add32(
                    multiply32(b_sin, a_sin),
                    multiply32(c_sin_b_cos, a_cos),
                )
            ),
            shift15(multiply32(b_sin, c_cos)),
            c_sin,
            shift15(multiply32(c_cos, b_cos)),
        ]
        terms = [b_cos, b_sin, c_cos, c_sin, a_cos, a_sin]
        expected_work = struct.pack("<6i9i", *(terms + matrix))
        tail = bytes((0xA0 + index * 7) & 0xFF for index in range(6))
        work_before = bytes((0x31 + index * 13) & 0xFF for index in range(60)) + tail
        angle_words = struct.pack("<HHH", angle_b, angle_c, angle_a)
        decoy_table = bytes((0xD3 + index * 11) & 0xFF for index in range(len(table)))
        game_table_decoy = bytes(
            (0x4B + index * 17) & 0xFF for index in range(len(table))
        )
        decoy_angles = bytes.fromhex("5aa596698778")
        decoy_work = bytes((0x6D + index * 5) & 0xFF for index in range(len(work_before)))
        stack_sentinel = bytes.fromhex("c33c5aa59669")
        initial = {
            "eax": 0xA1A11234,
            "ebx": 0xB2B22345,
            "ecx": 0xC3C33456,
            "edx": 0xD4D44567,
            "esi": 0xE5E55678,
            "edi": 0xF6F66789,
            "ebp": 0x9797789A,
            "sp": 0xFF00,
            "ds": data_segment,
            "es": extra_segment,
            "fs": 0x5000,
            "gs": game_segment,
            "ss": stack_segment,
            "flags": 0x0AD7,
        }
        phases = []

        def capture(machine: Uc, address: int, _size: int) -> None:
            if address == 0x98CB:
                phases.append(
                    {
                        "address": address,
                        "ds": machine.reg_read(UC_X86_REG_DS),
                        "es": machine.reg_read(UC_X86_REG_ES),
                    }
                )
            elif address == 0x992A:
                phases.append(
                    {
                        "address": address,
                        "terms": list(
                            struct.unpack(
                                "<6i",
                                machine.mem_read(game_segment * 16 + 0x2F7D, 24),
                            )
                        ),
                    }
                )
            elif address in store_addresses:
                phases.append(
                    {
                        "address": address,
                        "di": machine.reg_read(UC_X86_REG_DI),
                        "eax": signed32(machine.reg_read(UC_X86_REG_EAX)),
                        "es": machine.reg_read(UC_X86_REG_ES),
                    }
                )
            elif address == 0x9A03:
                phases.append(
                    {
                        "address": address,
                        "di": machine.reg_read(UC_X86_REG_DI),
                    }
                )

        machine = execute(
            entry,
            return_address,
            initial,
            [
                (0, return_address, b"\xcc"),
                (game_segment, 0x2F6D, angle_words),
                (game_segment, 0x2F7D, work_before),
                (game_segment, 0x4F45, game_table_decoy),
                (data_segment, 0x2F6D, decoy_angles),
                (data_segment, 0x2F7D, decoy_work),
                (data_segment, 0x4F45, decoy_table),
                (extra_segment, 0x2F6D, decoy_angles[::-1]),
                (extra_segment, 0x2F7D, decoy_work[::-1]),
                (extra_segment, 0x4F45, decoy_table[::-1]),
                (stack_segment, 0x4F45, bytes(table)),
                (
                    stack_segment,
                    0xFF00,
                    struct.pack("<HH", return_address, 0) + stack_sentinel,
                ),
            ],
            code_handler=capture,
        )

        expected_phases = [
            {"address": 0x98CB, "ds": game_segment, "es": game_segment},
            {"address": 0x992A, "terms": terms},
        ]
        expected_phases.extend(
            {
                "address": address,
                "di": 0x2F95 + index * 4,
                "eax": matrix[index],
                "es": game_segment,
            }
            for index, address in enumerate(store_addresses)
        )
        expected_phases.append({"address": 0x9A03, "di": 0x2FB9})
        if phases != expected_phases:
            raise AssertionError(
                f"0x98b9 {name}: phases={phases}, expected={expected_phases}"
            )

        actual_work = bytes(machine.mem_read(game_segment * 16 + 0x2F7D, 66))
        if actual_work != expected_work + tail:
            raise AssertionError(f"0x98b9 {name}: projection workspace mismatch")
        actual_angles = bytes(machine.mem_read(game_segment * 16 + 0x2F6D, 6))
        actual_table = bytes(machine.mem_read(stack_segment * 16 + 0x4F45, len(table)))
        if actual_angles != angle_words or actual_table != table:
            raise AssertionError(f"0x98b9 {name}: source data changed")
        if bytes(
            machine.mem_read(game_segment * 16 + 0x4F45, len(game_table_decoy))
        ) != game_table_decoy:
            raise AssertionError(f"0x98b9 {name}: GS table decoy changed")
        for segment, expected_angles, expected_segment_work, expected_table in (
            (data_segment, decoy_angles, decoy_work, decoy_table),
            (extra_segment, decoy_angles[::-1], decoy_work[::-1], decoy_table[::-1]),
        ):
            if bytes(machine.mem_read(segment * 16 + 0x2F6D, 6)) != expected_angles:
                raise AssertionError(f"0x98b9 {name}: angle decoy changed")
            if bytes(machine.mem_read(segment * 16 + 0x2F7D, 66)) != expected_segment_work:
                raise AssertionError(f"0x98b9 {name}: work decoy changed")
            if bytes(machine.mem_read(segment * 16 + 0x4F45, len(table))) != expected_table:
                raise AssertionError(f"0x98b9 {name}: table decoy changed")

        expected_registers = dict(initial)
        del expected_registers["flags"]
        expected_registers["sp"] = 0xFF04
        for register, expected in expected_registers.items():
            actual = machine.reg_read(REGISTERS[register])
            if actual != expected:
                raise AssertionError(
                    f"0x98b9 {name}: {register}={actual:#x}, expected={expected:#x}"
                )
        if machine.reg_read(UC_X86_REG_CS) != 0:
            raise AssertionError(f"0x98b9 {name}: far return did not restore CS")

        final_product = multiply32(c_cos, b_cos)
        expected_flags = {
            "cf": bool((final_product & 0xFFFFFFFF) & (1 << 14)),
            "pf": (matrix[8] & 0xFF).bit_count() % 2 == 0,
            "zf": matrix[8] == 0,
            "sf": matrix[8] < 0,
        }
        flags_after = machine.reg_read(UC_X86_REG_EFLAGS)
        masks = {"cf": 1, "pf": 4, "zf": 0x40, "sf": 0x80}
        actual_flags = {
            flag: bool(flags_after & mask) for flag, mask in masks.items()
        }
        if actual_flags != expected_flags:
            raise AssertionError(
                f"0x98b9 {name}: flags={actual_flags}, expected={expected_flags}"
            )
        if bytes(machine.mem_read(stack_segment * 16 + 0xFF04, 6)) != stack_sentinel:
            raise AssertionError(f"0x98b9 {name}: stack sentinel changed")

        vectors.append(
            {
                "name": name,
                "angles_a_b_c": [angle_a, angle_b, angle_c],
                "table_pairs_a_b_c": [list(pair_a), list(pair_b), list(pair_c)],
                "doubled_terms_b_c_a": terms,
                "matrix": matrix,
                "workspace_sha256": hashlib.sha256(actual_work).hexdigest(),
                "defined_flags": expected_flags,
            }
        )

    return vectors


def ship_3d_plot_point_vectors() -> list[dict[str, object]]:
    entry = 0x9B04
    expected_hash = "ac19f28f8de11959599f3709ac9a949cf4c83428d206d71a312b3cba58fd68a2"
    if hashlib.sha256(EXE[entry : entry + 68]).hexdigest() != expected_hash:
        raise AssertionError("0x9b04: recovered 68-byte body changed")

    cases = [
        ("x_below_left", -1, 50, 0x0000, (0, 320, 0, 200), 0, "x_low"),
        ("x_at_left", 0, 50, 0x0000, (0, 320, 0, 200), 0, "draw"),
        ("x_at_right_minus_one", 319, 199, 0xFFFF, (0, 320, 0, 200), 0, "draw"),
        ("x_at_right", 320, 50, 0x1000, (0, 320, 0, 200), 0, "x_high"),
        ("y_below_top", 100, 34, 0x2000, (0, 320, 35, 165), 0, "y_low"),
        ("y_at_top", 100, 35, 0x3000, (0, 320, 35, 165), 0, "draw"),
        ("y_at_bottom_minus_one", 200, 164, 0x7000, (0, 320, 35, 165), 0, "draw"),
        ("y_at_bottom", 200, 165, 0x8000, (0, 320, 35, 165), 0, "y_high"),
        ("occupied_pixel", 160, 100, 0x9000, (0, 320, 0, 200), 0x5A, "occupied"),
        ("depth_one", 17, 23, 0x1000, (0, 320, 0, 200), 0, "draw"),
        ("depth_fifteen", 18, 24, 0xFABC, (0, 320, 0, 200), 0, "draw"),
        ("negative_x_wrap", -1, 0, 0x4000, (-2, 2, 0, 1), 0, "draw"),
        ("high_row_formula", 0, 256, 0x5000, (0, 1, 0, 300), 0, "draw"),
        ("negative_row_formula", 0, -1, 0x6000, (0, 1, -2, 1), 0, "draw"),
    ]
    data_segment = 0x4000
    game_segment = 0x2C00
    stack_segment = 0x6800
    framebuffer_segment = 0xA000
    return_address = 0x6F00
    vectors = []

    def compare_flags(lhs: int, rhs: int) -> dict[str, bool]:
        lhs &= 0xFFFF
        rhs &= 0xFFFF
        result = (lhs - rhs) & 0xFFFF
        return {
            "cf": lhs < rhs,
            "pf": (result & 0xFF).bit_count() % 2 == 0,
            "af": bool((lhs ^ rhs ^ result) & 0x10),
            "zf": result == 0,
            "sf": bool(result & 0x8000),
            "of": bool(((lhs ^ rhs) & (lhs ^ result)) & 0x8000),
        }

    for name, x, y, depth, clip, pixel_before, outcome in cases:
        left, right, top, bottom = clip
        x_word = x & 0xFFFF
        y_word = y & 0xFFFF
        swapped_y = ((y_word & 0xFF) << 8) | (y_word >> 8)
        machine_offset = (((y_word << 6) & 0xFFFF) + swapped_y + x_word) & 0xFFFF
        natural_offset = (y_word * 320 + x_word) & 0xFFFF
        framebuffer_before = bytearray(
            (0x31 + index * 29) & 0xFF for index in range(0x10000)
        )
        framebuffer_before[machine_offset] = pixel_before
        framebuffer_expected = bytearray(framebuffer_before)
        shade = (0xEF - (depth >> 12)) & 0xFF
        if outcome == "draw":
            framebuffer_expected[machine_offset] = shade

        context = bytes((0xA5 + index * 7) & 0xFF for index in range(36))
        context += struct.pack("<HHH", x_word, y_word, depth)
        context_decoy = bytes((0x59 + index * 11) & 0xFF for index in range(42))
        clip_words = struct.pack(
            "<HHHH", left & 0xFFFF, right & 0xFFFF, top & 0xFFFF, bottom & 0xFFFF
        )
        clip_decoy = bytes.fromhex("5aa596698778c33c")
        stack_sentinel = bytes.fromhex("69965aa5c33c")
        initial = {
            "eax": 0xA1A11234,
            "ebx": 0xB2B22345,
            "ecx": 0xC3C33456,
            "edx": 0xD4D44567,
            "esi": 0xE5E55678,
            "edi": 0xF6F66789,
            "ebp": 0x97972F95,
            "sp": 0xFF00,
            "ds": data_segment,
            "es": framebuffer_segment,
            "fs": 0x5000,
            "gs": game_segment,
            "ss": stack_segment,
            "flags": 0x0AD7,
        }
        phases = []

        def capture(machine: Uc, address: int, _size: int) -> None:
            if address == 0x9B30:
                phases.append(
                    {
                        "address": address,
                        "di": machine.reg_read(UC_X86_REG_DI),
                        "es": machine.reg_read(UC_X86_REG_ES),
                    }
                )
            elif address == 0x9B41:
                phases.append(
                    {
                        "address": address,
                        "di": machine.reg_read(UC_X86_REG_DI),
                        "al": machine.reg_read(UC_X86_REG_AX) & 0xFF,
                        "es": machine.reg_read(UC_X86_REG_ES),
                    }
                )
            elif address == 0x9B44:
                phases.append({"address": address})

        machine = execute(
            entry,
            return_address,
            initial,
            [
                (0, return_address, b"\xcc"),
                (stack_segment, 0x2F95, context),
                (data_segment, 0x2F95, context_decoy),
                (game_segment, 0x2F95, context_decoy[::-1]),
                (data_segment, 0x5235, clip_words),
                (game_segment, 0x5235, clip_decoy),
                (stack_segment, 0x5235, clip_decoy[::-1]),
                (framebuffer_segment, 0, bytes(framebuffer_before)),
                (
                    stack_segment,
                    0xFF00,
                    struct.pack("<H", return_address) + stack_sentinel,
                ),
            ],
            code_handler=capture,
        )

        if outcome in ("draw", "occupied"):
            expected_phases = [
                {
                    "address": 0x9B30,
                    "di": machine_offset,
                    "es": framebuffer_segment,
                }
            ]
            if outcome == "draw":
                expected_phases.append(
                    {
                        "address": 0x9B41,
                        "di": machine_offset,
                        "al": shade,
                        "es": framebuffer_segment,
                    }
                )
            expected_phases.append({"address": 0x9B44})
        else:
            expected_phases = [{"address": 0x9B44}]
        if phases != expected_phases:
            raise AssertionError(
                f"0x9b04 {name}: phases={phases}, expected={expected_phases}"
            )

        framebuffer_after = bytes(
            machine.mem_read(framebuffer_segment * 16, len(framebuffer_expected))
        )
        if framebuffer_after != framebuffer_expected:
            raise AssertionError(f"0x9b04 {name}: framebuffer mismatch")
        if bytes(machine.mem_read(stack_segment * 16 + 0x2F95, 42)) != context:
            raise AssertionError(f"0x9b04 {name}: SS context changed")
        if bytes(machine.mem_read(data_segment * 16 + 0x2F95, 42)) != context_decoy:
            raise AssertionError(f"0x9b04 {name}: DS context decoy changed")
        if bytes(machine.mem_read(game_segment * 16 + 0x2F95, 42)) != context_decoy[::-1]:
            raise AssertionError(f"0x9b04 {name}: GS context decoy changed")
        if bytes(machine.mem_read(data_segment * 16 + 0x5235, 8)) != clip_words:
            raise AssertionError(f"0x9b04 {name}: DS clip changed")
        if bytes(machine.mem_read(game_segment * 16 + 0x5235, 8)) != clip_decoy:
            raise AssertionError(f"0x9b04 {name}: GS clip decoy changed")
        if bytes(machine.mem_read(stack_segment * 16 + 0x5235, 8)) != clip_decoy[::-1]:
            raise AssertionError(f"0x9b04 {name}: SS clip decoy changed")

        expected_registers = dict(initial)
        del expected_registers["flags"]
        expected_registers["sp"] = 0xFF02
        for register, expected in expected_registers.items():
            actual = machine.reg_read(REGISTERS[register])
            if actual != expected:
                raise AssertionError(
                    f"0x9b04 {name}: {register}={actual:#x}, expected={expected:#x}"
                )
        if machine.reg_read(UC_X86_REG_CS) != 0:
            raise AssertionError(f"0x9b04 {name}: near return changed CS")

        if outcome == "x_low":
            expected_flags = compare_flags(x_word, left)
        elif outcome == "x_high":
            expected_flags = compare_flags(x_word, right)
        elif outcome == "y_low":
            expected_flags = compare_flags(y_word, top)
        elif outcome == "y_high":
            expected_flags = compare_flags(y_word, bottom)
        elif outcome == "occupied":
            expected_flags = {
                "cf": False,
                "pf": pixel_before.bit_count() % 2 == 0,
                "zf": False,
                "sf": bool(pixel_before & 0x80),
                "of": False,
            }
        else:
            add_lhs = (-((depth >> 12) & 0xFF)) & 0xFF
            expected_flags = {
                "cf": add_lhs + 0xEF > 0xFF,
                "pf": shade.bit_count() % 2 == 0,
                "af": bool((add_lhs ^ 0xEF ^ shade) & 0x10),
                "zf": shade == 0,
                "sf": bool(shade & 0x80),
                "of": False,
            }
        flags_after = machine.reg_read(UC_X86_REG_EFLAGS)
        masks = {"cf": 1, "pf": 4, "af": 0x10, "zf": 0x40, "sf": 0x80, "of": 0x800}
        actual_flags = {
            flag: bool(flags_after & mask)
            for flag, mask in masks.items()
            if flag in expected_flags
        }
        if actual_flags != expected_flags:
            raise AssertionError(
                f"0x9b04 {name}: flags={actual_flags}, expected={expected_flags}"
            )
        if bytes(machine.mem_read(stack_segment * 16 + 0xFF02, 6)) != stack_sentinel:
            raise AssertionError(f"0x9b04 {name}: stack sentinel changed")

        vectors.append(
            {
                "name": name,
                "x": x,
                "y": y,
                "depth": depth,
                "clip": list(clip),
                "outcome": outcome,
                "machine_offset": machine_offset if outcome in ("draw", "occupied") else None,
                "natural_offset": natural_offset if outcome in ("draw", "occupied") else None,
                "natural_offset_matches": machine_offset == natural_offset,
                "pixel_before": pixel_before,
                "pixel_after": framebuffer_after[machine_offset],
                "defined_flags": expected_flags,
            }
        )

    return vectors


def presentation_line_helper_vectors() -> list[dict[str, object]]:
    entry = 0x7E1C
    resource_loader_entry = 0x24BB  # Runtime 01CE:07DB.
    entity_setter_entry = 0x3B4E  # Runtime 0299:11BE.
    data_segment = 0x4400
    resource_segment = 0x5000
    resource_offset = 0x3200
    fs_segment = 0x5400
    game_segment = 0x2C00
    extra_segment = 0x4800
    stack_segment = 0x9000
    line_offset = 0x6200
    return_address = 0x6F00
    cases = [
        {
            "name": "busy_gate",
            "ui": 0xA8,
            "flags": 0x80,
            "resource_id": 2,
            "terminal": 5,
            "frame": 3,
            "reverse": 1,
            "loaded_terminal": 7,
        },
        {
            "name": "loaded_forward_progress",
            "ui": 0x20,
            "flags": 0x84,
            "resource_id": 3,
            "terminal": 4,
            "frame": 1,
            "reverse": 0,
            "loaded_terminal": 9,
        },
        {
            "name": "loaded_forward_complete",
            "ui": 0x24,
            "flags": 0x04,
            "resource_id": 4,
            "terminal": 3,
            "frame": 3,
            "reverse": 0,
            "loaded_terminal": 8,
        },
        {
            "name": "loaded_reverse_progress",
            "ui": 0x40,
            "flags": 0x44,
            "resource_id": 5,
            "terminal": 6,
            "frame": 2,
            "reverse": 3,
            "loaded_terminal": 10,
        },
        {
            "name": "loaded_reverse_complete",
            "ui": 0xC4,
            "flags": 0x14,
            "resource_id": 6,
            "terminal": 6,
            "frame": 0,
            "reverse": 1,
            "loaded_terminal": 11,
        },
        {
            "name": "unloaded_forward",
            "ui": 0x10,
            "flags": 0x80,
            "resource_id": 2,
            "terminal": 0x7777,
            "frame": 0x8888,
            "reverse": 2,
            "loaded_terminal": 4,
        },
        {
            "name": "unloaded_reverse",
            "ui": 0x01,
            "flags": 0x08,
            "resource_id": 7,
            "terminal": 0x7777,
            "frame": 0x8888,
            "reverse": 3,
            "loaded_terminal": 4,
        },
        {
            "name": "unloaded_forward_terminal_zero",
            "ui": 0x15,
            "flags": 0x01,
            "resource_id": 8,
            "terminal": 0x7777,
            "frame": 0x8888,
            "reverse": 0,
            "loaded_terminal": 0,
        },
        {
            "name": "unloaded_reverse_terminal_zero",
            "ui": 0x02,
            "flags": 0x20,
            "resource_id": 9,
            "terminal": 0x7777,
            "frame": 0x8888,
            "reverse": 1,
            "loaded_terminal": 0,
        },
        {
            "name": "unloaded_resource_index_wrap",
            "ui": 0,
            "flags": 0,
            "resource_id": 0x1000,
            "terminal": 0x7777,
            "frame": 0x8888,
            "reverse": 0,
            "loaded_terminal": 2,
        },
        {
            "name": "loaded_forward_frame_wrap",
            "ui": 0x80,
            "flags": 0x04,
            "resource_id": 10,
            "terminal": 0,
            "frame": 0xFFFF,
            "reverse": 0,
            "loaded_terminal": 12,
        },
        {
            "name": "loaded_reverse_overflow",
            "ui": 0x08,
            "flags": 0x04,
            "resource_id": 11,
            "terminal": 0x9000,
            "frame": 0x8000,
            "reverse": 1,
            "loaded_terminal": 13,
            "ui_override": 0,
        },
    ]
    expected_hash = "73adf983beab60796f0f8075ee37a5e5d0a7ecc96c48979cbca01418d70bce6a"
    if hashlib.sha256(EXE[entry : entry + 152]).hexdigest() != expected_hash:
        raise AssertionError("0x7e1c: recovered 152-byte body changed")

    vectors = []
    for case_index, case in enumerate(cases):
        name = str(case["name"])
        ui_before = int(case.get("ui_override", case["ui"]))
        flags_before = int(case["flags"])
        resource_id = int(case["resource_id"])
        terminal_before = int(case["terminal"])
        frame_before = int(case["frame"])
        reverse_before = int(case["reverse"])
        loaded_terminal = int(case["loaded_terminal"])
        pad_01 = 0xA0 + case_index
        pad_04 = 0xB000 + case_index
        pad_0a = bytes((0xC0 + case_index + index) & 0xFF for index in range(10))
        draw_x = (0x1100 + case_index * 0x101) & 0xFFFF
        draw_y = (0x2200 + case_index * 0x111) & 0xFFFF
        record_before = struct.pack(
            "<BBHHHH10sHH",
            flags_before,
            pad_01,
            resource_id,
            pad_04,
            terminal_before,
            frame_before,
            pad_0a,
            draw_x,
            draw_y,
        )
        resource_before = bytes.fromhex("ccdd") + struct.pack(
            "<H", loaded_terminal
        ) + bytes.fromhex("a55a6996")
        name_offset = (0x0C04 + ((resource_id << 4) & 0xFFFF)) & 0xFFFF
        resource_name = (
            f"LINE{case_index:02d}.RES".encode("ascii") + b"\x00"
        ).ljust(16, b"\xcc")
        stack_sentinel = bytes.fromhex("5aa59669")
        memory = [
            (0, resource_loader_entry, b"\xcb"),
            (0, entity_setter_entry, b"\xcb"),
            (data_segment, 0x2793, bytes([ui_before])),
            (data_segment, 0x27E4, bytes([reverse_before])),
            (
                data_segment,
                0x0A80,
                struct.pack("<HH", resource_offset, resource_segment),
            ),
            (resource_segment, resource_offset, resource_before),
            (fs_segment, name_offset, resource_name),
            (stack_segment, line_offset, record_before),
            (game_segment, 0x2793, b"\x5a"),
            (game_segment, 0x27E4, b"\xa5"),
            (game_segment, 0x0A80, b"\x69\x96\x87\x78"),
            (extra_segment, 0x2793, b"\x3c"),
            (extra_segment, 0x27E4, b"\xc3"),
            (data_segment, name_offset, bytes([0x87]) * 16),
            (data_segment, line_offset, bytes([0x78]) * len(record_before)),
            (
                stack_segment,
                0xFF00,
                struct.pack("<H", return_address) + stack_sentinel,
            ),
            (0, return_address, b"\xcc"),
        ]
        initial = {
            "eax": 0xA1A1BEEF,
            "ebx": 0xB2B22345,
            "ecx": 0xC3C33456,
            "edx": 0xD4D44567,
            "esi": 0xE5E55678,
            "edi": 0xF6F66789,
            "ebp": 0x97970000 | line_offset,
            "sp": 0xFF00,
            "ds": data_segment,
            "es": extra_segment,
            "fs": fs_segment,
            "gs": game_segment,
            "ss": stack_segment,
            "flags": 0x0AD7,
        }
        calls = []

        def capture(machine: Uc, address: int, _size: int) -> None:
            if address == resource_loader_entry:
                calls.append(
                    {
                        "kind": "resource_load",
                        "ax": machine.reg_read(UC_X86_REG_AX),
                        "ds": machine.reg_read(UC_X86_REG_DS),
                        "si": machine.reg_read(UC_X86_REG_SI),
                        "es": machine.reg_read(UC_X86_REG_ES),
                        "di": machine.reg_read(UC_X86_REG_DI),
                        "sp": machine.reg_read(UC_X86_REG_SP),
                        "cs": machine.reg_read(UC_X86_REG_CS),
                    }
                )
            elif address == entity_setter_entry:
                calls.append(
                    {
                        "kind": "entity_setter",
                        "ax": machine.reg_read(UC_X86_REG_AX),
                        "bx": machine.reg_read(UC_X86_REG_BX),
                        "cx": machine.reg_read(UC_X86_REG_CX),
                        "bp": machine.reg_read(UC_X86_REG_BP),
                        "ds": machine.reg_read(UC_X86_REG_DS),
                        "es": machine.reg_read(UC_X86_REG_ES),
                        "di": machine.reg_read(UC_X86_REG_DI),
                        "sp": machine.reg_read(UC_X86_REG_SP),
                        "cs": machine.reg_read(UC_X86_REG_CS),
                    }
                )

        machine = execute(
            entry,
            return_address,
            initial,
            memory,
            code_handler=capture,
        )

        busy = (ui_before & 0x08) != 0
        loaded = (flags_before & 0x04) != 0
        expected_flags_byte = flags_before
        expected_terminal = terminal_before
        expected_frame = frame_before
        expected_ui = ui_before
        expected_reverse = reverse_before
        completed = False
        frame_drawn = None
        if busy:
            expected_calls = []
            flag_source = "busy_test"
            flag_value = ui_before & 0x08
        else:
            if not loaded:
                expected_ui |= 0x04
                expected_terminal = loaded_terminal
                expected_frame = (loaded_terminal - 1) & 0xFFFF
                if (reverse_before & 1) == 0:
                    expected_frame = 0
                    expected_reverse = 0
                expected_flags_byte |= 0x04
            frame_drawn = expected_frame
            if (expected_reverse & 1) != 0:
                if expected_frame == 0:
                    completed = True
                else:
                    decrement_input = expected_frame
                    expected_frame = (expected_frame - 1) & 0xFFFF
                    flag_source = "decrement"
                    flag_value = expected_frame
            elif expected_frame == expected_terminal:
                completed = True
            else:
                increment_input = expected_frame
                expected_frame = (expected_frame + 1) & 0xFFFF
                flag_source = "increment"
                flag_value = expected_frame
            if completed:
                expected_reverse = 0
                expected_ui &= 0xFB
                flag_source = "completion_and"
                flag_value = expected_ui

            expected_calls = []
            if not loaded:
                expected_calls.append(
                    {
                        "kind": "resource_load",
                        "ax": (resource_id << 4) & 0xFFFF,
                        "ds": fs_segment,
                        "si": name_offset,
                        "es": resource_segment,
                        "di": resource_offset,
                        "sp": 0xFEF0,
                        "cs": 0x01CE,
                    }
                )
            expected_calls.append(
                {
                    "kind": "entity_setter",
                    "ax": 4,
                    "bx": draw_x,
                    "cx": draw_y,
                    "bp": frame_drawn,
                    "ds": data_segment,
                    "es": resource_segment,
                    "di": resource_offset,
                    "sp": 0xFEF0,
                    "cs": 0x0299,
                }
            )
        if calls != expected_calls:
            raise AssertionError(
                f"0x7e1c {name}: calls={calls}, expected={expected_calls}"
            )

        expected_record = struct.pack(
            "<BBHHHH10sHH",
            expected_flags_byte,
            pad_01,
            resource_id,
            pad_04,
            expected_terminal,
            expected_frame,
            pad_0a,
            draw_x,
            draw_y,
        )
        actual_record = bytes(
            machine.mem_read(stack_segment * 16 + line_offset, len(record_before))
        )
        if actual_record != expected_record:
            raise AssertionError(
                f"0x7e1c {name}: record={actual_record.hex()}, "
                f"expected={expected_record.hex()}"
            )
        actual_ui = machine.mem_read(data_segment * 16 + 0x2793, 1)[0]
        actual_reverse = machine.mem_read(data_segment * 16 + 0x27E4, 1)[0]
        if (actual_ui, actual_reverse) != (expected_ui, expected_reverse):
            raise AssertionError(
                f"0x7e1c {name}: state={(actual_ui, actual_reverse)}, "
                f"expected={(expected_ui, expected_reverse)}"
            )
        for segment, offset, expected in (
            (resource_segment, resource_offset, resource_before),
            (fs_segment, name_offset, resource_name),
            (game_segment, 0x2793, b"\x5a"),
            (game_segment, 0x27E4, b"\xa5"),
            (game_segment, 0x0A80, b"\x69\x96\x87\x78"),
            (extra_segment, 0x2793, b"\x3c"),
            (extra_segment, 0x27E4, b"\xc3"),
            (data_segment, name_offset, bytes([0x87]) * 16),
            (data_segment, line_offset, bytes([0x78]) * len(record_before)),
        ):
            actual = bytes(machine.mem_read(segment * 16 + offset, len(expected)))
            if actual != expected:
                raise AssertionError(
                    f"0x7e1c {name}: immutable {segment:#x}:{offset:#x} changed"
                )

        expected_registers = dict(initial)
        del expected_registers["flags"]
        expected_registers["sp"] = 0xFF02
        if not busy:
            expected_registers["edi"] = (
                initial["edi"] & 0xFFFF0000
            ) | resource_offset
            expected_registers["es"] = resource_segment
        for register, expected in expected_registers.items():
            actual = machine.reg_read(REGISTERS[register])
            if actual != expected:
                raise AssertionError(
                    f"0x7e1c {name}: {register}={actual:#x}, expected={expected:#x}"
                )
        if machine.reg_read(UC_X86_REG_CS) != 0:
            raise AssertionError(f"0x7e1c {name}: far call did not restore CS")

        parity = (flag_value & 0xFF).bit_count() % 2 == 0
        expected_status = {
            "cf": completed,
            "pf": parity,
            "zf": flag_value == 0,
            "sf": bool(flag_value & (0x80 if flag_source.endswith("test") or flag_source.endswith("and") else 0x8000)),
            "of": False,
        }
        if flag_source == "increment":
            expected_status["af"] = (increment_input & 0x0F) == 0x0F
            expected_status["of"] = increment_input == 0x7FFF
        elif flag_source == "decrement":
            expected_status["af"] = (decrement_input & 0x0F) == 0
            expected_status["of"] = decrement_input == 0x8000
        flags_after = machine.reg_read(UC_X86_REG_EFLAGS)
        masks = {
            "cf": 1,
            "pf": 4,
            "af": 0x10,
            "zf": 0x40,
            "sf": 0x80,
            "of": 0x800,
        }
        actual_status = {
            flag: bool(flags_after & masks[flag]) for flag in expected_status
        }
        if actual_status != expected_status:
            raise AssertionError(
                f"0x7e1c {name}: flags={actual_status}, expected={expected_status}"
            )
        if bytes(machine.mem_read(stack_segment * 16 + 0xFF02, 4)) != stack_sentinel:
            raise AssertionError(f"0x7e1c {name}: stack sentinel changed")

        vectors.append(
            {
                "name": name,
                "busy_gate": busy,
                "loaded_before": loaded,
                "resource_id": resource_id,
                "resource_name_offset": name_offset if not loaded and not busy else None,
                "loaded_terminal_frame": loaded_terminal if not loaded and not busy else None,
                "frame_drawn": frame_drawn,
                "terminal_frame_after": expected_terminal,
                "frame_after": expected_frame,
                "ui_before": ui_before,
                "ui_after": actual_ui,
                "reverse_before": reverse_before,
                "reverse_after": actual_reverse,
                "completed_cf": completed,
                "helper_calls": [call["kind"] for call in calls],
                "defined_flags": expected_status,
            }
        )
    return vectors


def fs_name_area_read_vectors() -> list[dict[str, object]]:
    entry = 0x7788
    data_segment = 0x4400
    extra_segment = 0x4800
    fs_segment = 0x5000
    game_segment = 0x2C00
    stack_segment = 0x9000
    return_address = 0x6F00
    cases = [
        ("immediate_nul", 0x6400, b"\x00", 0x00, 0x0002),
        ("immediate_high", 0x6420, b"\xff", 0x7E, 0x0AD7),
        ("lower_printable", 0x6440, b"\x20\x1f", 0xA5, 0x0803),
        ("upper_printable", 0x6460, b"\x7f\x80", 0x5A, 0x00D6),
        ("ordinary_text", 0x6480, b"RESOURCE.DAT\x00", 0x00, 0x0812),
        ("source_wrap", 0xFFFF, b"A\x00", 0xFF, 0x00C3),
        ("low_stop_after_text", 0x64C0, b"ABC\x10", 0x12, 0x0896),
        ("high_stop_after_text", 0x64E0, b"XYZ\x80", 0x34, 0x0047),
    ]
    expected_hash = "d7a9b564c65a9ad53216b618864c4ee8519e6a58ae500780e3b0aefae6116fe0"
    if hashlib.sha256(EXE[entry : entry + 33]).hexdigest() != expected_hash:
        raise AssertionError("0x7788: recovered 33-byte body changed")

    vectors = []
    for name, start, payload, dirty_before, flags_before in cases:
        stop_index = next(
            index
            for index, byte in enumerate(payload)
            if byte < 0x20 or byte >= 0x80
        )
        copied = payload[:stop_index]
        stop_byte = payload[stop_index]
        final_source = (start + stop_index) & 0xFFFF
        final_destination = 0x0C74 + len(copied)
        destination_before = bytes([0xCC]) * (len(copied) + 3)
        stack_sentinel = bytes.fromhex("5aa59669")
        memory = [
            (fs_segment, 0x0C74, destination_before),
            (extra_segment, 0x0C74, bytes([0xDD]) * len(destination_before)),
            (game_segment, 0x0C74, bytes([0xA5]) * len(destination_before)),
            (game_segment, 0x27E8, bytes([dirty_before])),
            (data_segment, 0x27E8, b"\x69"),
            (extra_segment, 0x27E8, b"\x96"),
            (fs_segment, 0x27E8, b"\x5a"),
            (
                stack_segment,
                0xFF00,
                struct.pack("<H", return_address) + stack_sentinel,
            ),
            (0, return_address, b"\xcc"),
        ]
        immutable_source = []
        for index, byte in enumerate(payload):
            source_offset = (start + index) & 0xFFFF
            encoded = bytes([byte])
            memory.append((data_segment, source_offset, encoded))
            immutable_source.append((source_offset, encoded))

        initial = {
            "eax": 0xA1A1BE55,
            "ebx": 0xB2B22345,
            "ecx": 0xC3C33456,
            "edx": 0xD4D44567,
            "esi": 0xE5E50000 | start,
            "edi": 0xF6F66789,
            "ebp": 0x9797789A,
            "sp": 0xFF00,
            "ds": data_segment,
            "es": extra_segment,
            "fs": fs_segment,
            "gs": game_segment,
            "ss": stack_segment,
            "flags": flags_before,
        }
        machine = execute(entry, return_address, initial, memory)

        actual_destination = bytes(
            machine.mem_read(
                fs_segment * 16 + 0x0C74,
                len(destination_before),
            )
        )
        expected_destination = (
            copied + b"\x00" + destination_before[len(copied) + 1 :]
        )
        if actual_destination != expected_destination:
            raise AssertionError(
                f"0x7788 {name}: destination={actual_destination!r}, "
                f"expected={expected_destination!r}"
            )
        actual_dirty = machine.mem_read(game_segment * 16 + 0x27E8, 1)[0]
        if actual_dirty != 1:
            raise AssertionError(f"0x7788 {name}: dirty={actual_dirty:#x}")
        for source_offset, expected in immutable_source:
            actual = bytes(machine.mem_read(data_segment * 16 + source_offset, 1))
            if actual != expected:
                raise AssertionError(f"0x7788 {name}: source changed")
        for segment, offset, expected in (
            (extra_segment, 0x0C74, bytes([0xDD]) * len(destination_before)),
            (game_segment, 0x0C74, bytes([0xA5]) * len(destination_before)),
            (data_segment, 0x27E8, b"\x69"),
            (extra_segment, 0x27E8, b"\x96"),
            (fs_segment, 0x27E8, b"\x5a"),
        ):
            actual = bytes(machine.mem_read(segment * 16 + offset, len(expected)))
            if actual != expected:
                raise AssertionError(f"0x7788 {name}: segment decoy changed")

        expected_registers = dict(initial)
        del expected_registers["flags"]
        expected_registers.update(
            {
                "eax": (initial["eax"] & 0xFFFF0000) | (fs_segment & 0xFF00) | stop_byte,
                "esi": (initial["esi"] & 0xFFFF0000) | final_source,
                "edi": (initial["edi"] & 0xFFFF0000) | final_destination,
                "sp": 0xFF02,
            }
        )
        for register, expected in expected_registers.items():
            actual = machine.reg_read(REGISTERS[register])
            if actual != expected:
                raise AssertionError(
                    f"0x7788 {name}: {register}={actual:#x}, expected={expected:#x}"
                )

        dec_input = (final_source + 1) & 0xFFFF
        expected_flags = {
            "cf": stop_byte < 0x20,
            "pf": ((final_source & 0xFF).bit_count() % 2) == 0,
            "af": (dec_input & 0x0F) == 0,
            "zf": final_source == 0,
            "sf": bool(final_source & 0x8000),
            "of": dec_input == 0x8000,
        }
        flags_after = machine.reg_read(UC_X86_REG_EFLAGS)
        masks = {"cf": 1, "pf": 4, "af": 0x10, "zf": 0x40, "sf": 0x80, "of": 0x800}
        actual_flags = {
            flag: bool(flags_after & mask) for flag, mask in masks.items()
        }
        if actual_flags != expected_flags:
            raise AssertionError(
                f"0x7788 {name}: flags={actual_flags}, expected={expected_flags}"
            )
        if bytes(machine.mem_read(stack_segment * 16 + 0xFF02, 4)) != stack_sentinel:
            raise AssertionError(f"0x7788 {name}: stack sentinel changed")

        vectors.append(
            {
                "name": name,
                "source_offset": start,
                "input_hex": payload.hex(),
                "copied_hex": copied.hex(),
                "stopping_byte": stop_byte,
                "dirty_before": dirty_before,
                "dirty_after": actual_dirty,
                "final_source_offset": final_source,
                "final_destination_offset": final_destination,
                "defined_flags": expected_flags,
            }
        )
    return vectors


def music_voc_name_patcher_vectors() -> list[dict[str, object]]:
    entry = 0x77A9
    data_segment = 0x4400
    destination_segment = 0x4800
    game_segment = 0x2C00
    stack_segment = 0x9000
    return_address = 0x6F00
    cases = [
        ("immediate_space", 0x6600, b"\x20", b"OLD", 0, 0, 0x0002),
        ("immediate_nul", 0x6620, b"\x00", b"OLD", 0, 0x80, 0x0AD7),
        ("immediate_high", 0x6640, b"\xff", b"OLD", 0, 0x40, 0x0803),
        ("equal_upper", 0x6660, b"ABC\x20", b"ABCZ", 0, 0x20, 0x00D6),
        ("lowercase_matches", 0x6680, b"abc\x00", b"ABCZ", 0, 0x10, 0x0812),
        ("mismatch_sets_changed", 0x66A0, b"ABC\x00", b"AXCZ", 0, 0x44, 0x00C3),
        ("backtick_not_masked", 0x66C0, b"`\x00", b"`Z", 0, 0x08, 0x0896),
        ("brace_masks_to_bracket", 0x66E0, b"{\x00", b"[Z", 0, 0x04, 0x0047),
        ("prechanged_bit_blocks_unchanged", 0x6700, b"A\x00", b"AZ", 1, 0x22, 0x0012),
        ("even_changed_value_allows_unchanged", 0x6720, b"A\x00", b"AZ", 2, 0x40, 0x0802),
        ("source_wrap", 0xFFFF, b"a\x00", b"AZ", 0, 0, 0x0003),
    ]
    expected_hash = "2177b4dd9c7763c956100260a38cb70600c06256437e351830023f135f4cbf4e"
    if hashlib.sha256(EXE[entry : entry + 52]).hexdigest() != expected_hash:
        raise AssertionError("0x77a9: recovered 52-byte body changed")

    vectors = []
    for (
        name,
        start,
        payload,
        destination_before,
        changed_before,
        unchanged_before,
        flags_before,
    ) in cases:
        stop_index = next(
            index
            for index, byte in enumerate(payload)
            if byte <= 0x20 or byte >= 0x80
        )
        accepted = payload[:stop_index]
        stop_byte = payload[stop_index]
        transformed = bytes(
            byte & 0xDF if byte >= 0x61 else byte for byte in accepted
        )
        final_source = (start + stop_index) & 0xFFFF
        final_destination = 0x0D30 + len(transformed)
        destination_seed = destination_before + bytes([0xCC]) * max(
            0, len(transformed) + 2 - len(destination_before)
        )
        destination_seed = destination_seed[: len(transformed) + 2]
        expected_changed = changed_before
        for index, byte in enumerate(transformed):
            if byte != destination_seed[index]:
                expected_changed = 1
        expected_unchanged = unchanged_before
        if (expected_changed & 1) == 0:
            expected_unchanged |= 1
        expected_destination = (
            transformed + b"\x00" + destination_seed[len(transformed) + 1 :]
        )
        stack_sentinel = bytes.fromhex("5aa59669")
        memory = [
            (destination_segment, 0x0D30, destination_seed),
            (game_segment, 0x0D30, bytes([0xA5]) * len(destination_seed)),
            (game_segment, 0x0BA1, bytes([changed_before])),
            (game_segment, 0x0BA0, bytes([unchanged_before])),
            (data_segment, 0x0BA1, b"\x69"),
            (data_segment, 0x0BA0, b"\x96"),
            (destination_segment, 0x0BA1, b"\x87"),
            (destination_segment, 0x0BA0, b"\x78"),
            (
                stack_segment,
                0xFF00,
                struct.pack("<H", return_address) + stack_sentinel,
            ),
            (0, return_address, b"\xcc"),
        ]
        immutable_source = []
        for index, byte in enumerate(payload):
            source_offset = (start + index) & 0xFFFF
            encoded = bytes([byte])
            memory.append((data_segment, source_offset, encoded))
            immutable_source.append((source_offset, encoded))

        initial = {
            "eax": 0xA1A1BE55,
            "ebx": 0xB2B22345,
            "ecx": 0xC3C33456,
            "edx": 0xD4D44567,
            "esi": 0xE5E50000 | start,
            "edi": 0xF6F66789,
            "ebp": 0x9797789A,
            "sp": 0xFF00,
            "ds": data_segment,
            "es": destination_segment,
            "fs": 0x5000,
            "gs": game_segment,
            "ss": stack_segment,
            "flags": flags_before,
        }
        machine = execute(entry, return_address, initial, memory)

        actual_destination = bytes(
            machine.mem_read(
                destination_segment * 16 + 0x0D30,
                len(destination_seed),
            )
        )
        if actual_destination != expected_destination:
            raise AssertionError(
                f"0x77a9 {name}: destination={actual_destination!r}, "
                f"expected={expected_destination!r}"
            )
        actual_changed = machine.mem_read(game_segment * 16 + 0x0BA1, 1)[0]
        actual_unchanged = machine.mem_read(game_segment * 16 + 0x0BA0, 1)[0]
        if (
            actual_changed != expected_changed
            or actual_unchanged != expected_unchanged
        ):
            raise AssertionError(
                f"0x77a9 {name}: state={(actual_changed, actual_unchanged)}, "
                f"expected={(expected_changed, expected_unchanged)}"
            )
        for source_offset, expected in immutable_source:
            actual = bytes(machine.mem_read(data_segment * 16 + source_offset, 1))
            if actual != expected:
                raise AssertionError(f"0x77a9 {name}: source changed")
        for segment, offset, expected in (
            (game_segment, 0x0D30, bytes([0xA5]) * len(destination_seed)),
            (data_segment, 0x0BA1, b"\x69"),
            (data_segment, 0x0BA0, b"\x96"),
            (destination_segment, 0x0BA1, b"\x87"),
            (destination_segment, 0x0BA0, b"\x78"),
        ):
            actual = bytes(machine.mem_read(segment * 16 + offset, len(expected)))
            if actual != expected:
                raise AssertionError(f"0x77a9 {name}: segment decoy changed")

        expected_registers = dict(initial)
        del expected_registers["flags"]
        expected_registers.update(
            {
                "eax": (initial["eax"] & 0xFFFFFF00) | stop_byte,
                "esi": (initial["esi"] & 0xFFFF0000) | final_source,
                "edi": (initial["edi"] & 0xFFFF0000) | final_destination,
                "sp": 0xFF02,
            }
        )
        for register, expected in expected_registers.items():
            actual = machine.reg_read(REGISTERS[register])
            if actual != expected:
                raise AssertionError(
                    f"0x77a9 {name}: {register}={actual:#x}, expected={expected:#x}"
                )
        dec_input = (final_source + 1) & 0xFFFF
        expected_flags = {
            "cf": False,
            "pf": ((final_source & 0xFF).bit_count() % 2) == 0,
            "af": (dec_input & 0x0F) == 0,
            "zf": final_source == 0,
            "sf": bool(final_source & 0x8000),
            "of": dec_input == 0x8000,
        }
        flags_after = machine.reg_read(UC_X86_REG_EFLAGS)
        masks = {"cf": 1, "pf": 4, "af": 0x10, "zf": 0x40, "sf": 0x80, "of": 0x800}
        actual_flags = {
            flag: bool(flags_after & mask) for flag, mask in masks.items()
        }
        if actual_flags != expected_flags:
            raise AssertionError(
                f"0x77a9 {name}: flags={actual_flags}, expected={expected_flags}"
            )
        if bytes(machine.mem_read(stack_segment * 16 + 0xFF02, 4)) != stack_sentinel:
            raise AssertionError(f"0x77a9 {name}: stack sentinel changed")

        vectors.append(
            {
                "name": name,
                "source_offset": start,
                "input_hex": payload.hex(),
                "accepted_hex": accepted.hex(),
                "transformed_hex": transformed.hex(),
                "stopping_byte": stop_byte,
                "changed_before": changed_before,
                "changed_after": actual_changed,
                "unchanged_before": unchanged_before,
                "unchanged_after": actual_unchanged,
                "final_source_offset": final_source,
                "final_destination_offset": final_destination,
                "defined_flags": expected_flags,
            }
        )
    return vectors


def byte_parser_store_word_1fa5_vectors() -> list[dict[str, object]]:
    entry = 0x76BA
    data_segment = 0x4400
    extra_segment = 0x4800
    game_segment = 0x2C00
    stack_segment = 0x9000
    return_address = 0x6F00
    cases = [
        ("zero", 0x6000, 0x0000, 0x0002),
        ("all_ones", 0x6020, 0xFFFF, 0x0AD7),
        ("little_endian", 0x6040, 0x1234, 0x0803),
        ("high_bit", 0x6060, 0x8001, 0x00D6),
        ("low_byte_ff", 0x6080, 0x12FF, 0x0812),
        ("high_byte_ff", 0x60A0, 0xFF12, 0x00C3),
        ("unaligned", 0x60C1, 0xA55A, 0x0896),
        ("source_end_wrap", 0xFFFE, 0xCAFE, 0x0047),
    ]
    if EXE[entry : entry + 6] != bytes.fromhex("ad65a3a51fc3"):
        raise AssertionError("0x76ba: recovered six-byte body changed")

    vectors = []
    for case_index, (name, start, operand, flags_before) in enumerate(cases):
        destination_before = (0x3100 + case_index * 0x111) & 0xFFFF
        data_decoy = destination_before ^ 0xFFFF
        extra_decoy = destination_before ^ 0x5A5A
        stack_decoy = destination_before ^ 0xA5A5
        script = struct.pack("<H", operand)
        stack_sentinel = bytes.fromhex("5aa59669")
        memory = [
            (game_segment, 0x1FA5, struct.pack("<H", destination_before)),
            (data_segment, 0x1FA5, struct.pack("<H", data_decoy)),
            (extra_segment, 0x1FA5, struct.pack("<H", extra_decoy)),
            (stack_segment, 0x1FA5, struct.pack("<H", stack_decoy)),
            (
                stack_segment,
                0xFF00,
                struct.pack("<H", return_address) + stack_sentinel,
            ),
            (0, return_address, b"\xcc"),
        ]
        immutable_source = []
        for byte_index, byte in enumerate(script):
            source_offset = (start + byte_index) & 0xFFFF
            encoded = bytes([byte])
            memory.append((data_segment, source_offset, encoded))
            memory.append((extra_segment, source_offset, b"\x5a"))
            memory.append((game_segment, source_offset, b"\xa5"))
            immutable_source.append((source_offset, encoded))

        initial = {
            "eax": 0xA1A1BEEF,
            "ebx": 0xB2B22345,
            "ecx": 0xC3C33456,
            "edx": 0xD4D44567,
            "esi": 0xE5E50000 | start,
            "edi": 0xF6F66789,
            "ebp": 0x9797789A,
            "sp": 0xFF00,
            "ds": data_segment,
            "es": extra_segment,
            "fs": 0x4C00,
            "gs": game_segment,
            "ss": stack_segment,
            "flags": flags_before,
        }
        phases = []

        def capture(machine: Uc, address: int, _size: int) -> None:
            if address not in (0x76BB, 0x76BF):
                return
            phases.append(
                (
                    address,
                    machine.reg_read(UC_X86_REG_AX),
                    machine.reg_read(UC_X86_REG_SI),
                    struct.unpack(
                        "<H", machine.mem_read(game_segment * 16 + 0x1FA5, 2)
                    )[0],
                )
            )

        machine = execute(
            entry,
            return_address,
            initial,
            memory,
            code_handler=capture,
        )
        final_source_offset = (start + 2) & 0xFFFF
        expected_phases = [
            (0x76BB, operand, final_source_offset, destination_before),
            (0x76BF, operand, final_source_offset, operand),
        ]
        if phases != expected_phases:
            raise AssertionError(
                f"0x76ba {name}: phases={phases}, expected={expected_phases}"
            )

        expected_registers = dict(initial)
        del expected_registers["flags"]
        expected_registers.update(
            {
                "eax": (initial["eax"] & 0xFFFF0000) | operand,
                "esi": (initial["esi"] & 0xFFFF0000) | final_source_offset,
                "sp": 0xFF02,
            }
        )
        for register, expected in expected_registers.items():
            actual = machine.reg_read(REGISTERS[register])
            if actual != expected:
                raise AssertionError(
                    f"0x76ba {name}: {register}={actual:#x}, expected={expected:#x}"
                )

        destination_after = struct.unpack(
            "<H", machine.mem_read(game_segment * 16 + 0x1FA5, 2)
        )[0]
        if destination_after != operand:
            raise AssertionError(
                f"0x76ba {name}: destination={destination_after:#x}, "
                f"expected={operand:#x}"
            )
        for segment, expected in (
            (data_segment, data_decoy),
            (extra_segment, extra_decoy),
            (stack_segment, stack_decoy),
        ):
            actual = struct.unpack(
                "<H", machine.mem_read(segment * 16 + 0x1FA5, 2)
            )[0]
            if actual != expected:
                raise AssertionError(f"0x76ba {name}: segment decoy changed")
        for source_offset, expected in immutable_source:
            actual = bytes(machine.mem_read(data_segment * 16 + source_offset, 1))
            if actual != expected:
                raise AssertionError(f"0x76ba {name}: source changed")

        flag_masks = {
            "cf": 0x0001,
            "pf": 0x0004,
            "af": 0x0010,
            "zf": 0x0040,
            "sf": 0x0080,
            "of": 0x0800,
        }
        expected_flags = {
            flag: bool(flags_before & mask) for flag, mask in flag_masks.items()
        }
        flags_after = machine.reg_read(UC_X86_REG_EFLAGS)
        actual_flags = {
            flag: bool(flags_after & mask) for flag, mask in flag_masks.items()
        }
        if actual_flags != expected_flags:
            raise AssertionError(
                f"0x76ba {name}: flags={actual_flags}, expected={expected_flags}"
            )
        if bytes(machine.mem_read(stack_segment * 16 + 0xFF02, 4)) != stack_sentinel:
            raise AssertionError(f"0x76ba {name}: stack sentinel changed")

        vectors.append(
            {
                "name": name,
                "dispatch_opcode": "0x08",
                "source_offset": start,
                "operand": operand,
                "destination_before": destination_before,
                "destination_after": destination_after,
                "final_source_offset": final_source_offset,
                "defined_flags": expected_flags,
            }
        )
    return vectors


def sprite_blitter_noop_vectors(entry: int) -> list[dict[str, object]]:
    return_address = 0x6F00
    initial = {
        "eax": 0xA1A11234,
        "ebx": 0xB2B22345,
        "ecx": 0xC3C33456,
        "edx": 0xD4D44567,
        "esi": 0xE5E55678,
        "edi": 0xF6F66789,
        "ebp": 0x9797789A,
        "ds": 0x2000,
        "es": 0x2400,
        "gs": 0x2800,
        "flags": 0x0AD7,
    }
    stack_sentinel = bytes.fromhex("5aa59669")
    machine = execute(
        entry,
        return_address,
        initial,
        [
            (0x9000, 0xFF00, struct.pack("<H", return_address) + stack_sentinel),
            (0, return_address, b"\xcc"),
        ],
    )

    for register, value in initial.items():
        actual_register = machine.reg_read(REGISTERS[register])
        if actual_register != value:
            raise AssertionError(f"{entry:#x}: changed {register}")
    if machine.reg_read(UC_X86_REG_SP) != 0xFF02:
        raise AssertionError(f"{entry:#x}: near RET did not consume return word")
    actual_sentinel = bytes(machine.mem_read(0x9000 * 16 + 0xFF02, 4))
    if actual_sentinel != stack_sentinel:
        raise AssertionError(f"{entry:#x}: stack sentinel changed")
    if EXE[entry] != 0xC3:
        raise AssertionError(f"{entry:#x}: expected one-byte near RET")

    return [
        {
            "entry": f"0x{entry:06x}",
            "opcode": "c3",
            "stack_pointer_before": 0xFF00,
            "stack_pointer_after": 0xFF02,
            "return_address": return_address,
            "registers_and_flags_preserved": True,
        }
    ]


def mask_overlay_vectors() -> list[dict[str, object]]:
    patterns = [
        [0x0000] * 16,
        [0x8000 >> row for row in range(16)],
        [0xffff] * 16,
        [0xaaaa, 0x5555] * 8,
        [1 << (15 - row) for row in range(16)],
        [
            0x8001,
            0x4002,
            0x2004,
            0x1008,
            0x0810,
            0x0420,
            0x0240,
            0x0180,
        ]
        * 2,
    ]
    data_segment = 0x2000
    framebuffer_segment = 0x3000
    framebuffer_size = 0x4000
    table = bytearray()
    for rows in patterns:
        for bits in rows:
            table.extend(struct.pack(">H", bits))

    vectors = []
    for index, rows in enumerate(patterns):
        pointer_offset = 0 if index % 2 == 0 else 0x4567
        initial_framebuffer = bytes([0x5a]) * framebuffer_size
        machine = execute(
            0x7CB4,
            0x7CE7,
            {
                "ax": 0x1234,
                "bx": 0x2345,
                "cx": 0x3456,
                "dx": 0x4567,
                "si": 0x5678,
                "di": 0x6789,
                "bp": 0x789a,
                "ds": data_segment,
                "es": 0x3456,
            },
            [
                (data_segment, 0x27e3, bytes([index])),
                (
                    data_segment,
                    0x5221,
                    struct.pack("<HH", pointer_offset, framebuffer_segment),
                ),
                (data_segment, 0x7bb8, bytes(table)),
                (framebuffer_segment, 0, initial_framebuffer),
            ],
        )
        actual = bytes(
            machine.mem_read(framebuffer_segment * 16, framebuffer_size)
        )
        expected = bytearray(initial_framebuffer)
        changed_offsets = []
        for row, bits in enumerate(rows):
            for column in range(16):
                if (bits & (0x8000 >> column)) != 0:
                    offset = 0x12c5 + row * 320 + column
                    expected[offset] = 0xfe
                    changed_offsets.append(offset)
        if actual != bytes(expected):
            raise AssertionError(f"0x7CB4 mask {index} produced unexpected pixels")
        if machine.reg_read(UC_X86_REG_ES) != 0x3456:
            raise AssertionError("0x7CB4 did not preserve ES")
        if machine.reg_read(UC_X86_REG_DI) != 0x6789:
            raise AssertionError("0x7CB4 did not preserve DI")

        vectors.append(
            {
                "index": index,
                "input_pointer_offset_ignored": pointer_offset,
                "big_endian_rows": rows,
                "color": 0xfe,
                "changed_offsets": changed_offsets,
            }
        )

    return vectors


def queue_consume_vectors() -> list[dict[str, object]]:
    data_segment = 0x2000
    tail_segment = 0x3000
    cases = [
        {
            "name": "ordinary_advance",
            "tail": 0x0100,
            "entry_bytes": 0x0020,
            "byte_count": 0x0100,
            "buffer_end": 0x1000,
            "sequence": 0x0010,
            "read_index": 0x0003,
            "read_limit": 0x0007,
        },
        {
            "name": "candidate_equals_end",
            "tail": 0x0F00,
            "entry_bytes": 0x00FE,
            "byte_count": 0x0200,
            "buffer_end": 0x1000,
            "sequence": 0x0100,
            "read_index": 0x0006,
            "read_limit": 0x0007,
        },
        {
            "name": "candidate_past_end",
            "tail": 0x0F00,
            "entry_bytes": 0x0100,
            "byte_count": 0x0200,
            "buffer_end": 0x1000,
            "sequence": 0x1234,
            "read_index": 0x0002,
            "read_limit": 0x0007,
        },
        {
            "name": "candidate_add_carry",
            "tail": 0xFF00,
            "entry_bytes": 0x0200,
            "byte_count": 0x0300,
            "buffer_end": 0xFFFF,
            "sequence": 0xFFFE,
            "read_index": 0x0010,
            "read_limit": 0x0020,
        },
        {
            "name": "read_index_past_limit",
            "tail": 0x0200,
            "entry_bytes": 0x0010,
            "byte_count": 0x0010,
            "buffer_end": 0x1000,
            "sequence": 0xFFFF,
            "read_index": 0x0007,
            "read_limit": 0x0007,
        },
        {
            "name": "read_index_equals_limit",
            "tail": 0x0200,
            "entry_bytes": 0x0010,
            "byte_count": 0x0010,
            "buffer_end": 0x1000,
            "sequence": 0x2222,
            "read_index": 0x0006,
            "read_limit": 0x0007,
        },
        {
            "name": "read_index_word_wrap",
            "tail": 0x0200,
            "entry_bytes": 0x0010,
            "byte_count": 0x0008,
            "buffer_end": 0x1000,
            "sequence": 0x3333,
            "read_index": 0xFFFF,
            "read_limit": 0x0000,
        },
        {
            "name": "tail_plus_header_wrap_is_discarded",
            "tail": 0xFFFF,
            "entry_bytes": 0x0000,
            "byte_count": 0x0000,
            "buffer_end": 0x0001,
            "sequence": 0x4444,
            "read_index": 0x0000,
            "read_limit": 0xFFFF,
        },
    ]
    vectors = []

    for case in cases:
        initial = {
            "ax": 0x1111,
            "bx": 0x2222,
            "cx": 0x3333,
            "dx": 0x4444,
            "si": 0x5555,
            "di": 0x6666,
            "bp": 0x7777,
            "ds": data_segment,
            "es": 0x8888,
        }
        tail = int(case["tail"])
        entry_bytes = int(case["entry_bytes"])
        byte_count = int(case["byte_count"])
        buffer_end = int(case["buffer_end"])
        sequence = int(case["sequence"])
        read_index = int(case["read_index"])
        read_limit = int(case["read_limit"])
        machine = execute(
            0xA3D0,
            0xA40A,
            initial,
            [
                (
                    data_segment,
                    0x0D90,
                    struct.pack("<HH", tail, tail_segment),
                ),
                (data_segment, 0x0D9A, struct.pack("<H", byte_count)),
                (data_segment, 0x5233, struct.pack("<H", buffer_end)),
                (data_segment, 0x131C, struct.pack("<H", sequence)),
                (data_segment, 0x0D60, struct.pack("<H", read_index)),
                (data_segment, 0x0D64, struct.pack("<H", read_limit)),
                (tail_segment, tail, struct.pack("<H", entry_bytes)),
            ],
        )

        after_header = (tail + 2) & 0xFFFF
        sum_after_header = after_header + entry_bytes
        candidate = sum_after_header & 0xFFFF
        wrapped = sum_after_header > 0xFFFF or candidate > buffer_end
        expected_tail = (
            (entry_bytes - 2) & 0xFFFF
            if wrapped
            else (tail + entry_bytes) & 0xFFFF
        )
        expected_count = (byte_count - entry_bytes) & 0xFFFF
        expected_sequence = (sequence + 1) & 0xFFFF
        expected_index = (read_index + 1) & 0xFFFF
        expected_limit = read_limit
        if expected_index > read_limit:
            expected_index = 1
            expected_limit = 0xFFFF

        observed = {
            "tail": struct.unpack(
                "<H", machine.mem_read(data_segment * 16 + 0x0D90, 2)
            )[0],
            "byte_count": struct.unpack(
                "<H", machine.mem_read(data_segment * 16 + 0x0D9A, 2)
            )[0],
            "sequence": struct.unpack(
                "<H", machine.mem_read(data_segment * 16 + 0x131C, 2)
            )[0],
            "read_index": struct.unpack(
                "<H", machine.mem_read(data_segment * 16 + 0x0D60, 2)
            )[0],
            "read_limit": struct.unpack(
                "<H", machine.mem_read(data_segment * 16 + 0x0D64, 2)
            )[0],
        }
        expected = {
            "tail": expected_tail,
            "byte_count": expected_count,
            "sequence": expected_sequence,
            "read_index": expected_index,
            "read_limit": expected_limit,
        }
        if observed != expected:
            raise AssertionError(
                f"0xA3D0 {case['name']}: actual={observed}, expected={expected}"
            )
        for name in ("bx", "cx", "dx", "di", "bp"):
            actual_register = machine.reg_read(REGISTERS[name])
            if actual_register != initial[name]:
                raise AssertionError(f"0xA3D0 did not preserve {name}")

        vectors.append(
            {
                **case,
                "wrapped": wrapped,
                "result_tail": observed["tail"],
                "result_byte_count": observed["byte_count"],
                "result_sequence": observed["sequence"],
                "result_read_index": observed["read_index"],
                "result_read_limit": observed["read_limit"],
            }
        )

    return vectors


def queue_wrap_vectors() -> list[dict[str, object]]:
    data_segment = 0x2000
    cases = [
        {
            "name": "ordinary_advance",
            "cursor": 0x0100,
            "byte_count": 0x0020,
            "buffer_end": 0x1000,
            "head": 0x0555,
            "wrap_limit": 0xAAAA,
            "wrap_count": 0x0001,
        },
        {
            "name": "next_equals_end",
            "cursor": 0x0F00,
            "byte_count": 0x0100,
            "buffer_end": 0x1000,
            "head": 0x0666,
            "wrap_limit": 0xBBBB,
            "wrap_count": 0x0010,
        },
        {
            "name": "next_past_end",
            "cursor": 0x0F01,
            "byte_count": 0x0100,
            "buffer_end": 0x1000,
            "head": 0x0777,
            "wrap_limit": 0xCCCC,
            "wrap_count": 0x0100,
        },
        {
            "name": "cursor_add_carry",
            "cursor": 0xFF00,
            "byte_count": 0x0200,
            "buffer_end": 0xFFFF,
            "head": 0x0888,
            "wrap_limit": 0xDDDD,
            "wrap_count": 0x1000,
        },
        {
            "name": "zero_byte_count",
            "cursor": 0x1234,
            "byte_count": 0x0000,
            "buffer_end": 0xFFFF,
            "head": 0x0999,
            "wrap_limit": 0xEEEE,
            "wrap_count": 0xFFFE,
        },
        {
            "name": "one_byte_count",
            "cursor": 0x0000,
            "byte_count": 0x0001,
            "buffer_end": 0x0000,
            "head": 0x0AAA,
            "wrap_limit": 0x1111,
            "wrap_count": 0xFFFF,
        },
    ]
    vectors = []

    for case in cases:
        cursor = int(case["cursor"])
        byte_count = int(case["byte_count"])
        buffer_end = int(case["buffer_end"])
        head = int(case["head"])
        wrap_limit = int(case["wrap_limit"])
        wrap_count = int(case["wrap_count"])
        initial = {
            "ax": byte_count,
            "bx": 0x2222,
            "cx": 0x3333,
            "dx": 0x4444,
            "si": cursor,
            "di": 0x6666,
            "bp": 0x7777,
            "ds": data_segment,
            "es": 0x8888,
        }
        machine = execute(
            0xA38E,
            0xA3AC,
            initial,
            [
                (data_segment, 0x0D8C, struct.pack("<H", head)),
                (data_segment, 0x0D98, struct.pack("<H", wrap_limit)),
                (data_segment, 0x0DA0, struct.pack("<H", 0x5A5A)),
                (data_segment, 0x0D62, struct.pack("<H", wrap_count)),
                (data_segment, 0x5233, struct.pack("<H", buffer_end)),
            ],
        )

        full_next = cursor + byte_count
        next_cursor = full_next & 0xFFFF
        wrapped = full_next > 0xFFFF or next_cursor > buffer_end
        expected = {
            "head": 0 if wrapped else head,
            "wrap_limit": head if wrapped else wrap_limit,
            "iteration_count": (byte_count - 2) & 0xFFFF,
            "wrap_count": (wrap_count + 1) & 0xFFFF,
        }
        observed = {
            "head": struct.unpack(
                "<H", machine.mem_read(data_segment * 16 + 0x0D8C, 2)
            )[0],
            "wrap_limit": struct.unpack(
                "<H", machine.mem_read(data_segment * 16 + 0x0D98, 2)
            )[0],
            "iteration_count": struct.unpack(
                "<H", machine.mem_read(data_segment * 16 + 0x0DA0, 2)
            )[0],
            "wrap_count": struct.unpack(
                "<H", machine.mem_read(data_segment * 16 + 0x0D62, 2)
            )[0],
        }
        if observed != expected:
            raise AssertionError(
                f"0xA38E {case['name']}: actual={observed}, expected={expected}"
            )

        expected_registers = {
            "ax": (byte_count - 2) & 0xFFFF,
            "si": next_cursor,
            "cx": head if wrapped else initial["cx"],
        }
        for name, expected_register in expected_registers.items():
            actual_register = machine.reg_read(REGISTERS[name])
            if actual_register != expected_register:
                raise AssertionError(
                    f"0xA38E {case['name']} {name}: "
                    f"actual={actual_register:#x}, expected={expected_register:#x}"
                )
        for name in ("bx", "dx", "di", "bp"):
            actual_register = machine.reg_read(REGISTERS[name])
            if actual_register != initial[name]:
                raise AssertionError(f"0xA38E did not preserve {name}")

        carry = machine.reg_read(UC_X86_REG_EFLAGS) & 1
        expected_carry = int(byte_count < 2)
        if carry != expected_carry:
            raise AssertionError(
                f"0xA38E {case['name']} carry={carry}, expected={expected_carry}"
            )

        vectors.append(
            {
                **case,
                "wrapped": wrapped,
                "result_cursor": next_cursor,
                "result_head": observed["head"],
                "result_wrap_limit": observed["wrap_limit"],
                "result_iteration_count": observed["iteration_count"],
                "result_wrap_count": observed["wrap_count"],
                "result_carry": carry,
            }
        )

    return vectors


def queue_room_vectors() -> list[dict[str, object]]:
    data_segment = 0x2000
    cases = [
        {
            "name": "ordinary_room",
            "head": 0x0500,
            "tail": 0x0100,
            "byte_count": 0x0100,
            "wrap_limit": 0x0200,
            "request": 0x0020,
        },
        {
            "name": "gap_too_small",
            "head": 0x0100,
            "tail": 0x0120,
            "byte_count": 0x0010,
            "wrap_limit": 0x1000,
            "request": 0x0010,
        },
        {
            "name": "gap_exactly_large_enough",
            "head": 0x0100,
            "tail": 0x0122,
            "byte_count": 0x0010,
            "wrap_limit": 0x0040,
            "request": 0x0010,
        },
        {
            "name": "total_exceeds_wrap_limit",
            "head": 0x0500,
            "tail": 0x0100,
            "byte_count": 0x0100,
            "wrap_limit": 0x0129,
            "request": 0x0020,
        },
        {
            "name": "total_second_add_carry",
            "head": 0x0500,
            "tail": 0x0100,
            "byte_count": 0xFFF0,
            "wrap_limit": 0xFFFF,
            "request": 0x0010,
        },
        {
            "name": "count_plus_ten_carry_is_discarded",
            "head": 0x0500,
            "tail": 0x0100,
            "byte_count": 0xFFFC,
            "wrap_limit": 0x0007,
            "request": 0x0001,
        },
        {
            "name": "head_plus_request_carry_is_discarded",
            "head": 0xFFF0,
            "tail": 0xFFF5,
            "byte_count": 0x0000,
            "wrap_limit": 0x002A,
            "request": 0x0020,
        },
        {
            "name": "head_plus_padding_carry_is_discarded",
            "head": 0xFFF0,
            "tail": 0xFFF5,
            "byte_count": 0x0000,
            "wrap_limit": 0x000A,
            "request": 0x0000,
        },
    ]
    vectors = []

    for case in cases:
        head = int(case["head"])
        tail = int(case["tail"])
        byte_count = int(case["byte_count"])
        wrap_limit = int(case["wrap_limit"])
        request = int(case["request"])
        initial = {
            "ax": 0x1111,
            "bx": 0x2222,
            "cx": request,
            "dx": 0x4444,
            "si": 0x5555,
            "di": 0x6666,
            "bp": 0x7777,
            "ds": data_segment,
            "es": 0x8888,
        }
        globals_before = struct.pack("<HHHH", head, tail, byte_count, wrap_limit)
        machine = execute(
            0xA3AD,
            0xA3CF,
            initial,
            [
                (data_segment, 0x0D8C, struct.pack("<H", head)),
                (data_segment, 0x0D90, struct.pack("<H", tail)),
                (data_segment, 0x0D9A, struct.pack("<H", byte_count)),
                (data_segment, 0x0D98, struct.pack("<H", wrap_limit)),
            ],
        )

        gap_needed = ((head + request) & 0xFFFF) + 0x12
        gap_needed &= 0xFFFF
        early_failure = head < tail < gap_needed
        total_base = (byte_count + 0x0A) & 0xFFFF
        total_sum = total_base + request
        total_needed = total_sum & 0xFFFF
        total_carry = total_sum > 0xFFFF
        if early_failure:
            result_ax = gap_needed
            insufficient_room = True
        else:
            result_ax = total_needed
            insufficient_room = total_carry or wrap_limit < total_needed

        actual_ax = machine.reg_read(UC_X86_REG_AX)
        actual_bx = machine.reg_read(UC_X86_REG_BX)
        actual_carry = machine.reg_read(UC_X86_REG_EFLAGS) & 1
        if actual_ax != result_ax:
            raise AssertionError(
                f"0xA3AD {case['name']} AX={actual_ax:#x}, expected={result_ax:#x}"
            )
        if actual_bx != tail:
            raise AssertionError(
                f"0xA3AD {case['name']} BX={actual_bx:#x}, expected={tail:#x}"
            )
        if actual_carry != int(insufficient_room):
            raise AssertionError(
                f"0xA3AD {case['name']} carry={actual_carry}, "
                f"expected={int(insufficient_room)}"
            )
        for name in ("cx", "dx", "si", "di", "bp"):
            actual_register = machine.reg_read(REGISTERS[name])
            if actual_register != initial[name]:
                raise AssertionError(f"0xA3AD did not preserve {name}")

        globals_after = struct.pack(
            "<HHHH",
            struct.unpack(
                "<H", machine.mem_read(data_segment * 16 + 0x0D8C, 2)
            )[0],
            struct.unpack(
                "<H", machine.mem_read(data_segment * 16 + 0x0D90, 2)
            )[0],
            struct.unpack(
                "<H", machine.mem_read(data_segment * 16 + 0x0D9A, 2)
            )[0],
            struct.unpack(
                "<H", machine.mem_read(data_segment * 16 + 0x0D98, 2)
            )[0],
        )
        if globals_after != globals_before:
            raise AssertionError(f"0xA3AD {case['name']} changed queue globals")

        vectors.append(
            {
                **case,
                "early_gap_failure": early_failure,
                "result_needed": result_ax,
                "has_room": not insufficient_room,
                "result_carry": actual_carry,
            }
        )

    return vectors


def queue_state_le_one_vectors() -> list[dict[str, object]]:
    data_segment = 0x2000
    zero_flag_states = []

    for state in range(0x100):
        initial = {
            "ax": 0x1111,
            "bx": 0x2222,
            "cx": 0x3333,
            "dx": 0x4444,
            "si": 0x5555,
            "di": 0x6666,
            "bp": 0x7777,
            "ds": 0x2400,
            "es": 0x2800,
            "gs": data_segment,
        }
        machine = execute(
            0xA40B,
            0xA419,
            initial,
            [(data_segment, 0x0D5F, bytes([state]))],
        )

        zero_flag = (machine.reg_read(UC_X86_REG_EFLAGS) >> 6) & 1
        expected = int(state <= 1)
        if zero_flag != expected:
            raise AssertionError(
                f"0xA40B state={state:#x} ZF={zero_flag}, expected={expected}"
            )
        if zero_flag:
            zero_flag_states.append(state)
        for name, value in initial.items():
            if name in REGISTERS:
                actual_register = machine.reg_read(REGISTERS[name])
                if actual_register != value:
                    raise AssertionError(f"0xA40B did not preserve {name}")
        actual_state = machine.mem_read(data_segment * 16 + 0x0D5F, 1)[0]
        if actual_state != state:
            raise AssertionError(f"0xA40B changed state byte {state:#x}")

    return [
        {
            "tested_state_count": 0x100,
            "zero_flag_set_states": zero_flag_states,
            "zero_flag_clear_range": [2, 0xFF],
            "logical_result": "state <= 1",
        }
    ]


def flag_test_b17_vectors() -> list[dict[str, object]]:
    data_segment = 0x2000
    state_segment = 0x2600

    for state in range(0x100):
        initial = {
            "ax": 0x1111,
            "bx": 0x2222,
            "cx": 0x3333,
            "dx": 0x4444,
            "si": 0x5555,
            "di": 0x6666,
            "bp": 0x7777,
            "ds": data_segment,
            "es": 0x2800,
            "gs": state_segment,
            "flags": 0x0E93,
        }
        machine = execute(
            0xA634,
            0xA641,
            initial,
            [
                (data_segment, 0x0B17, bytes([state ^ 1])),
                (state_segment, 0x0B17, bytes([state])),
            ],
        )

        flags = machine.reg_read(UC_X86_REG_EFLAGS)
        zero_flag = (flags >> 6) & 1
        parity_flag = (flags >> 2) & 1
        expected_zero = int((state & 1) == 0)
        if zero_flag != expected_zero or parity_flag != expected_zero:
            raise AssertionError(
                f"0xA634 state={state:#x} ZF/PF={zero_flag}/{parity_flag}, "
                f"expected={expected_zero}"
            )
        if flags & ((1 << 0) | (1 << 7) | (1 << 11)):
            raise AssertionError(f"0xA634 state={state:#x} did not clear CF/SF/OF")
        if flags & ((1 << 9) | (1 << 10)) != initial["flags"] & (
            (1 << 9) | (1 << 10)
        ):
            raise AssertionError(f"0xA634 state={state:#x} changed IF/DF")
        for name in (
            "ax",
            "bx",
            "cx",
            "dx",
            "si",
            "di",
            "bp",
            "ds",
            "es",
            "gs",
        ):
            actual_register = machine.reg_read(REGISTERS[name])
            if actual_register != initial[name]:
                raise AssertionError(f"0xA634 did not preserve {name}")
        actual_state = machine.mem_read(state_segment * 16 + 0x0B17, 1)[0]
        if actual_state != state:
            raise AssertionError(f"0xA634 changed state byte {state:#x}")

    return [
        {
            "tested_state_count": 0x100,
            "zero_flag_set_rule": "(state & 1) == 0",
            "zero_flag_clear_rule": "(state & 1) != 0",
            "logical_result": "(state & 1) != 0",
            "caller_branch": "JE skips the enabled-only store when the result is false",
        }
    ]


def queue_enqueue_vectors() -> list[dict[str, object]]:
    data_segment = 0x2000
    cases = [
        ("zero", 0x0000, 0x0000, 0x0000),
        ("ordinary", 0x0100, 0x0200, 0x0020),
        ("head_wrap", 0xFFFF, 0x0000, 0x0001),
        ("count_wrap", 0x0000, 0xFFFF, 0x0001),
        ("both_wrap_to_zero", 0x8000, 0x8000, 0x8000),
        ("maximum_increment", 0x1234, 0xFFFF, 0xFFFF),
        ("both_adds_carry", 0xFFF0, 0xFFF8, 0x0020),
        ("high_bit_increment", 0xAAAA, 0x5555, 0x8001),
    ]
    vectors = []

    for name, head, byte_count, increment in cases:
        initial = {
            "ax": increment,
            "bx": 0x2222,
            "cx": 0x3333,
            "dx": 0x4444,
            "si": 0x5555,
            "di": 0x6666,
            "bp": 0x7777,
            "ds": data_segment,
            "es": 0x2800,
            "gs": 0x2C00,
        }
        machine = execute(
            0xA734,
            0xA73D,
            initial,
            [
                (data_segment, 0x0D8C, struct.pack("<H", head)),
                (data_segment, 0x0D9A, struct.pack("<H", byte_count)),
            ],
        )

        result_head = struct.unpack(
            "<H", machine.mem_read(data_segment * 16 + 0x0D8C, 2)
        )[0]
        result_byte_count = struct.unpack(
            "<H", machine.mem_read(data_segment * 16 + 0x0D9A, 2)
        )[0]
        expected_head = (head + increment) & 0xFFFF
        expected_byte_count = (byte_count + increment) & 0xFFFF
        if result_head != expected_head or result_byte_count != expected_byte_count:
            raise AssertionError(
                f"0xA734 {name}: head={result_head:#x}/{expected_head:#x}, "
                f"count={result_byte_count:#x}/{expected_byte_count:#x}"
            )
        for register, value in initial.items():
            if register in REGISTERS:
                actual_register = machine.reg_read(REGISTERS[register])
                if actual_register != value:
                    raise AssertionError(f"0xA734 did not preserve {register}")
        carry = machine.reg_read(UC_X86_REG_EFLAGS) & 1
        if carry != 0:
            raise AssertionError(f"0xA734 {name} did not clear carry")

        vectors.append(
            {
                "name": name,
                "head": head,
                "byte_count": byte_count,
                "increment": increment,
                "result_head": result_head,
                "result_byte_count": result_byte_count,
                "result_carry": carry,
            }
        )

    return vectors


def list_read_vectors() -> list[dict[str, object]]:
    data_segment = 0x2000
    buffer_segment = 0x3000
    cases = [
        {
            "name": "no_file_handle",
            "handle": 0,
            "head": 0x0100,
            "byte_count": 0x0020,
            "source_offset": 0x12345678,
            "source_remaining": 0x01020304,
            "reads": [],
        },
        {
            "name": "zero_extent",
            "handle": 1,
            "head": 0x0000,
            "byte_count": 0x0000,
            "source_offset": 0,
            "source_remaining": 2,
            "reads": [(0x0000, 2, False)],
        },
        {
            "name": "ordinary_extent",
            "handle": 5,
            "head": 0x0120,
            "byte_count": 0x0040,
            "source_offset": 0x00123456,
            "source_remaining": 0x00010000,
            "reads": [(0x1234, 2, False)],
        },
        {
            "name": "head_and_count_wrap",
            "handle": 0x7FFF,
            "head": 0xFFFF,
            "byte_count": 0xFFFF,
            "source_offset": 0xFFFFFFFF,
            "source_remaining": 1,
            "reads": [(0xBEEF, 2, False)],
        },
        {
            "name": "short_read_retry",
            "handle": 9,
            "head": 0x2200,
            "byte_count": 0x3333,
            "source_offset": 0x0000FFFF,
            "source_remaining": 0x00010001,
            "reads": [(0x00A5, 1, False), (0xCAFE, 2, False)],
        },
        {
            "name": "carry_set_short_retry",
            "handle": 0x1234,
            "head": 0x3456,
            "byte_count": 0xFFFE,
            "source_offset": 0xFFFF0000,
            "source_remaining": 0x00000000,
            "reads": [(0x0005, 1, True), (0x55AA, 2, False)],
        },
    ]
    vectors = []

    def read_u16(machine: Uc, offset: int) -> int:
        return struct.unpack(
            "<H", machine.mem_read(data_segment * 16 + offset, 2)
        )[0]

    def read_u32(machine: Uc, offset: int) -> int:
        return struct.unpack(
            "<I", machine.mem_read(data_segment * 16 + offset, 4)
        )[0]

    def set_carry(machine: Uc, carry: bool) -> None:
        flags = machine.reg_read(UC_X86_REG_EFLAGS)
        machine.reg_write(
            UC_X86_REG_EFLAGS, flags | 1 if carry else flags & ~1
        )

    for case_index, case in enumerate(cases):
        name = str(case["name"])
        handle = int(case["handle"])
        head = int(case["head"])
        byte_count = int(case["byte_count"])
        source_offset = int(case["source_offset"])
        source_remaining = int(case["source_remaining"])
        reads = list(case["reads"])
        calls: list[dict[str, int | bool | str]] = []
        read_index = 0
        initial_eax = 0xA5A50000 | (0x1000 + case_index)
        initial = {
            "eax": initial_eax,
            "bx": 0x2222,
            "cx": 0x3333,
            "dx": 0x4444,
            "si": 0x5555,
            "di": 0x6666,
            "bp": 0x7777,
            "sp": 0xFF00,
            "ds": data_segment,
            "es": 0x4000,
            "gs": data_segment,
            "flags": 0x0ED7,
        }

        def interrupt_handler(
            machine: Uc,
            number: int,
            case_name: str = name,
            call_log: list[dict[str, int | bool | str]] = calls,
            responses: list[object] = reads,
        ) -> None:
            nonlocal read_index
            if number != 0x21:
                raise AssertionError(
                    f"0xA622 {case_name} invoked unexpected INT {number:#x}"
                )

            function = machine.reg_read(UC_X86_REG_AX) >> 8
            if function == 0x42:
                call_log.append(
                    {
                        "call": "seek",
                        "handle": machine.reg_read(UC_X86_REG_BX),
                        "offset_high": machine.reg_read(UC_X86_REG_CX),
                        "offset_low": machine.reg_read(UC_X86_REG_DX),
                    }
                )
                machine.reg_write(UC_X86_REG_AX, source_offset & 0xFFFF)
                machine.reg_write(UC_X86_REG_DX, source_offset >> 16)
                set_carry(machine, False)
                return

            if function != 0x3F or read_index >= len(responses):
                raise AssertionError(
                    f"0xA622 {case_name} invoked unexpected DOS function "
                    f"{function:#x}"
                )

            value, returned, failed = responses[read_index]
            read_index += 1
            destination_segment = machine.reg_read(UC_X86_REG_DS)
            destination_offset = machine.reg_read(UC_X86_REG_DX)
            call_log.append(
                {
                    "call": "read",
                    "handle": machine.reg_read(UC_X86_REG_BX),
                    "requested": machine.reg_read(UC_X86_REG_CX),
                    "destination_segment": destination_segment,
                    "destination_offset": destination_offset,
                    "returned": int(returned),
                    "carry": bool(failed),
                }
            )
            machine.mem_write(
                destination_segment * 16 + destination_offset,
                struct.pack("<H", int(value))[: int(returned)],
            )
            machine.reg_write(UC_X86_REG_AX, int(returned))
            set_carry(machine, bool(failed))

        machine = execute(
            0xA622,
            0xA633,
            initial,
            [
                (data_segment, 0x0D5B, struct.pack("<H", handle)),
                (data_segment, 0x0D84, struct.pack("<I", source_offset)),
                (data_segment, 0x0D88, struct.pack("<I", source_remaining)),
                (
                    data_segment,
                    0x0D8C,
                    struct.pack("<HH", head, buffer_segment),
                ),
                (data_segment, 0x0D9A, struct.pack("<H", byte_count)),
                (data_segment, 0x0DBC, b"\x00"),
            ],
            interrupt_handler,
        )

        success = handle >= 1
        expected_extent = int(reads[-1][0]) if success else initial_eax & 0xFFFF
        expected_head = (head + 2) & 0xFFFF if success else head
        expected_byte_count = (
            (byte_count + 2) & 0xFFFF if success else byte_count
        )
        expected_source_offset = (
            (source_offset + 2) & 0xFFFFFFFF if success else source_offset
        )
        expected_source_remaining = (
            (source_remaining - 2) & 0xFFFFFFFF if success else source_remaining
        )
        expected_registers = {
            "eax": (initial_eax & 0xFFFF0000) | expected_extent,
            "bx": handle,
            "cx": 2,
            "dx": head if success else initial["dx"],
            "si": expected_head if success else initial["si"],
            "di": initial["di"],
            "bp": initial["bp"],
            "sp": initial["sp"],
            "ds": data_segment,
            "es": buffer_segment if success else initial["es"],
            "gs": data_segment,
        }
        for register, expected in expected_registers.items():
            actual = machine.reg_read(REGISTERS[register])
            if actual != expected:
                raise AssertionError(
                    f"0xA622 {name} {register}={actual:#x}, "
                    f"expected={expected:#x}"
                )

        observed = {
            "head": read_u16(machine, 0x0D8C),
            "byte_count": read_u16(machine, 0x0D9A),
            "source_offset": read_u32(machine, 0x0D84),
            "source_remaining": read_u32(machine, 0x0D88),
        }
        expected_state = {
            "head": expected_head,
            "byte_count": expected_byte_count,
            "source_offset": expected_source_offset,
            "source_remaining": expected_source_remaining,
        }
        if observed != expected_state:
            raise AssertionError(
                f"0xA622 {name} state={observed}, expected={expected_state}"
            )
        carry = machine.reg_read(UC_X86_REG_EFLAGS) & 1
        if carry != int(not success):
            raise AssertionError(
                f"0xA622 {name} carry={carry}, expected={int(not success)}"
            )
        if read_index != len(reads):
            raise AssertionError(f"0xA622 {name} did not consume all read responses")
        if len(calls) != len(reads) * 2:
            raise AssertionError(f"0xA622 {name} produced an unexpected call count")
        for call in calls:
            if call["handle"] != handle:
                raise AssertionError(f"0xA622 {name} used an unexpected file handle")
            if call["call"] == "seek" and (
                call["offset_high"] != source_offset >> 16
                or call["offset_low"] != source_offset & 0xFFFF
            ):
                raise AssertionError(f"0xA622 {name} sought to an unexpected offset")
            if call["call"] == "read" and (
                call["requested"] != 2
                or call["destination_segment"] != buffer_segment
                or call["destination_offset"] != head
            ):
                raise AssertionError(f"0xA622 {name} used an unexpected read request")

        vectors.append(
            {
                "name": name,
                "success": success,
                "handle": handle,
                "initial_head": head,
                "initial_byte_count": byte_count,
                "extent": expected_extent if success else None,
                "calls": calls,
                "result": observed,
                "result_cursor_segment": machine.reg_read(UC_X86_REG_ES),
                "result_cursor_offset": machine.reg_read(UC_X86_REG_SI),
                "result_carry": carry,
            }
        )

    return vectors


def list_d8c_activate_ready_vectors() -> list[dict[str, object]]:
    data_segment = 0x2000
    buffer_segment = 0x3000
    cases = [
        {
            "name": "already_active",
            "active_segment": 0x3456,
            "queued": 0,
            "extent": 0x1111,
            "marker": 0x2222,
            "tail": 0x0100,
            "flags": 0,
            "activate": False,
            "ready": True,
        },
        {
            "name": "empty_queue",
            "active_segment": 0,
            "queued": 0,
            "extent": 0x1111,
            "marker": 0x2222,
            "tail": 0x0200,
            "flags": 0,
            "activate": False,
            "ready": False,
        },
        {
            "name": "ordinary_incomplete",
            "active_segment": 0,
            "queued": 19,
            "extent": 20,
            "marker": 0x1234,
            "tail": 0x0300,
            "flags": 0,
            "activate": False,
            "ready": False,
        },
        {
            "name": "ordinary_exact_default_storage",
            "active_segment": 0,
            "queued": 20,
            "extent": 20,
            "marker": 0x1234,
            "tail": 0x0400,
            "flags": 0,
            "activate": True,
            "ready": True,
        },
        {
            "name": "ordinary_extra_alternate_storage",
            "active_segment": 0,
            "queued": 21,
            "extent": 20,
            "marker": 0x4321,
            "tail": 0x0500,
            "flags": 0x40,
            "activate": True,
            "ready": True,
        },
        {
            "name": "link_marker_bypasses_extent_check",
            "active_segment": 0,
            "queued": 2,
            "extent": 0x1000,
            "marker": 0x6D6D,
            "tail": 0x0600,
            "flags": 0x40,
            "activate": True,
            "ready": True,
        },
        {
            "name": "tail_offset_wrap",
            "active_segment": 0,
            "queued": 8,
            "extent": 8,
            "marker": 0x6D6D,
            "tail": 0xFFFE,
            "flags": 0,
            "activate": True,
            "ready": True,
        },
    ]
    vectors = []
    default_segment = 0x4567
    alternate_segment = 0x5678

    for case_index, case in enumerate(cases):
        name = str(case["name"])
        active_segment = int(case["active_segment"])
        queued = int(case["queued"])
        extent = int(case["extent"])
        marker = int(case["marker"])
        tail = int(case["tail"])
        resource_flags = int(case["flags"])
        should_activate = bool(case["activate"])
        ready = bool(case["ready"])
        calls: list[dict[str, int | str]] = []
        initial = {
            "eax": 0xA5A50000 | case_index,
            "bx": 0x2222,
            "cx": 0x3333,
            "dx": 0x4444,
            "si": 0x5555,
            "di": 0x6666,
            "bp": 0x7777,
            "sp": 0xFF00,
            "ds": data_segment,
            "es": 0x4000,
            "gs": data_segment,
            "flags": 0x0202,
        }
        buffer = bytearray([0xCC]) * 0x10000
        struct.pack_into("<H", buffer, tail, extent)
        struct.pack_into("<H", buffer, (tail + 2) & 0xFFFF, marker)

        def code_handler(
            machine: Uc,
            address: int,
            _size: int,
            call_log: list[dict[str, int | str]] = calls,
        ) -> None:
            if address == 0xA23A:
                call_log.append(
                    {
                        "call": "activate",
                        "extent": machine.reg_read(UC_X86_REG_AX),
                        "entry_segment": machine.reg_read(UC_X86_REG_ES),
                        "entry_offset": machine.reg_read(UC_X86_REG_SI),
                        "storage_segment": machine.reg_read(UC_X86_REG_BP),
                    }
                )

        machine = execute(
            0xA20C,
            0xA23F,
            initial,
            [
                (0, 0xA23A, b"\x90" * 3),
                (data_segment, 0x0ABE, struct.pack("<H", default_segment)),
                (data_segment, 0x0D76, struct.pack("<H", resource_flags)),
                (data_segment, 0x0D90, struct.pack("<HH", tail, buffer_segment)),
                (data_segment, 0x0D94, struct.pack("<H", 0x2468)),
                (data_segment, 0x0D96, struct.pack("<H", active_segment)),
                (data_segment, 0x0D9A, struct.pack("<H", queued)),
                (data_segment, 0x0DA8, struct.pack("<H", alternate_segment)),
                (buffer_segment, 0, bytes(buffer)),
            ],
            code_handler=code_handler,
        )

        expected_calls = []
        if should_activate:
            expected_calls.append(
                {
                    "call": "activate",
                    "extent": extent,
                    "entry_segment": buffer_segment,
                    "entry_offset": (tail + 2) & 0xFFFF,
                    "storage_segment": (
                        alternate_segment
                        if resource_flags & 0x40
                        else default_segment
                    ),
                }
            )
        if calls != expected_calls:
            raise AssertionError(
                f"0xA20C {name} calls={calls}, expected={expected_calls}"
            )

        carry = machine.reg_read(UC_X86_REG_EFLAGS) & 1
        if carry != int(not ready):
            raise AssertionError(
                f"0xA20C {name} carry={carry}, expected={int(not ready)}"
            )
        expected_ax = initial["eax"]
        expected_cx = initial["cx"]
        expected_es = initial["es"]
        expected_si = initial["si"]
        expected_bp = initial["bp"]
        if active_segment == 0:
            expected_cx = queued
            if queued != 0:
                expected_ax = (initial["eax"] & 0xFFFF0000) | (
                    0 if should_activate else extent
                )
                expected_es = buffer_segment
                expected_si = (tail + 2) & 0xFFFF
                if should_activate:
                    expected_bp = expected_calls[0]["storage_segment"]
        expected_registers = {
            "eax": expected_ax,
            "bx": initial["bx"],
            "cx": expected_cx,
            "dx": initial["dx"],
            "si": expected_si,
            "di": initial["di"],
            "bp": expected_bp,
            "sp": initial["sp"],
            "ds": initial["ds"],
            "es": expected_es,
            "gs": initial["gs"],
        }
        for register, expected in expected_registers.items():
            actual = machine.reg_read(REGISTERS[register])
            if actual != expected:
                raise AssertionError(
                    f"0xA20C {name} {register}={actual:#x}, "
                    f"expected={expected:#x}"
                )

        if machine.mem_read(
            buffer_segment * 16, 0x10000
        ) != bytes(buffer):
            raise AssertionError(f"0xA20C {name} modified the queue buffer")

        vectors.append(
            {
                "name": name,
                "ready": ready,
                "active_segment": active_segment,
                "queued_bytes": queued,
                "entry_extent": extent,
                "entry_marker": marker,
                "resource_flags": resource_flags,
                "calls": calls,
                "result_carry": carry,
                "result_ax": machine.reg_read(UC_X86_REG_AX),
            }
        )

    return vectors


def list_d8c_advance_due_vectors() -> list[dict[str, object]]:
    data_segment = 0x2000
    decoy_segment = 0x3000
    callback_segment = 0xF000
    callback_address = callback_segment * 16
    cases = [
        {
            "name": "audio_phase_below_threshold",
            "mode_27e0": 0x81,
            "mode_27e1": 0x01,
            "audio_enabled": 0x03,
            "callback": 0x3000,
            "phase_last": 0x0C69,
            "tick": 0x1111,
            "tick_last": 0x2222,
            "threshold": 0x55,
            "due": False,
        },
        {
            "name": "audio_phase_exact_threshold",
            "mode_27e0": 0x01,
            "mode_27e1": 0x81,
            "audio_enabled": 0x01,
            "callback": 0x3000,
            "phase_last": 0x0C68,
            "tick": 0x3333,
            "tick_last": 0x4444,
            "threshold": 0x66,
            "due": True,
        },
        {
            "name": "audio_negative_delta_corrected_below",
            "mode_27e0": 0x03,
            "mode_27e1": 0x05,
            "audio_enabled": 0x81,
            "callback": 0x3E00,
            "phase_last": 0x3E69,
            "tick": 0x5555,
            "tick_last": 0x6666,
            "threshold": 0x77,
            "due": False,
        },
        {
            "name": "audio_negative_delta_corrected_exact",
            "mode_27e0": 0x01,
            "mode_27e1": 0x03,
            "audio_enabled": 0x05,
            "callback": 0x3E00,
            "phase_last": 0x3E68,
            "tick": 0x7777,
            "tick_last": 0x8888,
            "threshold": 0x88,
            "due": True,
        },
        {
            "name": "audio_phase_wrap",
            "mode_27e0": 0xFF,
            "mode_27e1": 0xFF,
            "audio_enabled": 0xFF,
            "callback": 0x4010,
            "phase_last": 0xFC58,
            "tick": 0x9999,
            "tick_last": 0xAAAA,
            "threshold": 0x99,
            "due": True,
        },
        {
            "name": "mode_27e0_fallback_below",
            "mode_27e0": 0xFE,
            "mode_27e1": 0x01,
            "audio_enabled": 0x01,
            "callback": 0x1357,
            "phase_last": 0x2468,
            "tick": 0x0104,
            "tick_last": 0x0100,
            "threshold": 5,
            "due": False,
        },
        {
            "name": "mode_27e1_fallback_exact_and_reread",
            "mode_27e0": 0x01,
            "mode_27e1": 0x02,
            "audio_enabled": 0x01,
            "callback": 0x2468,
            "phase_last": 0x3579,
            "tick": 0x0105,
            "tick_last": 0x0100,
            "threshold": 5,
            "reread_tick": 0x0107,
            "due": True,
        },
        {
            "name": "audio_disabled_fallback_high_byte",
            "mode_27e0": 0x01,
            "mode_27e1": 0x01,
            "audio_enabled": 0x80,
            "callback": 0x3579,
            "phase_last": 0x468A,
            "tick": 0x0200,
            "tick_last": 0x0100,
            "threshold": 0xFF,
            "due": True,
        },
        {
            "name": "software_negative_delta_below",
            "mode_27e0": 0,
            "mode_27e1": 0,
            "audio_enabled": 0,
            "callback": 0x468A,
            "phase_last": 0x579B,
            "tick": 0x1000,
            "tick_last": 0x1005,
            "threshold": 6,
            "due": False,
        },
        {
            "name": "software_negative_delta_exact",
            "mode_27e0": 0,
            "mode_27e1": 1,
            "audio_enabled": 1,
            "callback": 0x579B,
            "phase_last": 0x68AC,
            "tick": 0x1000,
            "tick_last": 0x1005,
            "threshold": 5,
            "due": True,
        },
        {
            "name": "software_half_range_delta",
            "mode_27e0": 1,
            "mode_27e1": 0,
            "audio_enabled": 1,
            "callback": 0x68AC,
            "phase_last": 0x79BD,
            "tick": 0,
            "tick_last": 0x8000,
            "threshold": 0xFF,
            "due": True,
        },
        {
            "name": "software_zero_threshold",
            "mode_27e0": 1,
            "mode_27e1": 1,
            "audio_enabled": 0,
            "callback": 0x79BD,
            "phase_last": 0x8ACE,
            "tick": 0x1234,
            "tick_last": 0x1234,
            "threshold": 0,
            "due": True,
        },
    ]
    vectors = []

    for case_index, case in enumerate(cases):
        name = str(case["name"])
        mode_27e0 = int(case["mode_27e0"])
        mode_27e1 = int(case["mode_27e1"])
        audio_enabled = int(case["audio_enabled"])
        callback_value = int(case["callback"])
        phase_last = int(case["phase_last"])
        tick = int(case["tick"])
        tick_last = int(case["tick_last"])
        threshold = int(case["threshold"])
        due = bool(case["due"])
        reread_tick = int(case.get("reread_tick", tick))
        audio_clock = bool(
            mode_27e0 & 1 and mode_27e1 & 1 and audio_enabled & 1
        )

        data = bytearray([0xCC]) * 0x2800
        struct.pack_into("<B", data, 0x0ADE, audio_enabled)
        struct.pack_into("<H", data, 0x0B29, tick)
        struct.pack_into("<H", data, 0x0C41, phase_last)
        struct.pack_into("<HH", data, 0x0CF3, 0, callback_segment)
        struct.pack_into("<B", data, 0x0D77, threshold)
        struct.pack_into("<H", data, 0x0DA2, tick_last)
        struct.pack_into("<B", data, 0x27E0, mode_27e0)
        struct.pack_into("<B", data, 0x27E1, mode_27e1)
        expected_data = bytearray(data)
        callback_calls: list[int] = []
        tick_rereads: list[int] = []

        if audio_clock:
            current = (0x4000 - callback_value) & 0xFFFF
            delta = (current - phase_last) & 0xFFFF
            if delta & 0x8000:
                delta = (delta + 0x4000) & 0xFFFF
            expected_due = delta >= 0x0398
            result_ax = current
            if expected_due:
                struct.pack_into("<H", expected_data, 0x0C41, current)
        else:
            delta = (tick - tick_last) & 0xFFFF
            if delta & 0x8000:
                delta = (-delta) & 0xFFFF
            expected_due = (delta & 0xFF00) != 0 or (delta & 0xFF) >= threshold
            result_ax = delta
            if expected_due:
                result_ax = reread_tick
                struct.pack_into("<H", expected_data, 0x0B29, reread_tick)
                struct.pack_into("<H", expected_data, 0x0DA2, reread_tick)

        if expected_due != due:
            raise AssertionError(
                f"0xA240 {name} malformed test vector: "
                f"model due={expected_due}, declared={due}"
            )

        initial = {
            "eax": 0xA5A50000 | case_index,
            "bx": 0x2222,
            "cx": 0x3333,
            "dx": 0x4444,
            "si": 0x5555,
            "di": 0x6666,
            "bp": 0x7777,
            "sp": 0xFF00,
            "ds": data_segment,
            "es": 0x4000,
            "gs": decoy_segment,
            "flags": 0x0202,
        }

        def code_handler(machine: Uc, address: int, _size: int) -> None:
            if address == callback_address:
                callback_calls.append(address)
            if address == 0xA289:
                tick_rereads.append(address)
                machine.mem_write(
                    data_segment * 16 + 0x0B29,
                    struct.pack("<H", reread_tick),
                )

        machine = execute(
            0xA240,
            0xA290,
            initial,
            [
                (data_segment, 0, bytes(data)),
                (decoy_segment, 0, bytes(data)),
                (
                    callback_segment,
                    0,
                    b"\xB8" + struct.pack("<H", callback_value) + b"\xCB",
                ),
            ],
            code_handler=code_handler,
        )

        expected_callback_count = int(audio_clock)
        if len(callback_calls) != expected_callback_count:
            raise AssertionError(
                f"0xA240 {name} callback count={len(callback_calls)}, "
                f"expected={expected_callback_count}"
            )
        expected_rereads = int(not audio_clock and due)
        if len(tick_rereads) != expected_rereads:
            raise AssertionError(
                f"0xA240 {name} tick rereads={len(tick_rereads)}, "
                f"expected={expected_rereads}"
            )

        carry = machine.reg_read(UC_X86_REG_EFLAGS) & 1
        if carry != int(not due):
            raise AssertionError(
                f"0xA240 {name} carry={carry}, expected={int(not due)}"
            )

        expected_registers = {
            "eax": (initial["eax"] & 0xFFFF0000) | result_ax,
            "bx": initial["bx"],
            "cx": initial["cx"],
            "dx": initial["dx"],
            "si": initial["si"],
            "di": initial["di"],
            "bp": initial["bp"],
            "sp": initial["sp"],
            "ds": initial["ds"],
            "es": initial["es"],
            "gs": initial["gs"],
        }
        for register, expected in expected_registers.items():
            actual = machine.reg_read(REGISTERS[register])
            if actual != expected:
                raise AssertionError(
                    f"0xA240 {name} {register}={actual:#x}, expected={expected:#x}"
                )

        actual_data = bytes(machine.mem_read(data_segment * 16, len(expected_data)))
        if actual_data != bytes(expected_data):
            raise AssertionError(f"0xA240 {name} modified unexpected game data")
        actual_decoy = bytes(machine.mem_read(decoy_segment * 16, len(data)))
        if actual_decoy != bytes(data):
            raise AssertionError(f"0xA240 {name} modified GS decoy data")

        vectors.append(
            {
                "name": name,
                "audio_clock": audio_clock,
                "due": due,
                "mode_27e0": mode_27e0,
                "mode_27e1": mode_27e1,
                "audio_enabled": audio_enabled,
                "callback_value": callback_value,
                "previous_phase": phase_last,
                "tick": tick,
                "previous_tick": tick_last,
                "threshold": threshold,
                "reread_tick": reread_tick,
                "normalized_delta": delta,
                "result_ax": result_ax,
                "result_carry": carry,
            }
        )

    return vectors


def list_d8c_palette_blocks_apply_vectors() -> list[dict[str, object]]:
    data_segment = 0x2000
    decoy_segment = 0x3000
    cases = [
        ("buffer_start", 0x0123, 0x4000, 0x0000),
        ("ordinary_payload", 0x4567, 0x5000, 0x2345),
        ("last_byte", 0x89AB, 0x6000, 0xFFFF),
        ("zero_head_offset", 0x0000, 0x7000, 0x0100),
    ]
    vectors = []

    for case_index, (name, head_offset, buffer_segment, payload_offset) in enumerate(
        cases
    ):
        data = bytearray([0xCC]) * 0x1000
        struct.pack_into(
            "<HH", data, 0x0D8C, head_offset, buffer_segment
        )
        struct.pack_into("<H", data, 0x0D9E, payload_offset)
        calls: list[dict[str, int | str]] = []
        initial = {
            "eax": 0xA5A50000 | case_index,
            "bx": 0x2222,
            "cx": 0x3333,
            "dx": 0x4444,
            "si": 0x5555,
            "di": 0x6666,
            "bp": 0x7777,
            "sp": 0xFF00,
            "ds": data_segment,
            "es": 0x8000,
            "gs": decoy_segment,
            "flags": 0x0A93,
        }

        def code_handler(machine: Uc, address: int, _size: int) -> None:
            if address == 0xA780:
                calls.append(
                    {
                        "call": "resource_palette_blocks_apply",
                        "stream_segment": machine.reg_read(UC_X86_REG_ES),
                        "stream_offset": machine.reg_read(UC_X86_REG_SI),
                    }
                )

        machine = execute(
            0xA778,
            0xA783,
            initial,
            [
                (0, 0xA780, b"\x90" * 3),
                (data_segment, 0, bytes(data)),
                (decoy_segment, 0, bytes(data)),
            ],
            code_handler=code_handler,
        )

        expected_calls = [
            {
                "call": "resource_palette_blocks_apply",
                "stream_segment": buffer_segment,
                "stream_offset": payload_offset,
            }
        ]
        if calls != expected_calls:
            raise AssertionError(
                f"0xA778 {name} calls={calls}, expected={expected_calls}"
            )

        expected_registers = {
            "eax": initial["eax"],
            "bx": initial["bx"],
            "cx": initial["cx"],
            "dx": initial["dx"],
            "si": payload_offset,
            "di": initial["di"],
            "bp": initial["bp"],
            "sp": initial["sp"],
            "ds": initial["ds"],
            "es": buffer_segment,
            "gs": initial["gs"],
        }
        for register, expected in expected_registers.items():
            actual = machine.reg_read(REGISTERS[register])
            if actual != expected:
                raise AssertionError(
                    f"0xA778 {name} {register}={actual:#x}, expected={expected:#x}"
                )

        if machine.reg_read(UC_X86_REG_EFLAGS) != initial["flags"]:
            raise AssertionError(f"0xA778 {name} did not preserve flags")
        if bytes(machine.mem_read(data_segment * 16, len(data))) != bytes(data):
            raise AssertionError(f"0xA778 {name} modified game data")
        if bytes(machine.mem_read(decoy_segment * 16, len(data))) != bytes(data):
            raise AssertionError(f"0xA778 {name} modified GS decoy data")

        vectors.append(
            {
                "name": name,
                "head_offset_ignored": head_offset,
                "buffer_segment": buffer_segment,
                "payload_offset": payload_offset,
                "calls": calls,
                "result_es": machine.reg_read(UC_X86_REG_ES),
                "result_si": machine.reg_read(UC_X86_REG_SI),
            }
        )

    return vectors


def gfx_scanline_advance_vectors() -> list[dict[str, object]]:
    stack_segment = 0x9000
    frame_offset = 0xF000
    call_sp = 0xEFC0
    stack_base = call_sp
    cases = [
        ("ordinary_continue", 2, 0x1234, 0x0040, True),
        ("last_row_exits_decoder", 1, 0x5678, 0x0080, False),
        ("zero_rows_wraps_to_255", 0, 0x9ABC, 0x00C0, True),
        ("row_offset_wrap", 3, 0xFF00, 0x0100, True),
        ("high_row_byte_preserved", 0xAB02, 0x2468, 0x013F, True),
    ]
    vectors = []

    for case_index, (name, rows_word, row_offset, row_width, continues) in enumerate(
        cases
    ):
        stack = bytearray([0xCC]) * 0x50
        struct.pack_into("<H", stack, 0, 0xAC12)
        struct.pack_into("<H", stack, frame_offset - 0x0A - stack_base, row_width)
        struct.pack_into("<H", stack, frame_offset - 0x08 - stack_base, row_offset)
        struct.pack_into("<H", stack, frame_offset - 0x06 - stack_base, rows_word)
        struct.pack_into("<H", stack, frame_offset - stack_base, 0xBEEF)
        struct.pack_into("<H", stack, frame_offset + 2 - stack_base, 0x3456)
        struct.pack_into("<H", stack, frame_offset + 4 - stack_base, 0x789A)
        expected_stack = bytearray(stack)
        expected_rows = ((rows_word & 0xFF) - 1) & 0xFF
        expected_rows_word = (rows_word & 0xFF00) | expected_rows
        struct.pack_into(
            "<H",
            expected_stack,
            frame_offset - 0x06 - stack_base,
            expected_rows_word,
        )
        expected_offset = row_offset
        if continues:
            expected_offset = (row_offset + 0x0140) & 0xFFFF
            struct.pack_into(
                "<H",
                expected_stack,
                frame_offset - 0x08 - stack_base,
                expected_offset,
            )

        initial = {
            "eax": 0xA5A50000 | case_index,
            "bx": 0x2222,
            "cx": 0x3333,
            "dx": 0x4444,
            "si": 0x5555,
            "di": 0x6666,
            "bp": frame_offset,
            "sp": call_sp,
            "ds": 0x2000,
            "es": 0x4000,
            "gs": 0x5000,
            "flags": 0x0202,
        }
        machine = execute(
            0xAD96,
            0xADA8 if continues else 0xADAE,
            initial,
            [(stack_segment, stack_base, bytes(stack))],
        )

        expected_registers = {
            "eax": initial["eax"],
            "bx": initial["bx"],
            "cx": row_width if continues else initial["cx"],
            "dx": initial["dx"],
            "si": initial["si"],
            "di": expected_offset if continues else initial["di"],
            "bp": frame_offset if continues else 0xBEEF,
            "sp": call_sp if continues else frame_offset + 4,
            "ds": initial["ds"] if continues else 0x3456,
            "es": initial["es"],
            "gs": initial["gs"],
        }
        for register, expected in expected_registers.items():
            actual = machine.reg_read(REGISTERS[register])
            if actual != expected:
                raise AssertionError(
                    f"0xAD96 {name} {register}={actual:#x}, expected={expected:#x}"
                )
        if machine.reg_read(UC_X86_REG_SS) != stack_segment:
            raise AssertionError(f"0xAD96 {name} changed SS")

        actual_stack = bytes(
            machine.mem_read(stack_segment * 16 + stack_base, len(stack))
        )
        if actual_stack != bytes(expected_stack):
            raise AssertionError(f"0xAD96 {name} modified unexpected frame data")

        vectors.append(
            {
                "name": name,
                "continues": continues,
                "initial_rows_word": rows_word,
                "result_rows_word": expected_rows_word,
                "row_width": row_width,
                "initial_row_offset": row_offset,
                "result_row_offset": expected_offset,
                "result_cx": machine.reg_read(UC_X86_REG_CX),
                "result_di": machine.reg_read(UC_X86_REG_DI),
                "result_sp": machine.reg_read(UC_X86_REG_SP),
                "result_bp": machine.reg_read(UC_X86_REG_BP),
                "result_ds": machine.reg_read(UC_X86_REG_DS),
            }
        )

    return vectors


def banked_list_load_vectors() -> list[dict[str, object]]:
    data_segment = 0x2000
    buffer_segment = 0x3000
    wrapper = 0xF000
    cases = [
        {
            "name": "initial_no_handle",
            "file_handle": 0,
            "buffer_end": 0x9000,
            "extent": 10,
            "source_offset": 0x12345678,
            "source_remaining": 0x01020304,
            "reads": [],
        },
        {
            "name": "extent_two_empty_body",
            "file_handle": 4,
            "buffer_end": 0x9000,
            "extent": 2,
            "source_offset": 0,
            "source_remaining": 2,
            "reads": [
                {
                    "stage": "initial",
                    "returned": 2,
                    "carry": False,
                    "payload": struct.pack("<H", 2),
                },
                {
                    "stage": "body",
                    "returned": 0,
                    "carry": False,
                    "payload": b"",
                },
            ],
        },
        {
            "name": "ordinary_extent",
            "file_handle": 5,
            "buffer_end": 0xA000,
            "extent": 10,
            "source_offset": 0x0000FFFF,
            "source_remaining": 0x00010020,
            "reads": [
                {
                    "stage": "initial",
                    "returned": 2,
                    "carry": False,
                    "payload": struct.pack("<H", 10),
                },
                {
                    "stage": "body",
                    "returned": 8,
                    "carry": False,
                    "payload": b"BODYDATA",
                },
            ],
        },
        {
            "name": "short_reads_ignore_carry",
            "file_handle": 6,
            "buffer_end": 0x7F00,
            "extent": 12,
            "source_offset": 0x10203040,
            "source_remaining": 0x55667788,
            "reads": [
                {
                    "stage": "initial",
                    "returned": 1,
                    "carry": False,
                    "payload": b"\x0c",
                },
                {
                    "stage": "initial",
                    "returned": 2,
                    "carry": True,
                    "payload": struct.pack("<H", 12),
                },
                {
                    "stage": "body",
                    "returned": 3,
                    "carry": True,
                    "payload": b"bad",
                },
                {
                    "stage": "body",
                    "returned": 10,
                    "carry": False,
                    "payload": b"0123456789",
                },
            ],
        },
        {
            "name": "body_handle_removed",
            "file_handle": 7,
            "buffer_end": 0x8800,
            "extent": 6,
            "source_offset": 0xFFFFFFFE,
            "source_remaining": 8,
            "reads": [
                {
                    "stage": "initial",
                    "returned": 2,
                    "carry": False,
                    "payload": struct.pack("<H", 6),
                    "drop_handle": True,
                }
            ],
        },
        {
            "name": "extent_one_wraps_body_count",
            "file_handle": 8,
            "buffer_end": 0x7000,
            "extent": 1,
            "source_offset": 0x01020304,
            "source_remaining": 0x00020000,
            "reads": [
                {
                    "stage": "initial",
                    "returned": 2,
                    "carry": False,
                    "payload": struct.pack("<H", 1),
                },
                {
                    "stage": "body",
                    "returned": 0xFFFF,
                    "carry": False,
                    "payload": b"",
                },
            ],
        },
    ]
    vectors = []
    wrapper_code = b"\xe8" + struct.pack("<h", 0xA642 - (wrapper + 3))

    def read_u16(machine: Uc, offset: int) -> int:
        return struct.unpack(
            "<H", machine.mem_read(data_segment * 16 + offset, 2)
        )[0]

    def read_u32(machine: Uc, offset: int) -> int:
        return struct.unpack(
            "<I", machine.mem_read(data_segment * 16 + offset, 4)
        )[0]

    def set_carry(machine: Uc, carry: bool) -> None:
        flags = machine.reg_read(UC_X86_REG_EFLAGS)
        machine.reg_write(
            UC_X86_REG_EFLAGS, flags | 1 if carry else flags & ~1
        )

    for case_index, case in enumerate(cases):
        name = str(case["name"])
        file_handle = int(case["file_handle"])
        buffer_end = int(case["buffer_end"])
        extent = int(case["extent"])
        source_offset = int(case["source_offset"])
        source_remaining = int(case["source_remaining"])
        reads = list(case["reads"])
        body_count = (extent - 2) & 0xFFFF
        entry_start = (buffer_end - extent - 2) & 0xFFFF
        body_start = (entry_start + 2) & 0xFFFF
        read_index = 0
        calls: list[dict[str, int | bool | str]] = []

        def interrupt_handler(
            machine: Uc,
            number: int,
            case_name: str = name,
            responses: list[object] = reads,
            call_log: list[dict[str, int | bool | str]] = calls,
        ) -> None:
            nonlocal read_index
            if number != 0x21 or read_index >= len(responses):
                raise AssertionError(
                    f"0xA642 {case_name} invoked an unexpected interrupt"
                )

            response = responses[read_index]
            function = machine.reg_read(UC_X86_REG_AX) >> 8
            stage = str(response["stage"])
            expected_offset = source_offset + (2 if stage == "body" else 0)
            if function == 0x42:
                call_log.append(
                    {
                        "call": "seek",
                        "stage": stage,
                        "handle": machine.reg_read(UC_X86_REG_BX),
                        "offset": (
                            machine.reg_read(UC_X86_REG_CX) << 16
                            | machine.reg_read(UC_X86_REG_DX)
                        ),
                    }
                )
                machine.reg_write(UC_X86_REG_AX, expected_offset & 0xFFFF)
                machine.reg_write(UC_X86_REG_DX, expected_offset >> 16)
                set_carry(machine, False)
                return

            if function != 0x3F:
                raise AssertionError(
                    f"0xA642 {case_name} invoked DOS function {function:#x}"
                )
            requested = machine.reg_read(UC_X86_REG_CX)
            destination_segment = machine.reg_read(UC_X86_REG_DS)
            destination_offset = machine.reg_read(UC_X86_REG_DX)
            expected_count = 2 if stage == "initial" else body_count
            expected_destination = 0 if stage == "initial" else body_start
            if requested != expected_count or destination_offset != expected_destination:
                raise AssertionError(f"0xA642 {case_name} issued an unexpected read")
            if destination_segment != buffer_segment:
                raise AssertionError(
                    f"0xA642 {case_name} selected an unexpected buffer segment"
                )
            payload = bytes(response["payload"])
            if payload:
                machine.mem_write(
                    destination_segment * 16 + destination_offset, payload
                )
            call_log.append(
                {
                    "call": "read",
                    "stage": stage,
                    "handle": machine.reg_read(UC_X86_REG_BX),
                    "requested": requested,
                    "destination": destination_offset,
                    "returned": int(response["returned"]),
                    "carry": bool(response["carry"]),
                }
            )
            machine.reg_write(UC_X86_REG_AX, int(response["returned"]))
            set_carry(machine, bool(response["carry"]))
            if response.get("drop_handle"):
                machine.mem_write(
                    data_segment * 16 + 0x0D5B, struct.pack("<H", 0)
                )
            read_index += 1

        initial_words = {
            0x0D8C: 0x1111,
            0x0D8E: 0x2222,
            0x0D90: 0x3333,
            0x0D92: 0x4444,
            0x0D96: 0x5555,
            0x0D98: 0x6666,
            0x0D9A: 0x7777,
            0x0DA0: 0x8888,
        }
        memory = [
            (0, wrapper, wrapper_code),
            (data_segment, 0x0A56, struct.pack("<H", 0xFFFF)),
            (data_segment, 0x0A58, struct.pack("<H", 0xFFFF)),
            (data_segment, 0x0A7E, struct.pack("<H", buffer_segment)),
            (data_segment, 0x0D5B, struct.pack("<H", file_handle)),
            (data_segment, 0x0D84, struct.pack("<I", source_offset)),
            (data_segment, 0x0D88, struct.pack("<I", source_remaining)),
            (data_segment, 0x0DBC, b"\x00"),
            (data_segment, 0x5233, struct.pack("<H", buffer_end)),
            (buffer_segment, 0, bytes([0xCC]) * 0x10000),
        ]
        memory.extend(
            (data_segment, offset, struct.pack("<H", value))
            for offset, value in initial_words.items()
        )
        initial_eax = 0xA5A50000 | case_index
        machine = execute(
            wrapper,
            wrapper + len(wrapper_code),
            {
                "eax": initial_eax,
                "bx": 0x2222,
                "cx": 0x3333,
                "dx": 0x4444,
                "si": 0x5555,
                "di": 0x6666,
                "bp": 0x7777,
                "sp": 0xFF00,
                "ds": data_segment,
                "es": 0x4000,
                "gs": data_segment,
                "flags": 0x0202,
            },
            memory,
            interrupt_handler,
        )

        initial_failed = file_handle < 1
        body_failed = bool(reads and reads[-1].get("drop_handle"))
        success = not initial_failed and not body_failed
        carry = machine.reg_read(UC_X86_REG_EFLAGS) & 1
        if carry != int(not success):
            raise AssertionError(
                f"0xA642 {name} carry={carry}, expected={int(not success)}"
            )
        if read_index != len(reads) or len(calls) != len(reads) * 2:
            raise AssertionError(f"0xA642 {name} did not consume all DOS responses")
        for call in calls:
            if call["handle"] != file_handle:
                raise AssertionError(f"0xA642 {name} changed the DOS handle")
            expected_offset = source_offset + (
                2 if call["stage"] == "body" else 0
            )
            if call["call"] == "seek" and call["offset"] != expected_offset:
                raise AssertionError(f"0xA642 {name} sought unexpectedly")

        if initial_failed:
            transferred = 0
            expected_tail = 0
            expected_head = 0
            expected_queued = 0
        elif body_failed:
            transferred = 2
            expected_tail = entry_start
            expected_head = body_start
            expected_queued = 2
        else:
            transferred = 2 + body_count
            expected_tail = entry_start
            expected_head = (body_start + body_count) & 0xFFFF
            expected_queued = (2 + body_count) & 0xFFFF

        observed = {
            "head": read_u16(machine, 0x0D8C),
            "head_segment": read_u16(machine, 0x0D8E),
            "tail": read_u16(machine, 0x0D90),
            "tail_segment": read_u16(machine, 0x0D92),
            "active": read_u16(machine, 0x0D96),
            "wrap_limit": read_u16(machine, 0x0D98),
            "byte_count": read_u16(machine, 0x0D9A),
            "iteration_count": read_u16(machine, 0x0DA0),
            "source_offset": read_u32(machine, 0x0D84),
            "source_remaining": read_u32(machine, 0x0D88),
        }
        expected = {
            "head": expected_head,
            "head_segment": buffer_segment,
            "tail": expected_tail,
            "tail_segment": buffer_segment,
            "active": 0,
            "wrap_limit": buffer_end,
            "byte_count": expected_queued,
            "iteration_count": 0,
            "source_offset": (source_offset + transferred) & 0xFFFFFFFF,
            "source_remaining": (
                source_remaining - transferred
            ) & 0xFFFFFFFF,
        }
        if observed != expected:
            raise AssertionError(
                f"0xA642 {name} state={observed}, expected={expected}"
            )
        if machine.reg_read(UC_X86_REG_SP) != 0xFF00:
            raise AssertionError(f"0xA642 {name} did not restore the caller stack")

        if not initial_failed:
            initial_header = struct.unpack(
                "<H", machine.mem_read(buffer_segment * 16, 2)
            )[0]
            relocated_header = struct.unpack(
                "<H",
                machine.mem_read(buffer_segment * 16 + entry_start, 2),
            )[0]
            if initial_header != extent or relocated_header != extent:
                raise AssertionError(f"0xA642 {name} misplaced the extent header")
        if success:
            body_responses = [
                response for response in reads if response["stage"] == "body"
            ]
            expected_payload = bytes(body_responses[-1]["payload"])
            if expected_payload and machine.mem_read(
                buffer_segment * 16 + body_start, len(expected_payload)
            ) != expected_payload:
                raise AssertionError(f"0xA642 {name} misplaced the entry body")

        vectors.append(
            {
                "name": name,
                "success": success,
                "extent": extent,
                "body_count": body_count if not initial_failed else None,
                "entry_start": entry_start if not initial_failed else None,
                "calls": calls,
                "result": observed,
                "result_carry": carry,
            }
        )

    return vectors


def ems_paged_read_vectors() -> list[dict[str, object]]:
    data_segment = 0x2000
    buffer_segment = 0x3000
    page_frame_segment = 0x5000
    cases = [
        {
            "name": "direct_no_handle",
            "mode": "direct",
            "byte_count": 4,
            "file_handle": 0,
            "head": 0x0100,
            "queued": 0x0020,
            "source_offset": 0x12345678,
            "source_remaining": 0x01020304,
            "reads": [],
        },
        {
            "name": "direct_zero_count",
            "mode": "direct",
            "byte_count": 0,
            "file_handle": 1,
            "head": 0,
            "queued": 0,
            "source_offset": 0,
            "source_remaining": 0,
            "reads": [(0, False, b"")],
        },
        {
            "name": "direct_short_retry",
            "mode": "direct",
            "byte_count": 4,
            "file_handle": 5,
            "head": 0x0220,
            "queued": 0x0030,
            "source_offset": 0x0000FFFF,
            "source_remaining": 0x00010001,
            "reads": [(2, False, b"no"), (4, False, b"DATA")],
        },
        {
            "name": "direct_oversized_result",
            "mode": "direct",
            "byte_count": 2,
            "file_handle": 0x7FFF,
            "head": 0xFFFE,
            "queued": 0xFFFF,
            "source_offset": 0xFFFFFFFF,
            "source_remaining": 1,
            "reads": [(3, True, b"XYZ")],
        },
        {
            "name": "banked_ems_cross_page",
            "mode": "ems",
            "byte_count": 9,
            "ems_handle": 0x2345,
            "head": 0x1234,
            "queued": 0x4567,
            "source_offset": 0x0000BFFD,
            "source_remaining": 0x00020000,
            "payload": b"EMS-CROSS",
        },
        {
            "name": "banked_ems_page_wrap_zero_count",
            "mode": "ems",
            "byte_count": 0,
            "ems_handle": 0xFFFE,
            "head": 0xAAAA,
            "queued": 0x5555,
            "source_offset": 0xFFFFC000,
            "source_remaining": 0,
            "payload": b"",
        },
        {
            "name": "banked_xms_even",
            "mode": "xms",
            "byte_count": 6,
            "xms_handle": 0x1357,
            "head": 0x0300,
            "queued": 0x0400,
            "source_offset": 0x10203040,
            "source_remaining": 0x55667788,
            "payload": b"XMS123",
        },
        {
            "name": "banked_xms_odd_rounds_move",
            "mode": "xms",
            "byte_count": 5,
            "xms_handle": 0x2468,
            "head": 0xFFFC,
            "queued": 0xFFFE,
            "source_offset": 0xFFFFFFFE,
            "source_remaining": 3,
            "payload": b"odd5!+",
        },
        {
            "name": "banked_without_memory_falls_back_to_file",
            "mode": "fallback",
            "byte_count": 3,
            "file_handle": 9,
            "head": 0x0800,
            "queued": 0x0900,
            "source_offset": 0xAABBCCDD,
            "source_remaining": 0x11223344,
            "reads": [(3, False, b"DOS")],
        },
    ]
    vectors = []

    def read_u16(machine: Uc, offset: int) -> int:
        return struct.unpack(
            "<H", machine.mem_read(data_segment * 16 + offset, 2)
        )[0]

    def read_u32(machine: Uc, offset: int) -> int:
        return struct.unpack(
            "<I", machine.mem_read(data_segment * 16 + offset, 4)
        )[0]

    def set_carry(machine: Uc, carry: bool) -> None:
        flags = machine.reg_read(UC_X86_REG_EFLAGS)
        machine.reg_write(
            UC_X86_REG_EFLAGS, flags | 1 if carry else flags & ~1
        )

    for case_index, case in enumerate(cases):
        name = str(case["name"])
        mode = str(case["mode"])
        byte_count = int(case["byte_count"])
        file_handle = int(case.get("file_handle", 0))
        ems_handle = int(case.get("ems_handle", 0xFFFF))
        xms_handle = int(case.get("xms_handle", 0xFFFF))
        head = int(case["head"])
        queued = int(case["queued"])
        source_offset = int(case["source_offset"])
        source_remaining = int(case["source_remaining"])
        reads = list(case.get("reads", []))
        payload = bytes(case.get("payload", b""))
        calls: list[dict[str, int | bool | str]] = []
        read_index = 0
        initial_eax = 0xA5A50000 | (0x2000 + case_index)
        initial = {
            "eax": initial_eax,
            "bx": 0x2222,
            "cx": byte_count,
            "dx": 0x4444,
            "si": 0x5555,
            "di": 0x6666,
            "bp": 0x7777,
            "sp": 0xFF00,
            "ds": data_segment,
            "es": 0x4000,
            "gs": data_segment,
            "flags": 0x0202,
        }
        destination_seed = bytes([0xCC]) * 0x10000
        page_frame_seed = bytearray([0xEE]) * 0x10000
        if mode == "ems":
            page_offset = source_offset & 0x3FFF
            page_frame_seed[page_offset : page_offset + len(payload)] = payload
        xms_descriptor_seed = bytes(range(0x80, 0x90))

        def interrupt_handler(
            machine: Uc,
            number: int,
            case_name: str = name,
            call_log: list[dict[str, int | bool | str]] = calls,
            responses: list[object] = reads,
        ) -> None:
            nonlocal read_index
            if number == 0x67:
                ax = machine.reg_read(UC_X86_REG_AX)
                if ax >> 8 != 0x44:
                    raise AssertionError(
                        f"0xA664 {case_name} invoked unexpected EMS function"
                    )
                call_log.append(
                    {
                        "call": "ems_map",
                        "handle": machine.reg_read(UC_X86_REG_DX),
                        "logical_page": machine.reg_read(UC_X86_REG_BX),
                        "physical_page": ax & 0xFF,
                    }
                )
                machine.reg_write(UC_X86_REG_AX, ax & 0x00FF)
                return

            if number != 0x21:
                raise AssertionError(
                    f"0xA664 {case_name} invoked unexpected INT {number:#x}"
                )
            function = machine.reg_read(UC_X86_REG_AX) >> 8
            if function == 0x42:
                call_log.append(
                    {
                        "call": "seek",
                        "handle": machine.reg_read(UC_X86_REG_BX),
                        "offset_high": machine.reg_read(UC_X86_REG_CX),
                        "offset_low": machine.reg_read(UC_X86_REG_DX),
                    }
                )
                machine.reg_write(UC_X86_REG_AX, source_offset & 0xFFFF)
                machine.reg_write(UC_X86_REG_DX, source_offset >> 16)
                set_carry(machine, False)
                return

            if function != 0x3F or read_index >= len(responses):
                raise AssertionError(
                    f"0xA664 {case_name} invoked unexpected DOS function "
                    f"{function:#x}"
                )
            returned, failed, response_payload = responses[read_index]
            read_index += 1
            destination_segment = machine.reg_read(UC_X86_REG_DS)
            destination_offset = machine.reg_read(UC_X86_REG_DX)
            call_log.append(
                {
                    "call": "read",
                    "handle": machine.reg_read(UC_X86_REG_BX),
                    "requested": machine.reg_read(UC_X86_REG_CX),
                    "destination_segment": destination_segment,
                    "destination_offset": destination_offset,
                    "returned": int(returned),
                    "carry": bool(failed),
                }
            )
            machine.mem_write(
                destination_segment * 16 + destination_offset,
                bytes(response_payload),
            )
            machine.reg_write(UC_X86_REG_AX, int(returned))
            set_carry(machine, bool(failed))

        def code_handler(
            machine: Uc,
            address: int,
            _size: int,
            case_name: str = name,
            call_log: list[dict[str, int | bool | str]] = calls,
            xms_payload: bytes = payload,
        ) -> None:
            if address == 0xA6AA:
                transferred = machine.reg_read(UC_X86_REG_EAX)
                source_segment = machine.reg_read(UC_X86_REG_DS)
                source_pointer = machine.reg_read(UC_X86_REG_SI)
                destination_segment = machine.reg_read(UC_X86_REG_ES)
                destination_offset = machine.reg_read(UC_X86_REG_DI)
                call_log.append(
                    {
                        "call": "far_memmove",
                        "byte_count": transferred,
                        "source_segment": source_segment,
                        "source_offset": source_pointer,
                        "destination_segment": destination_segment,
                        "destination_offset": destination_offset,
                    }
                )
                copied = bytes(
                    machine.mem_read(
                        source_segment * 16 + source_pointer, transferred
                    )
                )
                machine.mem_write(
                    destination_segment * 16 + destination_offset, copied
                )
                flags = machine.reg_read(UC_X86_REG_EFLAGS)
                machine.reg_write(UC_X86_REG_EFLAGS, flags & ~(1 << 10))
            elif address == 0xA6F0:
                descriptor = struct.unpack(
                    "<IHIHI",
                    machine.mem_read(data_segment * 16 + 0x0A6C, 16),
                )
                destination_offset = descriptor[4] & 0xFFFF
                destination_segment = descriptor[4] >> 16
                call_log.append(
                    {
                        "call": "xms_move",
                        "function": machine.reg_read(UC_X86_REG_EAX),
                        "length": descriptor[0],
                        "source_handle": descriptor[1],
                        "source_offset": descriptor[2],
                        "destination_handle": descriptor[3],
                        "destination_segment": destination_segment,
                        "destination_offset": destination_offset,
                    }
                )
                machine.mem_write(
                    destination_segment * 16 + destination_offset,
                    xms_payload[: descriptor[0]],
                )
                machine.reg_write(UC_X86_REG_AX, 1)
                set_carry(machine, False)

        memory = [
            (0, 0xA6AA, b"\x90" * 5),
            (0, 0xA6F0, b"\x90" * 4),
            (data_segment, 0x0A56, struct.pack("<H", xms_handle)),
            (data_segment, 0x0A58, struct.pack("<H", ems_handle)),
            (data_segment, 0x0A66, struct.pack("<H", page_frame_segment)),
            (data_segment, 0x0A6C, xms_descriptor_seed),
            (data_segment, 0x0D5B, struct.pack("<H", file_handle)),
            (data_segment, 0x0D84, struct.pack("<I", source_offset)),
            (data_segment, 0x0D88, struct.pack("<I", source_remaining)),
            (
                data_segment,
                0x0D8C,
                struct.pack("<HH", head, buffer_segment),
            ),
            (data_segment, 0x0D9A, struct.pack("<H", queued)),
            (
                data_segment,
                0x0DBC,
                bytes([1 if mode in {"ems", "xms", "fallback"} else 0]),
            ),
            (buffer_segment, 0, destination_seed),
            (page_frame_segment, 0, bytes(page_frame_seed)),
        ]
        machine = execute(
            0xA664,
            0xA73D,
            initial,
            memory,
            interrupt_handler,
            code_handler,
        )

        success = not (mode in {"direct", "fallback"} and file_handle < 1)
        if mode in {"ems", "xms"}:
            transferred = byte_count
        elif success:
            transferred = int(reads[-1][0])
        else:
            transferred = initial_eax & 0xFFFF
        expected_increment = transferred if success else 0
        expected_state = {
            "source_offset": (
                source_offset + expected_increment
            ) & 0xFFFFFFFF,
            "source_remaining": (
                source_remaining - expected_increment
            ) & 0xFFFFFFFF,
            "head": (head + expected_increment) & 0xFFFF,
            "queued": (queued + expected_increment) & 0xFFFF,
        }
        observed_state = {
            "source_offset": read_u32(machine, 0x0D84),
            "source_remaining": read_u32(machine, 0x0D88),
            "head": read_u16(machine, 0x0D8C),
            "queued": read_u16(machine, 0x0D9A),
        }
        if observed_state != expected_state:
            raise AssertionError(
                f"0xA664 {name} state={observed_state}, "
                f"expected={expected_state}"
            )

        carry = machine.reg_read(UC_X86_REG_EFLAGS) & 1
        if carry != int(not success):
            raise AssertionError(
                f"0xA664 {name} carry={carry}, expected={int(not success)}"
            )
        expected_eax = initial_eax
        if success and mode in {"ems", "xms"}:
            expected_eax = transferred
        elif success:
            expected_eax = (initial_eax & 0xFFFF0000) | transferred
        expected_bx = initial["bx"]
        expected_dx = initial["dx"]
        if mode == "ems":
            expected_bx = ((source_offset >> 14) + 4) & 0xFFFF
            expected_dx = ems_handle
        elif mode in {"direct", "fallback"}:
            expected_bx = file_handle
            if success:
                expected_dx = head
        expected_registers = {
            "eax": expected_eax,
            "bx": expected_bx,
            "cx": byte_count,
            "dx": expected_dx,
            "si": initial["si"],
            "di": initial["di"],
            "bp": initial["bp"],
            "sp": initial["sp"],
            "ds": initial["ds"],
            "es": initial["es"],
            "gs": initial["gs"],
        }
        for register, expected in expected_registers.items():
            actual = machine.reg_read(REGISTERS[register])
            if actual != expected:
                raise AssertionError(
                    f"0xA664 {name} {register}={actual:#x}, "
                    f"expected={expected:#x}"
                )

        if mode == "ems":
            map_calls = [call for call in calls if call["call"] == "ems_map"]
            expected_pages = [
                ((source_offset >> 14) + index) & 0xFFFF for index in range(4)
            ]
            if [call["logical_page"] for call in map_calls] != expected_pages:
                raise AssertionError(f"0xA664 {name} mapped unexpected EMS pages")
            if [call["physical_page"] for call in map_calls] != list(range(4)):
                raise AssertionError(f"0xA664 {name} mapped unexpected EMS frames")
            if any(call["handle"] != ems_handle for call in map_calls):
                raise AssertionError(f"0xA664 {name} used an unexpected EMS handle")
            move_calls = [
                call for call in calls if call["call"] == "far_memmove"
            ]
            if move_calls != [
                {
                    "call": "far_memmove",
                    "byte_count": byte_count,
                    "source_segment": page_frame_segment,
                    "source_offset": source_offset & 0x3FFF,
                    "destination_segment": buffer_segment,
                    "destination_offset": head,
                }
            ]:
                raise AssertionError(f"0xA664 {name} used an unexpected far move")
        elif mode == "xms":
            rounded_length = byte_count + (byte_count & 1)
            expected_xms_call = {
                "call": "xms_move",
                "function": 0x0B00,
                "length": rounded_length,
                "source_handle": xms_handle,
                "source_offset": source_offset,
                "destination_handle": 0,
                "destination_segment": buffer_segment,
                "destination_offset": head,
            }
            if calls != [expected_xms_call]:
                raise AssertionError(f"0xA664 {name} built an unexpected XMS move")
        else:
            if read_index != len(reads):
                raise AssertionError(f"0xA664 {name} missed a DOS response")
            if len(calls) != len(reads) * 2:
                raise AssertionError(f"0xA664 {name} made unexpected DOS calls")
            for call in calls:
                if call["handle"] != file_handle:
                    raise AssertionError(
                        f"0xA664 {name} used an unexpected DOS handle"
                    )
                if call["call"] == "seek" and (
                    call["offset_high"] != source_offset >> 16
                    or call["offset_low"] != source_offset & 0xFFFF
                ):
                    raise AssertionError(f"0xA664 {name} sought unexpectedly")
                if call["call"] == "read" and (
                    call["requested"] != byte_count
                    or call["destination_segment"] != buffer_segment
                    or call["destination_offset"] != head
                ):
                    raise AssertionError(f"0xA664 {name} read unexpectedly")

        expected_payload = b""
        if mode in {"ems", "xms"}:
            expected_payload = payload
        elif success:
            expected_payload = bytes(reads[-1][2])
        actual_payload = bytes(
            machine.mem_read(
                buffer_segment * 16 + head, len(expected_payload)
            )
        )
        if actual_payload != expected_payload:
            raise AssertionError(f"0xA664 {name} copied unexpected bytes")
        descriptor = bytes(machine.mem_read(data_segment * 16 + 0x0A6C, 16))
        if mode != "xms" and descriptor != xms_descriptor_seed:
            raise AssertionError(f"0xA664 {name} changed the XMS descriptor")

        vectors.append(
            {
                "name": name,
                "mode": mode,
                "success": success,
                "requested": byte_count,
                "transferred": transferred if success else None,
                "calls": calls,
                "result": observed_state,
                "result_carry": carry,
                "xms_descriptor": list(descriptor) if mode == "xms" else None,
            }
        )

    return vectors


def list_init_vectors() -> list[dict[str, object]]:
    data_segment = 0x2000
    cases = [
        ("zero_bounds", 0x0000, 0x0000),
        ("ordinary", 0x3456, 0x4000),
        ("maximum_segment", 0xFFFF, 0x1234),
        ("maximum_end", 0x1357, 0xFFFF),
        ("matching_values", 0xAAAA, 0xAAAA),
    ]
    reset_offsets = (0x0D8C, 0x0D90, 0x0D96, 0x0D9A, 0x0DA0)
    vectors = []

    for name, base_segment, buffer_end in cases:
        initial_words = {
            0x0D8C: 0x1111,
            0x0D8E: 0x2222,
            0x0D90: 0x3333,
            0x0D92: 0x4444,
            0x0D94: 0x5555,
            0x0D96: 0x6666,
            0x0D98: 0x7777,
            0x0D9A: 0x8888,
            0x0D9C: 0x9999,
            0x0D9E: 0xAAAA,
            0x0DA0: 0xBBBB,
        }
        initial = {
            "ax": 0x1111,
            "bx": 0x2222,
            "cx": 0x3333,
            "dx": 0x4444,
            "si": 0x5555,
            "di": 0x6666,
            "bp": 0x7777,
            "ds": data_segment,
            "es": 0x2800,
            "gs": 0x2C00,
        }
        memory = [
            (data_segment, offset, struct.pack("<H", value))
            for offset, value in initial_words.items()
        ]
        memory.extend(
            [
                (data_segment, 0x0A7E, struct.pack("<H", base_segment)),
                (data_segment, 0x5233, struct.pack("<H", buffer_end)),
            ]
        )
        machine = execute(0xA757, 0xA777, initial, memory)

        expected_words = dict(initial_words)
        expected_words[0x0D8E] = base_segment
        expected_words[0x0D92] = base_segment
        expected_words[0x0D98] = buffer_end
        for offset in reset_offsets:
            expected_words[offset] = 0
        observed_words = {
            offset: struct.unpack(
                "<H", machine.mem_read(data_segment * 16 + offset, 2)
            )[0]
            for offset in initial_words
        }
        if observed_words != expected_words:
            raise AssertionError(
                f"0xA757 {name}: actual={observed_words}, expected={expected_words}"
            )
        expected_registers = dict(initial)
        expected_registers["ax"] = buffer_end
        for register, value in expected_registers.items():
            if register in REGISTERS:
                actual_register = machine.reg_read(REGISTERS[register])
                if actual_register != value:
                    raise AssertionError(f"0xA757 did not preserve {register}")

        vectors.append(
            {
                "name": name,
                "base_segment": base_segment,
                "buffer_end": buffer_end,
                "head_pointer": [0, base_segment],
                "tail_pointer": [0, base_segment],
                "cleared_offsets": list(reset_offsets),
                "result_wrap_limit": buffer_end,
                "preserved_sentinel_offsets": [0x0D94, 0x0D9C, 0x0D9E],
            }
        )

    return vectors


def mem_copy_words_vectors() -> list[dict[str, object]]:
    data_segment = 0x2000
    cases = [
        ("disjoint", 0x0100, 0x0200),
        ("same_pointer", 0x0100, 0x0100),
        ("forward_overlap", 0x0100, 0x0102),
        ("backward_overlap", 0x0102, 0x0100),
        ("source_offset_wrap", 0xFFFC, 0x0200),
        ("destination_offset_wrap", 0x0100, 0xFFFC),
    ]
    vectors = []

    def read_word(buffer: bytearray, offset: int) -> int:
        low = buffer[offset]
        high = buffer[(offset + 1) & 0xFFFF]
        return low | high << 8

    def write_word(buffer: bytearray, offset: int, value: int) -> None:
        buffer[offset] = value & 0xFF
        buffer[(offset + 1) & 0xFFFF] = value >> 8

    for name, source_offset, destination_offset in cases:
        initial_memory = bytearray((index * 37 + 11) & 0xFF for index in range(0x10000))
        source_words = (0x1122, 0x3344, 0x5566, 0x7788)
        for index, value in enumerate(source_words):
            write_word(initial_memory, (source_offset + index * 2) & 0xFFFF, value)

        initial = {
            "ax": 0x1111,
            "bx": 0x2222,
            "cx": 0x3333,
            "dx": 0x4444,
            "si": source_offset,
            "di": destination_offset,
            "bp": 0x7777,
            "ds": data_segment,
            "es": 0x2800,
            "gs": 0x2C00,
            "flags": 0x0AD7,
        }
        machine = execute(
            0xA7E6,
            0xA7EC,
            initial,
            [(data_segment, 0, bytes(initial_memory))],
        )

        expected_memory = bytearray(initial_memory)
        copied_words = []
        for index in range(4):
            source = (source_offset + index * 2) & 0xFFFF
            destination = (destination_offset + index * 2) & 0xFFFF
            value = read_word(expected_memory, source)
            copied_words.append(value)
            write_word(expected_memory, destination, value)
        actual_memory = bytes(
            machine.mem_read(data_segment * 16, len(expected_memory))
        )
        if actual_memory != bytes(expected_memory):
            raise AssertionError(f"0xA7E6 {name} produced unexpected memory")

        expected_registers = dict(initial)
        expected_registers["si"] = (source_offset + 8) & 0xFFFF
        expected_registers["di"] = (destination_offset + 8) & 0xFFFF
        expected_registers["es"] = data_segment
        for register, value in expected_registers.items():
            if register in REGISTERS:
                actual_register = machine.reg_read(REGISTERS[register])
                if actual_register != value:
                    raise AssertionError(
                        f"0xA7E6 {name} {register}={actual_register:#x}, "
                        f"expected={value:#x}"
                    )

        vectors.append(
            {
                "name": name,
                "source_offset": source_offset,
                "destination_offset": destination_offset,
                "copied_words_in_order": copied_words,
                "result_source_offset": expected_registers["si"],
                "result_destination_offset": expected_registers["di"],
                "result_es": data_segment,
                "preserved_flags": expected_registers["flags"],
            }
        )

    return vectors


def flag_gated_copy_vectors() -> list[dict[str, object]]:
    data_segment = 0x2000
    state_segment = 0x4000
    render_segment = 0x6000
    copy_size = 0x60 * 4
    cases = [
        ("clear_zero", 0x00),
        ("clear_other_bits", 0xFE),
        ("set_one", 0x01),
        ("set_all_bits", 0xFF),
    ]
    source = bytes((index * 43 + 17) & 0xFF for index in range(copy_size))
    destination = bytes((index * 19 + 7) & 0xFF for index in range(copy_size))
    vectors = []

    for name, state in cases:
        initial = {
            "ax": 0x1111,
            "bx": 0x2222,
            "cx": 0x3333,
            "dx": 0x4444,
            "si": 0x5555,
            "di": 0x6666,
            "bp": 0x7777,
            "ds": data_segment,
            "es": render_segment,
            "gs": state_segment,
            "flags": 0x0293,
        }
        render_memory = bytearray(0x800)
        render_memory[:] = bytes((index * 29 + 3) & 0xFF for index in range(0x800))
        source_index = 0x5251 - 0x5200
        destination_index = 0x5851 - 0x5200
        render_memory[source_index : source_index + copy_size] = source
        render_memory[destination_index : destination_index + copy_size] = destination
        data_decoy = bytes((index * 7 + 5) & 0xFF for index in range(0x800))
        state_decoy = bytes((index * 11 + 9) & 0xFF for index in range(0x800))

        machine = execute(
            0xA117,
            0xA133,
            initial,
            [
                (data_segment, 0x5200, data_decoy),
                (state_segment, 0x2751, bytes([state])),
                (state_segment, 0x5200, state_decoy),
                (render_segment, 0x5200, bytes(render_memory)),
            ],
        )

        expected_render = bytearray(render_memory)
        copied = (state & 1) == 0
        if copied:
            expected_render[
                destination_index : destination_index + copy_size
            ] = source
        actual_render = bytes(machine.mem_read(render_segment * 16 + 0x5200, 0x800))
        if actual_render != bytes(expected_render):
            raise AssertionError(f"0xA117 {name} produced unexpected ES memory")
        if machine.mem_read(data_segment * 16 + 0x5200, 0x800) != data_decoy:
            raise AssertionError(f"0xA117 {name} changed DS decoy memory")
        if machine.mem_read(state_segment * 16 + 0x5200, 0x800) != state_decoy:
            raise AssertionError(f"0xA117 {name} changed GS decoy memory")
        actual_state = machine.mem_read(state_segment * 16 + 0x2751, 1)[0]
        if actual_state != state:
            raise AssertionError(f"0xA117 {name} changed the gate byte")

        expected_registers = dict(initial)
        if copied:
            expected_registers["cx"] = 0
            expected_registers["di"] = 0x5851 + copy_size
        for register, value in expected_registers.items():
            if register == "flags":
                continue
            actual_register = machine.reg_read(REGISTERS[register])
            if actual_register != value:
                raise AssertionError(
                    f"0xA117 {name} {register}={actual_register:#x}, "
                    f"expected={value:#x}"
                )

        flags = machine.reg_read(UC_X86_REG_EFLAGS)
        expected_zero = int((state & 1) == 0)
        zero_flag = (flags >> 6) & 1
        parity_flag = (flags >> 2) & 1
        if zero_flag != expected_zero or parity_flag != expected_zero:
            raise AssertionError(
                f"0xA117 {name} ZF/PF={zero_flag}/{parity_flag}, "
                f"expected={expected_zero}"
            )
        if flags & ((1 << 0) | (1 << 7) | (1 << 11)):
            raise AssertionError(f"0xA117 {name} did not clear CF/SF/OF")
        if flags & (1 << 9) != initial["flags"] & (1 << 9):
            raise AssertionError(f"0xA117 {name} changed IF")

        vectors.append(
            {
                "name": name,
                "state": state,
                "copied_384_bytes": copied,
                "result_cx": expected_registers["cx"],
                "result_di": expected_registers["di"],
                "preserved_si": expected_registers["si"],
                "preserved_ds": expected_registers["ds"],
                "result_zero_flag": zero_flag,
            }
        )

    return vectors


def presentation_queue_finish_vectors() -> list[dict[str, object]]:
    data_segment = 0x2000
    cases = [
        ("nonzero_one", 0x00, 0x0001, 0x0000, 0x1234),
        ("nonzero_high_bit", 0x02, 0x8000, 0x0000, 0x1234),
        ("nonzero_max", 0xFE, 0xFFFF, 0x0000, 0x1234),
        ("zero_no_handle", 0x00, 0x0000, 0x0000, 0x1234),
        ("zero_reserved_handle", 0x01, 0x0000, 0x1234, 0x1234),
        ("zero_preserve_high_bits", 0xFC, 0x0000, 0xABCD, 0xABCD),
    ]
    vectors = []

    def parity_even(value: int) -> int:
        return int((value & 0xFF).bit_count() % 2 == 0)

    for name, state, byte_count, file_handle, reserved_handle in cases:
        initial = {
            "ax": 0x1111,
            "bx": 0x2222,
            "cx": 0x3333,
            "dx": 0x4444,
            "si": 0x5555,
            "di": 0x6666,
            "bp": 0x7777,
            "ds": data_segment,
            "es": 0x2800,
            "gs": 0x3000,
            "flags": 0x0ED7,
        }
        machine = execute(
            0xA2DD,
            0xA2F1,
            initial,
            [
                (data_segment, 0x0A86, struct.pack("<H", reserved_handle)),
                (data_segment, 0x0D5B, struct.pack("<H", file_handle)),
                (data_segment, 0x0D5F, bytes([state])),
                (data_segment, 0x0D9A, struct.pack("<H", byte_count)),
            ],
        )

        expected_state = state | 1
        close_called = byte_count == 0
        if close_called:
            expected_state |= 2
        actual_state = machine.mem_read(data_segment * 16 + 0x0D5F, 1)[0]
        actual_count = struct.unpack(
            "<H", machine.mem_read(data_segment * 16 + 0x0D9A, 2)
        )[0]
        actual_handle = struct.unpack(
            "<H", machine.mem_read(data_segment * 16 + 0x0D5B, 2)
        )[0]
        if actual_state != expected_state:
            raise AssertionError(
                f"0xA2DD {name} state={actual_state:#x}, expected={expected_state:#x}"
            )
        if actual_count != byte_count or actual_handle != file_handle:
            raise AssertionError(f"0xA2DD {name} changed queue count or file handle")

        expected_registers = dict(initial)
        if close_called:
            expected_registers["bx"] = file_handle
            expected_registers["cx"] = 0
        for register, value in expected_registers.items():
            if register == "flags":
                continue
            actual_register = machine.reg_read(REGISTERS[register])
            if actual_register != value:
                raise AssertionError(
                    f"0xA2DD {name} {register}={actual_register:#x}, "
                    f"expected={value:#x}"
                )

        flags = machine.reg_read(UC_X86_REG_EFLAGS)
        expected_zero = int(byte_count == 0)
        expected_sign = int(bool(byte_count & 0x8000))
        expected_parity = parity_even(byte_count)
        actual_zero = (flags >> 6) & 1
        actual_sign = (flags >> 7) & 1
        actual_parity = (flags >> 2) & 1
        if (actual_zero, actual_sign, actual_parity) != (
            expected_zero,
            expected_sign,
            expected_parity,
        ):
            raise AssertionError(f"0xA2DD {name} produced unexpected status flags")
        if flags & ((1 << 0) | (1 << 11)):
            raise AssertionError(f"0xA2DD {name} did not clear CF/OF")

        vectors.append(
            {
                "name": name,
                "initial_state": state,
                "byte_count": byte_count,
                "file_handle": file_handle,
                "reserved_handle": reserved_handle,
                "result_state": expected_state,
                "close_called": close_called,
                "result_bx": expected_registers["bx"],
                "result_cx": expected_registers["cx"],
                "result_zero_flag": actual_zero,
            }
        )

    return vectors


def resource_descriptor_lookup_vectors() -> list[dict[str, object]]:
    data_segment = 0x2000
    cases = [
        ("first_entry", 0x0000, 0x2069),
        ("second_entry", 0x0001, 0x207F),
        ("ordinary_index_nine", 0x0009, 0x1357),
        ("highest_reachable_word_start", 0x3812, 0x2468),
        ("stride_wraps_to_first", 0x4000, 0x369C),
        ("signed_high_bit", 0x8000, 0x48AD),
        ("addition_overflow", 0x2000, 0x5ABE),
        ("maximum_index", 0xFFFF, 0x6BCF),
    ]
    vectors = []

    def parity_even(value: int) -> int:
        return int((value & 0xFF).bit_count() % 2 == 0)

    for name, index, record_offset in cases:
        table_offset = (0x1FB5 + index * 4) & 0xFFFF
        before_final_add = (0x1FB5 + index * 3) & 0xFFFF
        full_sum = before_final_add + index
        initial = {
            "ax": index,
            "bx": 0xA55A,
            "cx": 0x1357,
            "dx": 0x2468,
            "si": 0x369C,
            "di": 0x48AD,
            "bp": 0x5ABE,
            "ds": data_segment,
            "es": 0x3000,
            "gs": 0x4000,
            "flags": 0x0ED7,
        }
        machine = execute(
            0x9F80,
            0x9F8D,
            initial,
            [
                (data_segment, table_offset, struct.pack("<H", record_offset)),
                (initial["es"], table_offset, struct.pack("<H", 0xDEAD)),
                (initial["gs"], table_offset, struct.pack("<H", 0xBEEF)),
            ],
        )

        actual_bx = machine.reg_read(UC_X86_REG_BX)
        if actual_bx != record_offset:
            raise AssertionError(
                f"0x9F80 {name} bx={actual_bx:#x}, expected={record_offset:#x}"
            )
        for register in ("ax", "cx", "dx", "si", "di", "bp", "ds", "es", "gs"):
            actual_register = machine.reg_read(REGISTERS[register])
            if actual_register != initial[register]:
                raise AssertionError(f"0x9F80 did not preserve {register}")

        result = full_sum & 0xFFFF
        expected_flags = {
            "carry": int(full_sum > 0xFFFF),
            "parity": parity_even(result),
            "auxiliary_carry": int(
                ((before_final_add & 0xF) + (index & 0xF)) > 0xF
            ),
            "zero": int(result == 0),
            "sign": int(bool(result & 0x8000)),
            "overflow": int(
                bool((~(before_final_add ^ index) & (before_final_add ^ result)) & 0x8000)
            ),
        }
        flags = machine.reg_read(UC_X86_REG_EFLAGS)
        actual_flags = {
            "carry": (flags >> 0) & 1,
            "parity": (flags >> 2) & 1,
            "auxiliary_carry": (flags >> 4) & 1,
            "zero": (flags >> 6) & 1,
            "sign": (flags >> 7) & 1,
            "overflow": (flags >> 11) & 1,
        }
        if actual_flags != expected_flags:
            raise AssertionError(
                f"0x9F80 {name} flags={actual_flags}, expected={expected_flags}"
            )

        vectors.append(
            {
                "name": name,
                "index": index,
                "table_offset": table_offset,
                "record_offset": record_offset,
                "result_bx": actual_bx,
                "flags_from_final_add": actual_flags,
            }
        )

    return vectors


def presentation_update_vectors() -> list[dict[str, object]]:
    data_segment = 0x2000
    cases = [
        ("inactive_zero", 0x00, 0x0000, 0x08, 0x5A, 0x1234, 0xFF, 0x00),
        ("inactive_other_bits", 0xFE, 0x0001, 0x08, 0xA5, 0x2468, 0x83, 0x40),
        ("active_no_redraw", 0x01, 0x0001, 0x00, 0x5A, 0x1357, 0x03, 0x80),
        ("active_redraw", 0xFF, 0xFFFF, 0x08, 0xA5, 0x2468, 0xFF, 0x02),
        ("active_high_ship_byte_only", 0x03, 0x8000, 0x00, 0x7E, 0x369C, 0x02, 0x04),
        ("active_zero_count", 0x01, 0x0000, 0x08, 0x5A, 0x48AD, 0x00, 0x08),
        ("active_reserved_handle", 0x81, 0x0000, 0x08, 0xC3, 0x5ABE, 0xFC, 0x10),
        ("active_request_sign", 0x01, 0x0001, 0x08, 0x11, 0x6BCF, 0x82, 0x20),
    ]
    vectors = []

    def parity_even(value: int) -> int:
        return int((value & 0xFF).bit_count() % 2 == 0)

    for (
        name,
        gate,
        byte_count,
        ship_low,
        redraw,
        active_line,
        request_flags,
        list_state,
    ) in cases:
        ship_flags = ship_low | (0x0800 if name == "active_high_ship_byte_only" else 0)
        file_handle = 0x1234 if name == "active_reserved_handle" else 0
        initial = {
            "ax": 0x1111,
            "bx": 0x2222,
            "cx": 0x3333,
            "dx": 0x4444,
            "si": 0x5555,
            "di": 0x6666,
            "bp": 0x7777,
            "sp": 0xFF00,
            "ds": data_segment,
            "es": 0x3000,
            "gs": 0x4000,
            "flags": 0x0ED7,
        }
        machine = execute(
            0x9F53,
            0x9F7F,
            initial,
            [
                (data_segment, 0x0A86, struct.pack("<H", file_handle)),
                (data_segment, 0x0D5B, struct.pack("<H", file_handle)),
                (data_segment, 0x0D5F, bytes([list_state])),
                (data_segment, 0x0D9A, struct.pack("<H", byte_count)),
                (data_segment, 0x1FB2, bytes([gate])),
                (data_segment, 0x24F3, struct.pack("<H", ship_flags)),
                (data_segment, 0x27D8, bytes([redraw])),
                (data_segment, 0x6788, struct.pack("<H", active_line)),
                (data_segment, 0x67AA, bytes([request_flags])),
                (initial["gs"], 0x0D5F, bytes([0x99])),
                (initial["gs"], 0x0D9A, struct.pack("<H", 0x5555)),
                (initial["gs"], 0x1FB2, bytes([0xA4])),
                (initial["gs"], 0x24F3, struct.pack("<H", 0x8888)),
                (initial["gs"], 0x27D8, bytes([0x66])),
                (initial["gs"], 0x6788, struct.pack("<H", 0x7777)),
                (initial["gs"], 0x67AA, bytes([0xBB])),
            ],
        )

        active = bool(gate & 1)
        expected = {
            "gate": 0 if active else gate,
            "redraw": 1 if active and (ship_flags & 8) else redraw,
            "active_line": 0xFFFF if active else active_line,
            "request_flags": request_flags & 0xFD if active else request_flags,
            "list_state": list_state
            | (1 if active else 0)
            | (2 if active and byte_count == 0 else 0),
            "byte_count": byte_count,
            "file_handle": file_handle,
            "ship_flags": ship_flags,
        }
        observed = {
            "gate": machine.mem_read(data_segment * 16 + 0x1FB2, 1)[0],
            "redraw": machine.mem_read(data_segment * 16 + 0x27D8, 1)[0],
            "active_line": struct.unpack(
                "<H", machine.mem_read(data_segment * 16 + 0x6788, 2)
            )[0],
            "request_flags": machine.mem_read(data_segment * 16 + 0x67AA, 1)[0],
            "list_state": machine.mem_read(data_segment * 16 + 0x0D5F, 1)[0],
            "byte_count": struct.unpack(
                "<H", machine.mem_read(data_segment * 16 + 0x0D9A, 2)
            )[0],
            "file_handle": struct.unpack(
                "<H", machine.mem_read(data_segment * 16 + 0x0D5B, 2)
            )[0],
            "ship_flags": struct.unpack(
                "<H", machine.mem_read(data_segment * 16 + 0x24F3, 2)
            )[0],
        }
        if observed != expected:
            raise AssertionError(
                f"0x9F53 {name} state={observed}, expected={expected}"
            )
        for register, value in initial.items():
            if register == "flags":
                continue
            actual_register = machine.reg_read(REGISTERS[register])
            if actual_register != value:
                raise AssertionError(f"0x9F53 {name} did not preserve {register}")

        gs_decoys = {
            "list_state": machine.mem_read(initial["gs"] * 16 + 0x0D5F, 1)[0],
            "byte_count": struct.unpack(
                "<H", machine.mem_read(initial["gs"] * 16 + 0x0D9A, 2)
            )[0],
            "gate": machine.mem_read(initial["gs"] * 16 + 0x1FB2, 1)[0],
            "ship_flags": struct.unpack(
                "<H", machine.mem_read(initial["gs"] * 16 + 0x24F3, 2)
            )[0],
            "redraw": machine.mem_read(initial["gs"] * 16 + 0x27D8, 1)[0],
            "active_line": struct.unpack(
                "<H", machine.mem_read(initial["gs"] * 16 + 0x6788, 2)
            )[0],
            "request_flags": machine.mem_read(initial["gs"] * 16 + 0x67AA, 1)[0],
        }
        expected_decoys = {
            "list_state": 0x99,
            "byte_count": 0x5555,
            "gate": 0xA4,
            "ship_flags": 0x8888,
            "redraw": 0x66,
            "active_line": 0x7777,
            "request_flags": 0xBB,
        }
        if gs_decoys != expected_decoys:
            raise AssertionError(f"0x9F53 {name} accessed GS-owned decoy data")

        flag_result = expected["request_flags"] if active else gate & 1
        flags = machine.reg_read(UC_X86_REG_EFLAGS)
        actual_flags = {
            "carry": (flags >> 0) & 1,
            "parity": (flags >> 2) & 1,
            "zero": (flags >> 6) & 1,
            "sign": (flags >> 7) & 1,
            "overflow": (flags >> 11) & 1,
            "interrupt": (flags >> 9) & 1,
            "direction": (flags >> 10) & 1,
        }
        expected_flags = {
            "carry": 0,
            "parity": parity_even(flag_result),
            "zero": int(flag_result == 0),
            "sign": int(bool(flag_result & 0x80)),
            "overflow": 0,
            "interrupt": 1,
            "direction": 1,
        }
        if actual_flags != expected_flags:
            raise AssertionError(
                f"0x9F53 {name} flags={actual_flags}, expected={expected_flags}"
            )

        vectors.append(
            {
                "name": name,
                "initial_gate": gate,
                "byte_count": byte_count,
                "ship_flags": ship_flags,
                "initial_redraw": redraw,
                "initial_active_line": active_line,
                "initial_request_flags": request_flags,
                "initial_list_state": list_state,
                "result": observed,
                "final_flags": actual_flags,
            }
        )

    return vectors


def close_file_vectors() -> list[dict[str, object]]:
    data_segment = 0x2000
    cases = [
        ("zero_handle_zero_reserved", 0x0000, 0x0000, None),
        ("zero_handle", 0x0000, 0x1234, None),
        ("reserved_handle", 0x1234, 0x1234, None),
        ("reserved_max_handle", 0xFFFF, 0xFFFF, None),
        ("close_success", 0x0005, 0x1234, None),
        ("close_failure", 0x2468, 0x1234, 0x0006),
        ("close_max_handle", 0xFFFF, 0x0000, None),
    ]
    initial_bounds = (0x1111, 0x2222, 0x3333, 0x4444)
    vectors = []

    for case_index, (name, handle, reserved_handle, dos_error) in enumerate(cases):
        initial_ax = (0x1200 + case_index * 0x111 + 0x5A) & 0xFFFF
        initial = {
            "ax": initial_ax,
            "bx": 0xA55A,
            "cx": 0xB66B,
            "dx": 0xC77C,
            "si": 0xD88D,
            "di": 0xE99E,
            "bp": 0xFAAF,
            "sp": 0xFF00,
            "ds": data_segment,
            "es": 0x3000,
            "gs": 0x4000,
            "flags": 0x0ED7,
        }
        interrupts = []

        def interrupt_handler(
            machine: Uc,
            number: int,
            case_name: str = name,
            calls: list[dict[str, int]] = interrupts,
            error: int | None = dos_error,
        ) -> None:
            if number != 0x21 or machine.reg_read(UC_X86_REG_AX) >> 8 != 0x3E:
                raise AssertionError(
                    f"0xA141 {case_name} invoked unexpected INT {number:#x}"
                )
            calls.append(
                {
                    "number": number,
                    "handle": machine.reg_read(UC_X86_REG_BX),
                    "stored_handle": struct.unpack(
                        "<H", machine.mem_read(data_segment * 16 + 0x0D5B, 2)
                    )[0],
                }
            )
            flags = machine.reg_read(UC_X86_REG_EFLAGS)
            if error is None:
                machine.reg_write(UC_X86_REG_EFLAGS, flags & ~1)
            else:
                machine.reg_write(UC_X86_REG_AX, error)
                machine.reg_write(UC_X86_REG_EFLAGS, flags | 1)

        machine = execute(
            0xA141,
            0xA15E,
            initial,
            [
                (data_segment, 0x0A86, struct.pack("<H", reserved_handle)),
                (data_segment, 0x0D5B, struct.pack("<H", handle)),
                (data_segment, 0x0D60, struct.pack("<4H", *initial_bounds)),
                (initial["gs"], 0x0A86, struct.pack("<H", 0xABCD)),
                (initial["gs"], 0x0D5B, struct.pack("<H", 0xBCDE)),
                (
                    initial["gs"],
                    0x0D60,
                    struct.pack("<4H", 0x5555, 0x6666, 0x7777, 0x8888),
                ),
            ],
            interrupt_handler,
        )

        closed = handle != 0 and handle != reserved_handle
        expected_bounds = (
            (0x0000, 0x0000, 0xFFFF, 0xFFFF) if closed else initial_bounds
        )
        observed_handle = struct.unpack(
            "<H", machine.mem_read(data_segment * 16 + 0x0D5B, 2)
        )[0]
        observed_bounds = struct.unpack(
            "<4H", machine.mem_read(data_segment * 16 + 0x0D60, 8)
        )
        if observed_handle != (0 if closed else handle):
            raise AssertionError(f"0xA141 {name} produced an unexpected stored handle")
        if observed_bounds != expected_bounds:
            raise AssertionError(
                f"0xA141 {name} bounds={observed_bounds}, expected={expected_bounds}"
            )
        if len(interrupts) != int(closed):
            raise AssertionError(f"0xA141 {name} produced unexpected interrupt count")
        if closed and interrupts != [
            {"number": 0x21, "handle": handle, "stored_handle": 0}
        ]:
            raise AssertionError(f"0xA141 {name} violated DOS-close ordering")

        expected_ax = initial_ax
        if closed:
            expected_ax = (
                dos_error if dos_error is not None else 0x3E00 | (initial_ax & 0xFF)
            )
        expected_registers = {
            "ax": expected_ax,
            "bx": handle,
            "cx": 0,
            "dx": initial["dx"],
            "si": initial["si"],
            "di": initial["di"],
            "bp": initial["bp"],
            "sp": initial["sp"],
            "ds": initial["ds"],
            "es": initial["es"],
            "gs": initial["gs"],
        }
        for register, value in expected_registers.items():
            actual_register = machine.reg_read(REGISTERS[register])
            if actual_register != value:
                raise AssertionError(
                    f"0xA141 {name} {register}={actual_register:#x}, expected={value:#x}"
                )

        gs_decoys = (
            struct.unpack(
                "<H", machine.mem_read(initial["gs"] * 16 + 0x0A86, 2)
            )[0],
            struct.unpack(
                "<H", machine.mem_read(initial["gs"] * 16 + 0x0D5B, 2)
            )[0],
            struct.unpack(
                "<4H", machine.mem_read(initial["gs"] * 16 + 0x0D60, 8)
            ),
        )
        if gs_decoys != (
            0xABCD,
            0xBCDE,
            (0x5555, 0x6666, 0x7777, 0x8888),
        ):
            raise AssertionError(f"0xA141 {name} accessed GS-owned decoy data")

        flags = machine.reg_read(UC_X86_REG_EFLAGS)
        actual_flags = {
            "carry": (flags >> 0) & 1,
            "parity": (flags >> 2) & 1,
            "zero": (flags >> 6) & 1,
            "sign": (flags >> 7) & 1,
            "overflow": (flags >> 11) & 1,
            "interrupt": (flags >> 9) & 1,
            "direction": (flags >> 10) & 1,
        }
        expected_flags = {
            "carry": 0,
            "parity": 1,
            "zero": 1,
            "sign": 0,
            "overflow": 0,
            "interrupt": 1,
            "direction": 1,
        }
        if actual_flags != expected_flags:
            raise AssertionError(
                f"0xA141 {name} flags={actual_flags}, expected={expected_flags}"
            )

        vectors.append(
            {
                "name": name,
                "initial_handle": handle,
                "reserved_handle": reserved_handle,
                "dos_error": dos_error,
                "closed": closed,
                "interrupts": interrupts,
                "result_handle": observed_handle,
                "result_bounds": list(observed_bounds),
                "result_registers": expected_registers,
                "final_flags": actual_flags,
            }
        )

    return vectors


def resource_palette_blocks_vectors() -> list[dict[str, object]]:
    data_segment = 0x2000
    stream_segment = 0x3000
    state_segment = 0x4000
    stream_offset = 0x1000
    cases = [
        ("immediate_terminator", [], 1, 0x1234, False),
        ("zero_count", [(7, b"")], 2, 0x5678, False),
        (
            "single_block_render_copy",
            [(2, bytes([0x3F, 0x20, 0x01, 0x02, 0x03, 0x04]))],
            3,
            0x9ABC,
            True,
        ),
        (
            "multiple_blocks_metric",
            [(1, bytes([9, 8, 7])), (5, bytes([1, 3, 5, 7, 9, 11]))],
            0,
            0x0040,
            False,
        ),
        ("metric_underflow", [(10, bytes([0xAA, 0xBB, 0xCC]))], 0, 3, True),
    ]
    vectors = []

    def parity_even(value: int) -> int:
        return int((value & 0xFF).bit_count() % 2 == 0)

    for name, blocks, wrap_index, initial_metric, copy_render_state in cases:
        stream = bytearray()
        for palette_start, payload in blocks:
            if len(payload) % 3 != 0:
                raise AssertionError(f"0xA0C3 {name} payload is not RGB-aligned")
            stream.extend((palette_start, len(payload) // 3))
            stream.extend(payload)
        stream.extend(b"\xff\xff")
        consumed = len(stream)

        palette = bytearray((index * 17 + 5) & 0xFF for index in range(768))
        expected_palette = bytearray(palette)
        for palette_start, payload in blocks:
            destination = palette_start * 3
            expected_palette[destination : destination + len(payload)] = payload

        render_destination = bytes(
            (index * 29 + 7) & 0xFF for index in range(0x180)
        )
        expected_render_destination = (
            bytes(expected_palette[:0x180])
            if copy_render_state
            else render_destination
        )
        stream_palette_decoy = bytes(
            (index * 31 + 11) & 0xFF for index in range(768)
        )
        data_stream_decoy = bytes(
            (index * 13 + 3) & 0xFF for index in range(len(stream))
        )

        initial = {
            "ax": 0x1111,
            "bx": 0x2222,
            "cx": 0x3333,
            "dx": 0x4444,
            "si": stream_offset,
            "di": 0x6666,
            "bp": 0x7777,
            "sp": 0xFF00,
            "ds": data_segment,
            "es": stream_segment,
            "gs": state_segment,
            "flags": 0x0293,
        }
        machine = execute(
            0xA0C3,
            0xA116,
            initial,
            [
                (data_segment, stream_offset, data_stream_decoy),
                (data_segment, 0x5251, bytes(palette)),
                (data_segment, 0x5851, render_destination),
                (stream_segment, stream_offset, bytes(stream)),
                (stream_segment, 0x5251, stream_palette_decoy),
                (
                    state_segment,
                    0x2751,
                    bytes([0 if copy_render_state else 1]),
                ),
                (state_segment, 0x0D60, struct.pack("<H", wrap_index)),
                (state_segment, 0x0DAF, struct.pack("<H", initial_metric)),
                (state_segment, 0x5B55, b"\xA5"),
            ],
        )

        actual_palette = bytes(
            machine.mem_read(data_segment * 16 + 0x5251, len(palette))
        )
        if actual_palette != bytes(expected_palette):
            raise AssertionError(f"0xA0C3 {name} produced an unexpected palette")
        actual_render_destination = bytes(
            machine.mem_read(data_segment * 16 + 0x5851, 0x180)
        )
        if actual_render_destination != expected_render_destination:
            raise AssertionError(
                f"0xA0C3 {name} produced an unexpected render-state copy"
            )
        if (
            machine.mem_read(stream_segment * 16 + 0x5251, 768)
            != stream_palette_decoy
        ):
            raise AssertionError(f"0xA0C3 {name} changed the stream-segment decoy")
        if (
            machine.mem_read(data_segment * 16 + stream_offset, len(stream))
            != data_stream_decoy
        ):
            raise AssertionError(f"0xA0C3 {name} read the data-segment decoy stream")
        if machine.mem_read(state_segment * 16 + 0x5B55, 1) != b"\x01":
            raise AssertionError(f"0xA0C3 {name} did not set the palette dirty flag")

        expected_metric = initial_metric
        if wrap_index == 0:
            remaining = (initial_metric - consumed) & 0xFFFF
            expected_metric = ((remaining >> 2) - 2) & 0xFFFF
        actual_metric = struct.unpack(
            "<H", machine.mem_read(state_segment * 16 + 0x0DAF, 2)
        )[0]
        if actual_metric != expected_metric:
            raise AssertionError(
                f"0xA0C3 {name} metric={actual_metric:#x}, "
                f"expected={expected_metric:#x}"
            )

        expected_di = initial["di"]
        if blocks:
            last_start, last_payload = blocks[-1]
            expected_di = 0x5251 + last_start * 3 + len(last_payload)
        if copy_render_state:
            expected_di = 0x5851 + 0x180
        expected_registers = {
            "ax": initial["ax"],
            "bx": initial["bx"],
            "cx": initial["cx"],
            "dx": initial["dx"],
            "si": (stream_offset + consumed) & 0xFFFF,
            "di": expected_di & 0xFFFF,
            "bp": initial["bp"],
            "sp": initial["sp"],
            "ds": initial["ds"],
            "es": initial["es"],
            "gs": initial["gs"],
        }
        for register, value in expected_registers.items():
            actual_register = machine.reg_read(REGISTERS[register])
            if actual_register != value:
                raise AssertionError(
                    f"0xA0C3 {name} {register}={actual_register:#x}, "
                    f"expected={value:#x}"
                )

        if wrap_index == 0:
            shifted = ((initial_metric - consumed) & 0xFFFF) >> 2
            flag_result = (shifted - 2) & 0xFFFF
            carry = int(shifted < 2)
        else:
            flag_result = wrap_index
            carry = 0
        expected_flags = {
            "carry": carry,
            "parity": parity_even(flag_result),
            "zero": int(flag_result == 0),
            "sign": (flag_result >> 15) & 1,
            "overflow": 0,
            "interrupt": 1,
            "direction": 0,
        }
        flags = machine.reg_read(UC_X86_REG_EFLAGS)
        actual_flags = {
            "carry": (flags >> 0) & 1,
            "parity": (flags >> 2) & 1,
            "zero": (flags >> 6) & 1,
            "sign": (flags >> 7) & 1,
            "overflow": (flags >> 11) & 1,
            "interrupt": (flags >> 9) & 1,
            "direction": (flags >> 10) & 1,
        }
        if actual_flags != expected_flags:
            raise AssertionError(
                f"0xA0C3 {name} flags={actual_flags}, expected={expected_flags}"
            )

        vectors.append(
            {
                "name": name,
                "blocks": [
                    {"start": start, "payload": list(payload)}
                    for start, payload in blocks
                ],
                "consumed_bytes": consumed,
                "copied_render_state": copy_render_state,
                "initial_wrap_index": wrap_index,
                "initial_metric": initial_metric,
                "result_metric": actual_metric,
                "result_stream_offset": expected_registers["si"],
                "result_di": expected_registers["di"],
                "final_flags": actual_flags,
            }
        )

    return vectors


def resource_switch_vectors() -> list[dict[str, object]]:
    data_segment = 0x2000
    buffer_segment = 0x3000
    dta_segment = 0x5000
    dta_offset = 0x0200
    record_offset = 0x3000
    buffer_end = 0xFFFE
    cases = [
        {
            "name": "banked_primary_bounds",
            "mode": "banked",
            "resource_id": 2,
            "record_flags": 0x00,
            "variant": 0x0C,
            "inner_skip": 6,
            "palette_blocks": [],
            "padding": 2,
            "render_copy": False,
            "archive_size": 0x00123456,
            "primary_relative": 0x00000121,
            "alternate_relative": 0x00000341,
            "index_relative": 0x00000561,
            "read_failure": None,
        },
        {
            "name": "embedded_alternate_bounds",
            "mode": "embedded",
            "resource_id": 5,
            "record_flags": 0x04,
            "variant": 0x07,
            "inner_skip": 10,
            "palette_blocks": [(3, bytes([1, 2, 3]))],
            "padding": 4,
            "render_copy": True,
            "archive_offset": 0x12345670,
            "archive_remaining": 0x01020304,
            "path_handle": 0x0042,
            "primary_relative": 0x00001121,
            "alternate_relative": 0x00002231,
            "index_relative": 0x00003341,
            "read_failure": None,
        },
        {
            "name": "external_file_success",
            "mode": "external",
            "resource_id": 8,
            "record_flags": 0x00,
            "variant": 0x09,
            "inner_skip": 14,
            "palette_blocks": [
                (1, bytes([0x3F, 0x20, 0x10, 0x08, 0x04, 0x02])),
                (9, bytes([])),
            ],
            "padding": 1,
            "render_copy": False,
            "file_size": 0x00020000,
            "open_handle": 0x0033,
            "old_handle": 0x0022,
            "primary_relative": 0x00000211,
            "alternate_relative": 0x00000421,
            "index_relative": 0x00000631,
            "read_failure": None,
        },
        {
            "name": "wrapped_inner_cursor",
            "mode": "banked",
            "resource_id": 11,
            "record_flags": 0x00,
            "variant": 0x02,
            "inner_skip": 0xFFFF,
            "palette_blocks": [],
            "padding": 3,
            "render_copy": False,
            "archive_size": 0x00018000,
            "primary_relative": 0x00000031,
            "alternate_relative": 0x00000051,
            "index_relative": 0x00000071,
            "read_failure": None,
        },
        {
            "name": "external_open_failure",
            "mode": "external",
            "resource_id": 13,
            "record_flags": 0x04,
            "variant": 0x05,
            "file_size": 0x00009999,
            "open_error": 2,
            "path_handle": 0x1357,
            "archive_offset": 0x11112222,
            "archive_remaining": 0x33334444,
            "read_failure": "open",
        },
        {
            "name": "initial_read_failure",
            "mode": "banked",
            "resource_id": 17,
            "record_flags": 0x00,
            "variant": 0x03,
            "archive_size": 0x00012345,
            "read_failure": "initial",
        },
        {
            "name": "body_read_failure",
            "mode": "embedded",
            "resource_id": 19,
            "record_flags": 0x00,
            "variant": 0x01,
            "archive_offset": 0x01020304,
            "archive_remaining": 0x00054321,
            "path_handle": 0x0055,
            "read_failure": "body",
        },
    ]
    vectors = []

    def read_u16(machine: Uc, offset: int) -> int:
        return struct.unpack(
            "<H", machine.mem_read(data_segment * 16 + offset, 2)
        )[0]

    def read_u32(machine: Uc, offset: int) -> int:
        return struct.unpack(
            "<I", machine.mem_read(data_segment * 16 + offset, 4)
        )[0]

    def write_u32(machine: Uc, offset: int, value: int) -> None:
        machine.mem_write(
            data_segment * 16 + offset, struct.pack("<I", value & 0xFFFFFFFF)
        )

    def set_carry(machine: Uc, carry: bool) -> None:
        flags = machine.reg_read(UC_X86_REG_EFLAGS)
        machine.reg_write(
            UC_X86_REG_EFLAGS, flags | 1 if carry else flags & ~1
        )

    for case_index, case in enumerate(cases):
        name = str(case["name"])
        mode = str(case["mode"])
        resource_id = int(case["resource_id"])
        record_flags = int(case["record_flags"])
        variant = int(case["variant"])
        read_failure = case["read_failure"]
        old_handle = int(case.get("old_handle", 0))
        archive_offset = int(case.get("archive_offset", 0x55667788))
        archive_remaining = int(case.get("archive_remaining", 0x11223344))
        archive_size = int(case.get("archive_size", 0x01010101))
        file_size = int(case.get("file_size", 0x00020000))
        path_handle = int(case.get("path_handle", 0x2468))
        open_handle = int(case.get("open_handle", 0x0033))

        palette_blocks = list(case.get("palette_blocks", []))
        palette_stream = bytearray()
        for palette_start, payload in palette_blocks:
            palette_stream.extend((palette_start, len(payload) // 3))
            palette_stream.extend(payload)
        palette_stream.extend(b"\xff\xff")
        palette_consumed = len(palette_stream)
        read_extent = palette_consumed + 32
        final_metric = 6

        buffer = bytearray(0x10000)
        if read_failure not in {"open", "initial", "body"}:
            inner_skip = int(case["inner_skip"])
            struct.pack_into("<H", buffer, 0, inner_skip)
            cursor_after_length = 2
            unwrapped_cursor = cursor_after_length + inner_skip
            if unwrapped_cursor > 0xFFFF or unwrapped_cursor > buffer_end:
                palette_offset = 0
            else:
                palette_offset = cursor_after_length
            buffer[
                palette_offset : palette_offset + palette_consumed
            ] = palette_stream
            metadata_offset = (palette_offset + palette_consumed) & 0xFFFF
            padding = int(case["padding"])
            buffer[metadata_offset : metadata_offset + padding] = bytes(
                [0xFF]
            ) * padding
            metadata_offset = (metadata_offset + padding) & 0xFFFF
            primary_relative = int(case["primary_relative"])
            alternate_relative = int(case["alternate_relative"])
            index_relative = int(case["index_relative"])
            struct.pack_into("<I", buffer, metadata_offset, primary_relative)
            struct.pack_into(
                "<I", buffer, metadata_offset + 0x10, alternate_relative
            )
            struct.pack_into(
                "<I",
                buffer,
                metadata_offset + final_metric * 4,
                index_relative,
            )
        else:
            metadata_offset = 0
            primary_relative = 0
            alternate_relative = 0
            index_relative = 0

        initial_palette = bytes(
            (index * 7 + 3) & 0x3F for index in range(768)
        )
        initial_render = bytes(
            (index * 11 + 5) & 0xFF for index in range(0x180)
        )
        expected_palette = bytearray(initial_palette)
        for palette_start, payload in palette_blocks:
            destination = palette_start * 3
            expected_palette[destination : destination + len(payload)] = payload

        table_offset = 0x1FB5 + resource_id * 4
        table_entry = struct.pack("<HH", record_offset, 0xA000 + resource_id)
        record = bytes([record_flags, 0xA5]) + b"RESOURCE.DAT\0"
        initial_eax = 0x89AB0000 | resource_id
        calls: list[dict[str, int | str]] = []

        def code_handler(
            machine: Uc,
            address: int,
            _size: int,
            case_name: str = name,
            call_log: list[dict[str, int | str]] = calls,
            selected_mode: str = mode,
            selected_path_handle: int = path_handle,
            selected_failure: object = read_failure,
            selected_read_extent: int = read_extent,
        ) -> None:
            if address == 0x9FCB:
                if machine.reg_read(UC_X86_REG_DX) != record_offset + 2:
                    raise AssertionError(
                        f"0x9F8E {case_name} passed an unexpected filename pointer"
                    )
                call_log.append({"call": "path", "filename": record_offset + 2})
                machine.reg_write(UC_X86_REG_BX, selected_path_handle)
                machine.mem_write(
                    data_segment * 16 + 0x0AE2,
                    bytes([1 if selected_mode == "embedded" else 0]),
                )
            elif address == 0xA021:
                call_log.append({"call": "initial_read", "bytes": 2})
                if selected_failure == "initial":
                    set_carry(machine, True)
                    return
                source_offset = read_u32(machine, 0x0D84)
                source_remaining = read_u32(machine, 0x0D88)
                write_u32(machine, 0x0D84, source_offset + 2)
                write_u32(machine, 0x0D88, source_remaining - 2)
                machine.reg_write(UC_X86_REG_AX, selected_read_extent)
                machine.reg_write(UC_X86_REG_ES, buffer_segment)
                machine.reg_write(UC_X86_REG_SI, read_u16(machine, 0x0D8C))
                set_carry(machine, False)
            elif address == 0xA03E:
                byte_count = machine.reg_read(UC_X86_REG_CX)
                call_log.append({"call": "body_read", "bytes": byte_count})
                if selected_failure == "body":
                    set_carry(machine, True)
                    return
                source_offset = read_u32(machine, 0x0D84)
                source_remaining = read_u32(machine, 0x0D88)
                write_u32(machine, 0x0D84, source_offset + byte_count)
                write_u32(machine, 0x0D88, source_remaining - byte_count)
                machine.reg_write(UC_X86_REG_AX, byte_count)
                set_carry(machine, False)

        def interrupt_handler(
            machine: Uc,
            number: int,
            case_name: str = name,
            call_log: list[dict[str, int | str]] = calls,
            selected_file_size: int = file_size,
            selected_find_error: bool = case_index % 2 == 0,
            selected_failure: object = read_failure,
            selected_open_error: int = int(case.get("open_error", 0)),
            selected_open_handle: int = open_handle,
        ) -> None:
            if number != 0x21:
                raise AssertionError(
                    f"0x9F8E {case_name} invoked unexpected INT {number:#x}"
                )
            function = machine.reg_read(UC_X86_REG_AX) >> 8
            if function == 0x3E:
                call_log.append(
                    {"call": "close", "handle": machine.reg_read(UC_X86_REG_BX)}
                )
                set_carry(machine, False)
            elif function == 0x2F:
                call_log.append({"call": "get_dta"})
                machine.reg_write(UC_X86_REG_ES, dta_segment)
                machine.reg_write(UC_X86_REG_BX, dta_offset)
            elif function == 0x4E:
                call_log.append({"call": "find_first"})
                machine.mem_write(
                    dta_segment * 16 + dta_offset + 0x1A,
                    struct.pack("<I", selected_file_size),
                )
                set_carry(machine, selected_find_error)
            elif function == 0x3D:
                call_log.append({"call": "open"})
                if selected_failure == "open":
                    machine.reg_write(UC_X86_REG_AX, selected_open_error)
                    set_carry(machine, True)
                else:
                    machine.reg_write(UC_X86_REG_AX, selected_open_handle)
                    set_carry(machine, False)
            else:
                raise AssertionError(
                    f"0x9F8E {case_name} invoked unexpected DOS function "
                    f"{function:#x}"
                )

        memory = [
            (0, 0x9FCB, b"\x90" * 5),
            (0, 0xA021, b"\x90" * 3),
            (0, 0xA03E, b"\x90" * 3),
            (data_segment, 0x0A52, struct.pack("<I", archive_size)),
            (data_segment, 0x0A7E, struct.pack("<H", buffer_segment)),
            (data_segment, 0x0A86, struct.pack("<H", 0x7FFF)),
            (data_segment, 0x0A8A, struct.pack("<I", archive_offset)),
            (data_segment, 0x0A8E, struct.pack("<I", archive_remaining)),
            (data_segment, 0x0AE2, b"\xA5"),
            (data_segment, 0x0D5B, struct.pack("<H", old_handle)),
            (data_segment, 0x0D60, struct.pack("<4H", 9, 8, 7, 6)),
            (data_segment, 0x0DBC, bytes([1 if mode == "banked" else 0])),
            (data_segment, 0x1FB1, bytes([variant])),
            (data_segment, table_offset, table_entry),
            (data_segment, record_offset, record),
            (data_segment, 0x2751, bytes([0 if case.get("render_copy") else 1])),
            (data_segment, 0x5233, struct.pack("<H", buffer_end)),
            (data_segment, 0x5251, initial_palette),
            (data_segment, 0x5851, initial_render),
            (buffer_segment, 0, bytes(buffer)),
        ]
        machine = execute(
            0x9F8E,
            0xA0C2,
            {
                "eax": initial_eax,
                "bx": 0x2222,
                "cx": 0x3333,
                "dx": 0x4444,
                "si": 0x5555,
                "di": 0x6666,
                "bp": 0x7777,
                "sp": 0xFF00,
                "ds": data_segment,
                "es": 0x4000,
                "gs": data_segment,
                "flags": 0x0202,
            },
            memory,
            interrupt_handler,
            code_handler,
        )

        expected_success = read_failure is None
        carry = machine.reg_read(UC_X86_REG_EFLAGS) & 1
        if carry != int(not expected_success):
            raise AssertionError(
                f"0x9F8E {name} carry={carry}, expected={int(not expected_success)}"
            )
        if machine.reg_read(UC_X86_REG_EAX) != initial_eax:
            raise AssertionError(f"0x9F8E {name} did not preserve EAX")
        if machine.reg_read(UC_X86_REG_SP) != 0xFF00:
            raise AssertionError(f"0x9F8E {name} did not restore SP")
        if read_u16(machine, 0x0D80) != resource_id:
            raise AssertionError(f"0x9F8E {name} stored an unexpected requested id")
        if read_u16(machine, 0x0D82) != resource_id:
            raise AssertionError(f"0x9F8E {name} stored an unexpected active id")
        expected_flags_word = record_flags | variant << 8
        if read_u16(machine, 0x0D76) != expected_flags_word:
            raise AssertionError(f"0x9F8E {name} stored unexpected resource flags")
        if machine.mem_read(data_segment * 16 + record_offset + 1, 1) != bytes(
            [variant]
        ):
            raise AssertionError(f"0x9F8E {name} did not update the record variant")
        if machine.mem_read(data_segment * 16 + 0x0D5F, 1) != b"\x00":
            raise AssertionError(f"0x9F8E {name} did not clear list state")
        if struct.unpack(
            "<4H", machine.mem_read(data_segment * 16 + 0x0D60, 8)
        ) != (0, 0, 0xFFFF, 0xFFFF):
            raise AssertionError(f"0x9F8E {name} did not reset list bounds")

        if mode == "banked":
            source_base = 0
            source_total = archive_size
            expected_handle = 0
            expected_path_calls = 0
            expected_dos_calls = 0
        elif mode == "embedded":
            source_base = archive_offset
            source_total = archive_remaining
            expected_handle = path_handle
            expected_path_calls = 1
            expected_dos_calls = 0
        else:
            source_base = 0 if read_failure != "open" else archive_offset
            source_total = file_size
            expected_handle = (
                path_handle if read_failure == "open" else open_handle
            )
            expected_path_calls = 1
            expected_dos_calls = 3

        actual_path_calls = sum(call["call"] == "path" for call in calls)
        actual_dos_calls = sum(
            call["call"] in {"get_dta", "find_first", "open"} for call in calls
        )
        if actual_path_calls != expected_path_calls:
            raise AssertionError(f"0x9F8E {name} produced unexpected path calls")
        if actual_dos_calls != expected_dos_calls:
            raise AssertionError(f"0x9F8E {name} produced unexpected DOS calls")
        if read_u16(machine, 0x0D5B) != expected_handle:
            raise AssertionError(f"0x9F8E {name} stored an unexpected file handle")

        expected_source_offset = source_base
        expected_source_remaining = source_total
        if read_failure not in {"open", "initial"}:
            expected_source_offset = (expected_source_offset + 2) & 0xFFFFFFFF
            expected_source_remaining = (
                expected_source_remaining - 2
            ) & 0xFFFFFFFF
        if read_failure is None:
            expected_source_offset = (
                expected_source_offset + read_extent - 2
            ) & 0xFFFFFFFF
            expected_source_remaining = (
                expected_source_remaining - (read_extent - 2)
            ) & 0xFFFFFFFF
        if read_u32(machine, 0x0D84) != expected_source_offset:
            raise AssertionError(f"0x9F8E {name} stored an unexpected source offset")
        if read_u32(machine, 0x0D88) != expected_source_remaining:
            raise AssertionError(
                f"0x9F8E {name} stored unexpected source remaining bytes"
            )

        if expected_success:
            actual_metric = read_u16(machine, 0x0DAF)
            if actual_metric != final_metric:
                raise AssertionError(
                    f"0x9F8E {name} metric={actual_metric:#x}, "
                    f"expected={final_metric:#x}"
                )
            selected_relative = (
                alternate_relative if record_flags & 4 else primary_relative
            )
            expected_ranges = (
                (expected_source_offset + selected_relative) & 0xFFFFFFFF,
                (expected_source_remaining - selected_relative) & 0xFFFFFFFF,
                (expected_source_offset + index_relative) & 0xFFFFFFFF,
                (expected_source_remaining - index_relative) & 0xFFFFFFFF,
            )
            actual_ranges = tuple(
                read_u32(machine, offset)
                for offset in (0x0D6E, 0x0D72, 0x0D78, 0x0D7C)
            )
            if actual_ranges != expected_ranges:
                raise AssertionError(
                    f"0x9F8E {name} ranges={actual_ranges}, "
                    f"expected={expected_ranges}"
                )
            if machine.mem_read(data_segment * 16 + 0x0DB7, 1) != b"\xff":
                raise AssertionError(f"0x9F8E {name} did not set resource marker")
            actual_palette = bytes(
                machine.mem_read(data_segment * 16 + 0x5251, 768)
            )
            if actual_palette != bytes(expected_palette):
                raise AssertionError(f"0x9F8E {name} produced unexpected palette")
            expected_render = (
                bytes(expected_palette[:0x180])
                if case.get("render_copy")
                else initial_render
            )
            if machine.mem_read(
                data_segment * 16 + 0x5851, 0x180
            ) != expected_render:
                raise AssertionError(
                    f"0x9F8E {name} produced unexpected render-state bytes"
                )

        vectors.append(
            {
                "name": name,
                "mode": mode,
                "resource_id": resource_id,
                "record_flags": expected_flags_word,
                "success": expected_success,
                "calls": calls,
                "source_offset": read_u32(machine, 0x0D84),
                "source_remaining": read_u32(machine, 0x0D88),
                "file_handle": read_u16(machine, 0x0D5B),
                "entry_metric": read_u16(machine, 0x0DAF),
                "metadata_offset": metadata_offset,
                "range_start": read_u32(machine, 0x0D6E),
                "range_remaining": read_u32(machine, 0x0D72),
                "index_start": read_u32(machine, 0x0D78),
                "index_remaining": read_u32(machine, 0x0D7C),
                "carry": carry,
                "preserved_eax": machine.reg_read(UC_X86_REG_EAX),
            }
        )

    return vectors


def update_vector(path: Path, vectors: list[dict[str, object]], check: bool) -> None:
    encoded = json.dumps(vectors, indent=2) + "\n"
    if check:
        if not path.is_file() or path.read_text(encoding="ascii") != encoded:
            raise SystemExit(f"{path}: stale or missing; regenerate without --check")
        print(f"OK: {path.relative_to(REPO_ROOT)} ({len(vectors)} vectors)")
        return

    path.write_text(encoded, encoding="ascii")
    print(f"wrote {path.relative_to(REPO_ROOT)} ({len(vectors)} vectors)")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--check", action="store_true", help="fail unless committed vectors are current"
    )
    args = parser.parse_args()

    VECTOR_ROOT.mkdir(parents=True, exist_ok=True)
    update_vector(
        VECTOR_ROOT / "func_093b_natural.json", rtc_time_read_vectors(), args.check
    )
    update_vector(
        VECTOR_ROOT / "func_0986_natural.json", bcd_to_binary_vectors(), args.check
    )
    update_vector(
        VECTOR_ROOT / "func_0cc0_natural.json",
        set_video_mode_saved_vectors(),
        args.check,
    )
    update_vector(
        VECTOR_ROOT / "func_0b32_natural.json", detect_cdrom_vectors(), args.check
    )
    update_vector(
        VECTOR_ROOT / "func_0d4a_natural.json", mouse_set_range_vectors(), args.check
    )
    update_vector(
        VECTOR_ROOT / "func_0d61_natural.json",
        print_string_dos_vectors(),
        args.check,
    )
    update_vector(
        VECTOR_ROOT / "func_267d_natural.json", keyboard_read_vectors(), args.check
    )
    update_vector(
        VECTOR_ROOT / "func_2dd3_natural.json", cmos_rtc_read_vectors(), args.check
    )
    update_vector(
        VECTOR_ROOT / "func_2f90_natural.json",
        vga_palette_write_vectors(),
        args.check,
    )
    update_vector(
        VECTOR_ROOT / "func_2fa6_natural.json", vga_dac_clear_vectors(), args.check
    )
    update_vector(
        VECTOR_ROOT / "func_30cd_natural.json", text_width_vectors(), args.check
    )
    update_vector(
        VECTOR_ROOT / "func_41d1_natural.json",
        entity_flag_state_transition_vectors(),
        args.check,
    )
    update_vector(
        VECTOR_ROOT / "func_420d_natural.json",
        sprite_slot_position_update_vectors(),
        args.check,
    )
    update_vector(
        VECTOR_ROOT / "func_42cd_natural.json",
        sprite_slot_extent_update_vectors(),
        args.check,
    )
    update_vector(
        VECTOR_ROOT / "func_4240_natural.json",
        sprite_slot_range_mark_dirty_vectors(),
        args.check,
    )
    update_vector(
        VECTOR_ROOT / "func_43f7_natural.json",
        sprite_slot_commit_dirty_range_vectors(),
        args.check,
    )
    update_vector(
        VECTOR_ROOT / "func_4471_natural.json",
        sprite_slot_dirty_range_render_vectors(),
        args.check,
    )
    update_vector(
        VECTOR_ROOT / "func_4536_natural.json",
        sprite_blit_raw_transparent_vectors(),
        args.check,
    )
    update_vector(
        VECTOR_ROOT / "func_46bc_natural.json",
        sprite_blit_rle_transparent_vectors(),
        args.check,
    )
    update_vector(
        VECTOR_ROOT / "func_4ba8_natural.json",
        sprite_blit_raw_opaque_vectors(),
        args.check,
    )
    update_vector(
        VECTOR_ROOT / "func_4cd6_natural.json",
        sprite_blit_rle_opaque_vectors(),
        args.check,
    )
    update_vector(
        VECTOR_ROOT / "func_4f62_natural.json",
        sprite_blit_scaled_transparent_vectors(),
        args.check,
    )
    update_vector(
        VECTOR_ROOT / "func_509a_natural.json",
        sprite_blitter_noop_vectors(0x509A),
        args.check,
    )
    update_vector(
        VECTOR_ROOT / "func_509b_natural.json",
        sprite_blitter_noop_vectors(0x509B),
        args.check,
    )
    update_vector(
        VECTOR_ROOT / "func_509c_natural.json",
        sprite_blitter_noop_vectors(0x509C),
        args.check,
    )
    update_vector(
        VECTOR_ROOT / "func_509d_natural.json",
        dirty_rects_copy_secondary_to_primary_vectors(),
        args.check,
    )
    update_vector(
        VECTOR_ROOT / "func_5288_natural.json",
        resource_release_vectors(),
        args.check,
    )
    update_vector(
        VECTOR_ROOT / "func_529c_natural.json",
        resource_free_inner_vectors(),
        args.check,
    )
    update_vector(
        VECTOR_ROOT / "func_5320_natural.json",
        resource_handle_resolve_vectors(),
        args.check,
    )
    update_vector(
        VECTOR_ROOT / "func_533c_natural.json",
        resource_get_field4_vectors(),
        args.check,
    )
    update_vector(
        VECTOR_ROOT / "func_5fd8_natural.json",
        vm_special_slot_remove_vectors(),
        args.check,
    )
    update_vector(
        VECTOR_ROOT / "func_5ff6_natural.json",
        vm_special_slot_insert_vectors(),
        args.check,
    )
    update_vector(
        VECTOR_ROOT / "func_6023_natural.json",
        vm_field_offset_vectors(),
        args.check,
    )
    update_vector(
        VECTOR_ROOT / "func_6034_natural.json",
        vm_record_lookup_by_threshold_vectors(),
        args.check,
    )
    update_vector(
        VECTOR_ROOT / "func_604e_natural.json",
        active_object_list_vectors(),
        args.check,
    )
    update_vector(
        VECTOR_ROOT / "func_60dd_natural.json",
        ship_3d_position_distance_vectors(),
        args.check,
    )
    update_vector(
        VECTOR_ROOT / "func_61a6_natural.json",
        ship_3d_position_field_resolve_vectors(),
        args.check,
    )
    update_vector(
        VECTOR_ROOT / "func_6210_natural.json",
        ship_3d_object_table_bit_test_vectors(),
        args.check,
    )
    update_vector(
        VECTOR_ROOT / "func_624b_natural.json",
        ship_3d_nav_source_list_build_vectors(),
        args.check,
    )
    update_vector(
        VECTOR_ROOT / "func_6293_natural.json", vm_token_special_vectors(), args.check
    )
    update_vector(
        VECTOR_ROOT / "func_6339_natural.json", vm_condition_5_vectors(), args.check
    )
    update_vector(
        VECTOR_ROOT / "func_6433_natural.json", dic_word_lookup_vectors(), args.check
    )
    update_vector(
        VECTOR_ROOT / "func_6462_natural.json", vm_branch_vectors(), args.check
    )
    update_vector(
        VECTOR_ROOT / "func_647b_natural.json", scan_zero_word_vectors(), args.check
    )
    update_vector(
        VECTOR_ROOT / "func_6494_natural.json",
        vm_conditional_branch_vectors(0x6494, 0x649F, 0x2793),
        args.check,
    )
    update_vector(
        VECTOR_ROOT / "func_64a0_natural.json",
        vm_conditional_branch_vectors(0x64A0, 0x64AB, 0x252A),
        args.check,
    )
    update_vector(
        VECTOR_ROOT / "func_64ac_natural.json",
        vm_conditional_branch_vectors(0x64AC, 0x64B7, 0x274F),
        args.check,
    )
    update_vector(
        VECTOR_ROOT / "func_64b8_natural.json",
        vm_script_profile_request_vectors(),
        args.check,
    )
    update_vector(
        VECTOR_ROOT / "func_64c0_natural.json", vm_clear_state_vectors(), args.check
    )
    update_vector(
        VECTOR_ROOT / "func_64ce_natural.json",
        vm_record_string_copy_vectors(),
        args.check,
    )
    update_vector(
        VECTOR_ROOT / "func_64e5_natural.json",
        vm_tagged_word_compare_vectors(),
        args.check,
    )
    update_vector(
        VECTOR_ROOT / "func_6510_natural.json",
        vm_tagged_byte_pair_compare_vectors(),
        args.check,
    )
    update_vector(
        VECTOR_ROOT / "func_6559_natural.json",
        vm_branch_stack_push_vectors(),
        args.check,
    )
    update_vector(
        VECTOR_ROOT / "func_6572_natural.json",
        vm_branch_stack_pop_vectors(),
        args.check,
    )
    update_vector(
        VECTOR_ROOT / "func_6588_natural.json",
        vm_random_branch_vectors(),
        args.check,
    )
    update_vector(
        VECTOR_ROOT / "func_6596_natural.json",
        vm_conditional_block_vectors(),
        args.check,
    )
    update_vector(
        VECTOR_ROOT / "func_65db_natural.json",
        vm_script_jump_vectors(),
        args.check,
    )
    update_vector(
        VECTOR_ROOT / "func_65eb_natural.json",
        vm_cond_state_array_vectors(),
        args.check,
    )
    update_vector(
        VECTOR_ROOT / "func_660c_natural.json",
        vm_text_handler_vectors(),
        args.check,
    )
    update_vector(
        VECTOR_ROOT / "func_67a7_natural.json",
        strlen_b_vectors(),
        args.check,
    )
    update_vector(
        VECTOR_ROOT / "func_67ba_natural.json",
        vm_presentation_register_set_vectors(),
        args.check,
    )
    update_vector(
        VECTOR_ROOT / "func_67c8_natural.json",
        vm_load_string_vectors(),
        args.check,
    )
    update_vector(
        VECTOR_ROOT / "func_6830_natural.json",
        vm_conditional_jump_vectors(),
        args.check,
    )
    update_vector(
        VECTOR_ROOT / "func_684c_natural.json",
        vm_poke_byte_vectors(),
        args.check,
    )
    update_vector(
        VECTOR_ROOT / "func_6855_natural.json",
        vm_yield_vectors(0x6855),
        args.check,
    )
    update_vector(
        VECTOR_ROOT / "func_685c_natural.json",
        vm_yield_vectors(0x685C),
        args.check,
    )
    update_vector(
        VECTOR_ROOT / "func_6863_natural.json",
        vm_shared_state_vectors(),
        args.check,
    )
    update_vector(
        VECTOR_ROOT / "func_6902_natural.json",
        vm_shared_bit_state_vectors(),
        args.check,
    )
    update_vector(
        VECTOR_ROOT / "func_6946_natural.json",
        vm_record_wildcard_vectors(),
        args.check,
    )
    update_vector(
        VECTOR_ROOT / "func_69c7_natural.json",
        vm_cd_record_triple_vectors(),
        args.check,
    )
    update_vector(
        VECTOR_ROOT / "func_6aa7_natural.json",
        vm_b7_record_bit_vectors(),
        args.check,
    )
    update_vector(
        VECTOR_ROOT / "func_6b06_natural.json",
        vm_b8_record_pair_vectors(),
        args.check,
    )
    update_vector(
        VECTOR_ROOT / "func_6d18_natural.json",
        vm_c5_record_match_vectors(),
        args.check,
    )
    update_vector(
        VECTOR_ROOT / "func_6d80_natural.json",
        vm_c6_record_match_vectors(),
        args.check,
    )
    update_vector(
        VECTOR_ROOT / "func_6dcf_natural.json",
        vm_c7_record_match_vectors(),
        args.check,
    )
    update_vector(
        VECTOR_ROOT / "func_6f62_natural.json",
        vm_c8_record_match_vectors(),
        args.check,
    )
    update_vector(
        VECTOR_ROOT / "func_6fb9_natural.json",
        vm_c9_clear_record_vectors(),
        args.check,
    )
    update_vector(
        VECTOR_ROOT / "func_7542_natural.json",
        byte_parser_mark_b16_vectors(0x7542, 0x01),
        args.check,
    )
    update_vector(
        VECTOR_ROOT / "func_7549_natural.json",
        byte_parser_mark_b16_vectors(0x7549, 0x02),
        args.check,
    )
    update_vector(
        VECTOR_ROOT / "func_7550_natural.json",
        byte_parser_mark_b16_vectors(0x7550, 0x0F),
        args.check,
    )
    update_vector(
        VECTOR_ROOT / "func_7557_natural.json",
        byte_parser_mark_b16_vectors(0x7557, 0x04),
        args.check,
    )
    update_vector(
        VECTOR_ROOT / "func_7612_natural.json",
        credit_presenter_b_cryo_vectors(),
        args.check,
    )
    update_vector(
        VECTOR_ROOT / "func_7629_natural.json",
        byte_parser_copy_printable_vectors(0x7629, 0x20B8, 0x06),
        args.check,
    )
    update_vector(
        VECTOR_ROOT / "func_763e_natural.json",
        byte_parser_snd_bank_name_load_vectors(),
        args.check,
    )
    update_vector(
        VECTOR_ROOT / "func_7684_natural.json",
        dlg_line_asset_table_fill_vectors(),
        args.check,
    )
    update_vector(
        VECTOR_ROOT / "func_76ba_natural.json",
        byte_parser_store_word_1fa5_vectors(),
        args.check,
    )
    update_vector(
        VECTOR_ROOT / "func_766f_natural.json",
        byte_parser_copy_printable_vectors(0x766F, 0x24C6, 0x10),
        args.check,
    )
    update_vector(
        VECTOR_ROOT / "func_76c0_natural.json",
        byte_parser_copy_printable_vectors(0x76C0, 0x2460, 0x09),
        args.check,
    )
    update_vector(
        VECTOR_ROOT / "func_76d5_natural.json",
        byte_parser_copy_printable_vectors(0x76D5, 0x247A, 0x0A),
        args.check,
    )
    update_vector(
        VECTOR_ROOT / "func_76ea_natural.json",
        index_lookup_1fd7_vectors(),
        args.check,
    )
    update_vector(
        VECTOR_ROOT / "func_7754_natural.json",
        byte_parser_copy_131a_entry_vectors(),
        args.check,
    )
    update_vector(
        VECTOR_ROOT / "func_7776_natural.json",
        byte_parser_stream_0f18_append_vectors(),
        args.check,
    )
    update_vector(
        VECTOR_ROOT / "func_7788_natural.json",
        fs_name_area_read_vectors(),
        args.check,
    )
    update_vector(
        VECTOR_ROOT / "func_77a9_natural.json",
        music_voc_name_patcher_vectors(),
        args.check,
    )
    update_vector(
        VECTOR_ROOT / "func_7e1c_natural.json",
        presentation_line_helper_vectors(),
        args.check,
    )
    update_vector(
        VECTOR_ROOT / "func_8713_natural.json",
        nav_choice_handler_vectors(0x8713),
        args.check,
    )
    update_vector(
        VECTOR_ROOT / "func_8848_natural.json",
        nav_choice_handler_vectors(0x8848),
        args.check,
    )
    update_vector(
        VECTOR_ROOT / "func_933a_natural.json",
        back_buffer_copy_from_vectors(),
        args.check,
    )
    update_vector(
        VECTOR_ROOT / "func_9510_natural.json",
        presentation_mode_bits_update_vectors(),
        args.check,
    )
    update_vector(
        VECTOR_ROOT / "func_963f_natural.json",
        matrix_table_clear_2a1b_vectors(),
        args.check,
    )
    update_vector(
        VECTOR_ROOT / "func_98b9_natural.json",
        ship_3d_projection_matrix_build_vectors(),
        args.check,
    )
    update_vector(
        VECTOR_ROOT / "func_9b04_natural.json",
        ship_3d_plot_point_vectors(),
        args.check,
    )
    update_vector(
        VECTOR_ROOT / "func_7cb4_natural.json", mask_overlay_vectors(), args.check
    )
    update_vector(
        VECTOR_ROOT / "func_a3d0_natural.json", queue_consume_vectors(), args.check
    )
    update_vector(
        VECTOR_ROOT / "func_a38e_natural.json", queue_wrap_vectors(), args.check
    )
    update_vector(
        VECTOR_ROOT / "func_a3ad_natural.json", queue_room_vectors(), args.check
    )
    update_vector(
        VECTOR_ROOT / "func_a40b_natural.json", queue_state_le_one_vectors(), args.check
    )
    update_vector(
        VECTOR_ROOT / "func_a634_natural.json", flag_test_b17_vectors(), args.check
    )
    update_vector(
        VECTOR_ROOT / "func_a622_natural.json", list_read_vectors(), args.check
    )
    update_vector(
        VECTOR_ROOT / "func_a20c_natural.json",
        list_d8c_activate_ready_vectors(),
        args.check,
    )
    update_vector(
        VECTOR_ROOT / "func_a240_natural.json",
        list_d8c_advance_due_vectors(),
        args.check,
    )
    update_vector(
        VECTOR_ROOT / "func_a642_natural.json",
        banked_list_load_vectors(),
        args.check,
    )
    update_vector(
        VECTOR_ROOT / "func_a664_natural.json", ems_paged_read_vectors(), args.check
    )
    update_vector(
        VECTOR_ROOT / "func_a734_natural.json", queue_enqueue_vectors(), args.check
    )
    update_vector(
        VECTOR_ROOT / "func_a757_natural.json", list_init_vectors(), args.check
    )
    update_vector(
        VECTOR_ROOT / "func_a778_natural.json",
        list_d8c_palette_blocks_apply_vectors(),
        args.check,
    )
    update_vector(
        VECTOR_ROOT / "func_a7e6_natural.json", mem_copy_words_vectors(), args.check
    )
    update_vector(
        VECTOR_ROOT / "func_ad96_natural.json",
        gfx_scanline_advance_vectors(),
        args.check,
    )
    update_vector(
        VECTOR_ROOT / "func_a117_natural.json", flag_gated_copy_vectors(), args.check
    )
    update_vector(
        VECTOR_ROOT / "func_a2dd_natural.json",
        presentation_queue_finish_vectors(),
        args.check,
    )
    update_vector(
        VECTOR_ROOT / "func_9f80_natural.json",
        resource_descriptor_lookup_vectors(),
        args.check,
    )
    update_vector(
        VECTOR_ROOT / "func_9f53_natural.json",
        presentation_update_vectors(),
        args.check,
    )
    update_vector(
        VECTOR_ROOT / "func_a141_natural.json",
        close_file_vectors(),
        args.check,
    )
    update_vector(
        VECTOR_ROOT / "func_a0c3_natural.json",
        resource_palette_blocks_vectors(),
        args.check,
    )
    update_vector(
        VECTOR_ROOT / "func_9f8e_natural.json",
        resource_switch_vectors(),
        args.check,
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
