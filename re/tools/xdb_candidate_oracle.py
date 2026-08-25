#!/usr/bin/env python3
"""Verify selected natural-C semantics against direct XDB overlay execution."""

from __future__ import annotations

import os
import sys

_HERE = os.path.dirname(os.path.abspath(__file__))
sys.path[:] = [
    path for path in sys.path if os.path.abspath(path or os.curdir) != _HERE
]

import argparse
import hashlib
import importlib.util
import json
import struct
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


_COVERAGE_SPEC = importlib.util.spec_from_file_location(
    "oracle_branch_coverage", Path(_HERE) / "oracle_branch_coverage.py"
)
assert _COVERAGE_SPEC is not None and _COVERAGE_SPEC.loader is not None
oracle_branch_coverage = importlib.util.module_from_spec(_COVERAGE_SPEC)
sys.modules[_COVERAGE_SPEC.name] = oracle_branch_coverage
_COVERAGE_SPEC.loader.exec_module(oracle_branch_coverage)
CoverageRecorder = oracle_branch_coverage.CoverageRecorder
build_coverage_report = oracle_branch_coverage.build_report
require_complete_direct_coverage = (
    oracle_branch_coverage.require_complete_direct_coverage
)
require_reviewed_direct_coverage = (
    oracle_branch_coverage.require_reviewed_direct_coverage
)
update_coverage_report = oracle_branch_coverage.update_report


REPO_ROOT = Path(__file__).resolve().parents[2]
VECTOR_ROOT = REPO_ROOT / "re/tools/oracle_vectors"
IMAGE_PATHS = {
    "amer": REPO_ROOT / "output/_tmp_dat/amer.xdb",
    "croolis": REPO_ROOT / "output/_tmp_dat/croolis.xdb",
    "manu3": REPO_ROOT / "output/_tmp_dat/manu3.xdb",
    "scrut": REPO_ROOT / "output/_tmp_dat/scrut.xdb",
}
IMAGE_HASHES = {
    "amer": "6fddeb5cc7c62fe5e638900746decc299f1637ea11612c0c51aced367dd12b31",
    "croolis": "13eba7c4e4f38662c44c849bd07be293c7583d2b2dc9ae273d7fe94746048c31",
    "manu3": "d0f64e99a646197906e273edfa0124172307a5cd766c88591c12ebd9ea556d31",
    "scrut": "8522e77aad6639cf9bb06f148048fed3c6bf1cfbe6459c9a0be7b987c3c3ac77",
}
REGISTERS = {
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
    "flags": UC_X86_REG_EFLAGS,
}
XDB_MANIFEST = REPO_ROOT / "re/source/xdb/candidates/manifest.tsv"
XDB_COVERAGE_REPORT = REPO_ROOT / "re/source/xdb/oracle_branch_coverage.tsv"
XDB_COVERAGE_REVIEWS = REPO_ROOT / "re/source/xdb/oracle_branch_coverage_reviews.tsv"
_COVERAGE_RECORDER: CoverageRecorder | None = None


def load_image(module: str) -> bytes:
    path = IMAGE_PATHS[module]
    if not path.is_file():
        raise SystemExit(f"{path}: missing extracted XDB artifact")
    image = path.read_bytes()
    actual_hash = hashlib.sha256(image).hexdigest()
    if actual_hash != IMAGE_HASHES[module]:
        raise SystemExit(
            f"{path}: sha256 {actual_hash} does not match {IMAGE_HASHES[module]}"
        )
    return image


def execute(
    image: bytes,
    entry: int,
    return_address: int,
    registers: dict[str, int],
    memory: list[tuple[int, int, bytes]],
    interrupt_handler: object | None = None,
    max_instructions: int = 1000,
    input_handler: object | None = None,
    output_handler: object | None = None,
    code_handler: object | None = None,
    code_segment: int = 0,
    return_segment: int | None = None,
) -> Uc:
    if return_segment is None:
        return_segment = code_segment
    machine = Uc(UC_ARCH_X86, UC_MODE_16)
    machine.mem_map(0, 0x300000)
    code_base = code_segment * 16
    machine.mem_write(code_base, image)
    machine.reg_write(UC_X86_REG_CS, code_segment)
    for name, value in registers.items():
        machine.reg_write(REGISTERS[name], value)
    for segment, offset, data in memory:
        machine.mem_write(segment * 16 + offset, data)

    returned = []

    def stop_at_return(
        machine: Uc, address: int, _size: int, _data: object
    ) -> None:
        if address == return_segment * 16 + return_address:
            returned.append(address)
            machine.emu_stop()

    machine.hook_add(UC_HOOK_CODE, stop_at_return)
    if _COVERAGE_RECORDER is not None:
        machine.hook_add(
            UC_HOOK_CODE,
            _COVERAGE_RECORDER.hook_for(image, code_segment),
        )
    if code_handler is not None:
        machine.hook_add(UC_HOOK_CODE, code_handler)
    if interrupt_handler is not None:
        machine.hook_add(UC_HOOK_INTR, interrupt_handler)
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
    try:
        machine.emu_start(code_base + entry, 0x2FFFF0, count=max_instructions)
    except UcError as error:
        raise RuntimeError(
            f"{entry:#x}: execution failed at "
            f"{machine.reg_read(UC_X86_REG_CS):#x}:"
            f"{machine.reg_read(UC_X86_REG_IP):#x}; "
            f"sp={machine.reg_read(UC_X86_REG_SP):#x}"
        ) from error
    if not returned:
        raise RuntimeError(f"{entry:#x}: did not reach return at {return_address:#x}")
    return machine


def mouse_position_vectors(module: str, entry: int) -> list[dict[str, object]]:
    image = load_image(module)
    expected_hash = "6eef96589bdec402ce6079bdeac73e81b55468ed5c9e7ed666225fd3145ffe32"
    if hashlib.sha256(image[entry : entry + 14]).hexdigest() != expected_hash:
        raise AssertionError(f"{module}:{entry:#x}: recovered 14-byte body changed")

    cases = [
        ("origin", 0x0000, 0x0000),
        ("ordinary", 0x00A0, 0x0064),
        ("x_high_bit", 0x8000, 0x1234),
        ("y_high_bit", 0x4321, 0x8000),
        ("maximum", 0xFFFF, 0xFFFF),
        ("mixed", 0xA55A, 0x5AA5),
    ]
    data_segment = 0x4400
    extra_segment = 0x4800
    game_segment = 0x2C00
    stack_segment = 0x9000
    return_address = 0xF000
    driver_flags = (0x0202, 0x0AD7, 0x0646, 0x0283, 0x0A12, 0x0643)
    vectors = []

    for case_index, (name, x, y) in enumerate(cases):
        stack_sentinel = bytes.fromhex("5aa596698778")
        globals_before = bytes.fromhex("a1b2c3d4e5f6a7b8")
        globals_after = globals_before[:2] + struct.pack("<HH", x, y) + globals_before[6:]
        initial = {
            "eax": 0xA1A1BEEF + case_index,
            "ebx": 0xB2B22345 + case_index,
            "ecx": 0xC3C30000 | x,
            "edx": 0xD4D40000 | y,
            "esi": 0xE5E55678 + case_index,
            "edi": 0xF6F66789 + case_index,
            "ebp": 0x9797789A + case_index,
            "sp": 0xFF00,
            "ds": data_segment,
            "es": extra_segment,
            "fs": 0x4C00,
            "gs": game_segment,
            "ss": stack_segment,
            "flags": 0x0A93 | (0x0400 if case_index & 1 else 0),
        }
        driver_ax = (0x6100 + case_index) & 0xFFFF
        interrupts: list[dict[str, int | bytes]] = []

        def interrupt_handler(
            machine: Uc, interrupt_number: int, _data: object
        ) -> None:
            interrupts.append(
                {
                    "number": interrupt_number,
                    "ax": machine.reg_read(UC_X86_REG_EAX) & 0xFFFF,
                    "cx": machine.reg_read(UC_X86_REG_ECX) & 0xFFFF,
                    "dx": machine.reg_read(UC_X86_REG_EDX) & 0xFFFF,
                    "ds": machine.reg_read(UC_X86_REG_DS),
                    "sp": machine.reg_read(UC_X86_REG_SP),
                    "globals": bytes(
                        machine.mem_read(data_segment * 16 + 0x0028, 8)
                    ),
                }
            )
            machine.reg_write(
                UC_X86_REG_EAX,
                (machine.reg_read(UC_X86_REG_EAX) & 0xFFFF0000) | driver_ax,
            )
            machine.reg_write(UC_X86_REG_EFLAGS, driver_flags[case_index])

        immutable = [
            (extra_segment, 0x0028, bytes.fromhex("1122334455667788")),
            (game_segment, 0x0028, bytes.fromhex("8877665544332211")),
        ]
        machine = execute(
            image,
            entry,
            return_address,
            initial,
            [
                (0, return_address, b"\xcc"),
                *immutable,
                (data_segment, 0x0028, globals_before),
                (
                    stack_segment,
                    0xFF00,
                    struct.pack("<H", return_address) + stack_sentinel,
                ),
            ],
            interrupt_handler,
        )

        expected_interrupt = {
            "number": 0x33,
            "ax": 4,
            "cx": x,
            "dx": y,
            "ds": data_segment,
            "sp": 0xFF00,
            "globals": globals_after,
        }
        if interrupts != [expected_interrupt]:
            raise AssertionError(
                f"{module}:{entry:#x} {name}: "
                f"interrupts={interrupts}, expected={[expected_interrupt]}"
            )

        expected_registers = dict(initial)
        del expected_registers["flags"]
        expected_registers["eax"] = (
            initial["eax"] & 0xFFFF0000
        ) | driver_ax
        expected_registers["sp"] = 0xFF02
        for register, expected in expected_registers.items():
            actual = machine.reg_read(REGISTERS[register])
            if actual != expected:
                raise AssertionError(
                    f"{module}:{entry:#x} {name}: "
                    f"{register}={actual:#x}, expected={expected:#x}"
                )
        if machine.reg_read(UC_X86_REG_CS) != 0:
            raise AssertionError(f"{module}:{entry:#x} {name}: near return CS changed")
        if bytes(machine.mem_read(stack_segment * 16 + 0xFF02, 6)) != stack_sentinel:
            raise AssertionError(f"{module}:{entry:#x} {name}: stack sentinel changed")
        if bytes(machine.mem_read(data_segment * 16 + 0x0028, 8)) != globals_after:
            raise AssertionError(f"{module}:{entry:#x} {name}: mouse globals differ")
        for segment, offset, value in immutable:
            actual = bytes(machine.mem_read(segment * 16 + offset, len(value)))
            if actual != value:
                raise AssertionError(f"{module}:{entry:#x} {name}: decoy changed")

        flag_masks = {
            "cf": 0x0001,
            "pf": 0x0004,
            "af": 0x0010,
            "zf": 0x0040,
            "sf": 0x0080,
            "if": 0x0200,
            "df": 0x0400,
            "of": 0x0800,
        }
        expected_flags = {
            flag: bool(driver_flags[case_index] & mask)
            for flag, mask in flag_masks.items()
        }
        flags_after = machine.reg_read(UC_X86_REG_EFLAGS)
        actual_flags = {
            flag: bool(flags_after & mask) for flag, mask in flag_masks.items()
        }
        if actual_flags != expected_flags:
            raise AssertionError(
                f"{module}:{entry:#x} {name}: "
                f"flags={actual_flags}, expected={expected_flags}"
            )

        vectors.append(
            {
                "name": name,
                "module": module,
                "entry": entry,
                "x": x,
                "y": y,
                "interrupt": 0x33,
                "interrupt_function": 4,
                "globals_committed_before_interrupt": True,
                "driver_ax_after": driver_ax,
                "defined_flags": expected_flags,
            }
        )

    return vectors


def mouse_bounds_vectors(module: str, entry: int) -> list[dict[str, object]]:
    image = load_image(module)
    expected_hash = "9088c864b81d156291d0a7bcc1f0de09edfa68b14d89034733fd541a0d196efc"
    if hashlib.sha256(image[entry : entry + 17]).hexdigest() != expected_hash:
        raise AssertionError(f"{module}:{entry:#x}: recovered 17-byte body changed")

    cases = [
        ("zero", 0x0000, 0x0000),
        ("screen", 0x013F, 0x00C7),
        ("x_high_bit", 0x8000, 0x1234),
        ("y_high_bit", 0x4321, 0x8000),
        ("maximum", 0xFFFF, 0xFFFF),
        ("mixed", 0xA55A, 0x5AA5),
    ]
    stack_segment = 0x9000
    return_address = 0xF000
    first_flags = (0x0202, 0x0AD7, 0x0646, 0x0283, 0x0A12, 0x0643)
    second_flags = (0x0A93, 0x0246, 0x0683, 0x0A02, 0x0257, 0x0647)
    vectors = []

    for case_index, (name, max_x, max_y) in enumerate(cases):
        stack_sentinel = bytes.fromhex("5aa596698778")
        initial = {
            "eax": 0xA1A1BEEF + case_index,
            "ebx": 0xB2B22345 + case_index,
            "ecx": 0xC3C30000 | max_x,
            "edx": 0xD4D40000 | max_y,
            "esi": 0xE5E55678 + case_index,
            "edi": 0xF6F66789 + case_index,
            "ebp": 0x9797789A + case_index,
            "sp": 0xFF00,
            "ds": 0x4400,
            "es": 0x4800,
            "fs": 0x4C00,
            "gs": 0x2C00,
            "ss": stack_segment,
            "flags": 0x0A93 | (0x0400 if case_index & 1 else 0),
        }
        first_outputs = (
            (0x4100 + case_index) & 0xFFFF,
            (0x4200 + case_index) & 0xFFFF,
            (0x4300 + case_index) & 0xFFFF,
        )
        second_outputs = (
            (0x5100 + case_index) & 0xFFFF,
            (0x5200 + case_index) & 0xFFFF,
            (0x5300 + case_index) & 0xFFFF,
        )
        interrupts: list[dict[str, int]] = []

        def interrupt_handler(
            machine: Uc, interrupt_number: int, _data: object
        ) -> None:
            interrupt_index = len(interrupts)
            snapshot = {
                "number": interrupt_number,
                "ax": machine.reg_read(UC_X86_REG_EAX) & 0xFFFF,
                "cx": machine.reg_read(UC_X86_REG_ECX) & 0xFFFF,
                "dx": machine.reg_read(UC_X86_REG_EDX) & 0xFFFF,
                "sp": machine.reg_read(UC_X86_REG_SP),
                "saved_max_x": struct.unpack(
                    "<H", machine.mem_read(stack_segment * 16 + 0xFEFE, 2)
                )[0],
            }
            interrupts.append(snapshot)
            outputs = first_outputs if interrupt_index == 0 else second_outputs
            flags = (
                first_flags[case_index]
                if interrupt_index == 0
                else second_flags[case_index]
            )
            for register, value in zip(
                (UC_X86_REG_EAX, UC_X86_REG_ECX, UC_X86_REG_EDX), outputs
            ):
                machine.reg_write(
                    register,
                    (machine.reg_read(register) & 0xFFFF0000) | value,
                )
            machine.reg_write(UC_X86_REG_EFLAGS, flags)

        machine = execute(
            image,
            entry,
            return_address,
            initial,
            [
                (0, return_address, b"\xcc"),
                (
                    stack_segment,
                    0xFF00,
                    struct.pack("<H", return_address) + stack_sentinel,
                ),
            ],
            interrupt_handler,
        )

        expected_interrupts = [
            {
                "number": 0x33,
                "ax": 8,
                "cx": 0,
                "dx": max_y,
                "sp": 0xFEFE,
                "saved_max_x": max_x,
            },
            {
                "number": 0x33,
                "ax": 7,
                "cx": 0,
                "dx": max_x,
                "sp": 0xFF00,
                "saved_max_x": max_x,
            },
        ]
        if interrupts != expected_interrupts:
            raise AssertionError(
                f"{module}:{entry:#x} {name}: "
                f"interrupts={interrupts}, expected={expected_interrupts}"
            )

        expected_registers = dict(initial)
        del expected_registers["flags"]
        for name_key, value in zip(("eax", "ecx", "edx"), second_outputs):
            expected_registers[name_key] = (
                initial[name_key] & 0xFFFF0000
            ) | value
        expected_registers["sp"] = 0xFF02
        for register, expected in expected_registers.items():
            actual = machine.reg_read(REGISTERS[register])
            if actual != expected:
                raise AssertionError(
                    f"{module}:{entry:#x} {name}: "
                    f"{register}={actual:#x}, expected={expected:#x}"
                )
        if machine.reg_read(UC_X86_REG_CS) != 0:
            raise AssertionError(f"{module}:{entry:#x} {name}: near return CS changed")
        if bytes(machine.mem_read(stack_segment * 16 + 0xFF02, 6)) != stack_sentinel:
            raise AssertionError(f"{module}:{entry:#x} {name}: stack sentinel changed")

        flag_masks = {
            "cf": 0x0001,
            "pf": 0x0004,
            "af": 0x0010,
            "zf": 0x0040,
            "sf": 0x0080,
            "if": 0x0200,
            "df": 0x0400,
            "of": 0x0800,
        }
        expected_flags = {
            flag: bool(second_flags[case_index] & mask)
            for flag, mask in flag_masks.items()
        }
        flags_after = machine.reg_read(UC_X86_REG_EFLAGS)
        actual_flags = {
            flag: bool(flags_after & mask) for flag, mask in flag_masks.items()
        }
        if actual_flags != expected_flags:
            raise AssertionError(
                f"{module}:{entry:#x} {name}: "
                f"flags={actual_flags}, expected={expected_flags}"
            )

        vectors.append(
            {
                "name": name,
                "module": module,
                "entry": entry,
                "max_x": max_x,
                "max_y": max_y,
                "vertical_call": {"function": 8, "minimum": 0, "maximum": max_y},
                "horizontal_call": {"function": 7, "minimum": 0, "maximum": max_x},
                "max_x_saved_on_stack": True,
                "second_driver_outputs": {
                    "ax": second_outputs[0],
                    "cx": second_outputs[1],
                    "dx": second_outputs[2],
                },
                "defined_flags": expected_flags,
            }
        )

    return vectors


def add_flags_16(left: int, right: int, initial_flags: int) -> dict[str, bool]:
    total = left + right
    result = total & 0xFFFF
    return {
        "cf": total > 0xFFFF,
        "pf": (result & 0xFF).bit_count() % 2 == 0,
        "af": ((left & 0xF) + (right & 0xF)) > 0xF,
        "zf": result == 0,
        "sf": bool(result & 0x8000),
        "if": bool(initial_flags & 0x0200),
        "df": bool(initial_flags & 0x0400),
        "of": bool((~(left ^ right) & (left ^ result) & 0x8000)),
    }


def add_flags_8(left: int, right: int, initial_flags: int) -> dict[str, bool]:
    total = left + right
    result = total & 0xFF
    return {
        "cf": total > 0xFF,
        "pf": result.bit_count() % 2 == 0,
        "af": ((left & 0xF) + (right & 0xF)) > 0xF,
        "zf": result == 0,
        "sf": bool(result & 0x80),
        "if": bool(initial_flags & 0x0200),
        "df": bool(initial_flags & 0x0400),
        "of": bool((~(left ^ right) & (left ^ result) & 0x80)),
    }


def sub_flags_8(left: int, right: int, initial_flags: int) -> dict[str, bool]:
    result = (left - right) & 0xFF
    return {
        "cf": left < right,
        "pf": result.bit_count() % 2 == 0,
        "af": bool((left ^ right ^ result) & 0x10),
        "zf": result == 0,
        "sf": bool(result & 0x80),
        "if": bool(initial_flags & 0x0200),
        "df": bool(initial_flags & 0x0400),
        "of": bool(((left ^ right) & (left ^ result)) & 0x80),
    }


def sub_flags_16(left: int, right: int, initial_flags: int) -> dict[str, bool]:
    result = (left - right) & 0xFFFF
    return {
        "cf": left < right,
        "pf": (result & 0xFF).bit_count() % 2 == 0,
        "af": bool((left ^ right ^ result) & 0x0010),
        "zf": result == 0,
        "sf": bool(result & 0x8000),
        "if": bool(initial_flags & 0x0200),
        "df": bool(initial_flags & 0x0400),
        "of": bool(((left ^ right) & (left ^ result)) & 0x8000),
    }


def mouse_camera_step_vectors(
    module: str, entry: int, body_size: int, body_hash: str
) -> list[dict[str, object]]:
    image = load_image(module)
    if hashlib.sha256(image[entry : entry + body_size]).hexdigest() != body_hash:
        raise AssertionError(
            f"{module}:{entry:#x}: recovered {body_size}-byte body changed"
        )

    cases = (
        {
            "name": "centered_idle",
            "mouse": (0x0140, 0x0200, 0x0000),
            "filter_x": 0,
            "camera": (0, 0, 0, 0),
            "control": 0,
            "key": 0,
            "code_flags": 0x1203,
        },
        {
            "name": "positive_pan_left_and_up",
            "mouse": (0x01A4, 0x01D0, 0x0001),
            "filter_x": 7,
            "camera": (9, 0x0100, 0xFFE0, 4),
            "control": 0,
            "key": 0x4800,
            "code_flags": 0x2205,
        },
        {
            "name": "negative_pan_right_and_down",
            "mouse": (0x00DC, 0x0240, 0x0002),
            "filter_x": -9,
            "camera": (-12, -300, 0x0120, -20),
            "control": 0,
            "key": 0x5000,
            "code_flags": 0x3407,
        },
        {
            "name": "dead_zone_both_buttons_space",
            "mouse": (0x014A, 0x01FB, 0x0003),
            "filter_x": 3,
            "camera": (2, -1, 8, -8),
            "control": 1,
            "key": 0x3920,
            "code_flags": 0x0001,
        },
        {
            "name": "positive_depth_control_latch",
            "mouse": (0x0140, 0x0200, 0x0000),
            "filter_x": -32768,
            "camera": (-32768, 0x7FFF, 0x8000, 7),
            "control": 2,
            "key": 0x1234,
            "code_flags": 0x8010,
        },
        {
            "name": "negative_depth_control_latch",
            "mouse": (0xFFFF, 0x0000, 0x0000),
            "filter_x": 0x7FFF,
            "camera": (0x7FFF, 0x8000, 0x7FFF, -9),
            "control": 1,
            "key": 0x0020,
            "code_flags": 0xA500,
        },
        {
            "name": "wrapped_driver_coordinates",
            "mouse": (0x8000, 0x8000, 0x0003),
            "filter_x": 0x4000,
            "camera": (0x4000, 0xFFFC, 0x0003, 0x7FFF),
            "control": 0x8000,
            "key": 0x5020,
            "code_flags": 0x5A40,
        },
        {
            "name": "vertical_negate_minimum_wrap",
            "mouse": (0x0140, 0x8200, 0x0000),
            "filter_x": 0,
            "camera": (0, 0, 0, 0),
            "control": 0,
            "key": 0,
            "code_flags": 0x00F0,
        },
    )
    data_segment = 0x4400
    extra_segment = 0x6000
    fs_segment = 0x6800
    game_segment = 0x7000
    stack_segment = 0x9000
    return_address = 0xF000
    vectors = []

    def word(value: int) -> int:
        return value & 0xFFFF

    def signed_word(value: int) -> int:
        value &= 0xFFFF
        return value - 0x10000 if value & 0x8000 else value

    def sar_word(value: int, count: int) -> int:
        return word(signed_word(value) >> count)

    def dead_zone(value: int) -> int:
        value = word(value - 5)
        if value & 0x8000:
            value = word(value + 10)
            if not value & 0x8000:
                value = 0
        return value

    for case_index, case in enumerate(cases):
        mouse_x, mouse_y, buttons = case["mouse"]
        pitch, pan, pan_target, depth = case["camera"]
        data_before = bytearray(
            ((offset * 37 + case_index * 19 + 11) & 0xFF)
            for offset in range(0x2300)
        )
        data_expected = bytearray(data_before)
        struct.pack_into("<h", data_before, 0x1058, case["filter_x"])
        struct.pack_into("<H", data_before, 0x2282, word(case["control"]))
        for offset, value in zip(
            (0x22F6, 0x22F8, 0x22FA, 0x22FC),
            (pitch, pan, pan_target, depth),
        ):
            struct.pack_into("<H", data_before, offset, word(value))
        data_expected[:] = data_before

        centered_x = word(mouse_x - 0x0140)
        centered_y = word(mouse_y - 0x0200)
        struct.pack_into("<HHH", data_expected, 0x002A, centered_x, centered_y, buttons)

        x_delta = dead_zone(sar_word(centered_x, 1))
        x_delta = sar_word(word(x_delta - word(case["filter_x"])), 1)
        struct.pack_into("<H", data_expected, 0x1058, x_delta)
        pan = word(pan + x_delta)
        struct.pack_into("<H", data_expected, 0x22F8, pan)
        x_delta = sar_word(word(word(x_delta << 3) - word(pan_target)), 1)
        pan_target = word(pan_target + x_delta)
        struct.pack_into("<H", data_expected, 0x22FA, pan_target)

        y_delta = dead_zone(word(-centered_y))
        y_delta = sar_word(word(word(y_delta << 1) - word(pitch)), 4)
        pitch = word(pitch + y_delta)
        struct.pack_into("<H", data_expected, 0x22F6, pitch)

        depth = word(depth)
        if buttons & 1:
            depth = word(depth + 10)
        final_bx = buttons
        if buttons & 2:
            final_bx = sar_word(depth, 3)
            depth = word(depth - final_bx - 1)

        control_active = bool(
            word(case["control"]) & (1 if module == "amer" else 0xFFFF)
        )
        if signed_word(depth) <= -8:
            depth = word(depth + 8)
            if control_active:
                depth = word(depth - 0x40)
        elif control_active:
            depth = word(-100)
        struct.pack_into("<H", data_expected, 0x22FC, depth)
        if module == "amer":
            struct.pack_into("<H", data_expected, 0x2282, 0)

        key_before = word(case["key"])
        code_flags_before = word(case["code_flags"])
        key_after = key_before
        code_flags_after = code_flags_before
        if module != "amer":
            key_after = 0

        if key_before == 0x4800:
            key_after = 0
            before_key_step = depth
            depth = word(depth + 8)
            struct.pack_into("<H", data_expected, 0x22FC, depth)
            expected_flags = add_flags_16(before_key_step, 8, 0x0202)
            final_path = "cursor_up"
        elif key_before == 0x5000:
            key_after = 0
            before_key_step = depth
            depth = word(depth - 8)
            struct.pack_into("<H", data_expected, 0x22FC, depth)
            expected_flags = sub_flags_16(before_key_step, 8, 0x0202)
            final_path = "cursor_down"
        elif module != "amer" and (key_before & 0xFF) == 0x20:
            code_flags_after = code_flags_before | 0x10
            expected_flags = _logical_flags_16(code_flags_after, 0x0202)
            final_path = "space_action"
        elif module == "amer":
            expected_flags = sub_flags_16(key_before, 0x5000, 0x0202)
            final_path = "unhandled_retained"
        else:
            expected_flags = sub_flags_8(key_before & 0xFF, 0x20, 0x0202)
            final_path = "unhandled_cleared"

        handler_flags = 0x0202 | (0x0400 if case_index & 1 else 0)
        expected_flags["if"] = bool(handler_flags & 0x0200)
        expected_flags["df"] = bool(handler_flags & 0x0400)

        initial = {
            "eax": 0xA1A1BEEF + case_index,
            "ebx": 0xB2B22345 + case_index,
            "ecx": 0xC3C33456 + case_index,
            "edx": 0xD4D44567 + case_index,
            "esi": 0xE5E55678 + case_index,
            "edi": 0xF6F66789 + case_index,
            "ebp": 0x9797789A + case_index,
            "sp": 0xFF00,
            "ds": data_segment,
            "es": extra_segment,
            "fs": fs_segment,
            "gs": game_segment,
            "ss": stack_segment,
            "flags": 0x0A93,
        }
        interrupts: list[dict[str, int]] = []

        def interrupt_handler(
            machine: Uc, interrupt_number: int, _data: object
        ) -> None:
            interrupts.append(
                {
                    "number": interrupt_number,
                    "ax": machine.reg_read(UC_X86_REG_EAX) & 0xFFFF,
                    "ds": machine.reg_read(UC_X86_REG_DS),
                    "sp": machine.reg_read(UC_X86_REG_SP),
                    "mouse_x_before": struct.unpack(
                        "<H", machine.mem_read(data_segment * 16 + 0x002A, 2)
                    )[0],
                }
            )
            for register, value in (
                (UC_X86_REG_EBX, buttons),
                (UC_X86_REG_ECX, mouse_x),
                (UC_X86_REG_EDX, mouse_y),
            ):
                machine.reg_write(
                    register,
                    (machine.reg_read(register) & 0xFFFF0000) | value,
                )
            machine.reg_write(UC_X86_REG_EFLAGS, handler_flags)

        key_bytes = struct.pack("<H", key_before)
        code_flags_bytes = struct.pack("<H", code_flags_before)
        decoy = bytes.fromhex("102132435465768798a9bacbdcedfe0f")
        stack_sentinel = bytes.fromhex("5aa596698778")
        mouse_x_before = struct.unpack_from("<H", data_before, 0x002A)[0]
        machine = execute(
            image,
            entry,
            return_address,
            initial,
            [
                (0, return_address, b"\xcc"),
                (0, 0x0095, key_bytes),
                (0, 0x02FC, code_flags_bytes),
                (data_segment, 0, bytes(data_before)),
                (extra_segment, 0x0100, decoy),
                (fs_segment, 0x0100, decoy),
                (game_segment, 0x0100, decoy),
                (
                    stack_segment,
                    0xFF00,
                    struct.pack("<H", return_address) + stack_sentinel,
                ),
            ],
            interrupt_handler=interrupt_handler,
        )

        expected_interrupts = [
            {
                "number": 0x33,
                "ax": 3,
                "ds": data_segment,
                "sp": 0xFF00,
                "mouse_x_before": mouse_x_before,
            }
        ]
        if interrupts != expected_interrupts:
            raise AssertionError(
                f"{module}:{entry:#x} {case['name']}: "
                f"interrupts={interrupts}, expected={expected_interrupts}"
            )

        actual_data = bytes(machine.mem_read(data_segment * 16, len(data_expected)))
        if actual_data != bytes(data_expected):
            differences = [
                (offset, actual_data[offset], data_expected[offset])
                for offset in range(len(data_expected))
                if actual_data[offset] != data_expected[offset]
            ][:8]
            raise AssertionError(
                f"{module}:{entry:#x} {case['name']}: data differs at {differences}"
            )
        actual_key = struct.unpack("<H", machine.mem_read(0x0095, 2))[0]
        actual_code_flags = struct.unpack("<H", machine.mem_read(0x02FC, 2))[0]
        if (actual_key, actual_code_flags) != (key_after, code_flags_after):
            raise AssertionError(
                f"{module}:{entry:#x} {case['name']}: code data "
                f"{(actual_key, actual_code_flags)} != {(key_after, code_flags_after)}"
            )
        for segment in (extra_segment, fs_segment, game_segment):
            if bytes(machine.mem_read(segment * 16 + 0x0100, len(decoy))) != decoy:
                raise AssertionError(
                    f"{module}:{entry:#x} {case['name']}: decoy {segment:#x} changed"
                )

        expected_registers = dict(initial)
        del expected_registers["flags"]
        expected_registers["eax"] = (initial["eax"] & 0xFFFF0000) | key_before
        expected_registers["ebx"] = (initial["ebx"] & 0xFFFF0000) | final_bx
        expected_registers["ecx"] = (initial["ecx"] & 0xFFFF0000) | x_delta
        expected_registers["edx"] = (initial["edx"] & 0xFFFF0000) | y_delta
        expected_registers["sp"] = 0xFF02
        for register, expected in expected_registers.items():
            actual = machine.reg_read(REGISTERS[register])
            if actual != expected:
                raise AssertionError(
                    f"{module}:{entry:#x} {case['name']}: "
                    f"{register}={actual:#x}, expected={expected:#x}"
                )
        if machine.reg_read(UC_X86_REG_CS) != 0:
            raise AssertionError(f"{module}:{entry:#x}: near return CS changed")
        if bytes(machine.mem_read(stack_segment * 16 + 0xFF02, 6)) != stack_sentinel:
            raise AssertionError(f"{module}:{entry:#x}: stack sentinel changed")

        flag_masks = {
            "cf": 0x0001,
            "pf": 0x0004,
            "af": 0x0010,
            "zf": 0x0040,
            "sf": 0x0080,
            "if": 0x0200,
            "df": 0x0400,
            "of": 0x0800,
        }
        flags_after = machine.reg_read(UC_X86_REG_EFLAGS)
        actual_flags = {
            flag: bool(flags_after & flag_masks[flag]) for flag in expected_flags
        }
        if actual_flags != expected_flags:
            raise AssertionError(
                f"{module}:{entry:#x} {case['name']}: "
                f"flags={actual_flags}, expected={expected_flags}"
            )

        vectors.append(
            {
                "name": case["name"],
                "module": module,
                "entry": entry,
                "mouse": {"x": mouse_x, "y": mouse_y, "buttons": buttons},
                "filter_x_before": signed_word(word(case["filter_x"])),
                "camera_before": {
                    "pitch": signed_word(word(case["camera"][0])),
                    "pan": signed_word(word(case["camera"][1])),
                    "pan_target": signed_word(word(case["camera"][2])),
                    "depth_step": signed_word(word(case["camera"][3])),
                },
                "control_before": word(case["control"]),
                "key_before": key_before,
                "code_flags_before": code_flags_before,
                "centered": {
                    "x": signed_word(centered_x),
                    "y": signed_word(centered_y),
                },
                "filter_x_after": signed_word(
                    struct.unpack_from("<H", data_expected, 0x1058)[0]
                ),
                "camera_after": {
                    "pitch": signed_word(pitch),
                    "pan": signed_word(pan),
                    "pan_target": signed_word(pan_target),
                    "depth_step": signed_word(depth),
                },
                "control_after": struct.unpack_from(
                    "<H", data_expected, 0x2282
                )[0],
                "key_after": key_after,
                "code_flags_after": code_flags_after,
                "final_path": final_path,
                "defined_flags": expected_flags,
            }
        )

    return vectors


def alien_slot10_bounds_then_wrap_vectors(
    module: str, entry: int, wrap_entry: int
) -> list[dict[str, object]]:
    image = load_image(module)
    body_size = wrap_entry - entry
    expected_hash = "a6786d7561c37e5e6e2359d0d8bd9a28781b7f0f5196eeebf3c66d262ddb781d"
    if hashlib.sha256(image[entry:wrap_entry]).hexdigest() != expected_hash:
        raise AssertionError(
            f"{module}:{entry:#x}: recovered {body_size}-byte body changed"
        )

    cases = (
        ("inside_zero", 0x4000, 0, 0, 0, 0x0000, True),
        ("inclusive_limits", 0x4200, -100, 100, 100, 0x0040, True),
        ("unsigned_axis_above", 0x4400, 0, 0, 101, 0x7FFF, False),
        ("unsigned_axis_high_bit", 0x4600, 0, 0, 0xFFFF, 0xFFC0, False),
        ("first_signed_axis_above", 0x4800, 101, 0, 0, 0x1234, False),
        ("first_signed_axis_below", 0x4A00, -101, 0, 0, 0xABCD, False),
        ("second_signed_axis_above", 0x4C00, 0, 101, 0, 0x8000, False),
        ("second_signed_axis_below", 0x4E00, 0, -101, 0, 0x00C0, False),
        ("state_pointer_wrap", 0xFFC0, -100, -100, 100, 0xFFFE, True),
    )
    patched_image = bytearray(image)
    patched_image[wrap_entry] = 0xC3
    data_segment = 0x3000
    extra_segment = 0x5000
    active_segment = 0x7000
    game_segment = 0x9000
    stack_segment = 0xB000
    context_offset = 0x3000
    return_address = 0xF000
    vectors = []

    def word(value: int) -> int:
        return value & 0xFFFF

    def signed_word(value: int) -> int:
        value &= 0xFFFF
        return value - 0x10000 if value & 0x8000 else value

    for case_index, (
        name,
        state_base,
        first_signed,
        second_signed,
        unsigned_axis,
        accumulator,
        should_request_exit,
    ) in enumerate(cases):
        state = (state_base + 0x005E) & 0xFFFF
        data_before = bytearray(
            ((offset * 29 + case_index * 17 + 3) & 0xFF)
            for offset in range(0x10000)
        )
        struct.pack_into("<H", data_before, context_offset + 0x16, state_base)
        for offset, value in (
            (0x38, first_signed),
            (0x3C, second_signed),
            (0x40, unsigned_axis),
            (0x50, accumulator),
        ):
            struct.pack_into("<H", data_before, (state + offset) & 0xFFFF, word(value))
        data_expected = bytearray(data_before)
        accumulator_after = word(accumulator + 0x40)
        struct.pack_into("<H", data_expected, (state + 0x50) & 0xFFFF, accumulator_after)

        active_before = bytearray(
            ((offset * 31 + case_index * 23 + 5) & 0xFF)
            for offset in range(0x10000)
        )
        exit_before = word(0xA500 + case_index)
        struct.pack_into("<H", active_before, 0x226E, exit_before)
        active_expected = bytearray(active_before)
        exit_after = 1 if should_request_exit else exit_before
        struct.pack_into("<H", active_expected, 0x226E, exit_after)

        if unsigned_axis > 100:
            final_ax = word(unsigned_axis)
            compare_right = 100
            final_path = "unsigned_axis_rejected"
        elif first_signed > 100:
            final_ax = word(first_signed)
            compare_right = 100
            final_path = "first_signed_axis_above"
        elif first_signed < -100:
            final_ax = word(first_signed)
            compare_right = word(-100)
            final_path = "first_signed_axis_below"
        elif second_signed > 100:
            final_ax = word(second_signed)
            compare_right = 100
            final_path = "second_signed_axis_above"
        else:
            final_ax = word(second_signed)
            compare_right = word(-100)
            final_path = (
                "second_signed_axis_below"
                if second_signed < -100
                else "exit_requested"
            )

        initial_flags = 0x0293 | (0x0400 if case_index & 1 else 0)
        initial = {
            "eax": 0xA1A10000 | ((0xBEEF + case_index) & 0xFFFF),
            "ebx": 0xB2B22345 + case_index,
            "ecx": 0xC3C33456 + case_index,
            "edx": 0xD4D44567 + case_index,
            "esi": 0xE5E55678 + case_index,
            "edi": 0xF6F60000 | context_offset,
            "ebp": 0x9797789A + case_index,
            "sp": 0xFF00,
            "ds": data_segment,
            "es": extra_segment,
            "fs": active_segment,
            "gs": game_segment,
            "ss": stack_segment,
            "flags": initial_flags,
        }
        decoys = (
            bytes((offset * 13 + case_index + 7) & 0xFF for offset in range(0x10000)),
            bytes((offset * 11 + case_index + 9) & 0xFF for offset in range(0x10000)),
        )
        stack_sentinel = bytes.fromhex("5aa596698778")
        wrap_entries: list[dict[str, int]] = []

        def code_handler(
            machine: Uc, address: int, _size: int, _data: object
        ) -> None:
            if address == wrap_entry:
                wrap_entries.append(
                    {
                        "di": machine.reg_read(UC_X86_REG_EDI) & 0xFFFF,
                        "si": machine.reg_read(UC_X86_REG_ESI) & 0xFFFF,
                        "sp": machine.reg_read(UC_X86_REG_SP),
                        "ds": machine.reg_read(UC_X86_REG_DS),
                        "fs": machine.reg_read(UC_X86_REG_FS),
                    }
                )

        machine = execute(
            bytes(patched_image),
            entry,
            return_address,
            initial,
            [
                (0, return_address, b"\xcc"),
                (data_segment, 0, bytes(data_before)),
                (extra_segment, 0, decoys[0]),
                (active_segment, 0, bytes(active_before)),
                (game_segment, 0, decoys[1]),
                (
                    stack_segment,
                    0xFF00,
                    struct.pack("<H", return_address) + stack_sentinel,
                ),
            ],
            code_handler=code_handler,
        )

        expected_entry = {
            "di": context_offset,
            "si": state,
            "sp": 0xFF00,
            "ds": data_segment,
            "fs": active_segment,
        }
        if wrap_entries != [expected_entry]:
            raise AssertionError(
                f"{module}:{entry:#x} {name}: wrap entries={wrap_entries}, "
                f"expected={[expected_entry]}"
            )
        for segment, expected, label in (
            (data_segment, data_expected, "data"),
            (extra_segment, decoys[0], "extra"),
            (active_segment, active_expected, "active"),
            (game_segment, decoys[1], "game"),
        ):
            actual = bytes(machine.mem_read(segment * 16, len(expected)))
            if actual != bytes(expected):
                differences = [
                    (offset, actual[offset], expected[offset])
                    for offset in range(len(expected))
                    if actual[offset] != expected[offset]
                ][:8]
                raise AssertionError(
                    f"{module}:{entry:#x} {name}: {label} differs at {differences}"
                )

        expected_registers = dict(initial)
        del expected_registers["flags"]
        expected_registers["eax"] = (initial["eax"] & 0xFFFF0000) | final_ax
        expected_registers["esi"] = (initial["esi"] & 0xFFFF0000) | state
        expected_registers["sp"] = 0xFF02
        for register, expected in expected_registers.items():
            actual = machine.reg_read(REGISTERS[register])
            if actual != expected:
                raise AssertionError(
                    f"{module}:{entry:#x} {name}: "
                    f"{register}={actual:#x}, expected={expected:#x}"
                )
        if bytes(machine.mem_read(stack_segment * 16 + 0xFF02, 6)) != stack_sentinel:
            raise AssertionError(f"{module}:{entry:#x} {name}: stack changed")

        expected_flags = sub_flags_16(final_ax, compare_right, initial_flags)
        flag_masks = {
            "cf": 0x0001,
            "pf": 0x0004,
            "af": 0x0010,
            "zf": 0x0040,
            "sf": 0x0080,
            "if": 0x0200,
            "df": 0x0400,
            "of": 0x0800,
        }
        flags_after = machine.reg_read(UC_X86_REG_EFLAGS)
        actual_flags = {
            flag: bool(flags_after & mask) for flag, mask in flag_masks.items()
        }
        if actual_flags != expected_flags:
            raise AssertionError(
                f"{module}:{entry:#x} {name}: "
                f"flags={actual_flags}, expected={expected_flags}"
            )

        vectors.append(
            {
                "name": name,
                "module": module,
                "entry": entry,
                "state_base": state_base,
                "biased_state": state,
                "bounds": {
                    "first_signed": signed_word(first_signed),
                    "second_signed": signed_word(second_signed),
                    "unsigned_axis": word(unsigned_axis),
                },
                "accumulator_before": word(accumulator),
                "accumulator_after": accumulator_after,
                "exit_before": exit_before,
                "exit_after": exit_after,
                "final_path": final_path,
                "falls_through_to": wrap_entry,
                "fallthrough_stack_unchanged": True,
                "defined_flags": expected_flags,
            }
        )

    return vectors


def alien_slot1_wave_update_or_init_vectors(
    module: str,
    entry: int,
    block_start: int,
    body_hash: str,
    selection_state_offset: int,
    selected_state_offset: int,
    current_sample_offset: int,
    publish_initial_state: bool,
) -> list[dict[str, object]]:
    image = load_image(module)
    body_end = entry + 288
    if hashlib.sha256(image[block_start:body_end]).hexdigest() != body_hash:
        raise AssertionError(
            f"{module}:{entry:#x}: recovered slot-1 owner changed"
        )

    game_segment = 0x5000
    object_segment = 0x7000
    extra_segment = 0x9000
    decoy_segment = 0xB000
    stack_segment = 0xD000
    context_offset = 0x3000
    state_base = 0x3800
    state = state_base + 0x5E
    object_offset = 0x4000
    return_address = 0xF000
    stack_sentinel = bytes.fromhex("5aa596698778")
    vectors: list[dict[str, object]] = []

    def put_bytes(memory: bytearray, offset: int, value: bytes) -> None:
        for index, byte in enumerate(value):
            memory[(offset + index) & 0xFFFF] = byte

    def put_u16(memory: bytearray, offset: int, value: int) -> None:
        put_bytes(memory, offset, struct.pack("<H", value & 0xFFFF))

    def put_u32(memory: bytearray, offset: int, value: int) -> None:
        put_bytes(memory, offset, struct.pack("<I", value & 0xFFFFFFFF))

    def get_u16(memory: bytearray, offset: int) -> int:
        return memory[offset & 0xFFFF] | (memory[(offset + 1) & 0xFFFF] << 8)

    def signed16(value: int) -> int:
        value &= 0xFFFF
        return value - 0x10000 if value & 0x8000 else value

    def sample_at(memory: bytearray, offset: int) -> int:
        return signed16(get_u16(memory, 0x0036 + (offset & 0x0FFC)))

    def patterned(seed: int, stride: int) -> bytearray:
        return bytearray(
            ((offset * stride + seed) & 0xFF) for offset in range(0x10000)
        )

    init_game = patterned(3, 29)
    put_u16(init_game, context_offset + 0x16, state_base)
    put_u16(init_game, context_offset + 0x36, 0)
    init_expected = bytearray(init_game)
    put_u16(init_expected, context_offset + 0x36, 1)
    put_u16(init_expected, context_offset + 0x38, 4)
    put_u16(init_expected, context_offset + 0x3A, 0x30)
    put_u16(init_expected, context_offset + 0x3C, 4)
    put_u16(init_expected, context_offset + 0x3E, 0x10)
    put_u16(init_expected, state + 0x54, 0x0C)
    put_u16(init_expected, state + 0x4E, 0)
    put_u16(init_expected, state + 0x50, 0)
    put_u16(init_expected, state + 0x52, 0)
    init_code = bytearray(image)
    put_u16(init_code, selection_state_offset, 0xA55B)
    put_u16(init_code, selected_state_offset, 0x1357)
    put_u16(init_code, current_sample_offset, 0x2468)
    init_code_expected = bytearray(init_code)
    put_u16(init_code_expected, selection_state_offset, 0)
    if publish_initial_state:
        put_u16(init_code_expected, selected_state_offset, state)
    init_code_expected[return_address] = 0xCC
    init_object = bytes(patterned(7, 17))
    init_extra = bytes(patterned(9, 13))
    init_decoy = bytes(patterned(11, 19))
    initial = {
        "eax": 0xA1A1BEEF,
        "ebx": 0xB2B22345,
        "ecx": 0xC3C33456,
        "edx": 0xD4D44567,
        "esi": 0xE5E55678,
        "edi": 0xF6F60000 | context_offset,
        "ebp": 0x9797789A,
        "sp": 0xFF00,
        "ds": game_segment,
        "es": extra_segment,
        "fs": game_segment,
        "gs": decoy_segment,
        "ss": stack_segment,
        "flags": 0x0693,
    }
    machine = execute(
        bytes(init_code),
        entry,
        return_address,
        initial,
        [
            (0, return_address, b"\xcc"),
            (game_segment, 0, bytes(init_game)),
            (object_segment, 0, init_object),
            (extra_segment, 0, init_extra),
            (decoy_segment, 0, init_decoy),
            (
                stack_segment,
                0xFF00,
                struct.pack("<H", return_address) + stack_sentinel,
            ),
        ],
        max_instructions=1000,
    )
    if bytes(machine.mem_read(game_segment * 16, 0x10000)) != bytes(init_expected):
        raise AssertionError(f"{module}:{entry:#x} initialize: game data differs")
    if bytes(machine.mem_read(0, len(image))) != bytes(init_code_expected):
        raise AssertionError(f"{module}:{entry:#x} initialize: code data differs")
    for segment, expected in (
        (object_segment, init_object),
        (extra_segment, init_extra),
        (decoy_segment, init_decoy),
    ):
        if bytes(machine.mem_read(segment * 16, 0x10000)) != expected:
            raise AssertionError(
                f"{module}:{entry:#x} initialize: segment {segment:#x} changed"
            )
    if machine.reg_read(UC_X86_REG_SP) != 0xFF02:
        raise AssertionError(f"{module}:{entry:#x} initialize: stack mismatch")
    if bytes(machine.mem_read(stack_segment * 16 + 0xFF02, 6)) != stack_sentinel:
        raise AssertionError(f"{module}:{entry:#x} initialize: stack changed")
    vectors.append(
        {
            "name": "initialize",
            "module": module,
            "entry": entry,
            "path": "initialize",
            "state": state,
            "publishes_initial_state": publish_initial_state,
            "game_data_sha256": hashlib.sha256(init_expected).hexdigest(),
            "code_image_sha256": hashlib.sha256(init_code_expected).hexdigest(),
        }
    )

    cases = (
        ("selection_disabled", 0, 0, 0, 0, 0x0030, False),
        ("step_decay", 0, 0, 0, 0, 0x0034, False),
        ("selection_y_below", 1, 0, -5, 0, 0x0030, False),
        ("selection_y_above", 1, 0, 125, 0, 0x0030, False),
        ("selection_x_below", 1, -257, 0, 0, 0x0030, False),
        ("selection_x_above", 1, 257, 0, 0, 0x0030, False),
        ("selection_z_below", 1, 0, 0, -257, 0x0030, False),
        ("selection_z_above", 1, 0, 0, 257, 0x0030, False),
        ("selection_lower_edges", 1, -256, -4, -256, 0x0030, True),
        ("selection_upper_edges", 1, 256, 124, 256, 0x0030, True),
    )
    for case_index, (
        name,
        selection_state,
        position_x,
        position_y,
        position_z,
        primary_step,
        selected,
    ) in enumerate(cases):
        game_before = patterned(case_index * 17 + 5, 31)
        object_before = patterned(case_index * 23 + 7, 37)
        extra_before = bytes(patterned(case_index + 9, 13))
        decoy_before = bytes(patterned(case_index + 11, 19))
        primary_phase = (0x0FF0 + case_index * 4) & 0xFFFF
        secondary_phase = (0x0FF4 + case_index * 8) & 0xFFFF
        secondary_step = (0x0010 + case_index * 4) & 0xFFFF
        state_count = 3
        put_u16(game_before, 0x0002, object_segment)
        put_u16(game_before, context_offset + 0x16, state_base)
        put_u16(game_before, context_offset + 0x1C, object_offset)
        put_u16(game_before, context_offset + 0x20, state_count)
        put_u16(game_before, context_offset + 0x36, 1)
        put_u16(game_before, context_offset + 0x38, primary_phase)
        put_u16(game_before, context_offset + 0x3A, primary_step)
        put_u16(game_before, context_offset + 0x3C, secondary_phase)
        put_u16(game_before, context_offset + 0x3E, secondary_step)
        put_u32(game_before, state + 0x42, position_x)
        put_u32(game_before, state + 0x46, position_y)
        put_u32(game_before, state + 0x4A, position_z)
        put_u16(game_before, state + 0x50, 0xFFFF if case_index == 0 else case_index)
        put_u16(game_before, 0x22EC, 0)
        put_u16(game_before, 0x22F0, 0)
        put_u16(game_before, 0x22F4, 0)
        for offset in range(0, 0x1000, 2):
            value = (offset * 73 + case_index * 0x1111 + 0x8123) & 0xFFFF
            put_u16(game_before, 0x0036 + offset, value)
        put_u16(game_before, 0x0036 + (primary_phase & 0x0FFC), 0x4000)

        for index, (distance, motion, phase_value) in enumerate(
            ((40, 0x7FF8, 3), (-40, 0x8008, 0x07FF), (0, 0xFFF8, 0xFFFF))
        ):
            base = object_offset + index * 0x14
            put_u16(object_before, base + 0x04, distance)
            put_u16(object_before, base + 0x06, motion)
            put_u16(object_before, base + 0x08, phase_value)

        game_expected = bytearray(game_before)
        object_expected = bytearray(object_before)
        code_before = bytearray(image)
        put_u16(code_before, selection_state_offset, selection_state)
        put_u16(code_before, selected_state_offset, 0x1357)
        put_u16(code_before, current_sample_offset, 0x2468)
        code_expected = bytearray(code_before)
        code_expected[return_address] = 0xCC

        put_u16(
            game_expected,
            state + 0x50,
            get_u16(game_expected, state + 0x50) + 1,
        )
        current_sample = sample_at(game_expected, primary_phase) >> 8
        put_u16(code_expected, current_sample_offset, current_sample)
        effective_step = primary_step
        if selected:
            put_u16(code_expected, selection_state_offset, 2)
            put_u16(code_expected, selected_state_offset, state)
            effective_step = 0x0170
        if signed16(effective_step) > 0x30:
            effective_step = (effective_step - 4) & 0xFFFF
            put_u16(game_expected, context_offset + 0x3A, effective_step)

        put_u16(
            game_expected,
            context_offset + 0x38,
            primary_phase + effective_step,
        )
        for index in range(state_count):
            base = object_offset + index * 0x14
            sample_offset = (
                get_u16(object_expected, base + 0x08) * 2 + primary_phase
            ) & 0x0FFC
            motion = get_u16(object_expected, base + 0x06)
            motion = (motion - (sample_at(game_expected, sample_offset) >> 8)) & 0xFFFF
            sample_offset = (sample_offset + effective_step) & 0x0FFC
            motion = (motion + (sample_at(game_expected, sample_offset) >> 8)) & 0xFFFF
            put_u16(object_expected, base + 0x06, motion)

        secondary_phase = (secondary_phase + secondary_step) & 0xFFFF
        put_u16(game_expected, context_offset + 0x3C, secondary_phase)
        branch_classes: list[str] = []
        for index in range(state_count):
            base = object_offset + index * 0x14
            distance = signed16(get_u16(object_expected, base + 0x04) - 25)
            if distance < 0:
                distance = signed16(-distance)
                distance = signed16(distance - 50)
            if distance < 0:
                branch_classes.append("center_skip")
                continue
            branch_classes.append("negative" if index == 1 else "positive")
            scale = (distance * 2) & 0xFFFF
            sample_offset = (scale + secondary_phase) & 0x0FFC
            old_delta = (sample_at(game_expected, sample_offset) * scale) >> 17
            sample_offset = (sample_offset + secondary_step) & 0x0FFC
            new_delta = (sample_at(game_expected, sample_offset) * scale) >> 17
            motion = get_u16(object_expected, base + 0x06)
            motion = (motion - old_delta + new_delta) & 0xFFFF
            put_u16(object_expected, base + 0x06, motion)

        initial_flags = 0x0293 | (0x0400 if case_index & 1 else 0)
        initial = {
            "eax": 0xA1A1BEEF + case_index,
            "ebx": 0xB2B22345 + case_index,
            "ecx": 0xC3C33456 + case_index,
            "edx": 0xD4D44567 + case_index,
            "esi": 0xE5E55678 + case_index,
            "edi": 0xF6F60000 | context_offset,
            "ebp": 0x9797789A + case_index,
            "sp": 0xFF00,
            "ds": game_segment,
            "es": extra_segment,
            "fs": game_segment,
            "gs": decoy_segment,
            "ss": stack_segment,
            "flags": initial_flags,
        }
        machine = execute(
            bytes(code_before),
            entry,
            return_address,
            initial,
            [
                (0, return_address, b"\xcc"),
                (game_segment, 0, bytes(game_before)),
                (object_segment, 0, bytes(object_before)),
                (extra_segment, 0, extra_before),
                (decoy_segment, 0, decoy_before),
                (
                    stack_segment,
                    0xFF00,
                    struct.pack("<H", return_address) + stack_sentinel,
                ),
            ],
            max_instructions=10000,
        )
        actual_game = bytes(machine.mem_read(game_segment * 16, 0x10000))
        actual_object = bytes(machine.mem_read(object_segment * 16, 0x10000))
        actual_code = bytes(machine.mem_read(0, len(image)))
        if actual_game != bytes(game_expected):
            differences = [
                (offset, actual_game[offset], game_expected[offset])
                for offset in range(0x10000)
                if actual_game[offset] != game_expected[offset]
            ][:8]
            raise AssertionError(
                f"{module}:{entry:#x} {name}: game data differs at {differences}"
            )
        if actual_object != bytes(object_expected):
            differences = [
                (offset, actual_object[offset], object_expected[offset])
                for offset in range(0x10000)
                if actual_object[offset] != object_expected[offset]
            ][:8]
            raise AssertionError(
                f"{module}:{entry:#x} {name}: object data differs at {differences}"
            )
        if actual_code != bytes(code_expected):
            differences = [
                (offset, actual_code[offset], code_expected[offset])
                for offset in range(len(image))
                if actual_code[offset] != code_expected[offset]
            ][:8]
            raise AssertionError(
                f"{module}:{entry:#x} {name}: code data differs at {differences}"
            )
        for segment, expected in (
            (extra_segment, extra_before),
            (decoy_segment, decoy_before),
        ):
            if bytes(machine.mem_read(segment * 16, 0x10000)) != expected:
                raise AssertionError(
                    f"{module}:{entry:#x} {name}: segment {segment:#x} changed"
                )
        for register, expected in (
            (UC_X86_REG_DS, game_segment),
            (UC_X86_REG_ES, extra_segment),
            (UC_X86_REG_FS, game_segment),
            (UC_X86_REG_GS, decoy_segment),
            (UC_X86_REG_SS, stack_segment),
            (UC_X86_REG_SP, 0xFF02),
        ):
            actual = machine.reg_read(register)
            if actual != expected:
                raise AssertionError(
                    f"{module}:{entry:#x} {name}: register {register}="
                    f"{actual:#x}, expected={expected:#x}"
                )
        if bytes(machine.mem_read(stack_segment * 16 + 0xFF02, 6)) != stack_sentinel:
            raise AssertionError(f"{module}:{entry:#x} {name}: stack changed")

        expected_flags = add_flags_16(
            object_offset + (state_count - 1) * 0x14,
            0x14,
            initial_flags,
        )
        flag_masks = {
            "cf": 0x0001,
            "pf": 0x0004,
            "af": 0x0010,
            "zf": 0x0040,
            "sf": 0x0080,
            "if": 0x0200,
            "df": 0x0400,
            "of": 0x0800,
        }
        flags_after = machine.reg_read(UC_X86_REG_EFLAGS)
        actual_flags = {
            flag: bool(flags_after & mask) for flag, mask in flag_masks.items()
        }
        if actual_flags != expected_flags:
            raise AssertionError(
                f"{module}:{entry:#x} {name}: flags={actual_flags}, "
                f"expected={expected_flags}"
            )
        vectors.append(
            {
                "name": name,
                "module": module,
                "entry": entry,
                "path": "update",
                "selection_before": selection_state,
                "selection_after": 2 if selected else selection_state,
                "primary_step_after": effective_step,
                "primary_phase_after": (primary_phase + effective_step) & 0xFFFF,
                "secondary_phase_after": secondary_phase,
                "branch_classes": branch_classes,
                "game_data_sha256": hashlib.sha256(game_expected).hexdigest(),
                "object_data_sha256": hashlib.sha256(object_expected).hexdigest(),
                "code_image_sha256": hashlib.sha256(code_expected).hexdigest(),
                "defined_flags": expected_flags,
            }
        )

    return vectors


def alien_slot3_update_or_init_vectors(
    module: str,
    entry: int,
    block_start: int,
    timer_offset: int,
    generation_offset: int,
    cursor_offset: int,
    ring_offset: int,
    initial_callback: int,
    generic_callback: int,
    initial_position: tuple[int, int, int],
) -> list[dict[str, object]]:
    image = load_image(module)
    body_end = entry + 45
    expected_hashes = {
        "amer": "69eb44d3314aa67fcb427aa63c764d3b67ca4286e5ffad553b7488605b1d594b",
        "croolis": "b460ca3c02bd2ce6f46461755f989d0a2914ba888d86c4c665b0f4ad6cfb394b",
        "scrut": "20158654ed5aebb114a5588d50d623c6468dbf626533d846f5a47c5746e6c0a5",
    }
    if hashlib.sha256(image[block_start:body_end]).hexdigest() != expected_hashes[module]:
        raise AssertionError(
            f"{module}:{entry:#x}: recovered 336-byte slot-3 body changed"
        )

    data_segment = 0x5000
    extra_segment = 0x7000
    game_segment = 0x9000
    stack_segment = 0xB000
    context_offset = 0x3000
    return_address = 0xF000
    callback_stub = 0xF100
    callback_call = entry + 0x23
    callback_stub_bytes = bytes.fromhex(
        "b8 11 11 bb 22 22 b9 33 33 ba 44 44 c3"
    )
    stack_sentinel = bytes.fromhex("5aa596698778")
    vectors: list[dict[str, object]] = []

    def put_bytes(memory: bytearray, offset: int, value: bytes) -> None:
        for index, byte in enumerate(value):
            memory[(offset + index) & 0xFFFF] = byte

    def put_u16(memory: bytearray, offset: int, value: int) -> None:
        put_bytes(memory, offset, struct.pack("<H", value & 0xFFFF))

    def put_u32(memory: bytearray, offset: int, value: int) -> None:
        put_bytes(memory, offset, struct.pack("<I", value & 0xFFFFFFFF))

    def get_u16(memory: bytearray, offset: int) -> int:
        return memory[offset & 0xFFFF] | (memory[(offset + 1) & 0xFFFF] << 8)

    def put_state_common(
        memory: bytearray,
        state: int,
        callback: int,
        position: tuple[int, int, int],
    ) -> None:
        put_u16(memory, state + 0x0E, callback)
        for field in (0x4E, 0x50, 0x52, 0x54):
            put_u16(memory, state + field, 0)
        for field, value in zip((0x42, 0x46, 0x4A), position):
            put_u32(memory, state + field, value)

    init_cases = (
        ("initialize_one", 0x4000, 1, 0x0180, 0x1234),
        ("initialize_three", 0x4400, 3, 0x02A0, 0x0000),
        ("generation_wrap", 0x4800, 2, 0x0000, 0xFFFF),
        ("state_and_ring_wrap", 0xFFA0, 2, 0x0000, 0x0001),
    )
    for case_index, (name, state_base, state_count, initial_cursor, generation) in enumerate(init_cases):
        data_before = bytearray(
            ((offset * 29 + case_index * 17 + 3) & 0xFF)
            for offset in range(0x10000)
        )
        put_u16(data_before, context_offset + 0x16, state_base)
        put_u16(data_before, context_offset + 0x1A, state_count)
        put_u16(data_before, context_offset + 0x36, 0)
        data_expected = bytearray(data_before)
        code_before = bytearray(image)
        put_u16(code_before, cursor_offset, initial_cursor)
        put_u16(code_before, generation_offset, generation)
        code_expected = bytearray(code_before)
        put_u16(code_expected, timer_offset, 7)
        code_expected[return_address] = 0xCC

        state = (state_base + 0x5E) & 0xFFFF
        ring_cursor = initial_cursor
        put_u16(data_expected, context_offset + 0x36, 1)
        put_state_common(data_expected, state, initial_callback, initial_position)
        put_u16(data_expected, state + 0x56, 0x19)
        put_u16(data_expected, state + 0x58, 0)
        put_u16(data_expected, state + 0x5A, ring_cursor)
        put_u16(data_expected, state + 0x5C, 0xA957)
        for field, value in ((0, 0), (2, 0), (4, 0x46), (6, 0)):
            put_u16(code_expected, ring_offset + ring_cursor + field, value)

        remaining = (state_count - 1) & 0xFFFF
        phase = 0
        if remaining != 0:
            ring_cursor = (ring_cursor - 8) & 0xFFFF
            generation = (generation + 1) & 0xFFFF
            put_u16(code_expected, generation_offset, generation)
            if generation != 0:
                put_u16(data_expected, context_offset + 0x36, 0xFFFF)
                put_u16(data_expected, state + 0x0E, generic_callback)
                put_u16(code_expected, ring_offset + ring_cursor + 4, 0)
                for field in (0x4E, 0x50, 0x52):
                    put_u16(data_expected, state + field, 0)
                for field, value in zip((0x42, 0x46, 0x4A), initial_position):
                    put_u32(data_expected, state + field, value)

            while remaining != 0:
                state = (state + 0x5E) & 0xFFFF
                ring_cursor = (ring_cursor - 8) & 0x3FF
                phase = (phase + 0x100) & 0xFFFF
                put_state_common(
                    data_expected, state, generic_callback, initial_position
                )
                put_u16(data_expected, state + 0x58, phase)
                put_u16(data_expected, state + 0x5A, ring_cursor)
                put_u16(data_expected, state + 0x5C, 0)
                for field in (0, 2, 4, 6):
                    put_u16(code_expected, ring_offset + ring_cursor + field, 0)
                remaining = (remaining - 1) & 0xFFFF

        ring_cursor = (ring_cursor - 8) & 0x3FC
        put_u16(code_expected, cursor_offset, ring_cursor)
        initial_flags = 0x0693 | (0x0400 if case_index & 1 else 0)
        initial = {
            "eax": 0xA1A1BEEF + case_index,
            "ebx": 0xB2B22345 + case_index,
            "ecx": 0xC3C30000 | ((0x4567 + case_index) & 0xFFFF),
            "edx": 0xD4D44567 + case_index,
            "esi": 0xE5E55678 + case_index,
            "edi": 0xF6F60000 | context_offset,
            "ebp": 0x9797789A + case_index,
            "sp": 0xFF00,
            "ds": data_segment,
            "es": extra_segment,
            "fs": 0xA000,
            "gs": game_segment,
            "ss": stack_segment,
            "flags": initial_flags,
        }
        extra_before = bytes(
            (offset * 13 + case_index + 7) & 0xFF for offset in range(0x10000)
        )
        game_before = bytes(
            (offset * 11 + case_index + 9) & 0xFF for offset in range(0x10000)
        )
        machine = execute(
            bytes(code_before),
            entry,
            return_address,
            initial,
            [
                (0, return_address, b"\xcc"),
                (data_segment, 0, bytes(data_before)),
                (extra_segment, 0, extra_before),
                (game_segment, 0, game_before),
                (
                    stack_segment,
                    0xFF00,
                    struct.pack("<H", return_address) + stack_sentinel,
                ),
            ],
            max_instructions=10000,
        )
        actual_data = bytes(machine.mem_read(data_segment * 16, 0x10000))
        if actual_data != bytes(data_expected):
            differences = [
                (offset, actual_data[offset], data_expected[offset])
                for offset in range(0x10000)
                if actual_data[offset] != data_expected[offset]
            ][:8]
            raise AssertionError(
                f"{module}:{entry:#x} {name}: data differs at {differences}"
            )
        actual_code = bytes(machine.mem_read(0, len(image)))
        if actual_code != bytes(code_expected):
            differences = [
                (offset, actual_code[offset], code_expected[offset])
                for offset in range(len(image))
                if actual_code[offset] != code_expected[offset]
            ][:8]
            raise AssertionError(
                f"{module}:{entry:#x} {name}: code data differs at {differences}"
            )
        for segment, expected in ((extra_segment, extra_before), (game_segment, game_before)):
            if bytes(machine.mem_read(segment * 16, 0x10000)) != expected:
                raise AssertionError(
                    f"{module}:{entry:#x} {name}: decoy segment {segment:#x} changed"
                )

        expected_registers = dict(initial)
        del expected_registers["flags"]
        expected_registers["eax"] = initial_position[0] & 0xFFFFFFFF
        expected_registers["ebx"] = initial_position[1] & 0xFFFFFFFF
        expected_registers["ecx"] &= 0xFFFF0000
        expected_registers["edx"] = initial_position[2] & 0xFFFFFFFF
        expected_registers["esi"] = (initial["esi"] & 0xFFFF0000) | state
        expected_registers["ebp"] = (
            (initial["ebp"] & 0xFFFF0000) | ring_cursor
        )
        if state_count != 1:
            expected_registers["edi"] = (
                (initial["edi"] & 0xFFFF0000) | phase
            )
        expected_registers["sp"] = 0xFF02
        for register, expected in expected_registers.items():
            actual = machine.reg_read(REGISTERS[register])
            if actual != expected:
                raise AssertionError(
                    f"{module}:{entry:#x} {name}: "
                    f"{register}={actual:#x}, expected={expected:#x}"
                )
        expected_flags = {
            "cf": False,
            "pf": bin(ring_cursor & 0xFF).count("1") % 2 == 0,
            "zf": ring_cursor == 0,
            "sf": bool(ring_cursor & 0x8000),
            "if": bool(initial_flags & 0x0200),
            "df": bool(initial_flags & 0x0400),
            "of": False,
        }
        flags_after = machine.reg_read(UC_X86_REG_EFLAGS)
        flag_masks = {
            "cf": 0x0001,
            "pf": 0x0004,
            "zf": 0x0040,
            "sf": 0x0080,
            "if": 0x0200,
            "df": 0x0400,
            "of": 0x0800,
        }
        actual_flags = {
            flag: bool(flags_after & mask) for flag, mask in flag_masks.items()
        }
        if actual_flags != expected_flags:
            raise AssertionError(
                f"{module}:{entry:#x} {name}: "
                f"flags={actual_flags}, expected={expected_flags}"
            )
        if bytes(machine.mem_read(stack_segment * 16 + 0xFF02, 6)) != stack_sentinel:
            raise AssertionError(f"{module}:{entry:#x} {name}: stack changed")

        vectors.append(
            {
                "name": name,
                "module": module,
                "entry": entry,
                "path": "initialize",
                "state_count": state_count,
                "state_base": state_base,
                "generation_after": generation,
                "context_state_after": get_u16(data_expected, context_offset + 0x36),
                "ring_cursor_after": ring_cursor,
                "last_state": state,
                "data_sha256": hashlib.sha256(data_expected).hexdigest(),
                "code_image_sha256": hashlib.sha256(code_expected).hexdigest(),
                "defined_flags": expected_flags,
            }
        )

    callback_cases = (
        ("negative_state", 0xFFFF, 0xA55A, 2),
        ("positive_timer", 1, 2, 3),
        ("timer_reset", 1, 0, 1),
        ("timer_sign_cross", 1, 0x8000, 2),
        ("zero_count", 0xFFFF, 0x1357, 0),
    )
    for case_index, (name, method_state, timer_before, state_count) in enumerate(callback_cases):
        state_base = 0x2000 + case_index * 0x0200
        data_before = bytearray(
            ((offset * 37 + case_index * 19 + 5) & 0xFF)
            for offset in range(0x10000)
        )
        put_u16(data_before, context_offset + 0x16, state_base)
        put_u16(data_before, context_offset + 0x1A, state_count)
        put_u16(data_before, context_offset + 0x36, method_state)
        effective_count = state_count if state_count != 0 else 0x10000
        state = (state_base + 0x5E) & 0xFFFF
        expected_states = [] if state_count == 0 else [
            (state + index * 0x5E) & 0xFFFF for index in range(effective_count)
        ]
        if state_count != 0:
            for callback_state in expected_states:
                put_u16(data_before, callback_state + 0x0E, callback_stub)
        data_expected = bytearray(data_before)
        code_before = bytearray(image)
        put_u16(code_before, timer_offset, timer_before)
        timer_after = timer_before
        if method_state & 0x8000 == 0:
            timer_after = (timer_before - 1) & 0xFFFF
            if timer_after & 0x8000:
                timer_after = 7
        code_expected = bytearray(code_before)
        put_u16(code_expected, timer_offset, timer_after)
        initial_flags = 0x0293 | (0x0400 if case_index & 1 else 0)
        initial = {
            "eax": 0xA1A1BEEF + case_index,
            "ebx": 0xB2B22345 + case_index,
            "ecx": 0xC3C33456 + case_index,
            "edx": 0xD4D44567 + case_index,
            "esi": 0xE5E55678 + case_index,
            "edi": 0xF6F60000 | context_offset,
            "ebp": 0x9797789A + case_index,
            "sp": 0xFF00,
            "ds": data_segment,
            "es": extra_segment,
            "fs": 0xA000,
            "gs": game_segment,
            "ss": stack_segment,
            "flags": initial_flags,
        }
        callback_count = 0
        first_callback_state: int | None = None
        last_callback_state: int | None = None

        def code_handler(machine: Uc, address: int, _size: int, _data: object) -> None:
            nonlocal callback_count, first_callback_state, last_callback_state
            if address == callback_call and state_count == 0:
                callback_state = machine.reg_read(UC_X86_REG_ESI) & 0xFFFF
                put_u16(data_expected, callback_state + 0x0E, callback_stub)
                machine.mem_write(
                    data_segment * 16 + ((callback_state + 0x0E) & 0xFFFF),
                    struct.pack("<H", callback_stub),
                )
            if address != callback_stub:
                return
            callback_state = machine.reg_read(UC_X86_REG_ESI) & 0xFFFF
            expected_state = (state + callback_count * 0x5E) & 0xFFFF
            if callback_state != expected_state:
                raise AssertionError(
                    f"{module}:{entry:#x} {name}: callback {callback_count} "
                    f"SI={callback_state:#x}, expected={expected_state:#x}"
                )
            if machine.reg_read(UC_X86_REG_SP) != 0xFEFC:
                raise AssertionError(
                    f"{module}:{entry:#x} {name}: callback stack changed"
                )
            if first_callback_state is None:
                first_callback_state = callback_state
            last_callback_state = callback_state
            callback_count += 1

        extra_before = bytes(
            (offset * 13 + case_index + 7) & 0xFF for offset in range(0x10000)
        )
        game_before = bytes(
            (offset * 11 + case_index + 9) & 0xFF for offset in range(0x10000)
        )
        machine = execute(
            bytes(code_before),
            entry,
            return_address,
            initial,
            [
                (0, return_address, b"\xcc"),
                (0, callback_stub, callback_stub_bytes),
                (data_segment, 0, bytes(data_before)),
                (extra_segment, 0, extra_before),
                (game_segment, 0, game_before),
                (
                    stack_segment,
                    0xFF00,
                    struct.pack("<H", return_address) + stack_sentinel,
                ),
            ],
            code_handler=code_handler,
            max_instructions=1000000,
        )
        if callback_count != effective_count:
            raise AssertionError(
                f"{module}:{entry:#x} {name}: callbacks={callback_count}, "
                f"expected={effective_count}"
            )
        if bytes(machine.mem_read(data_segment * 16, 0x10000)) != bytes(data_expected):
            raise AssertionError(f"{module}:{entry:#x} {name}: data changed")
        if get_u16(bytearray(machine.mem_read(0, len(image))), timer_offset) != timer_after:
            raise AssertionError(f"{module}:{entry:#x} {name}: timer mismatch")
        for segment, expected in ((extra_segment, extra_before), (game_segment, game_before)):
            if bytes(machine.mem_read(segment * 16, 0x10000)) != expected:
                raise AssertionError(
                    f"{module}:{entry:#x} {name}: decoy segment {segment:#x} changed"
                )

        final_state = (state + effective_count * 0x5E) & 0xFFFF
        expected_registers = dict(initial)
        del expected_registers["flags"]
        expected_registers["eax"] = (initial["eax"] & 0xFFFF0000) | 0x1111
        expected_registers["ebx"] = (initial["ebx"] & 0xFFFF0000) | 0x2222
        expected_registers["ecx"] &= 0xFFFF0000
        expected_registers["edx"] = (initial["edx"] & 0xFFFF0000) | 0x4444
        expected_registers["esi"] = (initial["esi"] & 0xFFFF0000) | final_state
        expected_registers["sp"] = 0xFF02
        for register, expected in expected_registers.items():
            actual = machine.reg_read(REGISTERS[register])
            if actual != expected:
                raise AssertionError(
                    f"{module}:{entry:#x} {name}: "
                    f"{register}={actual:#x}, expected={expected:#x}"
                )
        previous_state = (final_state - 0x5E) & 0xFFFF
        expected_flags = add_flags_16(previous_state, 0x5E, initial_flags)
        flag_masks = {
            "cf": 0x0001,
            "pf": 0x0004,
            "af": 0x0010,
            "zf": 0x0040,
            "sf": 0x0080,
            "if": 0x0200,
            "df": 0x0400,
            "of": 0x0800,
        }
        flags_after = machine.reg_read(UC_X86_REG_EFLAGS)
        actual_flags = {
            flag: bool(flags_after & mask) for flag, mask in flag_masks.items()
        }
        if actual_flags != expected_flags:
            raise AssertionError(
                f"{module}:{entry:#x} {name}: "
                f"flags={actual_flags}, expected={expected_flags}"
            )
        if bytes(machine.mem_read(stack_segment * 16 + 0xFF02, 6)) != stack_sentinel:
            raise AssertionError(f"{module}:{entry:#x} {name}: stack changed")

        vectors.append(
            {
                "name": name,
                "module": module,
                "entry": entry,
                "path": "callbacks",
                "method_state": method_state,
                "timer_before": timer_before,
                "timer_after": timer_after,
                "state_count": state_count,
                "effective_callbacks": effective_count,
                "first_callback_state": first_callback_state,
                "last_callback_state": last_callback_state,
                "final_state": final_state,
                "data_sha256": hashlib.sha256(data_expected).hexdigest(),
                "defined_flags": expected_flags,
            }
        )

    return vectors


def alien_slot2_dispatch_or_init_vectors(
    module: str,
    entry: int,
    body_size: int,
    body_hash: str,
    seed_offset: int | None,
    seed_step: int,
    initial_callback: int,
) -> list[dict[str, object]]:
    image = load_image(module)
    if hashlib.sha256(image[entry : entry + body_size]).hexdigest() != body_hash:
        raise AssertionError(
            f"{module}:{entry:#x}: recovered {body_size}-byte slot-2 body changed"
        )

    data_segment = 0x5000
    extra_segment = 0x7000
    fs_segment = 0x9000
    game_segment = 0xA000
    stack_segment = 0xB000
    context_offset = 0x3000
    return_address = 0xF000
    callback_stub = 0xF100
    callback_stub_bytes = bytes.fromhex(
        "b8 11 11 bb 22 22 b9 33 33 ba 44 44 c3"
    )
    stack_sentinel = bytes.fromhex("5aa596698778")
    flag_masks = {
        "cf": 0x0001,
        "pf": 0x0004,
        "af": 0x0010,
        "zf": 0x0040,
        "sf": 0x0080,
        "if": 0x0200,
        "df": 0x0400,
        "of": 0x0800,
    }
    vectors: list[dict[str, object]] = []

    def put_bytes(memory: bytearray, offset: int, value: bytes) -> None:
        for index, byte in enumerate(value):
            memory[(offset + index) & 0xFFFF] = byte

    def put_u16(memory: bytearray, offset: int, value: int) -> None:
        put_bytes(memory, offset, struct.pack("<H", value & 0xFFFF))

    def put_u32(memory: bytearray, offset: int, value: int) -> None:
        put_bytes(memory, offset, struct.pack("<I", value & 0xFFFFFFFF))

    def get_u16(memory: bytearray, offset: int) -> int:
        return memory[offset & 0xFFFF] | (memory[(offset + 1) & 0xFFFF] << 8)

    def transform_random(value: int) -> int:
        rotated = ((value >> 7) | (value << 9)) & 0xFFFF
        return (rotated - ((value >> 6) & 1)) & 0xFFFF

    def patterned(multiplier: int, addend: int) -> bytes:
        return bytes(
            (offset * multiplier + addend) & 0xFF
            for offset in range(0x10000)
        )

    def initial_registers(case_index: int, flags: int) -> dict[str, int]:
        return {
            "eax": 0xA1A10000 | ((0xBEEF + case_index) & 0xFFFF),
            "ebx": 0xB2B20000 | ((0x2345 + case_index) & 0xFFFF),
            "ecx": 0xC3C30000 | ((0x3456 + case_index) & 0xFFFF),
            "edx": 0xD4D40000 | ((0x4567 + case_index) & 0xFFFF),
            "esi": 0xE5E50000 | ((0x5678 + case_index) & 0xFFFF),
            "edi": 0xF6F60000 | context_offset,
            "ebp": 0x97970000 | ((0x789A + case_index) & 0xFFFF),
            "sp": 0xFF00,
            "ds": data_segment,
            "es": extra_segment,
            "fs": fs_segment,
            "gs": game_segment,
            "ss": stack_segment,
            "flags": flags,
        }

    def compare_segment(
        machine: Uc, segment: int, expected: bytes, name: str
    ) -> None:
        actual = bytes(machine.mem_read(segment * 16, 0x10000))
        if actual == expected:
            return
        differences = [
            (offset, actual[offset], expected[offset])
            for offset in range(0x10000)
            if actual[offset] != expected[offset]
        ][:8]
        raise AssertionError(
            f"{module}:{entry:#x} {name}: segment {segment:#x} "
            f"differs at {differences}"
        )

    def compare_registers(
        machine: Uc, expected: dict[str, int], name: str
    ) -> None:
        for register, value in expected.items():
            actual = machine.reg_read(REGISTERS[register])
            if actual != value:
                raise AssertionError(
                    f"{module}:{entry:#x} {name}: "
                    f"{register}={actual:#x}, expected={value:#x}"
                )

    def compare_flags(
        machine: Uc, expected: dict[str, bool], name: str
    ) -> None:
        flags = machine.reg_read(UC_X86_REG_EFLAGS)
        actual = {
            flag: bool(flags & flag_masks[flag]) for flag in expected
        }
        if actual != expected:
            raise AssertionError(
                f"{module}:{entry:#x} {name}: "
                f"flags={actual}, expected={expected}"
            )

    # A nonzero control word must tail-jump through the first biased state.
    state_base = 0x2340
    state = (state_base + 0x5E) & 0xFFFF
    data_before = bytearray(patterned(37, 11))
    put_u16(data_before, context_offset + 0x16, state_base)
    put_u16(data_before, context_offset + 0x36, 0x8001)
    put_u16(data_before, state + 0x0E, callback_stub)
    code_before = bytearray(image)
    code_before[return_address] = 0xCC
    code_before[callback_stub : callback_stub + len(callback_stub_bytes)] = (
        callback_stub_bytes
    )
    fs_before = patterned(17, 5)
    extra_before = patterned(13, 7)
    game_before = patterned(11, 9)
    initial_flags = 0x0693
    initial = initial_registers(0, initial_flags)
    callback_entries: list[dict[str, int]] = []

    def callback_handler(
        machine: Uc, address: int, _size: int, _data: object
    ) -> None:
        if address == callback_stub:
            callback_entries.append(
                {
                    "si": machine.reg_read(UC_X86_REG_ESI) & 0xFFFF,
                    "di": machine.reg_read(UC_X86_REG_EDI) & 0xFFFF,
                    "sp": machine.reg_read(UC_X86_REG_SP),
                }
            )

    machine = execute(
        bytes(code_before),
        entry,
        return_address,
        initial,
        [
            (data_segment, 0, bytes(data_before)),
            (extra_segment, 0, extra_before),
            (fs_segment, 0, fs_before),
            (game_segment, 0, game_before),
            (
                stack_segment,
                0xFF00,
                struct.pack("<H", return_address) + stack_sentinel,
            ),
        ],
        code_handler=callback_handler,
    )
    expected_callback = {"si": state, "di": context_offset, "sp": 0xFF00}
    if callback_entries != [expected_callback]:
        raise AssertionError(
            f"{module}:{entry:#x} callback dispatch: "
            f"entries={callback_entries}, expected={[expected_callback]}"
        )
    compare_segment(machine, data_segment, bytes(data_before), "callback dispatch")
    compare_segment(machine, extra_segment, extra_before, "callback dispatch")
    compare_segment(machine, fs_segment, fs_before, "callback dispatch")
    compare_segment(machine, game_segment, game_before, "callback dispatch")
    expected_registers = dict(initial)
    del expected_registers["flags"]
    expected_registers.update(
        {
            "eax": (initial["eax"] & 0xFFFF0000) | 0x1111,
            "ebx": (initial["ebx"] & 0xFFFF0000) | 0x2222,
            "ecx": (initial["ecx"] & 0xFFFF0000) | 0x3333,
            "edx": (initial["edx"] & 0xFFFF0000) | 0x4444,
            "esi": (initial["esi"] & 0xFFFF0000) | state,
            "sp": 0xFF02,
        }
    )
    compare_registers(machine, expected_registers, "callback dispatch")
    expected_flags = {
        "cf": False,
        "pf": False,
        "zf": False,
        "sf": True,
        "if": True,
        "df": True,
        "of": False,
    }
    compare_flags(machine, expected_flags, "callback dispatch")
    if bytes(machine.mem_read(stack_segment * 16 + 0xFF02, 6)) != stack_sentinel:
        raise AssertionError(f"{module}:{entry:#x} callback dispatch: stack changed")
    vectors.append(
        {
            "name": "existing_callback_tail_jump",
            "module": module,
            "entry": entry,
            "path": "callback",
            "state": state,
            "callback_entry": expected_callback,
            "control_state": -32767,
            "defined_flags": expected_flags,
        }
    )

    if module == "amer":
        init_cases = (
            ("random_zero", 0x4000, 1, 0x0000, 0),
            ("random_carry", 0x4400, 3, 0x0040, 0),
            ("random_high", 0x4800, 0, 0x8001, 0),
            ("state_wrap", 0xFFA0, 2, 0xFFFF, 0),
        )
    else:
        init_cases = (
            ("two_states_positive_seed", 0x4000, 2, 0x0000, 0x1234),
            ("three_states_negative_seed", 0x4400, 3, 0x0040, 0x8001),
            ("count_one_full_loop", 0x4800, 1, 0x8001, 0x7FFF),
            ("count_zero_65535_loops", 0xFFA0, 0, 0xFFFF, 0xFFFF),
        )

    for case_index, case in enumerate(init_cases):
        name, state_base, state_count, random_before, seed_before = case
        data_before = bytearray(patterned(29, case_index * 17 + 3))
        put_u16(data_before, context_offset + 0x16, state_base)
        put_u16(data_before, context_offset + 0x1A, state_count)
        put_u16(data_before, context_offset + 0x36, 0)
        data_expected = bytearray(data_before)
        fs_before = bytearray(patterned(19, case_index * 23 + 5))
        put_u16(fs_before, 0x105C, random_before)
        fs_expected = bytearray(fs_before)
        code_before = bytearray(image)
        code_before[return_address] = 0xCC
        code_expected = bytearray(code_before)
        if seed_offset is not None:
            put_u16(code_before, seed_offset, seed_before)
            put_u16(code_expected, seed_offset, seed_before + seed_step)

        first_random = transform_random(random_before)
        second_random = transform_random(first_random)
        put_u16(fs_expected, 0x105C, first_random)
        state = (state_base + 0x5E) & 0xFFFF
        put_u16(data_expected, context_offset + 0x36, 1)
        previous_state = state

        if module == "amer":
            put_u16(data_expected, context_offset + 0x38, 0)
            put_u16(data_expected, context_offset + 0x40, first_random)
            put_u16(data_expected, state + 0x0E, initial_callback)
            put_u16(data_expected, state + 0x50, first_random & 0x0FFC)
            put_u16(data_expected, state + 0x58, 0x14)
            loop_iterations = 0
        else:
            signed_seed = seed_before
            if signed_seed & 0x8000:
                signed_seed -= 0x10000
            put_u16(data_expected, context_offset + 0x38, 0x32)
            if module == "croolis":
                put_u16(data_expected, context_offset + 0x3A, 0)
                put_u32(data_expected, context_offset + 0x3C, signed_seed)
            else:
                put_u32(data_expected, context_offset + 0x3A, signed_seed)
            put_u16(data_expected, context_offset + 0x42, second_random)
            put_u16(data_expected, state + 0x0E, initial_callback)
            put_u16(data_expected, state + 0x50, second_random & 0x0FFC)
            put_u16(data_expected, state + 0x52, 0)
            put_u16(data_expected, state + 0x54, 0)
            put_u16(data_expected, state + 0x56, 0)
            put_u16(data_expected, state + 0x58, 0)
            if module == "scrut":
                put_u16(data_expected, state + 0x5A, 0)

            remaining = (state_count - 1) & 0xFFFF
            loop_iterations = 0
            while True:
                previous_state = state
                state = (state + 0x5E) & 0xFFFF
                if module == "croolis":
                    put_u16(
                        data_expected,
                        state + 0x56,
                        get_u16(data_expected, state + 0x4A),
                    )
                else:
                    put_u16(
                        data_expected,
                        state + 0x56,
                        get_u16(data_expected, state + 0x42),
                    )
                    put_u16(
                        data_expected,
                        state + 0x5A,
                        get_u16(data_expected, state + 0x4A),
                    )
                loop_iterations += 1
                remaining = (remaining - 1) & 0xFFFF
                if remaining == 0:
                    break

        initial_flags = 0x0293 | (0x0400 if case_index & 1 else 0)
        initial = initial_registers(case_index, initial_flags)
        extra_before = patterned(13, case_index + 7)
        game_before = patterned(11, case_index + 9)
        machine = execute(
            bytes(code_before),
            entry,
            return_address,
            initial,
            [
                (data_segment, 0, bytes(data_before)),
                (extra_segment, 0, extra_before),
                (fs_segment, 0, bytes(fs_before)),
                (game_segment, 0, game_before),
                (
                    stack_segment,
                    0xFF00,
                    struct.pack("<H", return_address) + stack_sentinel,
                ),
            ],
            max_instructions=600000,
        )
        compare_segment(machine, data_segment, bytes(data_expected), name)
        compare_segment(machine, extra_segment, extra_before, name)
        compare_segment(machine, fs_segment, bytes(fs_expected), name)
        compare_segment(machine, game_segment, game_before, name)
        actual_code = bytes(machine.mem_read(0, len(image)))
        if actual_code != bytes(code_expected):
            raise AssertionError(f"{module}:{entry:#x} {name}: code data changed")

        expected_registers = dict(initial)
        del expected_registers["flags"]
        expected_registers["esi"] = (initial["esi"] & 0xFFFF0000) | state
        expected_registers["sp"] = 0xFF02
        if module == "amer":
            expected_registers["eax"] = (
                initial["eax"] & 0xFFFF0000
            ) | (first_random & 0x0FFC)
        else:
            expected_registers["ecx"] &= 0xFFFF0000
            ax_field = 0x4A if module == "croolis" else 0x42
            expected_registers["eax"] = (
                initial["eax"] & 0xFFFF0000
            ) | get_u16(data_expected, state + ax_field)
            seed_high = 0xFFFF0000 if seed_before & 0x8000 else 0
            bx_low = (seed_before + seed_step) & 0xFFFF
            if module == "scrut":
                bx_low = get_u16(data_expected, state + 0x4A)
            expected_registers["ebx"] = seed_high | bx_low
        compare_registers(machine, expected_registers, name)

        if module == "amer":
            masked = first_random & 0x0FFC
            expected_flags = {
                "cf": False,
                "pf": (masked & 0xFF).bit_count() % 2 == 0,
                "zf": masked == 0,
                "sf": False,
                "if": bool(initial_flags & 0x0200),
                "df": bool(initial_flags & 0x0400),
                "of": False,
            }
        else:
            expected_flags = add_flags_16(previous_state, 0x5E, initial_flags)
        compare_flags(machine, expected_flags, name)
        if bytes(machine.mem_read(stack_segment * 16 + 0xFF02, 6)) != stack_sentinel:
            raise AssertionError(f"{module}:{entry:#x} {name}: stack changed")

        vectors.append(
            {
                "name": name,
                "module": module,
                "entry": entry,
                "path": "initialize",
                "state_base": state_base,
                "state_count": state_count,
                "loop_iterations": loop_iterations,
                "last_state": state,
                "random_before": random_before,
                "random_after": first_random,
                "context_random": (
                    first_random if module == "amer" else second_random
                ),
                "seed_before": seed_before if seed_offset is not None else None,
                "seed_after": (
                    (seed_before + seed_step) & 0xFFFF
                    if seed_offset is not None
                    else None
                ),
                "data_sha256": hashlib.sha256(data_expected).hexdigest(),
                "fs_sha256": hashlib.sha256(fs_expected).hexdigest(),
                "defined_flags": expected_flags,
            }
        )

    return vectors


def amer_slot2_return_update_vectors(entry: int) -> list[dict[str, object]]:
    module = "amer"
    image = load_image(module)
    body_size = 107
    body_hash = "80758c3f56df8bc6ca5779d4813b1a9ee671aa7875e0cae20ffb03436de81de9"
    if hashlib.sha256(image[entry : entry + body_size]).hexdigest() != body_hash:
        raise AssertionError(f"{module}:{entry:#x}: recovered body changed")

    data_segment = 0x5000
    extra_segment = 0x7000
    fs_segment = 0x9000
    game_segment = 0xA000
    stack_segment = 0xB000
    context = 0x3000
    return_address = 0xF000
    active_offset = 0x1648
    stack_sentinel = bytes.fromhex("5aa596698778")
    cases = (
        ("continue_zero", 0x4000, 1, (0, 0, 0), (0, 0, 0), 0x1111, 0x2222),
        (
            "continue_signed_wrap",
            0x4400,
            2,
            (-1, 0x7FFF, -0x8000),
            (0xFFFFFFFF, 0x7FFFFFFF, 0x80000000),
            0xFFF0,
            0x0040,
        ),
        (
            "continue_countdown_sign_wrap",
            0xFFB8,
            0x8000,
            (0x7FFF, -0x8000, 1),
            (0x7FFFFFFF, 0x80000000, 0xFFFFFFFF),
            0xFF90,
            0x0074,
        ),
        ("transition_zero", 0x4800, 0, (1, 2, 3), (4, 5, 6), 0, 0),
        (
            "transition_positive_12bit",
            0x4C00,
            0x8001,
            (-9, 0x1234, -0x1234),
            (7, 8, 9),
            0x07FF,
            0x07FF,
        ),
        (
            "transition_negative_12bit",
            0x5000,
            0x0000,
            (0x1111, -1, 1),
            (0x12345678, 0x89ABCDEF, 0x80000000),
            0x0800,
            0x0800,
        ),
        (
            "transition_minus_one",
            0x5400,
            0xFFFF,
            (0x2222, -0x3333, 0x4444),
            (0xFFFFFFFF, 0, 0x7FFFFFFF),
            0xFFFF,
            0x0FFF,
        ),
    )
    flag_masks = {
        "cf": 0x0001,
        "pf": 0x0004,
        "af": 0x0010,
        "zf": 0x0040,
        "sf": 0x0080,
        "if": 0x0200,
        "df": 0x0400,
        "of": 0x0800,
    }
    vectors: list[dict[str, object]] = []

    def put_bytes(memory: bytearray, offset: int, value: bytes) -> None:
        offset &= 0xFFFF
        for index, byte in enumerate(value):
            memory[offset + index] = byte

    def put_u16(memory: bytearray, offset: int, value: int) -> None:
        put_bytes(memory, offset, struct.pack("<H", value & 0xFFFF))

    def put_u32(memory: bytearray, offset: int, value: int) -> None:
        put_bytes(memory, offset, struct.pack("<I", value & 0xFFFFFFFF))

    def get_u16(memory: bytearray, offset: int) -> int:
        offset &= 0xFFFF
        return memory[offset] | (memory[offset + 1] << 8)

    def get_u32(memory: bytearray, offset: int) -> int:
        offset &= 0xFFFF
        return sum(
            memory[offset + index] << (index * 8)
            for index in range(4)
        )

    def signed_word(value: int) -> int:
        value &= 0xFFFF
        return value if value < 0x8000 else value - 0x10000

    def sign_extend_32(value: int) -> int:
        return value & 0xFFFFFFFF

    def add_flags_32(left: int, right: int, initial_flags: int) -> dict[str, bool]:
        right &= 0xFFFFFFFF
        total = left + right
        result = total & 0xFFFFFFFF
        return {
            "cf": total > 0xFFFFFFFF,
            "pf": (result & 0xFF).bit_count() % 2 == 0,
            "af": ((left & 0xF) + (right & 0xF)) > 0xF,
            "zf": result == 0,
            "sf": bool(result & 0x80000000),
            "if": bool(initial_flags & 0x0200),
            "df": bool(initial_flags & 0x0400),
            "of": bool((~(left ^ right) & (left ^ result) & 0x80000000)),
        }

    for case_index, case in enumerate(cases):
        (
            name,
            state,
            countdown,
            velocities,
            positions,
            field_050,
            field_052,
        ) = case
        data_before = bytearray(
            (offset * 29 + case_index * 17 + 3) & 0xFF
            for offset in range(0x10004)
        )
        put_u16(data_before, context + 0x36, 0xA55A)
        put_u16(data_before, context + 0x38, countdown)
        for offset, value in zip((0x3A, 0x3C, 0x3E), velocities):
            put_u16(data_before, context + offset, value)
        put_u16(data_before, state + 0x0E, 0xBEEF)
        for offset, value in zip((0x42, 0x46, 0x4A), positions):
            put_u32(data_before, state + offset, value)
        put_u16(data_before, state + 0x50, field_050)
        put_u16(data_before, state + 0x52, field_052)
        put_u16(data_before, state + 0x54, 0x1357)
        data_expected = bytearray(data_before)
        code_before = bytearray(image)
        code_before[return_address] = 0xCC
        active_before = (0x7000 + case_index) & 0xFFFF
        put_u16(code_before, active_offset, active_before)
        code_expected = bytearray(code_before)

        countdown_after = (countdown - 1) & 0xFFFF
        put_u16(data_expected, context + 0x38, countdown_after)
        transition = bool(countdown_after & 0x8000)
        put_u16(data_expected, state + 0x54, 0)
        if not transition:
            put_u16(data_expected, state + 0x50, field_050 + 0x80)
            put_u16(data_expected, state + 0x52, field_052 - 0x75)
            for offset, velocity in zip((0x42, 0x46, 0x4A), velocities):
                put_u32(
                    data_expected,
                    state + offset,
                    get_u32(data_expected, state + offset) + velocity,
                )
        else:
            normalized_050 = signed_word((field_050 << 4) & 0xFFFF) >> 4
            normalized_052 = signed_word((field_052 << 4) & 0xFFFF) >> 4
            velocity_x = signed_word(-normalized_052) >> 5
            put_u16(data_expected, context + 0x36, 1)
            put_u16(data_expected, context + 0x38, 0x20)
            put_u16(data_expected, context + 0x3A, velocity_x)
            put_u16(data_expected, state + 0x0E, 0x1692)
            put_u16(data_expected, state + 0x50, normalized_050)
            put_u16(data_expected, state + 0x52, normalized_052)
            put_u16(code_expected, active_offset, 0)

        initial_flags = 0x0293 | (0x0400 if case_index & 1 else 0)
        initial = {
            "eax": 0xA1A10000 | ((0xBEEF + case_index) & 0xFFFF),
            "ebx": 0xB2B20000 | ((0x2345 + case_index) & 0xFFFF),
            "ecx": 0xC3C30000 | ((0x3456 + case_index) & 0xFFFF),
            "edx": 0xD4D40000 | ((0x4567 + case_index) & 0xFFFF),
            "esi": 0xE5E50000 | state,
            "edi": 0xF6F60000 | context,
            "ebp": 0x97970000 | ((0x789A + case_index) & 0xFFFF),
            "sp": 0xFF00,
            "ds": data_segment,
            "es": extra_segment,
            "fs": fs_segment,
            "gs": game_segment,
            "ss": stack_segment,
            "flags": initial_flags,
        }
        extra_before = bytes(
            (offset * 13 + case_index + 7) & 0xFF for offset in range(0x10000)
        )
        fs_before = bytes(
            (offset * 19 + case_index + 5) & 0xFF for offset in range(0x10000)
        )
        game_before = bytes(
            (offset * 11 + case_index + 9) & 0xFF for offset in range(0x10000)
        )
        machine = execute(
            bytes(code_before),
            entry,
            return_address,
            initial,
            [
                (data_segment, 0, bytes(data_before)),
                (extra_segment, 0, extra_before),
                (fs_segment, 0, fs_before),
                (game_segment, 0, game_before),
                (
                    stack_segment,
                    0xFF00,
                    struct.pack("<H", return_address) + stack_sentinel,
                ),
            ],
        )
        actual_data = bytes(machine.mem_read(data_segment * 16, 0x10004))
        if actual_data != bytes(data_expected):
            differences = [
                (offset, actual_data[offset], data_expected[offset])
                for offset in range(0x10004)
                if actual_data[offset] != data_expected[offset]
            ][:8]
            raise AssertionError(
                f"{module}:{entry:#x} {name}: data differs at {differences}"
            )
        actual_code = bytes(machine.mem_read(0, len(image)))
        if actual_code != bytes(code_expected):
            raise AssertionError(f"{module}:{entry:#x} {name}: code data changed")
        for segment, expected in (
            (extra_segment, extra_before),
            (fs_segment, fs_before),
            (game_segment, game_before),
        ):
            if bytes(machine.mem_read(segment * 16, 0x10000)) != expected:
                raise AssertionError(
                    f"{module}:{entry:#x} {name}: segment {segment:#x} changed"
                )

        expected_registers = dict(initial)
        del expected_registers["flags"]
        expected_registers["sp"] = 0xFF02
        if not transition:
            expected_registers["eax"] = sign_extend_32(velocities[0])
            expected_registers["ebx"] = sign_extend_32(velocities[1])
            expected_registers["ecx"] = sign_extend_32(velocities[2])
            left = positions[2] & 0xFFFFFFFF
            expected_flags = add_flags_32(left, velocities[2], initial_flags)
        else:
            expected_registers["ebx"] = (
                initial["ebx"] & 0xFFFF0000
            ) | get_u16(data_expected, state + 0x50)
            expected_registers["ecx"] = (
                initial["ecx"] & 0xFFFF0000
            ) | get_u16(data_expected, context + 0x3A)
            shift_source = (-normalized_052) & 0xFFFF
            shift_result = get_u16(data_expected, context + 0x3A)
            expected_flags = {
                "cf": bool(shift_source & 0x10),
                "pf": (shift_result & 0xFF).bit_count() % 2 == 0,
                "zf": shift_result == 0,
                "sf": bool(shift_result & 0x8000),
                "if": bool(initial_flags & 0x0200),
                "df": bool(initial_flags & 0x0400),
            }
        for register, expected in expected_registers.items():
            actual = machine.reg_read(REGISTERS[register])
            if actual != expected:
                raise AssertionError(
                    f"{module}:{entry:#x} {name}: "
                    f"{register}={actual:#x}, expected={expected:#x}"
                )
        flags_after = machine.reg_read(UC_X86_REG_EFLAGS)
        actual_flags = {
            flag: bool(flags_after & mask)
            for flag, mask in flag_masks.items()
            if flag in expected_flags
        }
        if actual_flags != expected_flags:
            raise AssertionError(
                f"{module}:{entry:#x} {name}: "
                f"flags={actual_flags}, expected={expected_flags}"
            )
        if bytes(machine.mem_read(stack_segment * 16 + 0xFF02, 6)) != stack_sentinel:
            raise AssertionError(f"{module}:{entry:#x} {name}: stack changed")

        vectors.append(
            {
                "name": name,
                "module": module,
                "entry": entry,
                "path": "transition" if transition else "continue",
                "countdown_before": countdown,
                "countdown_after": get_u16(data_expected, context + 0x38),
                "state": state,
                "velocity_x_after": signed_word(
                    get_u16(data_expected, context + 0x3A)
                ),
                "field_050_after": signed_word(
                    get_u16(data_expected, state + 0x50)
                ),
                "field_052_after": signed_word(
                    get_u16(data_expected, state + 0x52)
                ),
                "callback_after": get_u16(data_expected, state + 0x0E),
                "active_before": active_before,
                "active_after": get_u16(code_expected, active_offset),
                "data_sha256": hashlib.sha256(data_expected).hexdigest(),
                "defined_flags": expected_flags,
            }
        )

    return vectors


def amer_slot2_steer_update_vectors(entry: int) -> list[dict[str, object]]:
    module = "amer"
    image = load_image(module)
    body_size = 68
    body_hash = "96f1c02162c947f4c90f79d909f4aa38ed61c9a0b29e8ff07677ed5d7ecde293"
    if hashlib.sha256(image[entry : entry + body_size]).hexdigest() != body_hash:
        raise AssertionError(f"{module}:{entry:#x}: recovered body changed")

    data_segment = 0x5000
    extra_segment = 0x7000
    fs_segment = 0x9000
    game_segment = 0xA000
    stack_segment = 0xB000
    context = 0x3000
    return_address = 0xF000
    stack_sentinel = bytes.fromhex("a55a69967887")
    cases = (
        ("positive_score", 0x4000, 0, 0, 0, 1, 0, 0x0100, 1),
        ("negative_score_transition", 0x4400, 0, 0, 0, -1, 0, 0xFFF0, 0),
        ("zero_score", 0x4800, 0, 0, 0, 0, 0, 0x0010, 2),
        (
            "sum_overflow_to_positive",
            0x4C00,
            0,
            1,
            1,
            0x80000000,
            0x80000001,
            0,
            1,
        ),
        (
            "countdown_sign_wrap",
            0x5000,
            0,
            0,
            0,
            -1,
            0,
            0x7FF0,
            0x8000,
        ),
        (
            "cross_boundary_dword",
            0xFCCC,
            0,
            1,
            0,
            0,
            0xFFFFFFFF,
            0x8000,
            1,
        ),
        ("negative_countdown", 0x5400, 0, 0, 0, 1, 0, 0xFFFF, 0xFFFF),
    )
    flag_masks = {
        "cf": 0x0001,
        "pf": 0x0004,
        "af": 0x0010,
        "zf": 0x0040,
        "sf": 0x0080,
        "if": 0x0200,
        "df": 0x0400,
        "of": 0x0800,
    }
    vectors: list[dict[str, object]] = []

    def put_bytes(memory: bytearray, offset: int, value: bytes) -> None:
        offset &= 0xFFFF
        for index, byte in enumerate(value):
            memory[offset + index] = byte

    def put_u16(memory: bytearray, offset: int, value: int) -> None:
        put_bytes(memory, offset, struct.pack("<H", value & 0xFFFF))

    def put_u32(memory: bytearray, offset: int, value: int) -> None:
        put_bytes(memory, offset, struct.pack("<I", value & 0xFFFFFFFF))

    def get_u16(memory: bytearray, offset: int) -> int:
        offset &= 0xFFFF
        return memory[offset] | (memory[offset + 1] << 8)

    def signed_word(value: int) -> int:
        value &= 0xFFFF
        return value if value < 0x8000 else value - 0x10000

    def sign_extend_word(value: int) -> int:
        return signed_word(value) & 0xFFFFFFFF

    for case_index, case in enumerate(cases):
        (
            name,
            state,
            field_040,
            field_038,
            depth_step,
            field_01a,
            field_032,
            field_050,
            countdown,
        ) = case
        data_before = bytearray(
            (offset * 31 + case_index * 19 + 7) & 0xFF
            for offset in range(0x10004)
        )
        put_u16(data_before, state + 0x0E, 0xBEEF)
        put_u32(data_before, state + 0x1A, field_01a)
        put_u32(data_before, state + 0x32, field_032)
        put_u16(data_before, state + 0x38, field_038)
        put_u16(data_before, state + 0x40, field_040)
        put_u16(data_before, state + 0x50, field_050)
        put_u16(data_before, state + 0x56, countdown)
        put_u16(data_before, 0x22FC, depth_step)
        data_expected = bytearray(data_before)

        first_factor = (
            sign_extend_word(field_040) - depth_step - 0x03E8
        ) & 0xFFFFFFFF
        first_factor = (-first_factor) & 0xFFFFFFFF
        product_a = (first_factor * (field_01a & 0xFFFFFFFF)) & 0xFFFFFFFF
        product_b = (
            sign_extend_word(field_038) * (field_032 & 0xFFFFFFFF)
        ) & 0xFFFFFFFF
        score = (product_a + product_b) & 0xFFFFFFFF
        delta = 0x0020 if score & 0x80000000 else 0xFFE0
        field_after = (field_050 + delta) & 0xFFFF
        decremented = (countdown - 1) & 0xFFFF
        transition = bool(decremented & 0x8000)
        put_u16(data_expected, state + 0x50, field_after)
        put_u16(data_expected, state + 0x56, decremented)
        if transition:
            put_u16(data_expected, state + 0x0E, 0x1AA0)
            put_u16(data_expected, state + 0x56, 0x0040)

        initial_flags = 0x0293 | (0x0400 if case_index & 1 else 0)
        initial = {
            "eax": 0xA1A12345,
            "ebx": 0xB2B23456,
            "ecx": 0xC3C34567,
            "edx": 0xD4D45678,
            "esi": 0xE5E50000 | state,
            "edi": 0xF6F60000 | context,
            "ebp": 0x9797789A,
            "sp": 0xFF00,
            "ds": data_segment,
            "es": extra_segment,
            "fs": fs_segment,
            "gs": game_segment,
            "ss": stack_segment,
            "flags": initial_flags,
        }
        code_before = bytearray(image)
        code_before[return_address] = 0xCC
        extra_before = bytes(
            (offset * 13 + case_index + 3) & 0xFF for offset in range(0x10000)
        )
        fs_before = bytes(
            (offset * 17 + case_index + 5) & 0xFF for offset in range(0x10000)
        )
        game_before = bytes(
            (offset * 23 + case_index + 9) & 0xFF for offset in range(0x10000)
        )
        machine = execute(
            bytes(code_before),
            entry,
            return_address,
            initial,
            [
                (data_segment, 0, bytes(data_before)),
                (extra_segment, 0, extra_before),
                (fs_segment, 0, fs_before),
                (game_segment, 0, game_before),
                (
                    stack_segment,
                    0xFF00,
                    struct.pack("<H", return_address) + stack_sentinel,
                ),
            ],
        )
        actual_data = bytes(machine.mem_read(data_segment * 16, 0x10004))
        if actual_data != bytes(data_expected):
            differences = [
                (offset, actual_data[offset], data_expected[offset])
                for offset in range(0x10004)
                if actual_data[offset] != data_expected[offset]
            ][:8]
            raise AssertionError(
                f"{module}:{entry:#x} {name}: data differs at {differences}"
            )
        if bytes(machine.mem_read(0, len(image))) != bytes(code_before):
            raise AssertionError(f"{module}:{entry:#x} {name}: code changed")
        for segment, expected in (
            (extra_segment, extra_before),
            (fs_segment, fs_before),
            (game_segment, game_before),
        ):
            if bytes(machine.mem_read(segment * 16, 0x10000)) != expected:
                raise AssertionError(
                    f"{module}:{entry:#x} {name}: segment {segment:#x} changed"
                )

        expected_registers = dict(initial)
        del expected_registers["flags"]
        expected_registers["eax"] = (score & 0xFFFF0000) | delta
        expected_registers["ebx"] = product_b
        expected_registers["edx"] = depth_step
        expected_registers["sp"] = 0xFF02
        for register, expected in expected_registers.items():
            actual = machine.reg_read(REGISTERS[register])
            if actual != expected:
                raise AssertionError(
                    f"{module}:{entry:#x} {name}: "
                    f"{register}={actual:#x}, expected={expected:#x}"
                )

        expected_flags = {
            "cf": field_050 + delta > 0xFFFF,
            "pf": (decremented & 0xFF).bit_count() % 2 == 0,
            "af": (countdown & 0x0F) == 0,
            "zf": decremented == 0,
            "sf": bool(decremented & 0x8000),
            "if": bool(initial_flags & 0x0200),
            "df": bool(initial_flags & 0x0400),
            "of": countdown == 0x8000,
        }
        flags_after = machine.reg_read(UC_X86_REG_EFLAGS)
        actual_flags = {
            flag: bool(flags_after & mask) for flag, mask in flag_masks.items()
        }
        if actual_flags != expected_flags:
            raise AssertionError(
                f"{module}:{entry:#x} {name}: "
                f"flags={actual_flags}, expected={expected_flags}"
            )
        if bytes(machine.mem_read(stack_segment * 16 + 0xFF02, 6)) != stack_sentinel:
            raise AssertionError(f"{module}:{entry:#x} {name}: stack changed")

        vectors.append(
            {
                "name": name,
                "module": module,
                "entry": entry,
                "state": state,
                "score": score,
                "score_sign": "negative" if score & 0x80000000 else "nonnegative",
                "turn_delta": signed_word(delta),
                "field_050_after": get_u16(data_expected, state + 0x50),
                "countdown_before": countdown,
                "countdown_decremented": decremented,
                "countdown_after": get_u16(data_expected, state + 0x56),
                "callback_after": get_u16(data_expected, state + 0x0E),
                "data_sha256": hashlib.sha256(data_expected).hexdigest(),
                "defined_flags": expected_flags,
            }
        )

    return vectors


def alien_api_entry_vectors(
    module: str,
    body_hash: str,
    data_delta_slot: int,
    data_segment_slot: int,
    continuation_offset: int,
    continuation_target: int,
) -> list[dict[str, object]]:
    entry = 0x0000
    image = load_image(module)
    if hashlib.sha256(image[:149]).hexdigest() != body_hash:
        raise AssertionError(f"{module}:{entry:#x}: recovered 149-byte body changed")

    cases = (
        {
            "name": "zero_scale",
            "timing_scale": 0x0000,
            "code_segment": 0x1000,
            "data_segment": 0x7000,
            "segment_deltas": (0x1000, 0x1000, 0x1000),
            "main_delta": None,
        },
        {
            "name": "ordinary_scale",
            "timing_scale": 0x0001,
            "code_segment": 0x1000,
            "data_segment": 0x7000,
            "segment_deltas": (0x0100, 0x0200, 0x0300),
            "main_delta": None,
        },
        {
            "name": "largest_unclamped_scale",
            "timing_scale": 0x000F,
            "code_segment": 0x1000,
            "data_segment": 0x7000,
            "segment_deltas": (0x1000, 0x1000, 0x1000),
            "main_delta": None,
        },
        {
            "name": "clamped_scale_and_code_wrap",
            "timing_scale": 0x0010,
            "code_segment": 0xF000,
            "data_segment": 0x7000,
            "segment_deltas": (0x0100, 0x0200, 0x0300),
            "main_delta": None,
        },
        {
            "name": "shift_sign_rejection",
            "timing_scale": 0x1000,
            "code_segment": 0x1000,
            "data_segment": 0x7000,
            "segment_deltas": (0x1000, 0x1000, 0x1000),
            "main_delta": None,
        },
        {
            "name": "shift_wrap_and_segment_wrap",
            "timing_scale": 0x2000,
            "code_segment": 0x1000,
            "data_segment": 0xE000,
            "segment_deltas": (0x3000, 0x3000, 0x3000),
            "main_delta": None,
        },
        {
            "name": "high_scale_and_zero_final_segment",
            "timing_scale": 0xFFFF,
            "code_segment": 0x1000,
            "data_segment": 0xD000,
            "segment_deltas": (0x1000, 0x1000, 0x1000),
            "main_delta": None,
        },
        {
            "name": "main_updates_delta_before_readback",
            "timing_scale": 0x0007,
            "code_segment": 0x1000,
            "data_segment": 0x7000,
            "segment_deltas": (0x1000, 0x1000, 0x1000),
            "main_delta": 0x1234,
        },
    )
    caller_data_segment = 0x5200
    initial_extra_segment = 0x5300
    initial_fs_segment = 0x5400
    initial_game_segment = 0x5500
    stack_segment = 0x6000
    timing_segment = 0xB000
    timing_offset = 0x2000
    request_offset = 0x8000
    callback = (0x3456, 0x789A)
    return_segment = 0x5000
    return_address = 0xF000
    vectors: list[dict[str, object]] = []

    for case_index, case in enumerate(cases):
        code_segment = int(case["code_segment"])
        data_segment = int(case["data_segment"])
        segment_deltas = tuple(int(value) for value in case["segment_deltas"])
        data_delta = (data_segment - code_segment) & 0xFFFF
        segments = []
        segment = data_segment
        for delta in segment_deltas:
            segment = (segment + delta) & 0xFFFF
            segments.append(segment)
        final_segment = segments[-1]

        patched_image = bytearray(image)
        struct.pack_into("<H", patched_image, data_delta_slot, data_delta)
        struct.pack_into("<H", patched_image, data_segment_slot, 0xA55A)
        struct.pack_into("<HH", patched_image, 0x0099, 0x5AA5, 0x9669)
        patched_image[0x00A3] = 0xCB

        directory_before = bytearray(
            (offset * 29 + case_index * 17 + 3) & 0xFF
            for offset in range(0x40)
        )
        struct.pack_into("<HHH", directory_before, 0x000C, *segment_deltas)
        directory_expected = bytearray(directory_before)
        struct.pack_into("<HHH", directory_expected, 0x0002, *segments)
        struct.pack_into("<HH", directory_expected, 0x0020, *callback)

        continuation_before = bytes(
            (index * 37 + case_index * 11 + 5) & 0xFF for index in range(8)
        )
        continuation_expected = bytearray(continuation_before)
        struct.pack_into("<H", continuation_expected, 2, continuation_target)

        timing_before = bytearray(
            (index * 43 + case_index * 13 + 7) & 0xFF for index in range(6)
        )
        struct.pack_into("<H", timing_before, 2, int(case["timing_scale"]))
        timing_expected = bytearray(timing_before)

        scaled = (int(case["timing_scale"]) << 3) & 0xFFFF
        if scaled & 0x8000:
            scaled = 0
        if scaled >= 0x0080:
            scaled = 0x007F
        entry_delta = (scaled - 4) & 0xFFFF
        final_delta_value = case["main_delta"]
        if final_delta_value is None:
            final_delta = entry_delta
        else:
            final_delta = int(final_delta_value) & 0xFFFF
        readback_source = (final_delta + 4) & 0xFFFF
        timing_result = readback_source >> 3
        struct.pack_into("<H", timing_expected, 2, timing_result)

        code_expected = bytearray(patched_image)
        struct.pack_into("<H", code_expected, data_segment_slot, data_segment)
        struct.pack_into("<HH", code_expected, 0x0099, final_delta, 0)

        request = struct.pack(
            "<HHHH", timing_offset, timing_segment, callback[0], callback[1]
        )
        stack_sentinel = bytes.fromhex("5aa596698778")
        stack_frame = (
            struct.pack("<HH", return_address, return_segment) + stack_sentinel
        )
        host_decoy = bytes(
            (index * 31 + case_index * 19 + 9) & 0xFF
            for index in range(0x40)
        )
        initial_flags = 0x0A93 | (0x0400 if case_index & 1 else 0)
        initial = {
            "eax": 0xA1A11234 + case_index,
            "ebx": 0xB2B22345 + case_index,
            "ecx": 0xC3C33456 + case_index,
            "edx": 0xD4D44567 + case_index,
            "esi": 0xE5E55678 + case_index,
            "edi": 0xF6F66789 + case_index,
            "ebp": 0x97978000,
            "sp": 0xFF00,
            "ds": caller_data_segment,
            "es": initial_extra_segment,
            "fs": initial_fs_segment,
            "gs": initial_game_segment,
            "ss": stack_segment,
            "flags": initial_flags,
        }
        main_entries: list[dict[str, object]] = []

        def code_handler(
            machine: Uc, address: int, _size: int, _data: object
        ) -> None:
            if address != code_segment * 16 + 0x00A3:
                return
            sp = machine.reg_read(UC_X86_REG_SP)
            main_entries.append(
                {
                    "return_frame": struct.unpack(
                        "<HH", machine.mem_read(stack_segment * 16 + sp, 4)
                    ),
                    "ds": machine.reg_read(UC_X86_REG_DS),
                    "es": machine.reg_read(UC_X86_REG_ES),
                    "fs": machine.reg_read(UC_X86_REG_FS),
                    "data_segment": struct.unpack(
                        "<H",
                        machine.mem_read(code_segment * 16 + data_segment_slot, 2),
                    )[0],
                    "segments": struct.unpack(
                        "<HHH", machine.mem_read(data_segment * 16 + 2, 6)
                    ),
                    "method_delta": struct.unpack(
                        "<HH", machine.mem_read(code_segment * 16 + 0x0099, 4)
                    ),
                    "callback": struct.unpack(
                        "<HH", machine.mem_read(data_segment * 16 + 0x0020, 4)
                    ),
                    "timing_scale": struct.unpack(
                        "<H",
                        machine.mem_read(timing_segment * 16 + timing_offset, 2),
                    )[0],
                }
            )
            if case["main_delta"] is not None:
                machine.mem_write(
                    code_segment * 16 + 0x0099,
                    struct.pack("<H", final_delta),
                )

        machine = execute(
            bytes(patched_image),
            entry,
            return_address,
            initial,
            [
                (data_segment, 0, bytes(directory_before)),
                (
                    final_segment,
                    continuation_offset - 2,
                    continuation_before,
                ),
                (timing_segment, timing_offset - 2, bytes(timing_before)),
                (caller_data_segment, 0, host_decoy),
                (stack_segment, request_offset, request),
                (stack_segment, 0xFF00, stack_frame),
            ],
            code_handler=code_handler,
            code_segment=code_segment,
            return_segment=return_segment,
            max_instructions=200,
        )

        expected_main_entry = {
            "return_frame": (0x0070, code_segment),
            "ds": data_segment,
            "es": timing_segment,
            "fs": data_segment,
            "data_segment": data_segment,
            "segments": tuple(segments),
            "method_delta": (entry_delta, 0),
            "callback": callback,
            "timing_scale": int(case["timing_scale"]),
        }
        if main_entries != [expected_main_entry]:
            raise AssertionError(
                f"{module}:{entry:#x} {case['name']}: "
                f"main entries={main_entries}, expected={[expected_main_entry]}"
            )

        expected_registers = dict(initial)
        del expected_registers["flags"]
        expected_registers["sp"] = 0xFF04
        for register, expected in expected_registers.items():
            actual = machine.reg_read(REGISTERS[register])
            if actual != expected:
                raise AssertionError(
                    f"{module}:{entry:#x} {case['name']}: "
                    f"{register}={actual:#x}, expected={expected:#x}"
                )

        expected_flags = {
            "cf": bool(readback_source & 0x0004),
            "pf": (timing_result & 0xFF).bit_count() % 2 == 0,
            "zf": timing_result == 0,
            "sf": bool(timing_result & 0x8000),
            "if": bool(initial_flags & 0x0200),
            "df": bool(initial_flags & 0x0400),
        }
        flags_after = machine.reg_read(UC_X86_REG_EFLAGS)
        flag_masks = {
            "cf": 0x0001,
            "pf": 0x0004,
            "zf": 0x0040,
            "sf": 0x0080,
            "if": 0x0200,
            "df": 0x0400,
        }
        actual_flags = {
            name: bool(flags_after & flag_masks[name]) for name in expected_flags
        }
        if actual_flags != expected_flags:
            raise AssertionError(
                f"{module}:{entry:#x} {case['name']}: "
                f"flags={actual_flags}, expected={expected_flags}"
            )

        actual_code = bytes(
            machine.mem_read(code_segment * 16, len(code_expected))
        )
        if actual_code != bytes(code_expected):
            differing = next(
                index
                for index, (actual, expected) in enumerate(
                    zip(actual_code, code_expected, strict=True)
                )
                if actual != expected
            )
            raise AssertionError(
                f"{module}:{entry:#x} {case['name']}: "
                f"code mismatch at {differing:#x}"
            )
        if bytes(machine.mem_read(data_segment * 16, 0x40)) != bytes(
            directory_expected
        ):
            raise AssertionError(
                f"{module}:{entry:#x} {case['name']}: directory differs"
            )
        if bytes(
            machine.mem_read(final_segment * 16 + continuation_offset - 2, 8)
        ) != bytes(continuation_expected):
            raise AssertionError(
                f"{module}:{entry:#x} {case['name']}: continuation differs"
            )
        if bytes(
            machine.mem_read(timing_segment * 16 + timing_offset - 2, 6)
        ) != bytes(timing_expected):
            raise AssertionError(
                f"{module}:{entry:#x} {case['name']}: timing word differs"
            )
        if bytes(machine.mem_read(caller_data_segment * 16, 0x40)) != host_decoy:
            raise AssertionError(
                f"{module}:{entry:#x} {case['name']}: caller DS changed"
            )
        if bytes(
            machine.mem_read(stack_segment * 16 + request_offset, len(request))
        ) != request:
            raise AssertionError(
                f"{module}:{entry:#x} {case['name']}: request changed"
            )
        if bytes(machine.mem_read(stack_segment * 16 + 0xFF04, 6)) != stack_sentinel:
            raise AssertionError(
                f"{module}:{entry:#x} {case['name']}: stack sentinel changed"
            )

        vectors.append(
            {
                "name": case["name"],
                "module": module,
                "entry": entry,
                "code_segment": code_segment,
                "data_segment": data_segment,
                "segment_deltas": segment_deltas,
                "derived_segments": segments,
                "timing_scale_in": case["timing_scale"],
                "entry_method_delta": entry_delta,
                "main_method_delta": case["main_delta"],
                "timing_scale_out": timing_result,
                "frame_callback": callback,
                "render_continuation": {
                    "segment": final_segment,
                    "offset": continuation_offset,
                    "target": continuation_target,
                },
                "ordered_callees": [0x00A3],
                "preserves_all_general_and_segment_registers": True,
                "defined_flags": expected_flags,
            }
        )

    return vectors


def alien_main_vectors(
    module: str,
    entry: int,
    body_size: int,
    body_hash: str,
    data_segment_slot: int,
    direct_calls: dict[str, int],
    clears_control_latch: bool,
) -> list[dict[str, object]]:
    image = load_image(module)
    if hashlib.sha256(image[entry : entry + body_size]).hexdigest() != body_hash:
        raise AssertionError(f"{module}:{entry:#x}: recovered body changed")

    data_segment = 0x6000
    host_data_segment = 0x4200
    extra_segment = 0x7000
    initial_fs_segment = 0x8000
    game_segment = 0xB000
    stack_segment = 0xD000
    return_address = 0xF400
    method_stub = 0xF100
    callback_stub = 0xF200
    context_offsets = (0x3000, 0x3100)
    mask32 = 0xFFFFFFFF
    cases = (
        {
            "name": "exit_before_timer",
            "clock": 0x12345678,
            "countdown": 0x4321,
            "control": 0x6A6A,
            "keys": (),
            "exit_after_frame": 1,
            "controls": {},
            "frames": 1,
        },
        {
            "name": "positive_countdown_escape",
            "clock": 0x10203040,
            "countdown": 3,
            "control": 0x1111,
            "keys": (0x011B,),
            "exit_after_frame": None,
            "controls": {1: 0},
            "frames": 1,
        },
        {
            "name": "negative_adjusted_no_callback",
            "clock": 0x55667788,
            "countdown": 0,
            "control": 0x2222,
            "keys": (0x011B,),
            "exit_after_frame": None,
            "controls": {1: 0},
            "frames": 1,
        },
        {
            "name": "negative_active_callback",
            "clock": 0x89ABCDEF,
            "countdown": 0,
            "control": 0,
            "keys": (0x011B,),
            "exit_after_frame": None,
            "controls": {1: 0x8001},
            "frames": 1,
        },
        {
            "name": "countdown_and_clock_wrap",
            "clock": 0xFFFFFFFC,
            "countdown": 0x8000,
            "control": 0,
            "keys": (0x011B,),
            "exit_after_frame": None,
            "controls": {1: 0},
            "frames": 1,
        },
        {
            "name": "ordinary_key_drain",
            "clock": 0x31415926,
            "countdown": 0,
            "control": 0,
            "keys": (0x1E61,),
            "exit_after_frame": 2,
            "controls": {1: 0},
            "frames": 2,
        },
        {
            "name": "pause_until_matching_key",
            "clock": 0x27182818,
            "countdown": 0,
            "control": 0,
            "keys": (0x1970, 0x2D78, 0x1950),
            "exit_after_frame": 2,
            "controls": {1: 0},
            "frames": 2,
        },
        {
            "name": "callback_then_throttle",
            "clock": 0x01020304,
            "countdown": 0,
            "control": 0,
            "keys": (),
            "exit_after_frame": 3,
            "controls": {1: 1, 2: 0},
            "frames": 3,
        },
    )

    def put_u16(memory: bytearray, offset: int, value: int) -> None:
        struct.pack_into("<H", memory, offset, value & 0xFFFF)

    def get_u16(memory: bytes | bytearray, offset: int) -> int:
        return struct.unpack_from("<H", memory, offset)[0]

    def put_u32(memory: bytearray, offset: int, value: int) -> None:
        struct.pack_into("<I", memory, offset, value & mask32)

    def get_u32(memory: bytes | bytearray, offset: int) -> int:
        return struct.unpack_from("<I", memory, offset)[0]

    def signed_word(value: int) -> int:
        value &= 0xFFFF
        return value if value < 0x8000 else value - 0x10000

    def output_hash(outputs: list[tuple[int, int, int]]) -> str:
        packed = b"".join(
            struct.pack("<HBH", port, size, value)
            for port, size, value in outputs
        )
        return hashlib.sha256(packed).hexdigest()

    vectors: list[dict[str, object]] = []
    for case_index, case in enumerate(cases):
        patched_image = bytearray(image)
        put_u16(patched_image, data_segment_slot, data_segment)
        put_u16(patched_image, 0x0095, 0xBEEF)
        for callee in direct_calls.values():
            patched_image[callee] = 0xC3
        patched_image.extend(bytes(max(0, method_stub + 1 - len(patched_image))))
        patched_image[method_stub] = 0xC3
        patched_image.extend(bytes(max(0, callback_stub + 1 - len(patched_image))))
        patched_image[callback_stub] = 0xCB

        data_before = bytearray(
            (offset * 29 + case_index * 17 + 3) & 0xFF
            for offset in range(0x10000)
        )
        host_before = bytes(
            (offset * 31 + case_index * 7 + 5) & 0xFF
            for offset in range(0x10000)
        )
        extra_before = bytes(
            (offset * 19 + case_index * 11 + 7) & 0xFF
            for offset in range(0x10000)
        )
        initial_fs_before = bytes(
            (offset * 13 + case_index * 23 + 9) & 0xFF
            for offset in range(0x10000)
        )
        game_before = bytes(
            (offset * 37 + case_index * 5 + 11) & 0xFF
            for offset in range(0x10000)
        )
        video_before = bytearray(
            (offset * 7 + case_index * 31 + 13) & 0xFF
            for offset in range(0x10000)
        )
        palette = bytes(
            (index * 43 + case_index * 47 + 17) & 0xFF
            for index in range(0x300)
        )

        put_u32(data_before, 0x0016, case["clock"])
        put_u32(data_before, 0x001A, 0xDEADBEEF)
        put_u16(data_before, 0x001E, case["countdown"])
        struct.pack_into("<HH", data_before, 0x0020, callback_stub, 0)
        put_u16(data_before, 0x0026, 0x4000)
        put_u16(data_before, 0x0028, 0xA400)
        put_u16(data_before, 0x103A, method_stub)
        put_u16(data_before, 0x103C, method_stub)
        data_before[0x1F6A : 0x226A] = palette
        put_u16(data_before, 0x226E, 0xA55A)
        put_u16(data_before, 0x2278, 0x2468)
        put_u16(data_before, 0x2282, case["control"])
        put_u16(data_before, 0x22A8, 0x1357)
        put_u16(data_before, 0x2308, context_offsets[0])
        put_u16(data_before, 0x230A, context_offsets[1])
        put_u16(data_before, 0x230C, 0)
        put_u16(data_before, context_offsets[0] + 0x34, 0)
        put_u16(data_before, context_offsets[1] + 0x34, 2)

        data_expected = bytearray(data_before)
        code_expected = bytearray(patched_image)
        video_expected = bytearray(video_before)
        expected_keyboard_queue = list(case["keys"])
        expected_keyboard: list[dict[str, int | str | None]] = []
        expected_callbacks: list[dict[str, int]] = []
        expected_calls = ["vga_clear", "mouse_bounds", "mouse_position"]
        page = 0x4000
        framebuffer_segment = 0xA400
        last_cleared_segment = framebuffer_segment
        clock = case["clock"]
        countdown = case["countdown"]
        last_callback = (clock - 620) & mask32
        control = case["control"]
        key_event = 0xBEEF

        put_u16(data_expected, 0x226E, 0)
        put_u16(data_expected, 0x22A8, 0)
        put_u16(data_expected, 0x22EC, 0x075D)
        put_u16(data_expected, 0x22F0, 0xFF11)
        put_u16(data_expected, 0x22F4, 0xD9C2)
        put_u16(data_expected, 0x22F6, 0)
        put_u16(data_expected, 0x22F8, 0x0678)
        put_u16(data_expected, 0x22FA, 0)
        put_u16(data_expected, 0x22FC, 0)
        put_u32(data_expected, 0x001A, last_callback)

        for frame in range(1, case["frames"] + 1):
            expected_calls.extend(
                (
                    "mouse_camera",
                    "camera_matrix",
                    "primary_mesh",
                    "starfield",
                    f"method:{context_offsets[0]:04x}",
                    "transform",
                    f"method:{context_offsets[1]:04x}",
                    "transform",
                    "bucket_faces",
                )
            )
            last_cleared_segment = framebuffer_segment
            video_start = last_cleared_segment * 16 - 0xA0000
            if video_start < 0 or video_start + 0x3E80 > len(video_expected):
                raise AssertionError(
                    f"{module}:{entry:#x} {case['name']}: "
                    f"framebuffer {frame} escaped VGA aperture"
                )
            video_expected[video_start : video_start + 0x3E80] = bytes(0x3E80)
            put_u16(data_expected, 0x2278, context_offsets[-1])

            if clears_control_latch:
                control = 0
            if frame in case["controls"]:
                control = case["controls"][frame]
            put_u16(data_expected, 0x2282, control)
            if case["exit_after_frame"] == frame:
                put_u16(data_expected, 0x226E, 1)

            new_page = (page + 0x4000) & 0xFFFF
            framebuffer_segment = (
                (new_page & 0x00FF)
                | ((((new_page >> 8) >> 4) | 0xA0) << 8)
            )
            page = new_page
            put_u16(data_expected, 0x0026, page)
            put_u16(data_expected, 0x0028, framebuffer_segment)

            if case["exit_after_frame"] == frame:
                continue

            clock = (clock + 8) & mask32
            event = (countdown - 1) & 0xFFFF
            countdown = 0
            put_u32(data_expected, 0x0016, clock)
            put_u16(data_expected, 0x001E, 0)
            if signed_word(event) >= 0:
                expected_callbacks.append({"event": event, "clock": clock})
                last_callback = clock
                put_u32(data_expected, 0x001A, last_callback)
            else:
                elapsed = (clock - last_callback) & mask32
                if elapsed >= 600:
                    last_callback = (clock - 1000) & mask32
                    if control != 0:
                        expected_callbacks.append({"event": 2, "clock": clock})
                        last_callback = clock
                    put_u32(data_expected, 0x001A, last_callback)

            while True:
                ready_key = (
                    expected_keyboard_queue[0]
                    if expected_keyboard_queue
                    else None
                )
                expected_keyboard.append(
                    {"operation": "ready", "key": ready_key}
                )
                if ready_key is None:
                    break
                key = expected_keyboard_queue.pop(0)
                expected_keyboard.append({"operation": "read", "key": key})
                key_event = key
                character = key & 0xFF
                if character in (ord("p"), ord("P")):
                    while True:
                        if not expected_keyboard_queue:
                            raise AssertionError(
                                f"{module}:{entry:#x} {case['name']}: "
                                "pause fixture has no matching key"
                            )
                        key = expected_keyboard_queue.pop(0)
                        expected_keyboard.append(
                            {"operation": "read", "key": key}
                        )
                        if key & 0xFF in (ord("p"), ord("P")):
                            break
                    break
                if character == 0x1B:
                    break

        put_u16(code_expected, 0x0095, key_event)
        expected_calls.extend(("vga_clear", "mouse_bounds"))
        expected_outputs: list[tuple[int, int, int]] = [(0x03C8, 1, 0)]
        expected_outputs.extend((0x03C9, 1, value) for value in palette)
        expected_page = 0x4000
        for _frame in range(case["frames"]):
            expected_outputs.append((0x03C4, 2, 0x0F02))
            expected_outputs.append(
                (0x03D4, 2, (expected_page & 0xFF00) | 0x000C)
            )
            expected_page = (expected_page + 0x4000) & 0xFFFF

        calls: list[str] = []
        callbacks: list[dict[str, int]] = []
        keyboard: list[dict[str, int | str | None]] = []
        keyboard_queue = list(case["keys"])
        outputs: list[tuple[int, int, int]] = []
        recent_addresses: list[int] = []
        frame_counter = 0
        controls = case["controls"]
        exit_after_frame = case["exit_after_frame"]
        direct_names = {address: name for name, address in direct_calls.items()}

        def code_handler(
            machine: Uc, address: int, _size: int, _data: object
        ) -> None:
            nonlocal frame_counter
            recent_addresses.append(address)
            del recent_addresses[:-12]
            if address in direct_names:
                name = direct_names[address]
                calls.append(name)
                if name == "bucket_faces":
                    frame_counter += 1
                    if frame_counter in controls:
                        machine.mem_write(
                            data_segment * 16 + 0x2282,
                            struct.pack("<H", controls[frame_counter]),
                        )
                    if exit_after_frame == frame_counter:
                        machine.mem_write(
                            data_segment * 16 + 0x226E,
                            struct.pack("<H", 1),
                        )
            elif address == method_stub:
                context = machine.reg_read(UC_X86_REG_EDI) & 0xFFFF
                calls.append(f"method:{context:04x}")
            elif address == callback_stub:
                callbacks.append(
                    {
                        "event": machine.reg_read(UC_X86_REG_EAX) & 0xFFFF,
                        "clock": machine.reg_read(UC_X86_REG_EDX) & mask32,
                    }
                )

        def interrupt_handler(
            machine: Uc, interrupt_number: int, _data: object
        ) -> None:
            if interrupt_number != 0x16:
                raise AssertionError(
                    f"{module}:{entry:#x} {case['name']}: "
                    f"unexpected interrupt {interrupt_number:#x}"
                )
            eax = machine.reg_read(UC_X86_REG_EAX)
            operation = (eax >> 8) & 0xFF
            flags = machine.reg_read(UC_X86_REG_EFLAGS)
            if operation == 1:
                key = keyboard_queue[0] if keyboard_queue else None
                keyboard.append({"operation": "ready", "key": key})
                if key is None:
                    machine.reg_write(UC_X86_REG_EFLAGS, flags | 0x0040)
                else:
                    machine.reg_write(
                        UC_X86_REG_EAX, (eax & 0xFFFF0000) | key
                    )
                    machine.reg_write(UC_X86_REG_EFLAGS, flags & ~0x0040)
            elif operation == 0:
                if not keyboard_queue:
                    raise AssertionError(
                        f"{module}:{entry:#x} {case['name']}: "
                        "blocking keyboard read exhausted fixture"
                    )
                key = keyboard_queue.pop(0)
                keyboard.append({"operation": "read", "key": key})
                machine.reg_write(UC_X86_REG_EAX, (eax & 0xFFFF0000) | key)
            else:
                raise AssertionError(
                    f"{module}:{entry:#x} {case['name']}: "
                    f"unexpected INT 16h operation {operation:#x}"
                )

        def output_handler(
            _machine: Uc, port: int, size: int, value: int
        ) -> None:
            outputs.append((port, size, value))

        initial = {
            "eax": 0xA1A11234 + case_index,
            "ebx": 0xB2B22345 + case_index,
            "ecx": 0xC3C33456 + case_index,
            "edx": 0xD4D44567 + case_index,
            "esi": 0xE5E55678 + case_index,
            "edi": 0xF6F66789 + case_index,
            "ebp": 0x9797789A + case_index,
            "sp": 0xFF00,
            "ds": host_data_segment,
            "es": extra_segment,
            "fs": initial_fs_segment,
            "gs": game_segment,
            "ss": stack_segment,
            "flags": 0x0202,
        }
        stack_sentinel = bytes.fromhex("5aa569968778")
        stack_before = struct.pack("<HH", return_address, 0) + stack_sentinel
        try:
            machine = execute(
                bytes(patched_image),
                entry,
                return_address,
                initial,
                [
                    (data_segment, 0, bytes(data_before)),
                    (host_data_segment, 0, host_before),
                    (extra_segment, 0, extra_before),
                    (initial_fs_segment, 0, initial_fs_before),
                    (game_segment, 0, game_before),
                    (0xA000, 0, bytes(video_before)),
                    (stack_segment, 0xFF00, stack_before),
                    (0, method_stub, b"\xc3"),
                    (0, callback_stub, b"\xcb"),
                ],
                interrupt_handler=interrupt_handler,
                output_handler=output_handler,
                code_handler=code_handler,
                max_instructions=30000,
                return_segment=0,
            )
        except RuntimeError as error:
            raise AssertionError(
                f"{module}:{entry:#x} {case['name']}: {error}; "
                f"recent={[hex(address) for address in recent_addresses]}"
            ) from error

        if calls != expected_calls:
            raise AssertionError(
                f"{module}:{entry:#x} {case['name']}: "
                f"calls={calls}, expected={expected_calls}"
            )
        if callbacks != expected_callbacks:
            raise AssertionError(
                f"{module}:{entry:#x} {case['name']}: "
                f"callbacks={callbacks}, expected={expected_callbacks}"
            )
        if keyboard != expected_keyboard:
            raise AssertionError(
                f"{module}:{entry:#x} {case['name']}: "
                f"keyboard={keyboard}, expected={expected_keyboard}"
            )
        if keyboard_queue:
            raise AssertionError(
                f"{module}:{entry:#x} {case['name']}: "
                f"unconsumed keys={keyboard_queue}"
            )
        if outputs != expected_outputs:
            raise AssertionError(
                f"{module}:{entry:#x} {case['name']}: "
                f"VGA output mismatch at first differing record"
            )

        actual_code = bytes(machine.mem_read(0, len(code_expected)))
        actual_data = bytes(machine.mem_read(data_segment * 16, 0x10000))
        actual_video = bytes(machine.mem_read(0xA0000, 0x10000))
        if actual_code != bytes(code_expected):
            differing = next(
                index
                for index, (actual, expected) in enumerate(
                    zip(actual_code, code_expected, strict=True)
                )
                if actual != expected
            )
            raise AssertionError(
                f"{module}:{entry:#x} {case['name']}: "
                f"code mismatch at {differing:#x}: "
                f"{actual_code[differing]:#x} != {code_expected[differing]:#x}"
            )
        if actual_data != bytes(data_expected):
            differing = next(
                index
                for index, (actual, expected) in enumerate(
                    zip(actual_data, data_expected, strict=True)
                )
                if actual != expected
            )
            raise AssertionError(
                f"{module}:{entry:#x} {case['name']}: "
                f"data mismatch at {differing:#x}: "
                f"{actual_data[differing]:#x} != {data_expected[differing]:#x}"
            )
        if actual_video != bytes(video_expected):
            raise AssertionError(
                f"{module}:{entry:#x} {case['name']}: VGA memory mismatch"
            )
        for segment, expected in (
            (host_data_segment, host_before),
            (extra_segment, extra_before),
            (initial_fs_segment, initial_fs_before),
            (game_segment, game_before),
        ):
            if bytes(machine.mem_read(segment * 16, 0x10000)) != expected:
                raise AssertionError(
                    f"{module}:{entry:#x} {case['name']}: "
                    f"segment {segment:#x} changed"
                )

        expected_registers = {
            "ds": host_data_segment,
            "es": last_cleared_segment,
            "fs": data_segment,
            "gs": game_segment,
            "ss": stack_segment,
            "sp": 0xFF04,
        }
        for register, expected in expected_registers.items():
            actual = machine.reg_read(REGISTERS[register])
            if actual != expected:
                raise AssertionError(
                    f"{module}:{entry:#x} {case['name']}: "
                    f"{register}={actual:#x}, expected={expected:#x}"
                )
        if bytes(machine.mem_read(stack_segment * 16 + 0xFF04, 6)) != stack_sentinel:
            raise AssertionError(
                f"{module}:{entry:#x} {case['name']}: stack sentinel changed"
            )

        vectors.append(
            {
                "name": case["name"],
                "module": module,
                "entry": entry,
                "body_size": body_size,
                "body_sha256": body_hash,
                "frame_count": case["frames"],
                "clears_control_latch": clears_control_latch,
                "calls": calls,
                "callbacks": callbacks,
                "keyboard": keyboard,
                "output_count": len(outputs),
                "output_sha256": output_hash(outputs),
                "data_sha256": hashlib.sha256(actual_data).hexdigest(),
                "video_sha256": hashlib.sha256(actual_video).hexdigest(),
                "clock_after": get_u32(actual_data, 0x0016),
                "last_callback_after": get_u32(actual_data, 0x001A),
                "countdown_after": get_u16(actual_data, 0x001E),
                "page_after": get_u16(actual_data, 0x0026),
                "framebuffer_segment_after": get_u16(actual_data, 0x0028),
                "control_after": get_u16(actual_data, 0x2282),
                "exit_after": get_u16(actual_data, 0x226E),
                "key_event_after": get_u16(actual_code, 0x0095),
                "segments_after": {
                    name: machine.reg_read(REGISTERS[name])
                    for name in ("ds", "es", "fs", "gs", "ss", "sp")
                },
            }
        )

    return vectors


def alien_starfield_vectors(
    module: str,
    entry: int,
    body_hash: str,
    data_segment_slot: int,
    shade_table_offset: int,
    seed_offset: int,
    remaining_offset: int,
    cursors_offset: int,
    matrix_offset: int,
    camera_cells_offset: int,
    records_offset: int,
) -> list[dict[str, object]]:
    image = load_image(module)
    body_size = 497
    if hashlib.sha256(image[entry : entry + body_size]).hexdigest() != body_hash:
        raise AssertionError(f"{module}:{entry:#x}: recovered body changed")

    data_segment = 0x5000
    raster_segment = 0x7000
    framebuffer_segment = 0x9000
    extra_segment = 0xB000
    game_segment = 0xC000
    stack_segment = 0xD000
    return_address = 0xF000
    mask32 = 0xFFFFFFFF
    stack_sentinel = bytes.fromhex("a55a69967887")
    balanced_matrix = (
        0,
        0x00000100,
        0,
        0,
        0,
        0x00000100,
        0x00010000,
        0,
        0,
    )
    cases = (
        {
            "name": "zero_seed_zero_depth",
            "seed": 0,
            "camera": (0, 0, 0),
            "matrix": balanced_matrix,
        },
        {
            "name": "balanced_projection_all_planes",
            "seed": 0x12345678,
            "camera": (0, 0, 0),
            "matrix": balanced_matrix,
        },
        {
            "name": "logical_camera_cells_and_seed_wrap",
            "seed": 0xDEADBEEF,
            "camera": (0xFEDCBA98, 0x12345678, 0x80002000),
            "matrix": balanced_matrix,
        },
        {
            "name": "left_edge_rejection",
            "seed": 1,
            "camera": (0, 0, 0),
            "matrix": (
                (-161 * 256) & mask32,
                0,
                0,
                0,
                0,
                0,
                0x00010000,
                0,
                0,
            ),
        },
        {
            "name": "right_edge_rejection",
            "seed": 1,
            "camera": (0, 0, 0),
            "matrix": (
                160 * 256,
                0,
                0,
                0,
                0,
                0,
                0x00010000,
                0,
                0,
            ),
        },
        {
            "name": "top_edge_rejection",
            "seed": 1,
            "camera": (0, 0, 0),
            "matrix": (
                0,
                0,
                0,
                101 * 256,
                0,
                0,
                0x00010000,
                0,
                0,
            ),
        },
        {
            "name": "bottom_edge_rejection",
            "seed": 1,
            "camera": (0, 0, 0),
            "matrix": (
                0,
                0,
                0,
                (-100 * 256) & mask32,
                0,
                0,
                0x00010000,
                0,
                0,
            ),
        },
        {
            "name": "modular_product_overflow",
            "seed": 0x13579BDF,
            "camera": (0, 0, 0),
            "matrix": (
                0,
                0,
                0,
                0x70000000,
                0x90000000,
                0x12345678,
                0x70000000,
                0x90000000,
                0x12345678,
            ),
        },
    )

    def put_u16(memory: bytearray, offset: int, value: int) -> None:
        struct.pack_into("<H", memory, offset, value & 0xFFFF)

    def put_u32(memory: bytearray, offset: int, value: int) -> None:
        struct.pack_into("<I", memory, offset, value & mask32)

    def signed_word(value: int) -> int:
        value &= 0xFFFF
        return value if value < 0x8000 else value - 0x10000

    def signed_dword(value: int) -> int:
        value &= mask32
        return value if value < 0x80000000 else value - 0x100000000

    def random_step(value: int) -> int:
        rotated = ((value >> 7) | (value << 25)) & mask32
        return (rotated - (rotated >> 31)) & mask32

    def dot_product(
        matrix: tuple[int, ...],
        row: int,
        coordinates: tuple[int, int, int],
    ) -> int:
        accumulator = 0
        for column, coordinate in enumerate(coordinates):
            accumulator += (
                (matrix[row * 3 + column] & mask32)
                * (coordinate & mask32)
            )
        return accumulator & mask32

    def signed_divide(numerator: int, denominator: int) -> int:
        quotient = abs(numerator) // abs(denominator)
        if (numerator < 0) != (denominator < 0):
            quotient = -quotient
        return quotient

    vectors: list[dict[str, object]] = []
    for case_index, case in enumerate(cases):
        patched_image = bytearray(image)
        put_u16(patched_image, data_segment_slot, data_segment)
        data_before = bytearray(
            (offset * 29 + case_index * 17 + 3) & 0xFF
            for offset in range(0x10000)
        )
        raster_before = bytearray(
            (offset * 7 + case_index * 31 + 5) & 0xFF
            for offset in range(0x10000)
        )
        framebuffer_before = bytearray(
            (offset * 11 + case_index * 13 + 7) & 0xFF
            for offset in range(0x10000)
        )
        extra_before = bytes(
            (offset * 19 + case_index + 11) & 0xFF
            for offset in range(0x10000)
        )
        game_before = bytes(
            (offset * 23 + case_index + 17) & 0xFF
            for offset in range(0x10000)
        )
        matrix = case["matrix"]
        camera = case["camera"]

        put_u16(data_before, 0x0006, raster_segment)
        put_u16(data_before, 0x0028, framebuffer_segment)
        for matrix_index, value in enumerate(matrix):
            put_u32(data_before, 0x22BA + matrix_index * 4, value)
        for coordinate_index, value in enumerate(camera):
            put_u32(data_before, 0x22EA + coordinate_index * 4, value)
        for shade in range(256):
            raster_before[shade_table_offset + shade] = (
                shade * 37 + case_index * 41 + 13
            ) & 0xFF
        put_u32(raster_before, seed_offset, case["seed"])

        data_expected = bytearray(data_before)
        raster_expected = bytearray(raster_before)
        framebuffer_expected = bytearray(framebuffer_before)
        for matrix_index, value in enumerate(matrix):
            put_u32(raster_expected, matrix_offset + matrix_index * 4, value)
        camera_cells = tuple(
            ((value & mask32) >> 13) & 0xFFFF for value in camera
        )
        for coordinate_index, value in enumerate(camera_cells):
            put_u16(
                raster_expected,
                camera_cells_offset + coordinate_index * 4,
                value,
            )

        plane_records: list[list[tuple[int, int]]] = [[], [], [], []]
        visible_stars: list[dict[str, object]] = []
        rejections = {
            "negative_depth": 0,
            "zero_shifted_depth": 0,
            "left": 0,
            "right": 0,
            "top": 0,
            "bottom": 0,
        }
        random = case["seed"]
        for _star_index in range(1200):
            coordinates = []
            for coordinate_index in range(3):
                random = random_step(random)
                coordinates.append(
                    signed_word(
                        camera_cells[coordinate_index] - (random & 0xFFFF)
                    )
                )
            coordinate_tuple = tuple(coordinates)
            depth_accumulator = dot_product(matrix, 2, coordinate_tuple)
            if signed_dword(depth_accumulator) < 0:
                rejections["negative_depth"] += 1
                continue
            depth = signed_dword(depth_accumulator) >> 8
            if depth == 0:
                rejections["zero_shifted_depth"] += 1
                continue

            screen_x = signed_word(
                signed_divide(
                    signed_dword(dot_product(matrix, 0, coordinate_tuple)),
                    depth,
                )
                + 160
            )
            if screen_x < 0:
                rejections["left"] += 1
                continue
            if screen_x >= 320:
                rejections["right"] += 1
                continue
            screen_y = signed_word(
                -signed_divide(
                    signed_dword(dot_product(matrix, 1, coordinate_tuple)),
                    depth,
                )
                + 100
            )
            if screen_y < 0:
                rejections["top"] += 1
                continue
            if screen_y >= 200:
                rejections["bottom"] += 1
                continue

            plane = screen_x & 3
            plane_records[plane].append(
                (
                    (screen_y * 320 + screen_x) >> 2,
                    (depth & mask32) >> 15,
                )
            )
            shade = (depth & mask32) >> 15
            visible_stars.append(
                {
                    "screen": [screen_x, screen_y],
                    "shade": shade,
                    "palette_index": raster_before[
                        shade_table_offset + shade
                    ],
                }
            )

        cursors = []
        for plane, records in enumerate(plane_records):
            record_offset = (
                records_offset + plane * 0x0600
            ) & 0xFFFF
            for framebuffer_offset, shade in records:
                put_u16(raster_expected, record_offset, framebuffer_offset)
                put_u16(raster_expected, record_offset + 2, shade)
                record_offset = (record_offset + 4) & 0xFFFF
            cursors.append(record_offset)
            put_u16(raster_expected, cursors_offset + plane * 2, record_offset)
        put_u16(raster_expected, remaining_offset, 0xFFFF)

        expected_outputs = []
        plane_hashes = []
        for plane, records in enumerate(plane_records):
            if records:
                expected_outputs.append(
                    {
                        "port": 0x03C4,
                        "size": 2,
                        "value": ((plane + 1) << 8) | 0x0002,
                    }
                )
            record_offset = records_offset + plane * 0x0600
            for framebuffer_offset, shade in records:
                framebuffer_expected[framebuffer_offset] = (
                    raster_expected[shade_table_offset + shade]
                )
            plane_bytes = bytes(
                raster_expected[
                    record_offset : record_offset + len(records) * 4
                ]
            )
            plane_hashes.append(hashlib.sha256(plane_bytes).hexdigest())

        initial = {
            "eax": 0xA1A12345 + case_index,
            "ebx": 0xB2B23456 + case_index,
            "ecx": 0xC3C34567 + case_index,
            "edx": 0xD4D45678 + case_index,
            "esi": 0xE5E56789 + case_index,
            "edi": 0xF6F6789A + case_index,
            "ebp": 0x979789AB + case_index,
            "sp": 0xFF00,
            "ds": extra_segment,
            "es": extra_segment,
            "fs": data_segment,
            "gs": game_segment,
            "ss": stack_segment,
            # REP MOVSD inherits the process direction flag.  The routine is
            # entered under the C runtime ABI, which requires DF to be clear.
            "flags": 0x0293,
        }
        outputs: list[dict[str, int]] = []

        def output_handler(
            _machine: Uc,
            port: int,
            size: int,
            value: int,
        ) -> None:
            outputs.append({"port": port, "size": size, "value": value})

        machine = execute(
            bytes(patched_image),
            entry,
            return_address,
            initial,
            [
                (data_segment, 0, bytes(data_before)),
                (raster_segment, 0, bytes(raster_before)),
                (framebuffer_segment, 0, bytes(framebuffer_before)),
                (extra_segment, 0, extra_before),
                (game_segment, 0, game_before),
                (
                    stack_segment,
                    0xFF00,
                    struct.pack("<H", return_address) + stack_sentinel,
                ),
            ],
            max_instructions=300000,
            output_handler=output_handler,
        )
        if outputs != expected_outputs:
            raise AssertionError(
                f"{module}:{entry:#x} {case['name']}: "
                f"outputs {outputs}, expected {expected_outputs}"
            )
        for segment, expected, owner in (
            (data_segment, bytes(data_expected), "data"),
            (raster_segment, bytes(raster_expected), "raster"),
            (
                framebuffer_segment,
                bytes(framebuffer_expected),
                "framebuffer",
            ),
            (extra_segment, extra_before, "initial-extra"),
            (game_segment, game_before, "game"),
        ):
            actual = bytes(machine.mem_read(segment * 16, 0x10000))
            if actual != expected:
                differences = [
                    (offset, actual[offset], expected[offset])
                    for offset in range(0x10000)
                    if actual[offset] != expected[offset]
                ][:8]
                raise AssertionError(
                    f"{module}:{entry:#x} {case['name']}: "
                    f"{owner} differs at {differences}"
                )
        if bytes(machine.mem_read(0, len(image))) != bytes(patched_image):
            raise AssertionError(f"{module}:{entry:#x} {case['name']}: code changed")
        if bytes(machine.mem_read(stack_segment * 16 + 0xFF02, 6)) != stack_sentinel:
            raise AssertionError(f"{module}:{entry:#x} {case['name']}: stack changed")
        expected_registers = {
            "sp": 0xFF02,
            "ds": extra_segment,
            "es": framebuffer_segment,
            "fs": data_segment,
            "gs": game_segment,
            "ss": stack_segment,
        }
        for register, expected in expected_registers.items():
            actual = machine.reg_read(REGISTERS[register])
            if actual != expected:
                raise AssertionError(
                    f"{module}:{entry:#x} {case['name']}: "
                    f"{register}={actual:#x}, expected={expected:#x}"
                )

        vectors.append(
            {
                "name": case["name"],
                "module": module,
                "entry": entry,
                "seed": case["seed"],
                "camera_matrix": [
                    signed_dword(value) for value in case["matrix"]
                ],
                "camera_position": [
                    signed_dword(value) for value in case["camera"]
                ],
                "shade_table": list(
                    raster_before[
                        shade_table_offset : shade_table_offset + 256
                    ]
                ),
                "camera_cells": list(camera_cells),
                "random_after": random,
                "stars": visible_stars,
                "accepted_per_plane": [
                    len(records) for records in plane_records
                ],
                "rejections": rejections,
                "plane_cursors": cursors,
                "plane_record_sha256": plane_hashes,
                "outputs": outputs,
                "raster_sha256": hashlib.sha256(raster_expected).hexdigest(),
                "framebuffer_sha256": hashlib.sha256(
                    framebuffer_expected
                ).hexdigest(),
                "state_sha256": hashlib.sha256(
                    bytes(raster_expected) + bytes(framebuffer_expected)
                ).hexdigest(),
            }
        )

    return vectors


def alien_primary_mesh_vectors(
    module: str,
    entry: int,
    body_hash: str,
    renderer_entry: int,
    bucket_base: int,
) -> list[dict[str, object]]:
    image = load_image(module)
    body_size = 403
    if hashlib.sha256(image[entry : entry + body_size]).hexdigest() != body_hash:
        raise AssertionError(f"{module}:{entry:#x}: recovered body changed")

    data_segment = 0x5000
    geometry_segment = 0x7000
    raster_segment = 0x9000
    extra_segment = 0xB000
    game_segment = 0xC000
    stack_segment = 0xD000
    return_address = 0xF000
    context_offset = 0x3000
    vertex_offset = 0x1000
    face_offset = 0x4000
    mask32 = 0xFFFFFFFF
    stack_sentinel = bytes.fromhex("a55a69967887")
    projection_matrix = (
        0x00000100,
        0,
        0,
        0,
        0x00000100,
        0,
        0,
        0,
        0x00010000,
    )
    cases = (
        {
            "name": "negative_depth_accumulator",
            "matrix": (0, 0, 0, 0, 0, 0, 1, 0, 0),
            "center": (160, 100),
            "vertices": ((-1, 2, 3),),
            "faces": ((0, 0, 0),),
        },
        {
            "name": "positive_depth_shifts_to_zero",
            "matrix": (0, 0, 0, 0, 0, 0, 1, 0, 0),
            "center": (160, 100),
            "vertices": ((255, 1, 0),),
            "faces": ((0, 0, 0),),
        },
        {
            "name": "interior_projection_and_face",
            "matrix": projection_matrix,
            "center": (160, 100),
            "vertices": ((-10, 0, 1), (20, 5, 1), (40, -5, 1)),
            "faces": ((0, 1, 2),),
        },
        {
            "name": "all_four_clip_edges",
            "matrix": projection_matrix,
            "center": (160, 100),
            "vertices": (
                (-161, 0, 1),
                (160, 0, 1),
                (0, 101, 1),
                (0, -100, 1),
                (0, 0, 1),
            ),
            "faces": ((0, 1, 4), (2, 3, 4)),
        },
        {
            "name": "common_left_clip_rejection",
            "matrix": projection_matrix,
            "center": (0, 100),
            "vertices": ((-200, 0, 1), (-180, 5, 1), (-170, -5, 1)),
            "faces": ((0, 1, 2),),
        },
        {
            "name": "both_rotations_and_ties",
            "matrix": projection_matrix,
            "center": (0, 100),
            "vertices": (
                (30, 0, 1),
                (20, 0, 1),
                (10, 0, 1),
                (30, 0, 1),
                (10, 0, 1),
                (20, 0, 1),
                (20, 0, 1),
                (10, 0, 1),
                (10, 0, 1),
                (10, 0, 1),
                (20, 0, 1),
                (20, 0, 1),
            ),
            "faces": ((0, 1, 2), (3, 4, 5), (6, 7, 8), (9, 10, 11)),
        },
        {
            "name": "face_clip_and_width_boundaries",
            "matrix": projection_matrix,
            "center": (0, 100),
            "vertices": (
                (-10, 0, 1),
                (-9, 0, 1),
                (-8, 0, 1),
                (-100, 0, 1),
                (399, 0, 1),
                (0, 0, 1),
                (-100, 0, 1),
                (400, 0, 1),
                (0, 0, 1),
            ),
            "faces": ((0, 1, 2), (3, 4, 5), (6, 7, 8)),
        },
        {
            "name": "negative_bucket_and_lifo_links",
            "matrix": projection_matrix,
            "center": (0, 100),
            "vertices": (
                (-20, 0, 1),
                (0, 0, 1),
                (15, 0, 1),
                (-20, 0, 1),
                (8, 0, 1),
                (4, 0, 1),
            ),
            "faces": ((0, 1, 2), (3, 4, 5)),
        },
        {
            "name": "modular_products_and_centers",
            "matrix": (
                0x7FFFFFFF,
                0x90000004,
                0x1234567C,
                0xFEDCBA9C,
                0x60000004,
                0xA0000004,
                0,
                0,
                0x00010000,
            ),
            "center": (0x7FFFFFF0, 0x80000020),
            "vertices": (
                (32767, -32768, 1),
                (-32768, 32767, 2),
                (0, 0, 1),
            ),
            "faces": ((0, 1, 2),),
        },
    )

    def put_u16(memory: bytearray, offset: int, value: int) -> None:
        struct.pack_into("<H", memory, offset, value & 0xFFFF)

    def put_u32(memory: bytearray, offset: int, value: int) -> None:
        struct.pack_into("<I", memory, offset, value & mask32)

    def get_u16(memory: bytearray, offset: int) -> int:
        return struct.unpack_from("<H", memory, offset)[0]

    def signed_word(value: int) -> int:
        value &= 0xFFFF
        return value if value < 0x8000 else value - 0x10000

    def signed_dword(value: int) -> int:
        value &= mask32
        return value if value < 0x80000000 else value - 0x100000000

    def dot_product(
        matrix: tuple[int, ...],
        row: int,
        coordinates: tuple[int, int, int],
    ) -> int:
        accumulator = 0
        for column, coordinate in enumerate(coordinates):
            accumulator += (
                (matrix[row * 3 + column] & mask32)
                * (coordinate & mask32)
            )
        return accumulator & mask32

    def signed_divide(numerator: int, denominator: int) -> int:
        quotient = abs(numerator) // abs(denominator)
        if (numerator < 0) != (denominator < 0):
            quotient = -quotient
        return quotient

    vectors: list[dict[str, object]] = []
    for case_index, case in enumerate(cases):
        data_before = bytearray(
            (offset * 29 + case_index * 17 + 3) & 0xFF
            for offset in range(0x10000)
        )
        geometry_before = bytearray(
            (offset * 13 + case_index * 23 + 9) & 0xFF
            for offset in range(0x10000)
        )
        raster_before = bytearray(
            (offset * 7 + case_index * 31 + 5) & 0xFF
            for offset in range(0x10000)
        )
        matrix = case["matrix"]
        center = case["center"]
        vertices = case["vertices"]
        faces = case["faces"]

        put_u16(data_before, 0x0002, geometry_segment)
        put_u16(data_before, 0x0006, raster_segment)
        put_u32(data_before, 0x2270, center[0])
        put_u32(data_before, 0x2274, center[1])
        put_u16(data_before, 0x227C, 0xA100 + case_index)
        put_u16(data_before, 0x227E, 0xB200 + case_index)
        put_u16(data_before, 0x2280, 0xC300 + case_index)
        for matrix_index, value in enumerate(matrix):
            put_u32(data_before, 0x22BA + matrix_index * 4, value)
        put_u16(data_before, 0x2306, context_offset)
        put_u16(data_before, context_offset + 0x1C, vertex_offset)
        put_u16(data_before, context_offset + 0x20, len(vertices))
        put_u16(data_before, context_offset + 0x28, face_offset)
        put_u16(data_before, context_offset + 0x2C, len(faces))

        vertex_offsets = []
        for vertex_index, coordinates in enumerate(vertices):
            current_offset = vertex_offset + vertex_index * 0x14
            vertex_offsets.append(current_offset)
            for coordinate_index, coordinate in enumerate(coordinates):
                put_u16(
                    geometry_before,
                    current_offset + 4 + coordinate_index * 2,
                    coordinate,
                )
            put_u16(geometry_before, current_offset + 0x0A, 0xA100 + vertex_index)
            put_u16(geometry_before, current_offset + 0x0C, 0xB200 + vertex_index)
            put_u32(geometry_before, current_offset + 0x0E, 0xC3C40000 + vertex_index)
            put_u16(geometry_before, current_offset + 0x12, 0xD500 + vertex_index)

        face_offsets = []
        for face_index, vertex_indices in enumerate(faces):
            current_offset = face_offset + face_index * 8
            face_offsets.append(current_offset)
            put_u16(geometry_before, current_offset, 0xE600 + face_index)
            for index, vertex_index in enumerate(vertex_indices):
                put_u16(
                    geometry_before,
                    current_offset + 2 + index * 2,
                    vertex_offsets[vertex_index],
                )

        data_expected = bytearray(data_before)
        geometry_expected = bytearray(geometry_before)
        raster_expected = bytearray(raster_before)
        common_clip = 0x800F
        vertex_results = []
        for vertex_index, coordinates in enumerate(vertices):
            current_offset = vertex_offsets[vertex_index]
            clip_flags = 0x800F
            projected_x = get_u16(geometry_expected, current_offset + 0x0A)
            projected_y = get_u16(geometry_expected, current_offset + 0x0C)
            depth_accumulator = dot_product(matrix, 2, coordinates)
            valid = False
            if signed_dword(depth_accumulator) >= 0:
                depth = signed_dword(depth_accumulator) >> 8
                if depth != 0:
                    screen_x_accumulator = dot_product(matrix, 0, coordinates)
                    screen_y_accumulator = dot_product(matrix, 1, coordinates)
                    screen_x = signed_divide(
                        signed_dword(screen_x_accumulator),
                        depth,
                    )
                    screen_y = signed_divide(
                        signed_dword(screen_y_accumulator),
                        depth,
                    )
                    screen_y = signed_dword(-screen_y)
                    screen_x = signed_dword(screen_x + center[0])
                    clip_flags = 0
                    if screen_x < 0:
                        clip_flags = 0x0001
                    if screen_x >= 320:
                        clip_flags = 0x0002
                    screen_y = signed_dword(screen_y + center[1])
                    if screen_y < 0:
                        clip_flags |= 0x0004
                    if screen_y >= 200:
                        clip_flags |= 0x0008
                    common_clip &= clip_flags
                    projected_x = screen_x & 0xFFFF
                    projected_y = screen_y & 0xFFFF
                    put_u16(geometry_expected, current_offset + 0x0A, projected_x)
                    put_u16(geometry_expected, current_offset + 0x0C, projected_y)
                    valid = True
            put_u16(geometry_expected, current_offset + 0x12, clip_flags)
            vertex_results.append(
                {
                    "offset": current_offset,
                    "valid_depth": valid,
                    "screen_x": signed_word(projected_x),
                    "screen_y": signed_word(projected_y),
                    "clip_flags": clip_flags,
                }
            )

        put_u16(data_expected, 0x227C, 0)
        put_u16(data_expected, 0x227E, common_clip)
        put_u16(data_expected, 0x2280, 0)
        face_results = []
        semantic_buckets: dict[int, list[int]] = {}
        if common_clip == 0:
            for face_index, vertex_indices in enumerate(faces):
                current_face_offset = face_offsets[face_index]
                offsets = [vertex_offsets[index] for index in vertex_indices]
                clips = [
                    get_u16(geometry_expected, offset + 0x12)
                    for offset in offsets
                ]
                x_values = [
                    signed_word(get_u16(geometry_expected, offset + 0x0A))
                    for offset in offsets
                ]
                bucket_offset = None
                if clips[0] & clips[1] & clips[2] == 0:
                    if x_values[1] > x_values[2]:
                        if x_values[0] >= x_values[2]:
                            offsets = [offsets[2], offsets[0], offsets[1]]
                            x_values = [x_values[2], x_values[0], x_values[1]]
                    elif x_values[0] > x_values[1]:
                        offsets = [offsets[1], offsets[2], offsets[0]]
                        x_values = [x_values[1], x_values[2], x_values[0]]
                    for index, value in enumerate(offsets):
                        put_u16(
                            geometry_expected,
                            current_face_offset + 2 + index * 2,
                            value,
                        )

                    span_1 = (x_values[1] - x_values[0]) & 0xFFFF
                    span_2 = (x_values[2] - x_values[0]) & 0xFFFF
                    if span_1 < 500 and span_2 < 500:
                        doubled_x = (x_values[0] << 1) & 0xFFFF
                        bucket_offset = bucket_base
                        if signed_word(doubled_x) >= 0:
                            bucket_offset = (bucket_offset + doubled_x) & 0xFFFF
                        previous_head = get_u16(raster_expected, bucket_offset)
                        put_u16(
                            raster_expected,
                            bucket_offset,
                            current_face_offset,
                        )
                        put_u16(
                            geometry_expected,
                            current_face_offset,
                            previous_head,
                        )
                        bucket_column = (
                            (bucket_offset - bucket_base) & 0xFFFF
                        ) // 2
                        semantic_buckets.setdefault(bucket_column, []).insert(
                            0,
                            face_index,
                        )
                face_results.append(
                    {
                        "offset": current_face_offset,
                        "vertices": offsets,
                        "left_x": x_values[0],
                        "bucket_offset": bucket_offset,
                    }
                )

        patched_image = bytearray(image)
        patched_image[renderer_entry] = 0xC3
        initial = {
            "eax": 0xA1A12345 + case_index,
            "ebx": 0xB2B23456 + case_index,
            "ecx": 0xC3C34567 + case_index,
            "edx": 0xD4D45678 + case_index,
            "esi": 0xE5E56789 + case_index,
            "edi": 0xF6F6789A + case_index,
            "ebp": 0x979789AB + case_index,
            "sp": 0xFF00,
            "ds": data_segment,
            "es": extra_segment,
            "fs": data_segment,
            "gs": game_segment,
            "ss": stack_segment,
            "flags": 0x0293 | (0x0400 if case_index & 1 else 0),
        }
        extra_before = bytes(
            (offset * 11 + case_index + 7) & 0xFF for offset in range(0x10000)
        )
        game_before = bytes(
            (offset * 19 + case_index + 13) & 0xFF for offset in range(0x10000)
        )
        renderer_states: list[dict[str, int]] = []

        def code_handler(
            machine: Uc, address: int, _size: int, _data: object
        ) -> None:
            if address == renderer_entry:
                stack_pointer = machine.reg_read(UC_X86_REG_SP)
                renderer_states.append(
                    {
                        "address": address,
                        "ds": machine.reg_read(UC_X86_REG_DS),
                        "es": machine.reg_read(UC_X86_REG_ES),
                        "fs": machine.reg_read(UC_X86_REG_FS),
                        "sp": stack_pointer,
                        "return_offset": struct.unpack(
                            "<H",
                            machine.mem_read(
                                stack_segment * 16 + stack_pointer,
                                2,
                            ),
                        )[0],
                    }
                )

        machine = execute(
            bytes(patched_image),
            entry,
            return_address,
            initial,
            [
                (data_segment, 0, bytes(data_before)),
                (geometry_segment, 0, bytes(geometry_before)),
                (raster_segment, 0, bytes(raster_before)),
                (extra_segment, 0, extra_before),
                (game_segment, 0, game_before),
                (
                    stack_segment,
                    0xFF00,
                    struct.pack("<H", return_address) + stack_sentinel,
                ),
            ],
            max_instructions=100000,
            code_handler=code_handler,
        )
        expected_renderer_states = (
            [
                {
                    "address": renderer_entry,
                    "ds": geometry_segment,
                    "es": raster_segment,
                    "fs": data_segment,
                    "sp": 0xFEFC,
                    "return_offset": entry + 401,
                }
            ]
            if common_clip == 0
            else []
        )
        if renderer_states != expected_renderer_states:
            raise AssertionError(
                f"{module}:{entry:#x} {case['name']}: "
                f"renderer states {renderer_states}, "
                f"expected {expected_renderer_states}"
            )
        for segment, expected, owner in (
            (data_segment, bytes(data_expected), "data"),
            (geometry_segment, bytes(geometry_expected), "geometry"),
            (raster_segment, bytes(raster_expected), "raster"),
            (extra_segment, extra_before, "initial-extra"),
            (game_segment, game_before, "game"),
        ):
            actual = bytes(machine.mem_read(segment * 16, 0x10000))
            if actual != expected:
                differences = [
                    (offset, actual[offset], expected[offset])
                    for offset in range(0x10000)
                    if actual[offset] != expected[offset]
                ][:8]
                raise AssertionError(
                    f"{module}:{entry:#x} {case['name']}: "
                    f"{owner} differs at {differences}"
                )
        if bytes(machine.mem_read(0, len(image))) != bytes(patched_image):
            raise AssertionError(f"{module}:{entry:#x} {case['name']}: code changed")
        if bytes(machine.mem_read(stack_segment * 16 + 0xFF02, 6)) != stack_sentinel:
            raise AssertionError(f"{module}:{entry:#x} {case['name']}: stack changed")

        expected_registers = {
            "esi": (initial["esi"] & 0xFFFF0000)
            | (
                face_offset + len(faces) * 8
                if common_clip == 0
                else vertex_offset + len(vertices) * 0x14
            ),
            "sp": 0xFF02,
            "ds": data_segment,
            "es": raster_segment if common_clip == 0 else geometry_segment,
            "fs": data_segment,
            "gs": game_segment,
            "ss": stack_segment,
        }
        for register, expected in expected_registers.items():
            actual = machine.reg_read(REGISTERS[register])
            if actual != expected:
                raise AssertionError(
                    f"{module}:{entry:#x} {case['name']}: "
                    f"{register}={actual:#x}, expected={expected:#x}"
                )

        vectors.append(
            {
                "name": case["name"],
                "module": module,
                "entry": entry,
                "camera_matrix": [signed_dword(value) for value in matrix],
                "screen_center": [
                    signed_dword(center[0]),
                    signed_dword(center[1]),
                ],
                "vertices_before": [
                    {
                        "position": [signed_word(value) for value in coordinates],
                        "screen": [
                            signed_word(0xA100 + vertex_index),
                            signed_word(0xB200 + vertex_index),
                        ],
                        "raster_depth": signed_dword(
                            0xC3C40000 + vertex_index
                        ),
                    }
                    for vertex_index, coordinates in enumerate(vertices)
                ],
                "faces_before": [list(face) for face in faces],
                "projected_vertices": [
                    {
                        "valid_depth": vertex["valid_depth"],
                        "screen": [vertex["screen_x"], vertex["screen_y"]],
                        "clip_flags": vertex["clip_flags"],
                    }
                    for vertex in vertex_results
                ],
                "face_decisions": [
                    {
                        "vertices": [
                            vertex_offsets.index(vertex_offset)
                            for vertex_offset in face["vertices"]
                        ],
                        "left_x": face["left_x"],
                        "bucket_column": (
                            None
                            if face["bucket_offset"] is None
                            else (
                                (face["bucket_offset"] - bucket_base) & 0xFFFF
                            )
                            // 2
                        ),
                    }
                    for face in face_results
                ],
                "buckets": [
                    {
                        "column": column,
                        "faces": faces,
                    }
                    for column, faces in sorted(semantic_buckets.items())
                ],
                "render_requested": common_clip == 0,
                "vertex_count": len(vertices),
                "face_count": len(faces),
                "vertices": vertex_results,
                "faces": face_results,
                "common_clip": common_clip,
                "renderer_state": (
                    renderer_states[0] if renderer_states else None
                ),
                "geometry_sha256": hashlib.sha256(geometry_expected).hexdigest(),
                "raster_sha256": hashlib.sha256(raster_expected).hexdigest(),
            }
        )

    return vectors


def alien_face_bucket_vectors(
    module: str,
    entry: int,
    body_size: int,
    body_hash: str,
    continuation: int,
    bucket_base: int,
    per_context_signal: bool,
) -> list[dict[str, object]]:
    image = load_image(module)
    if hashlib.sha256(image[entry : entry + body_size]).hexdigest() != body_hash:
        raise AssertionError(f"{module}:{entry:#x}: recovered body changed")

    data_segment = 0x5000
    geometry_segment = 0x7000
    raster_segment = 0x9000
    extra_segment = 0xB000
    game_segment = 0xC000
    stack_segment = 0xD000
    return_address = 0xF000
    stack_sentinel = bytes.fromhex("a55a69967887")
    context_list_offset = 0x2308
    context_offsets = (0x3000, 0x3100, 0x3200)
    cases = (
        {
            "name": "accepted_without_rotation",
            "contexts": ((((10, 20, 30), (0, 0, 0)),),),
            "latch": 0xA55A,
        },
        {
            "name": "rotate_vertex_2_leftmost",
            "contexts": ((((30, 20, 10), (0, 0, 0)),),),
            "latch": 0x5AA5,
        },
        {
            "name": "rotate_vertex_1_leftmost",
            "contexts": ((((30, 10, 20), (0, 0, 0)),),),
            "latch": 0x1357,
        },
        {
            "name": "ties_preserve_or_rotate_by_branch",
            "contexts": (
                (
                    ((10, 20, 20), (0, 0, 0)),
                    ((20, 10, 10), (0, 0, 0)),
                ),
            ),
            "latch": 0x2468,
        },
        {
            "name": "common_clip_and_width_boundaries",
            "contexts": (
                (
                    ((1, 2, 3), (1, 1, 1)),
                    ((0, 500, 200), (0, 0, 0)),
                    ((0, 499, 200), (0, 0, 0)),
                ),
            ),
            "latch": 0x369C,
        },
        {
            "name": "negative_bucket_and_lifo_links",
            "contexts": (
                (
                    ((-20, 0, 15), (0, 0, 0)),
                    ((-20, 8, 4), (0, 0, 0)),
                    ((-32768, -32760, -32750), (0, 0, 0)),
                ),
            ),
            "latch": 0x48AD,
        },
        {
            "name": "first_context_behind_signal",
            "contexts": (
                (((0, 600, 200), (0x8000, 0, 0)),),
                (((5, 15, 25), (0, 0, 0)),),
            ),
            "latch": 0x59BE,
        },
        {
            "name": "last_context_behind_signal",
            "contexts": (
                (((5, 15, 25), (0, 0, 0)),),
                (((0, 100, 200), (0, 0x8000, 0)),),
            ),
            "latch": 0x6ACF,
        },
        {
            "name": "common_behind_clip_does_not_signal",
            "contexts": ((((0, 10, 20), (0x8000, 0x8000, 0x8000)),),),
            "latch": 0x7BD0,
        },
        {
            "name": "wrapped_unsigned_width_rejection",
            "contexts": (
                (
                    ((-32760, 32760, -32750), (0, 0, 0)),
                    ((32760, -32760, 32767), (0, 0, 0)),
                ),
            ),
            "latch": 0x8CE1,
        },
    )

    def put_u16(memory: bytearray, offset: int, value: int) -> None:
        struct.pack_into("<H", memory, offset, value & 0xFFFF)

    def get_u16(memory: bytearray, offset: int) -> int:
        return struct.unpack_from("<H", memory, offset)[0]

    def signed_word(value: int) -> int:
        value &= 0xFFFF
        return value if value < 0x8000 else value - 0x10000

    vectors: list[dict[str, object]] = []
    for case_index, case in enumerate(cases):
        data_before = bytearray(
            (offset * 29 + case_index * 17 + 3) & 0xFF
            for offset in range(0x10000)
        )
        geometry_before = bytearray(
            (offset * 13 + case_index * 23 + 9) & 0xFF
            for offset in range(0x10000)
        )
        raster_before = bytearray(
            (offset * 7 + case_index * 31 + 5) & 0xFF
            for offset in range(0x10000)
        )
        put_u16(data_before, 0x0002, geometry_segment)
        put_u16(data_before, 0x0006, raster_segment)
        put_u16(data_before, 0x2282, case["latch"])

        face_offsets: list[list[int]] = []
        vertex_offsets: list[list[tuple[int, int, int]]] = []
        next_vertex = 0x1000
        for context_index, faces in enumerate(case["contexts"]):
            context_offset = context_offsets[context_index]
            first_face = 0x4000 + context_index * 0x0800
            put_u16(data_before, context_list_offset + context_index * 2, context_offset)
            put_u16(data_before, context_offset + 0x28, first_face)
            put_u16(data_before, context_offset + 0x2C, len(faces))
            current_faces = []
            current_vertices = []
            for face_index, (screen_x, clip_flags) in enumerate(faces):
                face_offset = first_face + face_index * 8
                vertices = (next_vertex, next_vertex + 0x20, next_vertex + 0x40)
                next_vertex += 0x60
                current_faces.append(face_offset)
                current_vertices.append(vertices)
                put_u16(geometry_before, face_offset, 0x7000 + face_index)
                for vertex_index, vertex_offset in enumerate(vertices):
                    put_u16(geometry_before, face_offset + 2 + vertex_index * 2, vertex_offset)
                    put_u16(geometry_before, vertex_offset + 0x0A, screen_x[vertex_index])
                    put_u16(geometry_before, vertex_offset + 0x12, clip_flags[vertex_index])
            face_offsets.append(current_faces)
            vertex_offsets.append(current_vertices)
        put_u16(
            data_before,
            context_list_offset + len(case["contexts"]) * 2,
            0,
        )

        data_expected = bytearray(data_before)
        geometry_expected = bytearray(geometry_before)
        raster_expected = bytearray(raster_before)
        if per_context_signal:
            put_u16(raster_expected, 0x07D4, 0)
        accepted: list[list[dict[str, object]]] = []
        semantic_buckets: dict[int, list[list[int]]] = {}
        for context_index, faces in enumerate(case["contexts"]):
            context_offset = context_offsets[context_index]
            context_accepted = []
            signaled = False
            for face_index, (screen_x_values, clip_values) in enumerate(faces):
                face_offset = face_offsets[context_index][face_index]
                vertices = list(vertex_offsets[context_index][face_index])
                x_values = [signed_word(value) for value in screen_x_values]
                common_clip = clip_values[0] & clip_values[1] & clip_values[2]
                bucket_offset = None
                if common_clip == 0:
                    combined_clip = clip_values[0] | clip_values[1] | clip_values[2]
                    if combined_clip & 0x8000:
                        signaled = True
                        if per_context_signal:
                            put_u16(raster_expected, 0x07D4, 1)
                        else:
                            put_u16(data_expected, 0x2282, 1)

                    if x_values[1] > x_values[2]:
                        if x_values[0] >= x_values[2]:
                            vertices = [vertices[2], vertices[0], vertices[1]]
                            x_values = [x_values[2], x_values[0], x_values[1]]
                            for index, value in enumerate(vertices):
                                put_u16(geometry_expected, face_offset + 2 + index * 2, value)
                    elif x_values[0] > x_values[1]:
                        vertices = [vertices[1], vertices[2], vertices[0]]
                        x_values = [x_values[1], x_values[2], x_values[0]]
                        for index, value in enumerate(vertices):
                            put_u16(geometry_expected, face_offset + 2 + index * 2, value)

                    span_1 = (x_values[1] - x_values[0]) & 0xFFFF
                    span_2 = (x_values[2] - x_values[0]) & 0xFFFF
                    if span_1 < 500 and span_2 < 500:
                        doubled_x = (x_values[0] << 1) & 0xFFFF
                        bucket_offset = bucket_base
                        if signed_word(doubled_x) >= 0:
                            bucket_offset = (bucket_offset + doubled_x) & 0xFFFF
                        old_head = get_u16(raster_expected, bucket_offset)
                        put_u16(raster_expected, bucket_offset, face_offset)
                        put_u16(geometry_expected, face_offset, old_head)
                        bucket_column = (
                            (bucket_offset - bucket_base) & 0xFFFF
                        ) // 2
                        semantic_buckets.setdefault(bucket_column, []).insert(
                            0,
                            [context_index, face_index],
                        )
                context_accepted.append(
                    {
                        "face_offset": face_offset,
                        "vertices": vertices,
                        "left_x": x_values[0],
                        "bucket_offset": bucket_offset,
                    }
                )
            if per_context_signal and signaled:
                put_u16(raster_expected, 0x07D4, 0)
                put_u16(data_expected, 0x2282, context_offset)
            accepted.append(context_accepted)

        patched_image = bytearray(image)
        patched_image[continuation] = 0xC3
        initial = {
            "eax": 0xA1A12345 + case_index,
            "ebx": 0xB2B23456 + case_index,
            "ecx": 0xC3C34567 + case_index,
            "edx": 0xD4D45678 + case_index,
            "esi": 0xE5E56789 + case_index,
            "edi": 0xF6F6789A + case_index,
            "ebp": 0x979789AB + case_index,
            "sp": 0xFF00,
            "ds": extra_segment,
            "es": extra_segment,
            "fs": data_segment,
            "gs": game_segment,
            "ss": stack_segment,
            "flags": 0x0293 | (0x0400 if case_index & 1 else 0),
        }
        extra_before = bytes(
            (offset * 11 + case_index + 7) & 0xFF for offset in range(0x10000)
        )
        game_before = bytes(
            (offset * 19 + case_index + 13) & 0xFF for offset in range(0x10000)
        )
        continuation_entries: list[int] = []

        def code_handler(
            _machine: Uc, address: int, _size: int, _data: object
        ) -> None:
            if address == continuation:
                continuation_entries.append(address)

        machine = execute(
            bytes(patched_image),
            entry,
            return_address,
            initial,
            [
                (data_segment, 0, bytes(data_before)),
                (geometry_segment, 0, bytes(geometry_before)),
                (raster_segment, 0, bytes(raster_before)),
                (extra_segment, 0, extra_before),
                (game_segment, 0, game_before),
                (
                    stack_segment,
                    0xFF00,
                    struct.pack("<H", return_address) + stack_sentinel,
                ),
            ],
            max_instructions=100000,
            code_handler=code_handler,
        )
        if continuation_entries != [continuation]:
            raise AssertionError(
                f"{module}:{entry:#x} {case['name']}: "
                f"continuation entries {continuation_entries}"
            )
        for segment, expected, owner in (
            (data_segment, bytes(data_expected), "data"),
            (geometry_segment, bytes(geometry_expected), "geometry"),
            (raster_segment, bytes(raster_expected), "raster"),
            (extra_segment, extra_before, "initial-segments"),
            (game_segment, game_before, "game"),
        ):
            actual = bytes(machine.mem_read(segment * 16, 0x10000))
            if actual != expected:
                differences = [
                    (offset, actual[offset], expected[offset])
                    for offset in range(0x10000)
                    if actual[offset] != expected[offset]
                ][:8]
                raise AssertionError(
                    f"{module}:{entry:#x} {case['name']}: "
                    f"{owner} differs at {differences}"
                )
        if bytes(machine.mem_read(0, len(image))) != bytes(patched_image):
            raise AssertionError(f"{module}:{entry:#x} {case['name']}: code changed")
        if bytes(machine.mem_read(stack_segment * 16 + 0xFF02, 6)) != stack_sentinel:
            raise AssertionError(f"{module}:{entry:#x} {case['name']}: stack changed")
        expected_registers = {
            "esi": (initial["esi"] & 0xFFFF0000)
            | (context_list_offset + len(case["contexts"]) * 2),
            "sp": 0xFF02,
            "ds": geometry_segment,
            "es": raster_segment,
            "fs": data_segment,
            "gs": game_segment,
            "ss": stack_segment,
        }
        for register, expected in expected_registers.items():
            actual = machine.reg_read(REGISTERS[register])
            if actual != expected:
                raise AssertionError(
                    f"{module}:{entry:#x} {case['name']}: "
                    f"{register}={actual:#x}, expected={expected:#x}"
                )

        vectors.append(
            {
                "name": case["name"],
                "module": module,
                "entry": entry,
                "contexts_before": [
                    [
                        {
                            "screen_x": [
                                signed_word(value) for value in screen_x_values
                            ],
                            "clip_flags": [value & 0xFFFF for value in clip_values],
                        }
                        for screen_x_values, clip_values in faces
                    ]
                    for faces in case["contexts"]
                ],
                "decisions": [
                    [
                        {
                            "vertices": [
                                {
                                    offset: local_index
                                    for local_index, offset in enumerate(
                                        vertex
                                        for face_vertices in vertex_offsets[context_index]
                                        for vertex in face_vertices
                                    )
                                }[vertex_offset]
                                for vertex_offset in face["vertices"]
                            ],
                            "left_x": face["left_x"],
                            "bucket_column": (
                                None
                                if face["bucket_offset"] is None
                                else (
                                    (
                                        face["bucket_offset"] - bucket_base
                                    )
                                    & 0xFFFF
                                )
                                // 2
                            ),
                        }
                        for face in context_faces
                    ]
                    for context_index, context_faces in enumerate(accepted)
                ],
                "buckets": [
                    {
                        "column": column,
                        "faces": faces,
                    }
                    for column, faces in sorted(semantic_buckets.items())
                ],
                "behind_signal": (
                    {
                        "kind": "context",
                        "context": next(
                            (
                                context_index
                                for context_index in range(
                                    len(case["contexts"]) - 1,
                                    -1,
                                    -1,
                                )
                                if any(
                                    (clips[0] & clips[1] & clips[2]) == 0
                                    and (clips[0] | clips[1] | clips[2]) & 0x8000
                                    for _screen, clips in case["contexts"][context_index]
                                )
                            ),
                            None,
                        ),
                    }
                    if per_context_signal
                    and any(
                        (clips[0] & clips[1] & clips[2]) == 0
                        and (clips[0] | clips[1] | clips[2]) & 0x8000
                        for faces in case["contexts"]
                        for _screen, clips in faces
                    )
                    else {
                        "kind": (
                            "general"
                            if not per_context_signal
                            and any(
                                (clips[0] & clips[1] & clips[2]) == 0
                                and (clips[0] | clips[1] | clips[2]) & 0x8000
                                for faces in case["contexts"]
                                for _screen, clips in faces
                            )
                            else "unchanged"
                        ),
                        "context": None,
                    }
                ),
                "context_count": len(case["contexts"]),
                "face_counts": [len(faces) for faces in case["contexts"]],
                "faces": accepted,
                "control_latch_before": case["latch"],
                "control_latch_after": get_u16(data_expected, 0x2282),
                "behind_scratch_after": (
                    get_u16(raster_expected, 0x07D4)
                    if per_context_signal
                    else None
                ),
                "geometry_sha256": hashlib.sha256(geometry_expected).hexdigest(),
                "raster_sha256": hashlib.sha256(raster_expected).hexdigest(),
            }
        )

    return vectors


def alien_transform_and_project_vectors(
    module: str, entry: int
) -> list[dict[str, object]]:
    image = load_image(module)
    body_size = 1192
    body_hash = "684386c05fa5f8cf92643bbc57b996af068081eb76ff69ccf5278e67acb5691a"
    if hashlib.sha256(image[entry : entry + body_size]).hexdigest() != body_hash:
        raise AssertionError(f"{module}:{entry:#x}: recovered body changed")

    data_segment = 0x5000
    object_segment = 0x7000
    extra_segment = 0x9000
    game_segment = 0xB000
    stack_segment = 0xD000
    return_address = 0xF000
    context_offset = 0x3000
    root_offset = 0x4000
    first_state_offset = root_offset + 0x005E
    mask32 = 0xFFFFFFFF
    stack_sentinel = bytes.fromhex("a55a69967887")

    identity_matrix = (
        0x00008000,
        0,
        0,
        0,
        0x00008000,
        0,
        0,
        0,
        0x00008000,
    )
    cases = (
        {
            "name": "interior_projection_and_copy",
            "center": (160, 100),
            "trig": "identity",
            "root_matrix": identity_matrix,
            "root_translation": (0, 0, 0),
            "states": (
                {
                    "parent": None,
                    "angles": (0, 0, 0),
                    "radial": 0,
                    "local": (0, 0, 0),
                    "vertices": ((20, 30, 256), (-40, -25, 256)),
                },
            ),
            "copies": ((0, 1),),
        },
        {
            "name": "nonpositive_depth_shift",
            "center": (160, 100),
            "trig": "identity",
            "root_matrix": identity_matrix,
            "root_translation": (0, 0, 0),
            "states": (
                {
                    "parent": None,
                    "angles": (0x0FFC, 0x1001, 0x2002),
                    "radial": 0,
                    "local": (0, 0, 0),
                    "vertices": ((12, -7, 0), (-18, 9, -2)),
                },
            ),
            "copies": (),
        },
        {
            "name": "all_clip_and_clamp_edges",
            "center": (160, 100),
            "trig": "identity",
            "root_matrix": identity_matrix,
            "root_translation": (0, 0, 0),
            "states": (
                {
                    "parent": None,
                    "angles": (0, 0, 0),
                    "radial": 0,
                    "local": (0, 0, 0),
                    "vertices": (
                        (-300, 0, 256),
                        (300, 0, 256),
                        (0, 250, 256),
                        (0, -250, 256),
                        (-200, 150, 256),
                        (200, -150, 256),
                    ),
                },
            ),
            "copies": (),
        },
        {
            "name": "common_left_clip_rejection",
            "center": (160, 100),
            "trig": "identity",
            "root_matrix": identity_matrix,
            "root_translation": (0, 0, 0),
            "states": (
                {
                    "parent": None,
                    "angles": (0, 0, 0),
                    "radial": 0,
                    "local": (0, 0, 0),
                    "vertices": ((-170, 0, 256), (-300, 20, 256)),
                },
            ),
            "copies": (),
        },
        {
            "name": "masked_angles_and_radial_rounding",
            "center": (-17, 243),
            "trig": "pattern",
            "root_matrix": identity_matrix,
            "root_translation": (0x12345678, 0xFEDCBA98, 0x00400000),
            "states": (
                {
                    "parent": None,
                    "angles": (0xF337, 0x166A, 0x299D),
                    "radial": -32767,
                    "local": (0x7FFFFFFF, 0x80000001, 0x1234FFFF),
                    "vertices": ((3, -5, 17), (0, 0, 1)),
                },
            ),
            "copies": ((0, 0), (0, 1)),
        },
        {
            "name": "two_state_parent_chain",
            "center": (160, 100),
            "trig": "identity",
            "root_matrix": identity_matrix,
            "root_translation": (0x1000, 0x2000, 0x3000),
            "states": (
                {
                    "parent": None,
                    "angles": (0, 0, 0),
                    "radial": 5,
                    "local": (1, 2, 3),
                    "vertices": ((1, 2, 256),),
                },
                {
                    "parent": 0,
                    "angles": (0x0FFC, 0x0FFC, 0x0FFC),
                    "radial": -7,
                    "local": (-4, 5, 6),
                    "vertices": ((-8, 12, 300), (15, -20, 400)),
                },
            ),
            "copies": ((0, 0),),
        },
        {
            "name": "modular_transform_overflow",
            "center": (0x7FFFFFF0, 0x80000020),
            "trig": "pattern",
            "root_matrix": (
                0x70000004,
                0x90000004,
                0x1234567C,
                0xFEDCBA9C,
                0x60000004,
                0xA0000004,
                0x7FFFFFF8,
                0x80000008,
                0xFFFFFFFF,
            ),
            "root_translation": (0x7FFFFFFC, 0x80000004, 0x80000000),
            "states": (
                {
                    "parent": None,
                    "angles": (0x0554, 0x0AA8, 0x0EEC),
                    "radial": 0x4001,
                    "local": (0x7FFF8000, 0x80007FFF, 0xFFFF0001),
                    "vertices": ((0, 0, 1), (1, -1, 2)),
                },
            ),
            "copies": (),
        },
        {
            "name": "copy_list_indirection",
            "center": (211, 73),
            "trig": "identity",
            "root_matrix": identity_matrix,
            "root_translation": (0, 0, 0),
            "states": (
                {
                    "parent": None,
                    "angles": (0, 0, 0),
                    "radial": 0,
                    "local": (0, 0, 0),
                    "vertices": ((5, 7, 256), (11, -13, 512), (19, 23, 256)),
                },
            ),
            "copies": ((0, 2), (0, 0), (0, 1)),
        },
    )

    def put_u16(memory: bytearray, offset: int, value: int) -> None:
        struct.pack_into("<H", memory, offset, value & 0xFFFF)

    def put_u32(memory: bytearray, offset: int, value: int) -> None:
        struct.pack_into("<I", memory, offset, value & mask32)

    def get_u16(memory: bytearray, offset: int) -> int:
        return struct.unpack_from("<H", memory, offset)[0]

    def get_u32(memory: bytearray, offset: int) -> int:
        return struct.unpack_from("<I", memory, offset)[0]

    def signed_word(value: int) -> int:
        value &= 0xFFFF
        return value if value < 0x8000 else value - 0x10000

    def signed_dword(value: int) -> int:
        value &= mask32
        return value if value < 0x80000000 else value - 0x100000000

    def add32(left: int, right: int) -> int:
        return (left + right) & mask32

    def mul32(left: int, right: int) -> int:
        return ((left & mask32) * (right & mask32)) & mask32

    def trunc_div(dividend: int, divisor: int) -> int:
        quotient = abs(dividend) // abs(divisor)
        return -quotient if (dividend < 0) != (divisor < 0) else quotient

    def sample(memory: bytearray, angle: int) -> tuple[int, int]:
        offset = 0x0036 + (angle & 0x0FFC)
        return (
            signed_word(get_u16(memory, offset)),
            signed_word(get_u16(memory, offset + 2)),
        )

    def rotation_matrix(memory: bytearray, angles: tuple[int, int, int]) -> list[int]:
        angle_0, angle_1, angle_2 = (value & 0x0FFC for value in angles)
        result = [0] * 9
        _cosine, sine = sample(memory, angle_0)
        result[5] = (-2 * sine) & mask32

        first_cos, first_sin = sample(memory, angle_0 - angle_1 - angle_2)
        second_cos, second_sin = sample(memory, angle_0 + angle_1 + angle_2)
        base_cos, base_sin = sample(memory, angle_1 + angle_2)
        value_0 = signed_dword(first_cos - second_cos) >> 1
        value_0 = add32(value_0, base_sin)
        value_1 = signed_dword(first_sin + second_sin) >> 1
        value_1 = add32(value_1, base_cos)
        result[1] = value_0
        result[6] = (-value_0) & mask32
        result[0] = value_1
        result[7] = value_1

        first_cos, first_sin = sample(memory, angle_0 - angle_1 + angle_2)
        second_cos, second_sin = sample(memory, angle_0 + angle_1 - angle_2)
        base_cos, base_sin = sample(memory, angle_1 - angle_2)
        value_0 = signed_dword(first_cos - second_cos) >> 1
        value_1 = signed_dword(first_sin + second_sin) >> 1
        source_0 = (base_sin - value_0) & mask32
        source_1 = (base_cos - value_1) & mask32
        result[1] = (result[1] - source_0) & mask32
        result[6] = (result[6] - source_0) & mask32
        result[0] = (result[0] + source_1) & mask32
        result[7] = (result[7] - source_1) & mask32

        first_cos, first_sin = sample(memory, angle_2 + angle_0)
        second_cos, second_sin = sample(memory, angle_2 - angle_0)
        result[4] = (first_cos + second_cos) & mask32
        result[3] = (-(first_sin + second_sin)) & mask32

        first_cos, first_sin = sample(memory, angle_1 + angle_0)
        second_cos, second_sin = sample(memory, angle_1 - angle_0)
        result[8] = (first_cos + second_cos) & mask32
        result[2] = (first_sin + second_sin) & mask32
        return result

    vectors: list[dict[str, object]] = []
    for case_index, case in enumerate(cases):
        data_before = bytearray(
            (offset * 29 + case_index * 17 + 3) & 0xFF
            for offset in range(0x10000)
        )
        object_before = bytearray(
            (offset * 13 + case_index * 23 + 9) & 0xFF
            for offset in range(0x10000)
        )
        for table_index in range(1024):
            if case["trig"] == "identity":
                cosine = 0x4000
                sine = 0
            else:
                cosine = (table_index * 0x9E37 + case_index * 0x2105 + 0x1357) & 0xFFFF
                sine = (table_index * 0x6D2B + case_index * 0x4211 + 0xA5A5) & 0xFFFF
            put_u16(data_before, 0x0036 + table_index * 4, cosine)
            put_u16(data_before, 0x0038 + table_index * 4, sine)

        put_u16(data_before, 0x0002, object_segment)
        put_u32(data_before, 0x2270, case["center"][0])
        put_u32(data_before, 0x2274, case["center"][1])
        put_u16(data_before, 0x2278, context_offset)
        put_u16(data_before, context_offset + 0x16, root_offset)
        put_u16(data_before, context_offset + 0x1A, len(case["states"]))

        for index, value in enumerate(case["root_matrix"]):
            put_u32(data_before, root_offset + 0x12 + index * 4, value)
        for index, value in enumerate(case["root_translation"]):
            put_u32(data_before, root_offset + 0x36 + index * 4, value)

        state_offsets: list[int] = []
        vertex_offsets: list[list[int]] = []
        for state_index, state_case in enumerate(case["states"]):
            state_offset = first_state_offset + state_index * 0x005E
            first_vertex = 0x1000 + state_index * 0x0800
            parent_index = state_case["parent"]
            parent_offset = (
                root_offset if parent_index is None else state_offsets[parent_index]
            )
            state_offsets.append(state_offset)
            put_u16(data_before, state_offset, parent_offset)
            put_u16(data_before, state_offset + 0x02, len(state_case["vertices"]))
            put_u16(data_before, state_offset + 0x06, first_vertex)
            for index in range(9):
                put_u32(data_before, state_offset + 0x12 + index * 4, 0xDEAD0000 + index)
            for index, value in enumerate(state_case["local"]):
                put_u32(data_before, state_offset + 0x42 + index * 4, value)
            for index, value in enumerate(state_case["angles"]):
                put_u16(data_before, state_offset + 0x4E + index * 2, value)
            put_u16(data_before, state_offset + 0x54, state_case["radial"])
            current_vertices = []
            for vertex_index, coordinates in enumerate(state_case["vertices"]):
                vertex_offset = first_vertex + vertex_index * 0x14
                current_vertices.append(vertex_offset)
                for coordinate_index, value in enumerate(coordinates):
                    put_u16(object_before, vertex_offset + 0x04 + coordinate_index * 2, value)
            vertex_offsets.append(current_vertices)

        copy_offset = 0x3000
        put_u16(data_before, context_offset + 0x22, copy_offset)
        put_u16(data_before, context_offset + 0x26, len(case["copies"]))
        for copy_index, (source_state, source_vertex) in enumerate(case["copies"]):
            destination = copy_offset + copy_index * 0x14
            put_u16(
                object_before,
                destination + 0x04,
                vertex_offsets[source_state][source_vertex],
            )

        data_expected = bytearray(data_before)
        object_expected = bytearray(object_before)
        put_u16(data_expected, 0x227C, len(case["states"]))
        projected: list[list[dict[str, int]]] = []
        for state_index, state_case in enumerate(case["states"]):
            state_offset = state_offsets[state_index]
            parent_offset = get_u16(data_expected, state_offset)
            angles = tuple(
                get_u16(data_expected, state_offset + 0x4E + index * 2)
                for index in range(3)
            )
            masked_angles = tuple(value & 0x0FFC for value in angles)
            put_u16(data_expected, 0x227A, state_offset)
            put_u16(data_expected, 0x0030, masked_angles[1])
            put_u16(data_expected, 0x0032, masked_angles[0])
            put_u16(data_expected, 0x0034, masked_angles[2])
            rotation = rotation_matrix(data_expected, angles)
            for index, value in enumerate(rotation):
                put_u32(data_expected, 0x2284 + index * 4, value)

            radial = signed_word(get_u16(data_expected, state_offset + 0x54))
            if radial != 0:
                for index, matrix_index in enumerate((2, 5, 8)):
                    product = mul32(rotation[matrix_index], radial)
                    delta = signed_dword(product) >> 16
                    if index == 1:
                        delta += (product >> 15) & 1
                    position_offset = state_offset + 0x42 + index * 4
                    put_u32(
                        data_expected,
                        position_offset,
                        add32(get_u32(data_expected, position_offset), delta),
                    )

            sources = [
                signed_word(get_u16(data_expected, state_offset + 0x42 + index * 4))
                for index in range(3)
            ]
            for row in (2, 1, 0):
                accumulator = mul32(
                    get_u32(data_expected, parent_offset + 0x12 + row * 12),
                    sources[0],
                )
                accumulator = add32(
                    accumulator,
                    mul32(
                        get_u32(data_expected, parent_offset + 0x16 + row * 12),
                        sources[1],
                    ),
                )
                accumulator = add32(
                    accumulator,
                    mul32(
                        get_u32(data_expected, parent_offset + 0x1A + row * 12),
                        sources[2],
                    ),
                )
                accumulator = add32(
                    accumulator,
                    get_u32(data_expected, parent_offset + 0x36 + row * 4),
                )
                put_u32(data_expected, state_offset + 0x36 + row * 4, accumulator)

            for row in range(3):
                for column in range(3):
                    accumulator = 0
                    for term in range(3):
                        accumulator = add32(
                            accumulator,
                            mul32(
                                get_u32(
                                    data_expected,
                                    parent_offset + 0x12 + (row * 3 + term) * 4,
                                ),
                                rotation[term * 3 + column],
                            ),
                        )
                    put_u32(
                        data_expected,
                        state_offset + 0x12 + (row * 3 + column) * 4,
                        signed_dword(accumulator) >> 15,
                    )

            put_u16(data_expected, 0x227E, 0x800F)
            put_u16(data_expected, 0x2280, 0)
            state_projected = []
            for vertex_offset in vertex_offsets[state_index]:
                coordinates = [
                    signed_word(get_u16(object_expected, vertex_offset + 4 + index * 2))
                    for index in range(3)
                ]
                row_values = []
                for row in range(3):
                    accumulator = 0
                    for column in range(3):
                        accumulator = add32(
                            accumulator,
                            mul32(
                                get_u32(
                                    data_expected,
                                    state_offset + 0x12 + (row * 3 + column) * 4,
                                ),
                                coordinates[column],
                            ),
                        )
                    accumulator = add32(
                        accumulator,
                        get_u32(data_expected, state_offset + 0x36 + row * 4),
                    )
                    row_values.append(signed_dword(accumulator))

                depth = row_values[2] >> 8
                put_u32(object_expected, vertex_offset + 0x0E, depth)
                if depth > 0:
                    screen_x = trunc_div(row_values[0], depth)
                    screen_y = trunc_div(row_values[1], depth)
                    flags = 0
                else:
                    screen_x = row_values[0] >> 12
                    screen_y = row_values[1] >> 12
                    flags = 0x8000
                screen_y = signed_dword(-screen_y)
                screen_x = signed_dword(screen_x + case["center"][0])
                if screen_x < 0:
                    flags = (flags & 0xFF00) | 1
                    if screen_x <= -90:
                        screen_x = -89
                if screen_x >= 320:
                    flags = (flags & 0xFF00) | 2
                    if screen_x >= 410:
                        screen_x = 409
                screen_y = signed_dword(screen_y + case["center"][1])
                if screen_y < 0:
                    flags |= 4
                    if screen_y <= -150:
                        screen_y = -149
                if screen_y >= 200:
                    flags |= 8
                    if screen_y >= 350:
                        screen_y = 349
                common_clip = get_u16(data_expected, 0x227E) & flags
                put_u16(data_expected, 0x227E, common_clip)
                put_u16(object_expected, vertex_offset + 0x0A, screen_x)
                put_u16(object_expected, vertex_offset + 0x0C, screen_y)
                put_u16(object_expected, vertex_offset + 0x12, flags)
                state_projected.append(
                    {
                        "screen_x": signed_word(screen_x),
                        "screen_y": signed_word(screen_y),
                        "depth": signed_dword(depth),
                        "flags": flags,
                    }
                )
            if get_u16(data_expected, 0x227E) != 0:
                for vertex_index, vertex_offset in enumerate(vertex_offsets[state_index]):
                    put_u16(object_expected, vertex_offset + 0x12, 0x00FF)
                    state_projected[vertex_index]["flags"] = 0x00FF
            projected.append(state_projected)
            put_u16(
                data_expected,
                0x227C,
                get_u16(data_expected, 0x227C) - 1,
            )

        for copy_index, (source_state, source_vertex) in enumerate(case["copies"]):
            source = vertex_offsets[source_state][source_vertex]
            destination = copy_offset + copy_index * 0x14
            object_expected[destination + 0x0A : destination + 0x14] = (
                object_expected[source + 0x0A : source + 0x14]
            )

        initial = {
            "eax": 0xA1A12345 + case_index,
            "ebx": 0xB2B23456 + case_index,
            "ecx": 0xC3C34567 + case_index,
            "edx": 0xD4D45678 + case_index,
            "esi": 0xE5E56789 + case_index,
            "edi": 0xF6F6789A + case_index,
            "ebp": 0x979789AB + case_index,
            "sp": 0xFF00,
            "ds": data_segment,
            "es": extra_segment,
            "fs": data_segment,
            "gs": game_segment,
            "ss": stack_segment,
            "flags": 0x0293 | (0x0400 if case_index & 1 else 0),
        }
        extra_before = bytes(
            (offset * 7 + case_index + 5) & 0xFF for offset in range(0x10000)
        )
        game_before = bytes(
            (offset * 19 + case_index + 11) & 0xFF for offset in range(0x10000)
        )
        machine = execute(
            image,
            entry,
            return_address,
            initial,
            [
                (data_segment, 0, bytes(data_before)),
                (object_segment, 0, bytes(object_before)),
                (extra_segment, 0, extra_before),
                (game_segment, 0, game_before),
                (
                    stack_segment,
                    0xFF00,
                    struct.pack("<H", return_address) + stack_sentinel,
                ),
            ],
            max_instructions=500000,
        )

        for segment, expected, owner in (
            (data_segment, bytes(data_expected), "data"),
            (object_segment, bytes(object_expected), "object"),
            (extra_segment, extra_before, "initial-es"),
            (game_segment, game_before, "game"),
        ):
            actual = bytes(machine.mem_read(segment * 16, 0x10000))
            if actual != expected:
                differences = [
                    (offset, actual[offset], expected[offset])
                    for offset in range(0x10000)
                    if actual[offset] != expected[offset]
                ][:8]
                raise AssertionError(
                    f"{module}:{entry:#x} {case['name']}: "
                    f"{owner} differs at {differences}"
                )
        actual_image = bytes(machine.mem_read(0, len(image)))
        if actual_image != image:
            differences = [
                (offset, actual_image[offset], image[offset])
                for offset in range(len(image))
                if actual_image[offset] != image[offset]
            ][:8]
            raise AssertionError(
                f"{module}:{entry:#x} {case['name']}: code differs at {differences}"
            )
        if bytes(machine.mem_read(stack_segment * 16 + 0xFF02, 6)) != stack_sentinel:
            raise AssertionError(f"{module}:{entry:#x} {case['name']}: stack changed")
        expected_segments = {
            "ds": data_segment,
            "es": object_segment,
            "fs": data_segment,
            "gs": game_segment,
            "ss": stack_segment,
            "sp": 0xFF02,
        }
        for register, expected in expected_segments.items():
            actual = machine.reg_read(REGISTERS[register])
            if actual != expected:
                raise AssertionError(
                    f"{module}:{entry:#x} {case['name']}: "
                    f"{register}={actual:#x}, expected={expected:#x}"
                )

        vectors.append(
            {
                "name": case["name"],
                "module": module,
                "entry": entry,
                "screen_center": [
                    signed_dword(case["center"][0]),
                    signed_dword(case["center"][1]),
                ],
                "trigonometry_pattern": {
                    "cosine_multiplier": (
                        0 if case["trig"] == "identity" else 0x9E37
                    ),
                    "cosine_offset": (
                        0x4000
                        if case["trig"] == "identity"
                        else (case_index * 0x2105 + 0x1357) & 0xFFFF
                    ),
                    "sine_multiplier": (
                        0 if case["trig"] == "identity" else 0x6D2B
                    ),
                    "sine_offset": (
                        0
                        if case["trig"] == "identity"
                        else (case_index * 0x4211 + 0xA5A5) & 0xFFFF
                    ),
                },
                "root": {
                    "matrix": [signed_dword(value) for value in case["root_matrix"]],
                    "translation": [
                        signed_dword(value) for value in case["root_translation"]
                    ],
                },
                "nodes_before": [
                    {
                        "parent": state["parent"],
                        "angles": [value & 0xFFFF for value in state["angles"]],
                        "radial_offset": signed_word(state["radial"]),
                        "local_position": [
                            signed_dword(value) for value in state["local"]
                        ],
                        "vertices": [
                            [signed_word(value) for value in vertex]
                            for vertex in state["vertices"]
                        ],
                    }
                    for state in case["states"]
                ],
                "nodes_after": [
                    {
                        "local_position": [
                            signed_dword(
                                get_u32(
                                    data_expected,
                                    state_offsets[state_index] + 0x42 + axis * 4,
                                )
                            )
                            for axis in range(3)
                        ],
                        "matrix": [
                            signed_dword(
                                get_u32(
                                    data_expected,
                                    state_offsets[state_index] + 0x12 + index * 4,
                                )
                            )
                            for index in range(9)
                        ],
                        "translation": [
                            signed_dword(
                                get_u32(
                                    data_expected,
                                    state_offsets[state_index] + 0x36 + axis * 4,
                                )
                            )
                            for axis in range(3)
                        ],
                    }
                    for state_index in range(len(case["states"]))
                ],
                "projection_copies": [
                    {
                        "source": sum(
                            len(state["vertices"])
                            for state in case["states"][:source_state]
                        )
                        + source_vertex,
                        "destination": sum(
                            len(state["vertices"]) for state in case["states"]
                        )
                        + copy_index,
                    }
                    for copy_index, (source_state, source_vertex) in enumerate(
                        case["copies"]
                    )
                ],
                "state_count": len(case["states"]),
                "vertex_counts": [len(state["vertices"]) for state in case["states"]],
                "copy_count": len(case["copies"]),
                "projected": projected,
                "projected_vertices": [
                    vertex
                    for state_vertices in projected
                    for vertex in state_vertices
                ]
                + [
                    projected[source_state][source_vertex]
                    for source_state, source_vertex in case["copies"]
                ],
                "last_rotation_matrix": [
                    signed_dword(get_u32(data_expected, 0x2284 + index * 4))
                    for index in range(9)
                ],
                "last_common_clip": get_u16(data_expected, 0x227E),
                "data_sha256": hashlib.sha256(data_expected).hexdigest(),
                "object_sha256": hashlib.sha256(object_expected).hexdigest(),
            }
        )

    return vectors


def alien_camera_matrix_update_vectors(
    module: str, entry: int
) -> list[dict[str, object]]:
    image = load_image(module)
    body_size = 591
    body_hash = "6f5317ac95a203f579dc60dd859573d7eb7f965bc22fc5298ade3e47b1ae2511"
    if hashlib.sha256(image[entry : entry + body_size]).hexdigest() != body_hash:
        raise AssertionError(f"{module}:{entry:#x}: recovered body changed")

    data_segment = 0x5000
    extra_segment = 0x7000
    fs_segment = 0x9000
    game_segment = 0xB000
    stack_segment = 0xD000
    return_address = 0xF000
    stack_sentinel = bytes.fromhex("a55a69967887")
    mask32 = 0xFFFFFFFF
    cases = (
        (
            "positive_rounding",
            0x0000,
            0x0000,
            0x0000,
            0x0000,
            (0, 1, 2, 3, 4, 5, 7, 8, 0xFFFFFFFF),
            (0x00000000, 0x7FFF0001, 0x8000FFFF),
        ),
        (
            "negative_rounding_and_masking",
            0xFFFF,
            0x8003,
            0x1006,
            0xFFFF,
            (
                0xFFFFFFFF,
                0xFFFFFFFD,
                0xFFFFFFFC,
                0xFFFFFFFB,
                0xFFFFFFF9,
                0xFFFFFFF8,
                0xFFFFFFF7,
                0x00000004,
                0x0000000C,
            ),
            (0x12345678, 0x89ABCDEF, 0x0FEDCBA9),
        ),
        (
            "wrapped_angle_sums",
            0x0FFC,
            0x0FFC,
            0x0FFC,
            0x7FFF,
            (0x10, 0x20, 0x40, 0x80, 0x100, 0x200, 0x400, 0x800, 0x1000),
            (0xFFFFFFFF, 0x80000000, 0x7FFFFFFF),
        ),
        (
            "delta_sign_boundaries",
            0x0802,
            0x0C03,
            0xF805,
            0x8000,
            (
                0x7FFFFFFC,
                0x7FFFFFFF,
                0x80000000,
                0x80000004,
                0x80000007,
                0xFFFFFFFC,
                0x00000004,
                0x40000004,
                0xC0000004,
            ),
            (0x7FFFFFFC, 0x80000004, 0xFFFF0001),
        ),
        (
            "product_and_dot_overflow",
            0x0554,
            0x0AA8,
            0x0EEC,
            0x4001,
            (
                0x70000004,
                0x90000004,
                0x1234567C,
                0xFEDCBA9C,
                0x60000004,
                0xA0000004,
                0x7FFFFFF8,
                0x80000008,
                0xFFFFFFFF,
            ),
            (0x13572468, 0x24681357, 0xA5A55A5A),
        ),
        (
            "position_high_words_feed_view",
            0x0337,
            0x066A,
            0x099D,
            0xFFFE,
            (0x24, 0x44, 0x64, 0x84, 0xA4, 0xC4, 0xE4, 0x104, 0x124),
            (0x0000FFFF, 0xFFFF0000, 0x7FFF8000),
        ),
    )
    flag_masks = {
        "cf": 0x0001,
        "pf": 0x0004,
        "af": 0x0010,
        "zf": 0x0040,
        "sf": 0x0080,
        "if": 0x0200,
        "df": 0x0400,
        "of": 0x0800,
    }
    vectors: list[dict[str, object]] = []

    def put_u16(memory: bytearray, offset: int, value: int) -> None:
        struct.pack_into("<H", memory, offset, value & 0xFFFF)

    def put_u32(memory: bytearray, offset: int, value: int) -> None:
        struct.pack_into("<I", memory, offset, value & mask32)

    def get_u16(memory: bytearray, offset: int) -> int:
        return struct.unpack_from("<H", memory, offset)[0]

    def get_u32(memory: bytearray, offset: int) -> int:
        return struct.unpack_from("<I", memory, offset)[0]

    def signed_word(value: int) -> int:
        value &= 0xFFFF
        return value if value < 0x8000 else value - 0x10000

    def signed_dword(value: int) -> int:
        value &= mask32
        return value if value < 0x80000000 else value - 0x100000000

    def sample(memory: bytearray, angle: int) -> tuple[int, int]:
        offset = 0x0036 + (angle & 0x0FFC)
        return (
            signed_word(get_u16(memory, offset)),
            signed_word(get_u16(memory, offset + 2)),
        )

    def target_matrix(
        memory: bytearray, pitch: int, pan: int, secondary: int
    ) -> list[int]:
        target = [0] * 9
        _cosine, sine = sample(memory, pitch)
        target[7] = (-2 * sine) & mask32

        combined = (pan + secondary) & 0x0FFC
        first_cos, first_sin = sample(memory, pitch - combined)
        second_cos, second_sin = sample(memory, pitch + combined)
        cosine_half_difference = (first_cos - second_cos) >> 1
        sine_half_sum = (first_sin + second_sin) >> 1
        axis_cos, axis_sin = sample(memory, combined)
        correction = cosine_half_difference + axis_sin
        target[3] = correction & mask32
        target[2] = (-correction) & mask32
        correction = sine_half_sum + axis_cos
        target[0] = correction & mask32
        target[5] = correction & mask32

        combined = (pan - secondary) & 0x0FFC
        first_cos, first_sin = sample(memory, pitch - combined)
        second_cos, second_sin = sample(memory, pitch + combined)
        cosine_half_difference = (first_cos - second_cos) >> 1
        sine_half_sum = (first_sin + second_sin) >> 1
        axis_cos, axis_sin = sample(memory, combined)
        correction = axis_sin - cosine_half_difference
        target[3] = (target[3] - correction) & mask32
        target[2] = (target[2] - correction) & mask32
        correction = axis_cos - sine_half_sum
        target[0] = (target[0] + correction) & mask32
        target[5] = (target[5] - correction) & mask32

        first_cos, first_sin = sample(memory, secondary + pitch)
        second_cos, second_sin = sample(memory, secondary - pitch)
        target[4] = (first_cos + second_cos) & mask32
        target[1] = (-(first_sin + second_sin)) & mask32

        first_cos, first_sin = sample(memory, pan + pitch)
        second_cos, second_sin = sample(memory, pan - pitch)
        target[8] = (first_cos + second_cos) & mask32
        target[6] = (first_sin + second_sin) & mask32
        return target

    def add_flags_32(left: int, right: int, initial_flags: int) -> dict[str, bool]:
        left &= mask32
        right &= mask32
        total = left + right
        result = total & mask32
        return {
            "cf": total > mask32,
            "pf": (result & 0xFF).bit_count() % 2 == 0,
            "af": ((left & 0xF) + (right & 0xF)) > 0xF,
            "zf": result == 0,
            "sf": bool(result & 0x80000000),
            "if": bool(initial_flags & 0x0200),
            "df": bool(initial_flags & 0x0400),
            "of": bool((~(left ^ right) & (left ^ result) & 0x80000000)),
        }

    for case_index, case in enumerate(cases):
        (
            name,
            pitch_input,
            pan_input,
            secondary_input,
            depth_input,
            desired_deltas,
            positions,
        ) = case
        data_before = bytearray(
            (offset * 31 + case_index * 19 + 7) & 0xFF
            for offset in range(0x10000)
        )
        for table_index in range(1024):
            cosine = (
                table_index * 0x9E37 + case_index * 0x2105 + 0x1357
            ) & 0xFFFF
            sine = (
                table_index * 0x6D2B + case_index * 0x4211 + 0xA5A5
            ) & 0xFFFF
            put_u16(data_before, 0x0036 + table_index * 4, cosine)
            put_u16(data_before, 0x0038 + table_index * 4, sine)
        put_u16(data_before, 0x22F6, pitch_input)
        put_u16(data_before, 0x22F8, pan_input)
        put_u16(data_before, 0x22FA, secondary_input)
        put_u16(data_before, 0x22FC, depth_input)
        for index, position in enumerate(positions):
            put_u32(data_before, 0x22EA + index * 4, position)

        pitch = pitch_input & 0x0FFC
        pan = pan_input & 0x0FFC
        secondary = secondary_input & 0x0FFC
        target = target_matrix(data_before, pitch, pan, secondary)
        for index, desired_delta in enumerate(desired_deltas):
            put_u32(
                data_before,
                0x22BA + index * 4,
                target[index] - desired_delta,
            )

        data_expected = bytearray(data_before)
        put_u16(data_expected, 0x0030, pan)
        put_u16(data_expected, 0x0032, pitch)
        put_u16(data_expected, 0x0034, secondary)
        for index, value in enumerate(target):
            put_u32(data_expected, 0x2284 + index * 4, value)

        matrix: list[int] = []
        for index in range(9):
            current = get_u32(data_expected, 0x22BA + index * 4)
            delta = (target[index] - current) & mask32
            step = signed_dword(delta) >> 3
            current = (current + step + ((delta >> 2) & 1)) & mask32
            matrix.append(current)
            put_u32(data_expected, 0x22BA + index * 4, current)

        depth_factor = (-signed_word(depth_input)) & mask32
        last_full_product = 0
        for index in range(3):
            product = (matrix[index + 6] * depth_factor) & mask32
            last_full_product = (
                signed_dword(matrix[index + 6]) * signed_dword(depth_factor)
            )
            position = get_u32(data_expected, 0x22EA + index * 4)
            position = (position + (signed_dword(product) >> 3)) & mask32
            put_u32(data_expected, 0x22EA + index * 4, position)

        view = [
            signed_word(get_u16(data_expected, offset))
            for offset in (0x22EC, 0x22F0, 0x22F4)
        ]
        results = [0, 0, 0]
        partial_row_zero = 0
        final_left = 0
        for row in (2, 1, 0):
            terms = [
                (matrix[row * 3 + column] * (view[column] & mask32)) & mask32
                for column in range(3)
            ]
            partial = (terms[0] + terms[1]) & mask32
            result = (partial + terms[2]) & mask32
            results[row] = result
            put_u32(data_expected, 0x22DE + row * 4, result)
            if row == 0:
                partial_row_zero = partial
                final_left = terms[2]

        initial_flags = 0x0293 | (0x0400 if case_index & 1 else 0)
        initial = {
            "eax": 0xA1A12345 + case_index,
            "ebx": 0xB2B23456 + case_index,
            "ecx": 0xC3C34567 + case_index,
            "edx": 0xD4D45678 + case_index,
            "esi": 0xE5E56789 + case_index,
            "edi": 0xF6F6789A + case_index,
            "ebp": 0x979789AB + case_index,
            "sp": 0xFF00,
            "ds": data_segment,
            "es": extra_segment,
            "fs": fs_segment,
            "gs": game_segment,
            "ss": stack_segment,
            "flags": initial_flags,
        }
        code_before = bytes(image)
        extra_before = bytes(
            (offset * 13 + case_index + 3) & 0xFF for offset in range(0x10000)
        )
        fs_before = bytes(
            (offset * 17 + case_index + 5) & 0xFF for offset in range(0x10000)
        )
        game_before = bytes(
            (offset * 23 + case_index + 9) & 0xFF for offset in range(0x10000)
        )
        machine = execute(
            code_before,
            entry,
            return_address,
            initial,
            [
                (data_segment, 0, bytes(data_before)),
                (extra_segment, 0, extra_before),
                (fs_segment, 0, fs_before),
                (game_segment, 0, game_before),
                (
                    stack_segment,
                    0xFF00,
                    struct.pack("<H", return_address) + stack_sentinel,
                ),
            ],
        )
        actual_data = bytes(machine.mem_read(data_segment * 16, 0x10000))
        if actual_data != bytes(data_expected):
            differences = [
                (offset, actual_data[offset], data_expected[offset])
                for offset in range(0x10000)
                if actual_data[offset] != data_expected[offset]
            ][:8]
            raise AssertionError(
                f"{module}:{entry:#x} {name}: data differs at {differences}"
            )
        if bytes(machine.mem_read(0, len(image))) != code_before:
            raise AssertionError(f"{module}:{entry:#x} {name}: code changed")
        for segment, expected in (
            (extra_segment, extra_before),
            (fs_segment, fs_before),
            (game_segment, game_before),
        ):
            if bytes(machine.mem_read(segment * 16, 0x10000)) != expected:
                raise AssertionError(
                    f"{module}:{entry:#x} {name}: segment {segment:#x} changed"
                )

        expected_registers = dict(initial)
        del expected_registers["flags"]
        expected_registers["eax"] = results[0]
        expected_registers["ebx"] = view[0] & mask32
        expected_registers["ecx"] = view[1] & mask32
        expected_registers["edx"] = (last_full_product >> 32) & mask32
        expected_registers["esi"] = view[2] & mask32
        expected_registers["edi"] = (initial["edi"] & 0xFFFF0000) | 0x22DE
        expected_registers["ebp"] = partial_row_zero
        expected_registers["sp"] = 0xFF02
        for register, expected in expected_registers.items():
            actual = machine.reg_read(REGISTERS[register])
            if actual != expected:
                raise AssertionError(
                    f"{module}:{entry:#x} {name}: "
                    f"{register}={actual:#x}, expected={expected:#x}"
                )

        expected_flags = add_flags_32(final_left, partial_row_zero, initial_flags)
        flags_after = machine.reg_read(UC_X86_REG_EFLAGS)
        actual_flags = {
            flag: bool(flags_after & mask) for flag, mask in flag_masks.items()
        }
        if actual_flags != expected_flags:
            raise AssertionError(
                f"{module}:{entry:#x} {name}: "
                f"flags={actual_flags}, expected={expected_flags}"
            )
        if bytes(machine.mem_read(stack_segment * 16 + 0xFF02, 6)) != stack_sentinel:
            raise AssertionError(f"{module}:{entry:#x} {name}: stack changed")

        vectors.append(
            {
                "name": name,
                "module": module,
                "entry": entry,
                "trigonometry_pattern": {
                    "cosine_multiplier": 0x9E37,
                    "cosine_offset": (
                        case_index * 0x2105 + 0x1357
                    ) & 0xFFFF,
                    "sine_multiplier": 0x6D2B,
                    "sine_offset": (
                        case_index * 0x4211 + 0xA5A5
                    ) & 0xFFFF,
                },
                "angles_before": [
                    pitch_input,
                    pan_input,
                    secondary_input,
                ],
                "normalized_angles": [pitch, pan, secondary],
                "depth_step": signed_word(depth_input),
                "camera_matrix_before": [
                    signed_dword(get_u32(data_before, 0x22BA + index * 4))
                    for index in range(9)
                ],
                "target_matrix": [signed_dword(value) for value in target],
                "camera_matrix": [signed_dword(value) for value in matrix],
                "camera_position_before": [
                    signed_dword(value) for value in positions
                ],
                "camera_position": [
                    signed_dword(get_u32(data_expected, 0x22EA + index * 4))
                    for index in range(3)
                ],
                "view": view,
                "result": [signed_dword(value) for value in results],
                "data_sha256": hashlib.sha256(data_expected).hexdigest(),
                "defined_flags": expected_flags,
            }
        )

    return vectors


def alien_slot7_palette_update_vectors(
    module: str, entry: int
) -> list[dict[str, object]]:
    image = load_image(module)
    if module == "amer":
        body_size = 326
        body_hash = "ece70386a3be89e1fee265e7a6574ab62278cba59efa250d2bbe20bd19a17249"
        remap_offset = 0x049B
        has_pulse = False
    else:
        body_size = 370
        body_hash = "7835f5dd49d2936d8747723768ac7008a4ed64a11b3791df33e35222bec184f8"
        remap_offset = 0x04DC
        has_pulse = True
    if hashlib.sha256(image[entry : entry + body_size]).hexdigest() != body_hash:
        raise AssertionError(f"{module}:{entry:#x}: recovered body changed")
    remap_hash = "6af1ae4b4333d445ca410bb39b95986483a032741a6928c7ce6a8b776ffd56e9"
    if hashlib.sha256(image[remap_offset : remap_offset + 256]).hexdigest() != remap_hash:
        raise AssertionError(f"{module}:{entry:#x}: palette remap changed")

    data_segment = 0x5000
    palette_segment = 0x7000
    extra_segment = 0x9000
    fs_segment = 0xB000
    game_segment = 0xC000
    stack_segment = 0xE000
    context = 0x3000
    root = 0x4000
    state = root + 0x005E
    return_address = 0xF000
    stack_sentinel = bytes.fromhex("a55a69967887")
    mask32 = 0xFFFFFFFF
    cases = (
        ("phase_above_range", 0x0081, 0x0020, 1, 2, 0x0000),
        ("next_phase_negative", 0x007F, 0x0070, 1, 2, 0x0001),
        ("stationary_phase", 0x0040, 0x0040, 0, 2, 0x0002),
        ("high_palette_forward", 0x003C, 0x0038, -4, 2, 0x0003),
        ("low_palette_forward", 0x0064, 0x0060, -4, 1, 0x0004),
        ("cross_palette_ranges", 0x0050, 0x003C, -2, 3, 0x0005),
        ("swapped_palette_ranges", 0x003C, 0x0050, 2, 4, 0x0100),
        ("countdown_flip", 0x0060, 0x0064, -3, 0, 0xFFFF),
    )
    mouse_cases = (
        (0, 0),
        (1, -1),
        (-1, 1),
        (0x7FFF, -0x8000),
        (-0x8000, 0x7FFF),
        (0x1234, -0x2345),
        (-0x3456, 0x4567),
        (0x5A5A, -0x6B6B),
    )
    flag_masks = {
        "cf": 0x0001,
        "pf": 0x0004,
        "af": 0x0010,
        "zf": 0x0040,
        "sf": 0x0080,
        "if": 0x0200,
        "df": 0x0400,
        "of": 0x0800,
    }
    vectors: list[dict[str, object]] = []

    def put_u16(memory: bytearray, offset: int, value: int) -> None:
        struct.pack_into("<H", memory, offset, value & 0xFFFF)

    def put_u32(memory: bytearray, offset: int, value: int) -> None:
        struct.pack_into("<I", memory, offset, value & mask32)

    def get_u16(memory: bytearray, offset: int) -> int:
        return struct.unpack_from("<H", memory, offset)[0]

    def get_u32(memory: bytearray, offset: int) -> int:
        return struct.unpack_from("<I", memory, offset)[0]

    def signed_word(value: int) -> int:
        value &= 0xFFFF
        return value if value < 0x8000 else value - 0x10000

    def signed_dword(value: int) -> int:
        value &= mask32
        return value if value < 0x80000000 else value - 0x100000000

    def with_low_word(value: int, low: int) -> int:
        return (value & 0xFFFF0000) | (low & 0xFFFF)

    def sub_flags_16(left: int, right: int, initial_flags: int) -> dict[str, bool]:
        left &= 0xFFFF
        right &= 0xFFFF
        result = (left - right) & 0xFFFF
        return {
            "cf": left < right,
            "pf": (result & 0xFF).bit_count() % 2 == 0,
            "af": (left & 0xF) < (right & 0xF),
            "zf": result == 0,
            "sf": bool(result & 0x8000),
            "if": bool(initial_flags & 0x0200),
            "df": bool(initial_flags & 0x0400),
            "of": bool(((left ^ right) & (left ^ result) & 0x8000)),
        }

    def add_flags_8(left: int, right: int, initial_flags: int) -> dict[str, bool]:
        left &= 0xFF
        right &= 0xFF
        total = left + right
        result = total & 0xFF
        return {
            "cf": total > 0xFF,
            "pf": result.bit_count() % 2 == 0,
            "af": ((left & 0xF) + (right & 0xF)) > 0xF,
            "zf": result == 0,
            "sf": bool(result & 0x80),
            "if": bool(initial_flags & 0x0200),
            "df": bool(initial_flags & 0x0400),
            "of": bool((~(left ^ right) & (left ^ result) & 0x80)),
        }

    def remap_pages(
        palette: bytearray,
        remap: bytes,
        first_page: int,
        last_page: int,
        first_byte: int,
        last_byte: int,
    ) -> int:
        last_word = 0
        for page in range(first_page, last_page):
            base = (page << 8) & 0xFFFF
            for offset in range(first_byte, last_byte, 2):
                low_offset = (base + offset) & 0xFFFF
                high_offset = (low_offset + 1) & 0xFFFF
                low = remap[palette[low_offset]]
                high = remap[palette[high_offset]]
                palette[low_offset] = low
                palette[high_offset] = high
                last_word = low | (high << 8)
        return last_word

    for case_index, case in enumerate(cases):
        name, level, previous, step, countdown, code_flags = case
        mouse_x, mouse_y = mouse_cases[case_index]
        data_before = bytearray(
            (offset * 29 + case_index * 31 + 11) & 0xFF
            for offset in range(0x10000)
        )
        palette_before = bytearray(
            (offset * 37 + case_index * 43 + 17) & 0xFF
            for offset in range(0x10000)
        )
        put_u16(data_before, 0x0004, palette_segment)
        put_u16(data_before, 0x002A, mouse_x)
        put_u16(data_before, 0x002C, mouse_y)
        put_u16(data_before, context + 0x16, root)
        put_u32(data_before, state + 0x36, 0x70000004 + case_index * 0x11111111)
        put_u32(data_before, state + 0x3A, 0x90000004 - case_index * 0x01020304)
        put_u32(data_before, state + 0x3E, 0x80000100 + case_index * 0x1234567)
        put_u32(data_before, state + 0x42, 0x7FFFFFFC - case_index * 0x10203)
        put_u32(data_before, state + 0x46, 0x80000004 + case_index * 0x20304)

        code_before = bytearray(image)
        put_u16(code_before, 0x0099, level)
        put_u16(code_before, 0x009B, previous)
        control = ((countdown & 0xFF) << 8) | (step & 0xFF)
        put_u16(code_before, 0x009F, control)
        put_u16(code_before, 0x02FC, code_flags)
        code_expected = bytearray(code_before)
        data_expected = bytearray(data_before)
        palette_expected = bytearray(palette_before)

        put_u32(data_expected, root + 0x36, 0)
        put_u32(data_expected, root + 0x3A, 0)
        put_u32(data_expected, root + 0x12, 0x00008000)
        put_u32(data_expected, root + 0x22, 0x00008000)
        put_u32(data_expected, root + 0x32, 0x00008000)
        put_u16(data_expected, state, root)

        scaled = signed_dword(get_u32(data_expected, state + 0x3E)) >> 8
        delta_y = (-60 * scaled) & mask32
        delta_x = (scaled * (mouse_x & mask32)) & mask32
        delta_x = (signed_dword(delta_x) >> 2) & mask32
        delta_x = (delta_x - get_u32(data_expected, state + 0x36)) & mask32
        delta_y = (delta_y - get_u32(data_expected, state + 0x3A)) & mask32
        delta_x = (signed_dword(delta_x) >> 16) & mask32
        delta_y = (signed_dword(delta_y) >> 16) & mask32
        put_u16(data_expected, state + 0x52, (mouse_x << 2) & 0xFFFF)
        put_u16(data_expected, state + 0x50, (mouse_x << 2) & 0xFFFF)
        put_u16(data_expected, state + 0x4E, -mouse_y)
        put_u32(
            data_expected,
            state + 0x42,
            get_u32(data_expected, state + 0x42) + delta_x,
        )
        put_u32(
            data_expected,
            state + 0x46,
            get_u32(data_expected, state + 0x46) + delta_y,
        )

        initial_flags = 0x0293 | (0x0400 if case_index & 1 else 0)
        initial = {
            "eax": 0xA1A12345 + case_index,
            "ebx": 0xB2B23456 + case_index,
            "ecx": 0xC3C34567 + case_index,
            "edx": 0xD4D45678 + case_index,
            "esi": 0xE5E56789 + case_index,
            "edi": 0xF6F60000 | context,
            "ebp": 0x979789AB + case_index,
            "sp": 0xFF00,
            "ds": data_segment,
            "es": extra_segment,
            "fs": fs_segment,
            "gs": game_segment,
            "ss": stack_segment,
            "flags": initial_flags,
        }

        eax = mouse_x & mask32
        eax = with_low_word(eax, mouse_x << 2)
        ebx = (-signed_word(mouse_y)) & mask32
        ecx = delta_x
        edx = delta_y
        esi = with_low_word(initial["esi"], state)
        es_after = extra_segment

        if has_pulse:
            ecx = with_low_word(ecx, code_flags)
            if code_flags != 0:
                pulse = (code_flags - 1) & 0xFFFF
                put_u16(code_expected, 0x02FC, pulse)
                shift = pulse & 3
                ecx = with_low_word(ecx, shift)
                eax = with_low_word(eax, 10 << shift)
                ebx = with_low_word(ebx, 13 << shift)
                edx = with_low_word(edx, 11 << shift)
                put_u16(data_expected, 0x2536, 10 << shift)
                put_u16(data_expected, 0x2594, 13 << shift)
                put_u16(data_expected, 0x25F2, 11 << shift)

        eax = with_low_word(eax, level)
        expected_flags = sub_flags_16(level, 0x0080, initial_flags)
        palette_path = "phase_above_range"
        if level <= 0x0080:
            lower = (0x0080 - level) & 0xFFFF
            upper = (0x0080 - previous) & 0xFFFF
            esi = with_low_word(esi, lower)
            edx = with_low_word(edx, upper)
            put_u16(code_expected, 0x009B, level)
            ebx = with_low_word(ebx, control)
            next_level = ((level & 0xFF) + (step & 0xFF)) & 0xFF
            eax = (eax & 0xFFFFFF00) | next_level
            expected_flags = add_flags_8(level, step, initial_flags)
            palette_path = "next_phase_negative"
            if (next_level & 0x80) == 0:
                next_countdown = (countdown - 1) & 0xFF
                next_step = step & 0xFF
                if next_countdown & 0x80:
                    next_countdown = 3
                    next_step = (-next_step) & 0xFF
                next_control = (next_countdown << 8) | next_step
                ebx = with_low_word(ebx, next_control)
                put_u16(code_expected, 0x009F, next_control)
                put_u16(code_expected, 0x0099, next_level)
                expected_flags = sub_flags_16(lower, upper, initial_flags)
                palette_path = "stationary"
                if lower != upper:
                    if signed_word(lower) > signed_word(upper):
                        lower, upper = upper, lower
                    esi = with_low_word(esi, lower)
                    edx = with_low_word(edx, upper)
                    eax = with_low_word(eax, palette_segment)
                    ebx = with_low_word(ebx, remap_offset)
                    es_after = palette_segment
                    remap = bytes(code_expected[remap_offset : remap_offset + 256])
                    high_lower = max(lower - 0x003F, 0)
                    high_upper = max(upper - 0x003F, 0)
                    last_word = 0
                    if high_lower != high_upper:
                        last_word = remap_pages(
                            palette_expected,
                            remap,
                            high_lower,
                            high_upper,
                            0x1E,
                            0x100,
                        )
                        ecx = with_low_word(ecx, 0x0071)
                    low_lower = min(lower, 0x003F)
                    low_upper = min(upper, 0x003F)
                    esi = with_low_word(esi, low_lower)
                    edx = with_low_word(edx, low_upper - low_lower)
                    if low_lower != low_upper:
                        last_word = remap_pages(
                            palette_expected,
                            remap,
                            low_lower,
                            low_upper,
                            0,
                            0x1E,
                        )
                        esi = with_low_word(esi, low_upper << 8)
                        edx = with_low_word(edx, 0)
                        ecx = with_low_word(ecx, 0x000F)
                        palette_path = "high_and_low" if high_lower != high_upper else "low"
                    else:
                        edx = with_low_word(edx, 0)
                        palette_path = "high"
                    eax = with_low_word(eax, last_word)
                    expected_flags = sub_flags_16(1, 1, initial_flags)

        extra_before = bytes(
            (offset * 13 + case_index + 3) & 0xFF for offset in range(0x10000)
        )
        fs_before = bytes(
            (offset * 17 + case_index + 5) & 0xFF for offset in range(0x10000)
        )
        game_before = bytes(
            (offset * 23 + case_index + 9) & 0xFF for offset in range(0x10000)
        )
        machine = execute(
            bytes(code_before),
            entry,
            return_address,
            initial,
            [
                (data_segment, 0, bytes(data_before)),
                (palette_segment, 0, bytes(palette_before)),
                (extra_segment, 0, extra_before),
                (fs_segment, 0, fs_before),
                (game_segment, 0, game_before),
                (
                    stack_segment,
                    0xFF00,
                    struct.pack("<H", return_address) + stack_sentinel,
                ),
            ],
            max_instructions=100000,
        )
        actual_data = bytes(machine.mem_read(data_segment * 16, 0x10000))
        if actual_data != bytes(data_expected):
            differences = [
                (offset, actual_data[offset], data_expected[offset])
                for offset in range(0x10000)
                if actual_data[offset] != data_expected[offset]
            ][:8]
            raise AssertionError(
                f"{module}:{entry:#x} {name}: data differs at {differences}"
            )
        actual_code = bytes(machine.mem_read(0, len(image)))
        if actual_code != bytes(code_expected):
            differences = [
                (offset, actual_code[offset], code_expected[offset])
                for offset in range(len(image))
                if actual_code[offset] != code_expected[offset]
            ][:8]
            raise AssertionError(
                f"{module}:{entry:#x} {name}: code differs at {differences}"
            )
        actual_palette = bytes(machine.mem_read(palette_segment * 16, 0x10000))
        if actual_palette != bytes(palette_expected):
            differences = [
                (offset, actual_palette[offset], palette_expected[offset])
                for offset in range(0x10000)
                if actual_palette[offset] != palette_expected[offset]
            ][:8]
            raise AssertionError(
                f"{module}:{entry:#x} {name}: palette differs at {differences}"
            )
        for segment, expected in (
            (extra_segment, extra_before),
            (fs_segment, fs_before),
            (game_segment, game_before),
        ):
            if bytes(machine.mem_read(segment * 16, 0x10000)) != expected:
                raise AssertionError(
                    f"{module}:{entry:#x} {name}: segment {segment:#x} changed"
                )

        expected_registers = dict(initial)
        del expected_registers["flags"]
        expected_registers.update(
            {
                "eax": eax,
                "ebx": ebx,
                "ecx": ecx,
                "edx": edx,
                "esi": esi,
                "es": es_after,
                "sp": 0xFF02,
            }
        )
        for register, expected in expected_registers.items():
            actual = machine.reg_read(REGISTERS[register])
            if actual != expected:
                raise AssertionError(
                    f"{module}:{entry:#x} {name}: "
                    f"{register}={actual:#x}, expected={expected:#x}"
                )
        flags_after = machine.reg_read(UC_X86_REG_EFLAGS)
        actual_flags = {
            flag: bool(flags_after & mask) for flag, mask in flag_masks.items()
        }
        if actual_flags != expected_flags:
            raise AssertionError(
                f"{module}:{entry:#x} {name}: "
                f"flags={actual_flags}, expected={expected_flags}"
            )
        if bytes(machine.mem_read(stack_segment * 16 + 0xFF02, 6)) != stack_sentinel:
            raise AssertionError(f"{module}:{entry:#x} {name}: stack changed")

        changed_palette_bytes = sum(
            actual != before
            for actual, before in zip(palette_expected, palette_before)
        )
        vectors.append(
            {
                "name": name,
                "module": module,
                "entry": entry,
                "path": palette_path,
                "mouse": [signed_word(mouse_x), signed_word(mouse_y)],
                "position_after": [
                    signed_dword(get_u32(data_expected, state + offset))
                    for offset in (0x42, 0x46)
                ],
                "level_before": level,
                "level_after": get_u16(code_expected, 0x0099),
                "previous_after": get_u16(code_expected, 0x009B),
                "control_after": get_u16(code_expected, 0x009F),
                "pulse_after": get_u16(code_expected, 0x02FC),
                "changed_palette_bytes": changed_palette_bytes,
                "data_sha256": hashlib.sha256(data_expected).hexdigest(),
                "palette_sha256": hashlib.sha256(palette_expected).hexdigest(),
                "defined_flags": expected_flags,
            }
        )

    return vectors


def wrap_position(value: int, origin: int) -> tuple[int, int]:
    relative = (value + origin) & 0xFFFF
    windowed = (((relative + 0x4000) & 0xFFFF) & 0x7FFF)
    windowed = (windowed - 0x4000) & 0xFFFF
    return (windowed - origin) & 0xFFFF, windowed


def wrap_positions_zero_count_vector(
    image: bytes, module: str, entry: int
) -> dict[str, object]:
    data_segment = 0x4200
    extra_segment = 0x6000
    game_segment = 0x7000
    stack_segment = 0x9000
    context_offset = 0x3000
    state_base = 0x2000
    return_address = 0xF000
    initial_flags = 0x0693
    data_before = bytearray(
        ((offset * 37 + 11) & 0xFF) for offset in range(0x10002)
    )
    struct.pack_into("<H", data_before, context_offset + 0x16, state_base)
    struct.pack_into("<H", data_before, context_offset + 0x1A, 0)
    for offset, origin in zip(
        (0x22EC, 0x22F0, 0x22F4), (0x1357, 0x8001, 0xFEDC)
    ):
        struct.pack_into("<H", data_before, offset, origin)
    data_expected = bytearray(data_before)

    state_cursor = state_base
    final_words = [0, 0, 0]
    final_windowed_z = 0
    final_origin_z = 0
    for _ in range(0x10000):
        state_cursor = (state_cursor + 0x005E) & 0xFFFF
        origins = [
            struct.unpack_from("<H", data_expected, offset)[0]
            for offset in (0x22EC, 0x22F0, 0x22F4)
        ]
        coordinate_offsets = [
            (state_cursor + field_offset) & 0xFFFF
            for field_offset in (0x42, 0x46, 0x4A)
        ]
        positions = [
            struct.unpack_from("<H", data_expected, offset)[0]
            for offset in coordinate_offsets
        ]
        wrapped = [
            wrap_position(value, origin)
            for value, origin in zip(positions, origins)
        ]
        final_words = [result for result, _windowed in wrapped]
        final_windowed_z = wrapped[2][1]
        final_origin_z = origins[2]
        for offset, value in zip(coordinate_offsets, final_words):
            signed_dword = value | (0xFFFF0000 if value & 0x8000 else 0)
            struct.pack_into("<I", data_expected, offset, signed_dword)

    extra_before = bytes((offset * 13 + 5) & 0xFF for offset in range(0x10000))
    game_before = bytes((offset * 7 + 9) & 0xFF for offset in range(0x10000))
    initial = {
        "eax": 0xA1A1BEEF,
        "ebx": 0xB2B22345,
        "ecx": 0xC3C30000,
        "edx": 0xD4D44567,
        "esi": 0xE5E55678,
        "edi": 0xF6F60000 | context_offset,
        "ebp": 0x9797789A,
        "sp": 0xFF00,
        "ds": data_segment,
        "es": extra_segment,
        "fs": 0x8000,
        "gs": game_segment,
        "ss": stack_segment,
        "flags": initial_flags,
    }
    stack_sentinel = bytes.fromhex("5aa596698778")
    machine = execute(
        image,
        entry,
        return_address,
        initial,
        [
            (0, return_address, b"\xcc"),
            (data_segment, 0, bytes(data_before)),
            (extra_segment, 0, extra_before),
            (game_segment, 0, game_before),
            (
                stack_segment,
                0xFF00,
                struct.pack("<H", return_address) + stack_sentinel,
            ),
        ],
        max_instructions=2000000,
    )

    actual_data = bytes(machine.mem_read(data_segment * 16, 0x10002))
    if actual_data != bytes(data_expected):
        differences = [
            (offset, actual_data[offset], data_expected[offset])
            for offset in range(0x10002)
            if actual_data[offset] != data_expected[offset]
        ][:8]
        raise AssertionError(
            f"{module}:{entry:#x} zero_count: data differs at {differences}"
        )
    for segment, expected in (
        (extra_segment, extra_before),
        (game_segment, game_before),
    ):
        actual = bytes(machine.mem_read(segment * 16, 0x10000))
        if actual != expected:
            raise AssertionError(
                f"{module}:{entry:#x} zero_count: decoy segment {segment:#x} changed"
            )

    expected_registers = dict(initial)
    del expected_registers["flags"]
    expected_registers["eax"] = final_words[0] | (
        0xFFFF0000 if final_words[0] & 0x8000 else 0
    )
    expected_registers["ebx"] = final_words[1] | (
        0xFFFF0000 if final_words[1] & 0x8000 else 0
    )
    expected_registers["ecx"] &= 0xFFFF0000
    expected_registers["edx"] = final_words[2] | (
        0xFFFF0000 if final_words[2] & 0x8000 else 0
    )
    expected_registers["esi"] = (initial["esi"] & 0xFFFF0000) | state_base
    expected_registers["edi"] = (initial["edi"] & 0xFFFF0000) | 0x4000
    expected_registers["ebp"] = (initial["ebp"] & 0xFFFF0000) | 0x7FFF
    expected_registers["sp"] = 0xFF02
    for register, expected in expected_registers.items():
        actual = machine.reg_read(REGISTERS[register])
        if actual != expected:
            raise AssertionError(
                f"{module}:{entry:#x} zero_count: "
                f"{register}={actual:#x}, expected={expected:#x}"
            )
    if machine.reg_read(UC_X86_REG_CS) != 0:
        raise AssertionError(f"{module}:{entry:#x} zero_count: near return CS changed")
    if bytes(machine.mem_read(stack_segment * 16 + 0xFF02, 6)) != stack_sentinel:
        raise AssertionError(f"{module}:{entry:#x} zero_count: stack sentinel changed")

    expected_flags = sub_flags_16(
        final_windowed_z, final_origin_z, initial_flags
    )
    flag_masks = {
        "cf": 0x0001,
        "pf": 0x0004,
        "af": 0x0010,
        "zf": 0x0040,
        "sf": 0x0080,
        "if": 0x0200,
        "df": 0x0400,
        "of": 0x0800,
    }
    flags_after = machine.reg_read(UC_X86_REG_EFLAGS)
    actual_flags = {
        flag: bool(flags_after & mask) for flag, mask in flag_masks.items()
    }
    if actual_flags != expected_flags:
        raise AssertionError(
            f"{module}:{entry:#x} zero_count: "
            f"flags={actual_flags}, expected={expected_flags}"
        )

    return {
        "name": "zero_count",
        "module": module,
        "entry": entry,
        "context_offset": context_offset,
        "state_base": state_base,
        "state_count": 0,
        "effective_iterations": 0x10000,
        "final_state_offset": state_cursor,
        "final_output_words": final_words,
        "checked_bytes_from_ds_base": 0x10002,
        "data_segment_sha256": hashlib.sha256(data_expected).hexdigest(),
        "defined_flags": expected_flags,
    }


def wrap_positions_vectors(
    module: str, entry: int
) -> list[dict[str, object]]:
    image = load_image(module)
    expected_hash = "440aacc88cf7b7f15e7fd49e6827fa3b922943ad509d3437e5f71510515f1928"
    if hashlib.sha256(image[entry : entry + 92]).hexdigest() != expected_hash:
        raise AssertionError(f"{module}:{entry:#x}: recovered 92-byte body changed")

    cases = [
        (
            "ordinary",
            0x4000,
            (0x0000, 0x0000, 0x0000),
            [(0x12345678, 0x89ABCDEF, 0x00000000)],
        ),
        (
            "positive_boundary",
            0x4200,
            (0x0000, 0x0000, 0x0000),
            [(0xAAAA3FFF, 0xBBBB4000, 0xCCCC7FFF)],
        ),
        (
            "negative_boundary",
            0x4400,
            (0x0000, 0x0000, 0x0000),
            [(0xDDDDC000, 0xEEEEBFFF, 0xFFFF8000)],
        ),
        (
            "large_origins",
            0x4600,
            (0x7FFF, 0x8000, 0xFFFF),
            [(0x11118001, 0x22227FFF, 0x3333C001)],
        ),
        (
            "high_words_replaced",
            0x4800,
            (0x1357, 0x2468, 0xA55A),
            [(0x12348000, 0x56783FFF, 0x9ABCC000)],
        ),
        (
            "three_states",
            0x4A00,
            (0x0102, 0x7F00, 0x8001),
            [
                (0x11110001, 0x2222FFFE, 0x33334000),
                (0x44443FFE, 0x55558001, 0x6666C001),
                (0x77747FFF, 0x88888000, 0x9999FFFF),
            ],
        ),
        (
            "state_pointer_wrap",
            0xFFA0,
            (0x4000, 0xC000, 0x1234),
            [
                (0xAAAA0000, 0xBBBB3FFF, 0xCCCC4000),
                (0xDDDD7FFF, 0xEEEE8000, 0xFFFFC000),
            ],
        ),
    ]
    data_segment = 0x4000
    extra_segment = 0x6000
    game_segment = 0x7000
    stack_segment = 0x9000
    context_offset = 0x3000
    return_address = 0xF000
    vectors = []

    for case_index, (name, state_base, origins, positions) in enumerate(cases):
        data_before = bytearray(
            ((offset * 29 + case_index * 17) & 0xFF)
            for offset in range(0x10000)
        )
        data_expected = bytearray(data_before)
        struct.pack_into("<H", data_before, context_offset + 0x16, state_base)
        struct.pack_into("<H", data_before, context_offset + 0x1A, len(positions))
        for offset, origin in zip((0x22EC, 0x22F0, 0x22F4), origins):
            struct.pack_into("<H", data_before, offset, origin)
        data_expected[:] = data_before

        state_cursor = state_base
        state_results = []
        last_windowed_z = 0
        for position_x, position_y, position_z in positions:
            state_cursor = (state_cursor + 0x005E) & 0xFFFF
            offsets = tuple(
                (state_cursor + field_offset) & 0xFFFF
                for field_offset in (0x42, 0x46, 0x4A)
            )
            for offset, value in zip(offsets, (position_x, position_y, position_z)):
                struct.pack_into("<I", data_before, offset, value)
                struct.pack_into("<I", data_expected, offset, value)

            wrapped = []
            windowed = []
            for value, origin in zip(
                (position_x, position_y, position_z), origins
            ):
                result, value_before_origin = wrap_position(value & 0xFFFF, origin)
                wrapped.append(result)
                windowed.append(value_before_origin)
            for offset, value in zip(offsets, wrapped):
                signed_dword = value | (0xFFFF0000 if value & 0x8000 else 0)
                struct.pack_into("<I", data_expected, offset, signed_dword)

            last_windowed_z = windowed[2]
            state_results.append(
                {
                    "state_offset": state_cursor,
                    "coordinate_offsets": list(offsets),
                    "input_low_words": [
                        position_x & 0xFFFF,
                        position_y & 0xFFFF,
                        position_z & 0xFFFF,
                    ],
                    "output_words": wrapped,
                    "output_dwords": [
                        value | (0xFFFF0000 if value & 0x8000 else 0)
                        for value in wrapped
                    ],
                }
            )

        extra_before = bytes(
            ((offset * 13 + case_index * 23 + 1) & 0xFF)
            for offset in range(0x10000)
        )
        game_before = bytes(
            ((offset * 7 + case_index * 31 + 3) & 0xFF)
            for offset in range(0x10000)
        )
        initial_flags = 0x0293 | (0x0400 if case_index & 1 else 0)
        initial = {
            "eax": 0xA1A1BEEF + case_index,
            "ebx": 0xB2B22345 + case_index,
            "ecx": 0xC3C33456 + case_index,
            "edx": 0xD4D44567 + case_index,
            "esi": 0xE5E55678 + case_index,
            "edi": 0xF6F60000 | context_offset,
            "ebp": 0x9797789A + case_index,
            "sp": 0xFF00,
            "ds": data_segment,
            "es": extra_segment,
            "fs": 0x8000,
            "gs": game_segment,
            "ss": stack_segment,
            "flags": initial_flags,
        }
        stack_sentinel = bytes.fromhex("5aa596698778")
        machine = execute(
            image,
            entry,
            return_address,
            initial,
            [
                (0, return_address, b"\xcc"),
                (data_segment, 0, bytes(data_before)),
                (extra_segment, 0, extra_before),
                (game_segment, 0, game_before),
                (
                    stack_segment,
                    0xFF00,
                    struct.pack("<H", return_address) + stack_sentinel,
                ),
            ],
            max_instructions=1000,
        )

        actual_data = bytes(machine.mem_read(data_segment * 16, 0x10000))
        if actual_data != bytes(data_expected):
            differences = [
                (offset, actual_data[offset], data_expected[offset])
                for offset in range(0x10000)
                if actual_data[offset] != data_expected[offset]
            ][:8]
            raise AssertionError(
                f"{module}:{entry:#x} {name}: data differs at {differences}"
            )
        for segment, expected in (
            (extra_segment, extra_before),
            (game_segment, game_before),
        ):
            actual = bytes(machine.mem_read(segment * 16, 0x10000))
            if actual != expected:
                raise AssertionError(
                    f"{module}:{entry:#x} {name}: decoy segment {segment:#x} changed"
                )

        final_words = state_results[-1]["output_words"]
        assert isinstance(final_words, list)
        expected_registers = dict(initial)
        del expected_registers["flags"]
        expected_registers["eax"] = final_words[0] | (
            0xFFFF0000 if final_words[0] & 0x8000 else 0
        )
        expected_registers["ebx"] = final_words[1] | (
            0xFFFF0000 if final_words[1] & 0x8000 else 0
        )
        expected_registers["ecx"] &= 0xFFFF0000
        expected_registers["edx"] = final_words[2] | (
            0xFFFF0000 if final_words[2] & 0x8000 else 0
        )
        expected_registers["esi"] = (
            initial["esi"] & 0xFFFF0000
        ) | state_cursor
        expected_registers["edi"] = (
            initial["edi"] & 0xFFFF0000
        ) | 0x4000
        expected_registers["ebp"] = (
            initial["ebp"] & 0xFFFF0000
        ) | 0x7FFF
        expected_registers["sp"] = 0xFF02
        for register, expected in expected_registers.items():
            actual = machine.reg_read(REGISTERS[register])
            if actual != expected:
                raise AssertionError(
                    f"{module}:{entry:#x} {name}: "
                    f"{register}={actual:#x}, expected={expected:#x}"
                )
        if machine.reg_read(UC_X86_REG_CS) != 0:
            raise AssertionError(f"{module}:{entry:#x} {name}: near return CS changed")
        if bytes(machine.mem_read(stack_segment * 16 + 0xFF02, 6)) != stack_sentinel:
            raise AssertionError(f"{module}:{entry:#x} {name}: stack sentinel changed")

        expected_flags = sub_flags_16(last_windowed_z, origins[2], initial_flags)
        flag_masks = {
            "cf": 0x0001,
            "pf": 0x0004,
            "af": 0x0010,
            "zf": 0x0040,
            "sf": 0x0080,
            "if": 0x0200,
            "df": 0x0400,
            "of": 0x0800,
        }
        flags_after = machine.reg_read(UC_X86_REG_EFLAGS)
        actual_flags = {
            flag: bool(flags_after & mask) for flag, mask in flag_masks.items()
        }
        if actual_flags != expected_flags:
            raise AssertionError(
                f"{module}:{entry:#x} {name}: "
                f"flags={actual_flags}, expected={expected_flags}"
            )

        vectors.append(
            {
                "name": name,
                "module": module,
                "entry": entry,
                "context_offset": context_offset,
                "state_base": state_base,
                "state_count": len(positions),
                "view_origin_words": list(origins),
                "states": state_results,
                "final_state_offset": state_cursor,
                "data_segment_sha256": hashlib.sha256(data_expected).hexdigest(),
                "defined_flags": expected_flags,
            }
        )

    vectors.append(wrap_positions_zero_count_vector(image, module, entry))
    return vectors


def sample_delta_vectors(
    module: str, entry: int, scaled: bool
) -> list[dict[str, object]]:
    image = load_image(module)
    body_size = 51 if scaled else 48
    expected_hash = (
        "adbd3507c4073fd2f2e866dc269fcb1885d1873f896c7d1861824f46492dfafe"
        if scaled
        else "946199cf60611843e6ebdcba55201fae95864e43b62c23206d65165c5169174c"
    )
    if hashlib.sha256(image[entry : entry + body_size]).hexdigest() != expected_hash:
        raise AssertionError(
            f"{module}:{entry:#x}: recovered {body_size}-byte body changed"
        )

    cases = [
        ("one_positive", 0x0000, 0x1234, 0x1200, 1, 0x0200),
        ("three_negative", 0x0004, 0xEDCC, 0x0100, 3, 0x1000),
        ("cursor_wrap", 0x0FFC, 0x7FFF, 0xFFFE, 2, 0x2000),
        ("delta_wrap", 0x0800, 0x8000, 0x7FFF, 4, 0x3000),
        ("object_wrap", 0x0040, 0x4000, 0xC000, 3, 0xFFF0),
        ("odd_cursor", 0xFFFF, 0xA55A, 0x5AA5, 2, 0x4102),
        ("zero_count", 0x000C, 0x0010, 0x0001, 0, 0x0000),
    ]
    game_segment = 0x2000
    data_segment = 0x4000
    object_segment = 0x6000
    extra_segment = 0x8000
    stack_segment = 0xA000
    context_offset = 0x3000
    return_address = 0xF000
    vectors = []

    for case_index, (
        name,
        cursor,
        raw_sample,
        previous,
        count,
        object_offset,
    ) in enumerate(cases):
        data_before = bytearray(0x10000)
        struct.pack_into("<H", data_before, 0x0002, object_segment)
        struct.pack_into("<H", data_before, context_offset + 0x001C, object_offset)
        struct.pack_into("<H", data_before, context_offset + 0x001E, 0xDEAD)
        struct.pack_into("<H", data_before, context_offset + 0x0020, count)
        struct.pack_into("<H", data_before, context_offset + 0x0038, cursor)
        struct.pack_into("<H", data_before, context_offset + 0x003A, previous)
        sample_offset = (cursor + 0x0036) & 0xFFFF
        struct.pack_into("<H", data_before, sample_offset, raw_sample)
        data_expected = bytearray(data_before)

        signed_sample = raw_sample if raw_sample < 0x8000 else raw_sample - 0x10000
        current = signed_sample >> 4 if scaled else signed_sample
        current_word = current & 0xFFFF
        delta = (current_word - previous) & 0xFFFF
        next_cursor = (cursor + 4) & 0x0FFC
        struct.pack_into(
            "<H", data_expected, context_offset + 0x0038, next_cursor
        )
        struct.pack_into(
            "<H", data_expected, context_offset + 0x003A, current_word
        )

        object_before = bytearray(
            ((index * 37 + case_index * 11) & 0xFF) for index in range(0x10000)
        )
        object_expected = bytearray(object_before)
        iterations = count if count != 0 else 0x10000
        object_cursor = object_offset
        changed_offsets = []
        for iteration in range(iterations):
            old_value = struct.unpack_from("<H", object_expected, object_cursor)[0]
            struct.pack_into(
                "<H", object_expected, object_cursor, (old_value + delta) & 0xFFFF
            )
            if iteration < 4:
                changed_offsets.append(object_cursor)
            object_cursor = (object_cursor + 0x14) & 0xFFFF

        initial_flags = 0x0A93 | (0x0400 if case_index & 1 else 0)
        initial = {
            "eax": 0xA1A10000 | ((0xBEEF + case_index) & 0xFFFF),
            "ebx": 0xB2B20000 | ((0x2345 + case_index) & 0xFFFF),
            "ecx": 0xC3C30000 | ((0x3456 + case_index) & 0xFFFF),
            "edx": 0xD4D40000 | ((0x4567 + case_index) & 0xFFFF),
            "esi": 0xE5E50000 | ((0x5678 + case_index) & 0xFFFF),
            "edi": 0xF6F60000 | context_offset,
            "ebp": 0x97970000 | ((0x789A + case_index) & 0xFFFF),
            "sp": 0xFF00,
            "ds": data_segment,
            "es": extra_segment,
            "fs": data_segment,
            "gs": game_segment,
            "ss": stack_segment,
            "flags": initial_flags,
        }
        stack_sentinel = bytes.fromhex("5aa596698778")
        decoy = bytes.fromhex("112233445566778899aabbccddeeff00")
        machine = execute(
            image,
            entry,
            return_address,
            initial,
            [
                (0, return_address, b"\xcc"),
                (data_segment, 0, bytes(data_before)),
                (object_segment, 0, bytes(object_before)),
                (extra_segment, context_offset, decoy),
                (game_segment, context_offset, decoy),
                (
                    stack_segment,
                    0xFF00,
                    struct.pack("<H", return_address) + stack_sentinel,
                ),
            ],
            max_instructions=300000 if count == 0 else 1000,
        )

        actual_data = bytes(machine.mem_read(data_segment * 16, 0x10000))
        if actual_data != bytes(data_expected):
            differences = [
                (offset, actual_data[offset], data_expected[offset])
                for offset in range(0x10000)
                if actual_data[offset] != data_expected[offset]
            ][:8]
            raise AssertionError(
                f"{module}:{entry:#x} {name}: data segment differs at {differences}"
            )
        actual_objects = bytes(machine.mem_read(object_segment * 16, 0x10000))
        if actual_objects != bytes(object_expected):
            raise AssertionError(f"{module}:{entry:#x} {name}: object segment differs")
        for segment in (extra_segment, game_segment):
            if bytes(machine.mem_read(segment * 16 + context_offset, len(decoy))) != decoy:
                raise AssertionError(
                    f"{module}:{entry:#x} {name}: decoy segment {segment:#x} changed"
                )

        expected_registers = dict(initial)
        del expected_registers["flags"]
        expected_registers["eax"] = (initial["eax"] & 0xFFFF0000) | delta
        expected_registers["ebx"] = (initial["ebx"] & 0xFFFF0000) | previous
        expected_registers["ecx"] &= 0xFFFF0000
        expected_registers["esi"] = (initial["esi"] & 0xFFFF0000) | object_cursor
        expected_registers["sp"] = 0xFF02
        for register, expected in expected_registers.items():
            actual = machine.reg_read(REGISTERS[register])
            if actual != expected:
                raise AssertionError(
                    f"{module}:{entry:#x} {name}: "
                    f"{register}={actual:#x}, expected={expected:#x}"
                )
        if machine.reg_read(UC_X86_REG_CS) != 0:
            raise AssertionError(f"{module}:{entry:#x} {name}: near return CS changed")
        if bytes(machine.mem_read(stack_segment * 16 + 0xFF02, 6)) != stack_sentinel:
            raise AssertionError(f"{module}:{entry:#x} {name}: stack sentinel changed")

        last_add_left = (object_cursor - 0x14) & 0xFFFF
        expected_flags = add_flags_16(last_add_left, 0x14, initial_flags)
        flag_masks = {
            "cf": 0x0001,
            "pf": 0x0004,
            "af": 0x0010,
            "zf": 0x0040,
            "sf": 0x0080,
            "if": 0x0200,
            "df": 0x0400,
            "of": 0x0800,
        }
        flags_after = machine.reg_read(UC_X86_REG_EFLAGS)
        actual_flags = {
            flag: bool(flags_after & mask) for flag, mask in flag_masks.items()
        }
        if actual_flags != expected_flags:
            raise AssertionError(
                f"{module}:{entry:#x} {name}: "
                f"flags={actual_flags}, expected={expected_flags}"
            )

        vectors.append(
            {
                "name": name,
                "module": module,
                "entry": entry,
                "scaled": scaled,
                "sample_cursor_before": cursor,
                "sample_cursor_after": next_cursor,
                "raw_sample": raw_sample,
                "current_sample": current_word,
                "previous_sample": previous,
                "delta": delta,
                "object_offset": object_offset,
                "object_count": count,
                "effective_iterations": iterations,
                "first_changed_offsets": changed_offsets,
                "object_memory_sha256": hashlib.sha256(object_expected).hexdigest(),
                "register_results": {
                    "ax_delta": delta,
                    "bx_previous": previous,
                    "cx": 0,
                    "si_next_object": object_cursor,
                },
                "defined_flags": expected_flags,
            }
        )

    return vectors


def vga_clear_and_sync_vectors(
    module: str, entry: int
) -> list[dict[str, object]]:
    image = load_image(module)
    expected_hash = "3efa9eb40129f518dfc3dc860ca40a59ddb841eee7db8cada6ea9c15839291df"
    if hashlib.sha256(image[entry : entry + 70]).hexdigest() != expected_hash:
        raise AssertionError(f"{module}:{entry:#x}: recovered 70-byte body changed")

    cases = [
        ("immediate", [0x00, 0x08]),
        ("busy_then_edge", [0x08, 0x08, 0x00, 0x00, 0x08]),
        ("unrelated_bits", [0xF7, 0xFF]),
        ("multiple_phases", [0x18, 0x08, 0x07, 0x00, 0x88]),
    ]
    data_segment = 0x4000
    extra_segment = 0x6000
    game_segment = 0x7000
    stack_segment = 0x9000
    video_segment = 0xA000
    return_address = 0xF000
    expected_outputs = (
        [(0x03C8, 1, 0)]
        + [(0x03C9, 1, 0)] * 0x0300
        + [(0x03D4, 2, 0x000C), (0x03C4, 2, 0x0F02)]
    )
    vectors = []

    for case_index, (name, input_values) in enumerate(cases):
        globals_before = bytes.fromhex("a1b2c3d4e5f6a7b8")
        globals_after = (
            globals_before[:2]
            + struct.pack("<HH", 0x4000, 0xA400)
            + globals_before[6:]
        )
        video_before = bytes(
            ((index * 29 + case_index * 17) & 0xFF)
            for index in range(0x10000)
        )
        video_after = bytes(0xFA00) + video_before[0xFA00:]
        initial_flags = 0x0A93 | (0x0400 if case_index & 1 else 0)
        initial = {
            "eax": 0xA1A1BEEF + case_index,
            "ebx": 0xB2B22345 + case_index,
            "ecx": 0xC3C33456 + case_index,
            "edx": 0xD4D44567 + case_index,
            "esi": 0xE5E55678 + case_index,
            "edi": 0xF6F66789 + case_index,
            "ebp": 0x9797789A + case_index,
            "sp": 0xFF00,
            "ds": data_segment,
            "es": extra_segment,
            "fs": 0x8000,
            "gs": game_segment,
            "ss": stack_segment,
            "flags": initial_flags,
        }
        inputs = []
        outputs = []
        control_snapshots = []
        video_at_first_input = []

        def input_handler(
            _machine: Uc, port: int, size: int, _data: object = None
        ) -> int:
            if port != 0x03DA or size != 1:
                raise AssertionError(
                    f"{module}:{entry:#x} {name}: unexpected IN {port:#x}/{size}"
                )
            if len(inputs) >= len(input_values):
                raise AssertionError(
                    f"{module}:{entry:#x} {name}: extra status read"
                )
            value = input_values[len(inputs)]
            if not inputs:
                video_at_first_input.append(
                    hashlib.sha256(
                        bytes(_machine.mem_read(video_segment * 16, 0x10000))
                    ).hexdigest()
                )
            inputs.append((port, size, value))
            return value

        def output_handler(
            _machine: Uc,
            port: int,
            size: int,
            value: int,
            _data: object = None,
        ) -> None:
            outputs.append((port, size, value))
            if port in (0x03D4, 0x03C4):
                control_snapshots.append(
                    (
                        port,
                        bytes(
                            _machine.mem_read(data_segment * 16 + 0x0024, 8)
                        ),
                        hashlib.sha256(
                            bytes(
                                _machine.mem_read(video_segment * 16, 0x10000)
                            )
                        ).hexdigest(),
                    )
                )

        stack_sentinel = bytes.fromhex("5aa596698778")
        decoy = bytes.fromhex("1122334455667788")
        machine = execute(
            image,
            entry,
            return_address,
            initial,
            [
                (0, return_address, b"\xcc"),
                (data_segment, 0x0024, globals_before),
                (extra_segment, 0x0024, decoy),
                (game_segment, 0x0024, decoy),
                (video_segment, 0, video_before),
                (
                    stack_segment,
                    0xFF00,
                    struct.pack("<H", return_address) + stack_sentinel,
                ),
            ],
            max_instructions=100000,
            input_handler=input_handler,
            output_handler=output_handler,
        )

        if outputs != expected_outputs:
            raise AssertionError(
                f"{module}:{entry:#x} {name}: VGA output sequence differs"
            )
        expected_inputs = [
            (0x03DA, 1, value) for value in input_values
        ]
        if inputs != expected_inputs:
            raise AssertionError(
                f"{module}:{entry:#x} {name}: "
                f"inputs={inputs}, expected={expected_inputs}"
            )
        expected_control_snapshots = [
            (0x03D4, globals_after, hashlib.sha256(video_before).hexdigest()),
            (0x03C4, globals_after, hashlib.sha256(video_before).hexdigest()),
        ]
        if control_snapshots != expected_control_snapshots:
            raise AssertionError(
                f"{module}:{entry:#x} {name}: control-write ordering differs"
            )
        expected_cleared_hash = hashlib.sha256(video_after).hexdigest()
        if video_at_first_input != [expected_cleared_hash]:
            raise AssertionError(
                f"{module}:{entry:#x} {name}: retrace wait preceded framebuffer clear"
            )
        actual_globals = bytes(
            machine.mem_read(data_segment * 16 + 0x0024, 8)
        )
        if actual_globals != globals_after:
            raise AssertionError(
                f"{module}:{entry:#x} {name}: globals={actual_globals.hex()}"
            )
        actual_video = bytes(machine.mem_read(video_segment * 16, 0x10000))
        if actual_video != video_after:
            raise AssertionError(f"{module}:{entry:#x} {name}: VGA memory differs")
        for segment in (extra_segment, game_segment):
            if bytes(machine.mem_read(segment * 16 + 0x0024, 8)) != decoy:
                raise AssertionError(
                    f"{module}:{entry:#x} {name}: decoy segment {segment:#x} changed"
                )

        final_status = input_values[-1]
        expected_registers = dict(initial)
        del expected_registers["flags"]
        expected_registers["eax"] = final_status
        expected_registers["ebx"] = (initial["ebx"] & 0xFFFF0000) | 0xA000
        expected_registers["ecx"] &= 0xFFFF0000
        expected_registers["edx"] = (initial["edx"] & 0xFFFF0000) | 0x03DA
        expected_registers["edi"] = (initial["edi"] & 0xFFFF0000) | 0xFA00
        expected_registers["es"] = video_segment
        expected_registers["sp"] = 0xFF02
        for register, expected in expected_registers.items():
            actual = machine.reg_read(REGISTERS[register])
            if actual != expected:
                raise AssertionError(
                    f"{module}:{entry:#x} {name}: "
                    f"{register}={actual:#x}, expected={expected:#x}"
                )
        if machine.reg_read(UC_X86_REG_CS) != 0:
            raise AssertionError(f"{module}:{entry:#x} {name}: near return CS changed")
        if bytes(machine.mem_read(stack_segment * 16 + 0xFF02, 6)) != stack_sentinel:
            raise AssertionError(f"{module}:{entry:#x} {name}: stack sentinel changed")

        flags_after = machine.reg_read(UC_X86_REG_EFLAGS)
        flag_masks = {
            "cf": 0x0001,
            "pf": 0x0004,
            "zf": 0x0040,
            "sf": 0x0080,
            "if": 0x0200,
            "df": 0x0400,
            "of": 0x0800,
        }
        expected_flags = {
            "cf": False,
            "pf": False,
            "zf": False,
            "sf": False,
            "if": bool(initial_flags & 0x0200),
            "df": False,
            "of": False,
        }
        actual_flags = {
            flag: bool(flags_after & mask) for flag, mask in flag_masks.items()
        }
        if actual_flags != expected_flags:
            raise AssertionError(
                f"{module}:{entry:#x} {name}: "
                f"flags={actual_flags}, expected={expected_flags}"
            )

        vectors.append(
            {
                "name": name,
                "module": module,
                "entry": entry,
                "palette_index_writes": 1,
                "palette_component_zero_writes": 0x0300,
                "crtc_word": {"port": 0x03D4, "value": 0x000C},
                "sequencer_word": {"port": 0x03C4, "value": 0x0F02},
                "video_segment": video_segment,
                "video_bytes_cleared": 0xFA00,
                "video_clear_precedes_status_reads": True,
                "video_tail_sha256": hashlib.sha256(
                    video_after[0xFA00:]
                ).hexdigest(),
                "status_values": input_values,
                "register_results": {
                    "eax": final_status,
                    "bx": 0xA000,
                    "cx": 0,
                    "dx": 0x03DA,
                    "di": 0xFA00,
                    "es": video_segment,
                },
                "defined_flags": expected_flags,
            }
        )

    return vectors


def anchor_state_vectors(
    module: str, entry: int, cursor_offset: int
) -> list[dict[str, object]]:
    image = load_image(module)
    routine_hashes = {
        "amer": "bf622e9b3898598d1a4b96727eea2ded42c454fcd7720fd868f5bdcb219858c5",
        "croolis": "109d245c3c4255132c8885031405d043b675d83028f3e698bf65d345ccba27cb",
        "scrut": "8d3c03afa6218eb2a8d1038774d66e6b1df824f3f51a9a9744d099cf3ce8a5af",
    }
    if hashlib.sha256(image[entry : entry + 16]).hexdigest() != routine_hashes[module]:
        raise AssertionError(f"{module}:{entry:#x}: recovered 16-byte body changed")

    cases = [
        ("zero", 0x1000, 0x2000, 0x0000),
        ("delta_exact", 0x1101, 0x2103, 0x000F),
        ("positive", 0x1200, 0x3000, 0x7FFF),
        ("signed_wrap", 0x1300, 0x4000, 0x8000),
        ("word_wrap", 0x1400, 0x5000, 0xFFFF),
        ("context_offset_wrap", 0xFFF0, 0x6000, 0x1234),
        ("state_bias_wrap", 0x1500, 0xFFC0, 0xBEEF),
    ]
    data_segment = 0x4400
    extra_segment = 0x4800
    game_segment = 0x2C00
    stack_segment = 0x9000
    return_address = 0xF000
    vectors = []

    for case_index, (name, context_offset, state_offset, field_before) in enumerate(cases):
        context_state_offset = (context_offset + 0x16) & 0xFFFF
        biased_offset = (state_offset + 0x5E) & 0xFFFF
        field_offset = (biased_offset + 0x52) & 0xFFFF
        field_after = (field_before - 0x000F) & 0xFFFF
        stack_sentinel = bytes.fromhex("5aa596698778")
        cursor_sentinel = bytes.fromhex("a1b2c3d4e5f6")
        initial_flags = 0x0A93 | (0x0400 if case_index & 1 else 0)
        initial = {
            "eax": 0xA1A1BEEF + case_index,
            "ebx": 0xB2B22345 + case_index,
            "ecx": 0xC3C33456 + case_index,
            "edx": 0xD4D44567 + case_index,
            "esi": 0xE5E55678 + case_index,
            "edi": 0xF6F60000 | context_offset,
            "ebp": 0x9797789A + case_index,
            "sp": 0xFF00,
            "ds": data_segment,
            "es": extra_segment,
            "fs": 0x4C00,
            "gs": game_segment,
            "ss": stack_segment,
            "flags": initial_flags,
        }
        immutable = [
            (data_segment, context_state_offset, struct.pack("<H", state_offset)),
            (extra_segment, context_state_offset, struct.pack("<H", state_offset ^ 0x5A5A)),
            (game_segment, context_state_offset, struct.pack("<H", state_offset ^ 0xA5A5)),
            (extra_segment, field_offset, struct.pack("<H", field_before ^ 0x3C3C)),
            (game_segment, field_offset, struct.pack("<H", field_before ^ 0xC3C3)),
        ]
        machine = execute(
            image,
            entry,
            return_address,
            initial,
            [
                (0, return_address, b"\xcc"),
                *immutable,
                (data_segment, field_offset, struct.pack("<H", field_before)),
                (0, cursor_offset - 2, cursor_sentinel),
                (
                    stack_segment,
                    0xFF00,
                    struct.pack("<H", return_address) + stack_sentinel,
                ),
            ],
        )

        expected_registers = dict(initial)
        del expected_registers["flags"]
        expected_registers["esi"] = (
            initial["esi"] & 0xFFFF0000
        ) | biased_offset
        expected_registers["sp"] = 0xFF02
        for register, expected in expected_registers.items():
            actual = machine.reg_read(REGISTERS[register])
            if actual != expected:
                raise AssertionError(
                    f"{module}:{entry:#x} {name}: "
                    f"{register}={actual:#x}, expected={expected:#x}"
                )
        if machine.reg_read(UC_X86_REG_CS) != 0:
            raise AssertionError(f"{module}:{entry:#x} {name}: near return CS changed")
        if bytes(machine.mem_read(stack_segment * 16 + 0xFF02, 6)) != stack_sentinel:
            raise AssertionError(f"{module}:{entry:#x} {name}: stack sentinel changed")

        actual_field = struct.unpack(
            "<H", machine.mem_read(data_segment * 16 + field_offset, 2)
        )[0]
        if actual_field != field_after:
            raise AssertionError(
                f"{module}:{entry:#x} {name}: field={actual_field:#x}, "
                f"expected={field_after:#x}"
            )
        expected_cursor = cursor_sentinel[:2] + struct.pack("<H", biased_offset) + cursor_sentinel[4:]
        actual_cursor = bytes(machine.mem_read(cursor_offset - 2, 6))
        if actual_cursor != expected_cursor:
            raise AssertionError(
                f"{module}:{entry:#x} {name}: cursor={actual_cursor.hex()}, "
                f"expected={expected_cursor.hex()}"
            )
        for segment, offset, value in immutable:
            actual = bytes(machine.mem_read(segment * 16 + offset, len(value)))
            if actual != value:
                raise AssertionError(
                    f"{module}:{entry:#x} {name}: input or decoy changed"
                )

        difference = field_after
        expected_flags = {
            "cf": field_before < 0x000F,
            "pf": (difference & 0xFF).bit_count() % 2 == 0,
            "af": (field_before & 0x0F) < 0x0F,
            "zf": difference == 0,
            "sf": bool(difference & 0x8000),
            "df": bool(initial_flags & 0x0400),
            "of": bool(
                ((field_before ^ 0x000F) & (field_before ^ difference))
                & 0x8000
            ),
        }
        flag_masks = {
            "cf": 0x0001,
            "pf": 0x0004,
            "af": 0x0010,
            "zf": 0x0040,
            "sf": 0x0080,
            "df": 0x0400,
            "of": 0x0800,
        }
        flags_after = machine.reg_read(UC_X86_REG_EFLAGS)
        actual_flags = {
            flag: bool(flags_after & mask) for flag, mask in flag_masks.items()
        }
        if actual_flags != expected_flags:
            raise AssertionError(
                f"{module}:{entry:#x} {name}: "
                f"flags={actual_flags}, expected={expected_flags}"
            )

        vectors.append(
            {
                "name": name,
                "module": module,
                "entry": entry,
                "context_offset": context_offset,
                "state_offset": state_offset,
                "biased_offset": biased_offset,
                "field_offset": field_offset,
                "field_before": field_before,
                "field_after": field_after,
                "cursor_offset": cursor_offset,
                "defined_flags": expected_flags,
            }
        )

    return vectors


def apply_delta_vectors(module: str, entry: int) -> list[dict[str, object]]:
    image = load_image(module)
    expected_hash = "c74ca55adaf050bd2fcd4d0ec1d112f3a4341bf8fe309e5eeb801255cf127b59"
    if hashlib.sha256(image[entry : entry + 16]).hexdigest() != expected_hash:
        raise AssertionError(f"{module}:{entry:#x}: recovered 16-byte body changed")

    cases = [
        ("zero", 0x1000, 0x2000, 0x0000, 0x1234),
        ("positive_round_down", 0x1100, 0x2100, 0x0001, 0x1111),
        ("positive_even", 0x1200, 0x2200, 0x0002, 0x2222),
        ("positive_odd", 0x1300, 0x2300, 0x0003, 0x3333),
        ("positive_sum_wrap", 0x1400, 0x2400, 0x7FFF, 0xC001),
        ("negative_even", 0x1500, 0x2500, 0x8000, 0x5555),
        ("negative_odd", 0x1600, 0x2600, 0x8001, 0x6666),
        ("negative_one", 0x1700, 0x2700, 0xFFFF, 0x7777),
        ("context_offset_wrap", 0xFFF0, 0x2800, 0x0004, 0xFFFF),
        ("state_field_wrap", 0x1800, 0xFF80, 0x0006, 0x7FFF),
    ]
    data_segment = 0x4400
    extra_segment = 0x4800
    game_segment = 0x2C00
    stack_segment = 0x9000
    return_address = 0xF000
    vectors = []

    for case_index, (
        name,
        context_offset,
        state_offset,
        delta_before,
        field_before,
    ) in enumerate(cases):
        context_state_offset = (context_offset + 0x16) & 0xFFFF
        field_offset = (state_offset + 0xB0) & 0xFFFF
        half_delta = ((delta_before >> 1) | (delta_before & 0x8000)) & 0xFFFF
        applied = not bool(half_delta & 0x8000)
        field_after = (
            (field_before + half_delta) & 0xFFFF if applied else field_before
        )
        stack_sentinel = bytes.fromhex("5aa596698778")
        delta_sentinel = bytes.fromhex("a1b20000e5f6")
        delta_bytes = delta_sentinel[:2] + struct.pack("<H", delta_before) + delta_sentinel[4:]
        initial_flags = 0x0A93 | (0x0400 if case_index & 1 else 0)
        initial = {
            "eax": 0xA1A10000 | (0xBEEF + case_index),
            "ebx": 0xB2B22345 + case_index,
            "ecx": 0xC3C33456 + case_index,
            "edx": 0xD4D44567 + case_index,
            "esi": 0xE5E55678 + case_index,
            "edi": 0xF6F60000 | context_offset,
            "ebp": 0x9797789A + case_index,
            "sp": 0xFF00,
            "ds": data_segment,
            "es": extra_segment,
            "fs": 0x4C00,
            "gs": game_segment,
            "ss": stack_segment,
            "flags": initial_flags,
        }
        immutable = [
            (data_segment, context_state_offset, struct.pack("<H", state_offset)),
            (extra_segment, context_state_offset, struct.pack("<H", state_offset ^ 0x5A5A)),
            (game_segment, context_state_offset, struct.pack("<H", state_offset ^ 0xA5A5)),
            (data_segment, 0x0099, struct.pack("<H", delta_before ^ 0xFFFF)),
            (extra_segment, field_offset, struct.pack("<H", field_before ^ 0x3C3C)),
            (game_segment, field_offset, struct.pack("<H", field_before ^ 0xC3C3)),
        ]
        machine = execute(
            image,
            entry,
            return_address,
            initial,
            [
                (0, return_address, b"\xcc"),
                *immutable,
                (data_segment, field_offset, struct.pack("<H", field_before)),
                (0, 0x0097, delta_bytes),
                (
                    stack_segment,
                    0xFF00,
                    struct.pack("<H", return_address) + stack_sentinel,
                ),
            ],
        )

        expected_registers = dict(initial)
        del expected_registers["flags"]
        expected_registers["eax"] = (
            initial["eax"] & 0xFFFF0000
        ) | half_delta
        expected_registers["esi"] = (
            initial["esi"] & 0xFFFF0000
        ) | state_offset
        expected_registers["sp"] = 0xFF02
        for register, expected in expected_registers.items():
            actual = machine.reg_read(REGISTERS[register])
            if actual != expected:
                raise AssertionError(
                    f"{module}:{entry:#x} {name}: "
                    f"{register}={actual:#x}, expected={expected:#x}"
                )
        if machine.reg_read(UC_X86_REG_CS) != 0:
            raise AssertionError(f"{module}:{entry:#x} {name}: near return CS changed")
        if bytes(machine.mem_read(stack_segment * 16 + 0xFF02, 6)) != stack_sentinel:
            raise AssertionError(f"{module}:{entry:#x} {name}: stack sentinel changed")

        actual_field = struct.unpack(
            "<H", machine.mem_read(data_segment * 16 + field_offset, 2)
        )[0]
        if actual_field != field_after:
            raise AssertionError(
                f"{module}:{entry:#x} {name}: field={actual_field:#x}, "
                f"expected={field_after:#x}"
            )
        if bytes(machine.mem_read(0x0097, 6)) != delta_bytes:
            raise AssertionError(f"{module}:{entry:#x} {name}: CS delta changed")
        for segment, offset, value in immutable:
            actual = bytes(machine.mem_read(segment * 16 + offset, len(value)))
            if actual != value:
                raise AssertionError(
                    f"{module}:{entry:#x} {name}: input or decoy changed"
                )

        if applied:
            expected_flags = {
                "cf": field_before + half_delta > 0xFFFF,
                "pf": (field_after & 0xFF).bit_count() % 2 == 0,
                "af": (field_before & 0x0F) + (half_delta & 0x0F) > 0x0F,
                "zf": field_after == 0,
                "sf": bool(field_after & 0x8000),
                "if": bool(initial_flags & 0x0200),
                "df": bool(initial_flags & 0x0400),
                "of": bool(
                    (~(field_before ^ half_delta) & (field_before ^ field_after))
                    & 0x8000
                ),
            }
        else:
            expected_flags = {
                "cf": bool(delta_before & 1),
                "pf": (half_delta & 0xFF).bit_count() % 2 == 0,
                "zf": half_delta == 0,
                "sf": bool(half_delta & 0x8000),
                "if": bool(initial_flags & 0x0200),
                "df": bool(initial_flags & 0x0400),
                "of": False,
            }
        flag_masks = {
            "cf": 0x0001,
            "pf": 0x0004,
            "af": 0x0010,
            "zf": 0x0040,
            "sf": 0x0080,
            "if": 0x0200,
            "df": 0x0400,
            "of": 0x0800,
        }
        flags_after = machine.reg_read(UC_X86_REG_EFLAGS)
        actual_flags = {
            flag: bool(flags_after & flag_masks[flag]) for flag in expected_flags
        }
        if actual_flags != expected_flags:
            raise AssertionError(
                f"{module}:{entry:#x} {name}: "
                f"flags={actual_flags}, expected={expected_flags}"
            )

        vectors.append(
            {
                "name": name,
                "module": module,
                "entry": entry,
                "context_offset": context_offset,
                "state_offset": state_offset,
                "field_offset": field_offset,
                "delta_before": delta_before,
                "half_delta": half_delta,
                "applied": applied,
                "field_before": field_before,
                "field_after": field_after,
                "defined_flags": expected_flags,
            }
        )

    return vectors


def lower_state_vectors(module: str, entry: int, cursor_offset: int) -> list[dict[str, object]]:
    image = load_image(module)
    expected_hash = "07a87d58a684bc18b00cfeeb2cd87d37018e440df7ab597ef4704fb517532d33"
    if hashlib.sha256(image[entry : entry + 11]).hexdigest() != expected_hash:
        raise AssertionError(f"{module}:{entry:#x}: recovered 11-byte body changed")

    cases = [
        ("zero", 0x1000, 0x2000, 0x0000),
        ("delta_exact", 0x1101, 0x2103, 0x000F),
        ("positive", 0x1200, 0x3000, 0x7FFF),
        ("signed_wrap", 0x1300, 0x4000, 0x8000),
        ("word_wrap", 0x1400, 0x5000, 0xFFFF),
        ("context_offset_wrap", 0xFFF0, 0x6000, 0x1234),
        ("state_bias_wrap", 0x1500, 0xFFC0, 0xBEEF),
    ]
    data_segment = 0x4400
    extra_segment = 0x4800
    game_segment = 0x2C00
    stack_segment = 0x9000
    return_address = 0xF000
    vectors = []

    for case_index, (name, context_offset, state_offset, field_before) in enumerate(cases):
        context_state_offset = (context_offset + 0x16) & 0xFFFF
        biased_offset = (state_offset + 0x5E) & 0xFFFF
        field_offset = (biased_offset + 0x52) & 0xFFFF
        field_after = (field_before - 0x000F) & 0xFFFF
        stack_sentinel = bytes.fromhex("5aa596698778")
        cursor_sentinel = bytes.fromhex("a1b2c3d4e5f6")
        initial_flags = 0x0A93 | (0x0400 if case_index & 1 else 0)
        initial = {
            "eax": 0xA1A1BEEF + case_index,
            "ebx": 0xB2B22345 + case_index,
            "ecx": 0xC3C33456 + case_index,
            "edx": 0xD4D44567 + case_index,
            "esi": 0xE5E55678 + case_index,
            "edi": 0xF6F60000 | context_offset,
            "ebp": 0x9797789A + case_index,
            "sp": 0xFF00,
            "ds": data_segment,
            "es": extra_segment,
            "fs": 0x4C00,
            "gs": game_segment,
            "ss": stack_segment,
            "flags": initial_flags,
        }
        immutable = [
            (data_segment, context_state_offset, struct.pack("<H", state_offset)),
            (extra_segment, context_state_offset, struct.pack("<H", state_offset ^ 0x5A5A)),
            (game_segment, context_state_offset, struct.pack("<H", state_offset ^ 0xA5A5)),
            (extra_segment, field_offset, struct.pack("<H", field_before ^ 0x3C3C)),
            (game_segment, field_offset, struct.pack("<H", field_before ^ 0xC3C3)),
        ]
        machine = execute(
            image,
            entry,
            return_address,
            initial,
            [
                (0, return_address, b"\xcc"),
                *immutable,
                (data_segment, field_offset, struct.pack("<H", field_before)),
                (0, cursor_offset - 2, cursor_sentinel),
                (
                    stack_segment,
                    0xFF00,
                    struct.pack("<H", return_address) + stack_sentinel,
                ),
            ],
        )

        expected_registers = dict(initial)
        del expected_registers["flags"]
        expected_registers["esi"] = (
            initial["esi"] & 0xFFFF0000
        ) | biased_offset
        expected_registers["sp"] = 0xFF02
        for register, expected in expected_registers.items():
            actual = machine.reg_read(REGISTERS[register])
            if actual != expected:
                raise AssertionError(
                    f"{module}:{entry:#x} {name}: "
                    f"{register}={actual:#x}, expected={expected:#x}"
                )
        if machine.reg_read(UC_X86_REG_CS) != 0:
            raise AssertionError(f"{module}:{entry:#x} {name}: near return CS changed")
        if bytes(machine.mem_read(stack_segment * 16 + 0xFF02, 6)) != stack_sentinel:
            raise AssertionError(f"{module}:{entry:#x} {name}: stack sentinel changed")

        actual_field = struct.unpack(
            "<H", machine.mem_read(data_segment * 16 + field_offset, 2)
        )[0]
        if actual_field != field_after:
            raise AssertionError(
                f"{module}:{entry:#x} {name}: field={actual_field:#x}, "
                f"expected={field_after:#x}"
            )
        if bytes(machine.mem_read(cursor_offset - 2, 6)) != cursor_sentinel:
            raise AssertionError(f"{module}:{entry:#x} {name}: cursor changed")
        for segment, offset, value in immutable:
            actual = bytes(machine.mem_read(segment * 16 + offset, len(value)))
            if actual != value:
                raise AssertionError(
                    f"{module}:{entry:#x} {name}: input or decoy changed"
                )

        difference = field_after
        expected_flags = {
            "cf": field_before < 0x000F,
            "pf": (difference & 0xFF).bit_count() % 2 == 0,
            "af": (field_before & 0x0F) < 0x0F,
            "zf": difference == 0,
            "sf": bool(difference & 0x8000),
            "if": bool(initial_flags & 0x0200),
            "df": bool(initial_flags & 0x0400),
            "of": bool(
                ((field_before ^ 0x000F) & (field_before ^ difference))
                & 0x8000
            ),
        }
        flag_masks = {
            "cf": 0x0001,
            "pf": 0x0004,
            "af": 0x0010,
            "zf": 0x0040,
            "sf": 0x0080,
            "if": 0x0200,
            "df": 0x0400,
            "of": 0x0800,
        }
        flags_after = machine.reg_read(UC_X86_REG_EFLAGS)
        actual_flags = {
            flag: bool(flags_after & mask) for flag, mask in flag_masks.items()
        }
        if actual_flags != expected_flags:
            raise AssertionError(
                f"{module}:{entry:#x} {name}: "
                f"flags={actual_flags}, expected={expected_flags}"
            )

        vectors.append(
            {
                "name": name,
                "module": module,
                "entry": entry,
                "context_offset": context_offset,
                "state_offset": state_offset,
                "biased_offset": biased_offset,
                "field_offset": field_offset,
                "field_before": field_before,
                "field_after": field_after,
                "cursor_offset": cursor_offset,
                "cursor_unchanged": True,
                "defined_flags": expected_flags,
            }
        )

    return vectors


def resume_or_init_vectors(
    module: str, entry: int, initial_resume: int
) -> list[dict[str, object]]:
    image = load_image(module)
    routine_hashes = {
        "amer": "9d298b17dbf20f335fe135f28ed40e81110af1973e9628dd3ab5d420b60e9eed",
        "croolis": "efadb9db7ca5a5948b626039dc5b44f96b1eb61b8ad6ebe0e2758ec249323a81",
        "scrut": "b1a0b8943aaa5f76c2091d91bb198b2df18d4e3ffd989a47b3cc8bafc61b41d6",
    }
    if hashlib.sha256(image[entry : entry + 25]).hexdigest() != routine_hashes[module]:
        raise AssertionError(f"{module}:{entry:#x}: recovered 25-byte body changed")

    cases = [
        ("initialize", 0x1000, 0x0000, 0x1111, 0x2222),
        ("initialize_extremes", 0x1200, 0x0000, 0xFFFF, 0x8000),
        ("initialize_offset_wrap", 0xFFC8, 0x0000, 0x3333, 0x4444),
        ("dispatch_low", 0x1400, 0x7001, 0x5555, 0x6666),
        ("dispatch_signed", 0x1500, 0x8001, 0x7777, 0x8888),
        ("dispatch_high", 0x1600, 0xF101, 0x9999, 0xAAAA),
    ]
    data_segment = 0x4400
    extra_segment = 0x4800
    game_segment = 0x2C00
    stack_segment = 0x9000
    return_address = 0xF000
    capture_offset = 0x7000
    vectors = []

    for case_index, (
        name,
        context_offset,
        resume_before,
        step_before,
        value_before,
    ) in enumerate(cases):
        resume_offset = (context_offset + 0x36) & 0xFFFF
        step_offset = (context_offset + 0x38) & 0xFFFF
        value_offset = (context_offset + 0x3A) & 0xFFFF
        padding_before_offset = (context_offset + 0x34) & 0xFFFF
        padding_after_offset = (context_offset + 0x3C) & 0xFFFF
        dispatched = resume_before != 0
        stack_sentinel = bytes.fromhex("5aa596698778")
        capture_sentinel = bytes.fromhex("a1b2c3d4e5f6")
        callback_ax = (0x4A00 + case_index) & 0xFFFF
        callback_stub = (
            b"\xb8"
            + struct.pack("<H", callback_ax)
            + bytes.fromhex("893e007089260270891e0470c3")
        )
        initial_flags = 0x0A93 | (0x0400 if case_index & 1 else 0)
        initial = {
            "eax": 0xA1A1BEEF + case_index,
            "ebx": 0xB2B22345 + case_index,
            "ecx": 0xC3C33456 + case_index,
            "edx": 0xD4D44567 + case_index,
            "esi": 0xE5E55678 + case_index,
            "edi": 0xF6F60000 | context_offset,
            "ebp": 0x9797789A + case_index,
            "sp": 0xFF00,
            "ds": data_segment,
            "es": extra_segment,
            "fs": 0x4C00,
            "gs": game_segment,
            "ss": stack_segment,
            "flags": initial_flags,
        }
        field_values = [
            (resume_offset, resume_before),
            (step_offset, step_before),
            (value_offset, value_before),
        ]
        immutable = [
            (data_segment, padding_before_offset, bytes.fromhex("5aa5")),
            (data_segment, padding_after_offset, bytes.fromhex("9669")),
        ]
        for segment, xor_mask in (
            (extra_segment, 0x5A5A),
            (game_segment, 0xA5A5),
        ):
            immutable.extend(
                (segment, offset, struct.pack("<H", value ^ xor_mask))
                for offset, value in field_values
            )
        memory = [
            (0, return_address, b"\xcc"),
            *immutable,
            *(
                (data_segment, offset, struct.pack("<H", value))
                for offset, value in field_values
            ),
            (data_segment, capture_offset, capture_sentinel),
            (
                stack_segment,
                0xFF00,
                struct.pack("<H", return_address) + stack_sentinel,
            ),
        ]
        if dispatched:
            memory.append((0, resume_before, callback_stub))
        machine = execute(
            image,
            entry,
            return_address,
            initial,
            memory,
        )

        expected_registers = dict(initial)
        del expected_registers["flags"]
        expected_registers["ebx"] = (
            initial["ebx"] & 0xFFFF0000
        ) | resume_before
        if dispatched:
            expected_registers["eax"] = (
                initial["eax"] & 0xFFFF0000
            ) | callback_ax
        expected_registers["sp"] = 0xFF02
        for register, expected in expected_registers.items():
            actual = machine.reg_read(REGISTERS[register])
            if actual != expected:
                raise AssertionError(
                    f"{module}:{entry:#x} {name}: "
                    f"{register}={actual:#x}, expected={expected:#x}"
                )
        if machine.reg_read(UC_X86_REG_CS) != 0:
            raise AssertionError(f"{module}:{entry:#x} {name}: near return CS changed")
        if bytes(machine.mem_read(stack_segment * 16 + 0xFF02, 6)) != stack_sentinel:
            raise AssertionError(f"{module}:{entry:#x} {name}: stack sentinel changed")

        expected_fields = (
            (resume_before, step_before, value_before)
            if dispatched
            else (initial_resume, 0, 0)
        )
        for (offset, _before), expected in zip(field_values, expected_fields):
            actual = struct.unpack(
                "<H", machine.mem_read(data_segment * 16 + offset, 2)
            )[0]
            if actual != expected:
                raise AssertionError(
                    f"{module}:{entry:#x} {name}: context field at {offset:#x} "
                    f"is {actual:#x}, expected {expected:#x}"
                )
        for segment, offset, value in immutable:
            actual = bytes(machine.mem_read(segment * 16 + offset, len(value)))
            if actual != value:
                raise AssertionError(
                    f"{module}:{entry:#x} {name}: padding or decoy changed"
                )

        actual_capture = bytes(
            machine.mem_read(data_segment * 16 + capture_offset, 6)
        )
        if dispatched:
            expected_capture = struct.pack(
                "<HHH", context_offset, 0xFF00, resume_before
            )
        else:
            expected_capture = capture_sentinel
        if actual_capture != expected_capture:
            raise AssertionError(
                f"{module}:{entry:#x} {name}: capture={actual_capture.hex()}, "
                f"expected={expected_capture.hex()}"
            )

        flag_value = resume_before
        expected_flags = {
            "cf": False,
            "pf": (flag_value & 0xFF).bit_count() % 2 == 0,
            "zf": flag_value == 0,
            "sf": bool(flag_value & 0x8000),
            "if": bool(initial_flags & 0x0200),
            "df": bool(initial_flags & 0x0400),
            "of": False,
        }
        flag_masks = {
            "cf": 0x0001,
            "pf": 0x0004,
            "zf": 0x0040,
            "sf": 0x0080,
            "if": 0x0200,
            "df": 0x0400,
            "of": 0x0800,
        }
        flags_after = machine.reg_read(UC_X86_REG_EFLAGS)
        actual_flags = {
            flag: bool(flags_after & mask) for flag, mask in flag_masks.items()
        }
        if actual_flags != expected_flags:
            raise AssertionError(
                f"{module}:{entry:#x} {name}: "
                f"flags={actual_flags}, expected={expected_flags}"
            )

        vectors.append(
            {
                "name": name,
                "module": module,
                "entry": entry,
                "context_offset": context_offset,
                "resume_before": resume_before,
                "resume_after": expected_fields[0],
                "resume_step_before": step_before,
                "resume_step_after": expected_fields[1],
                "resume_value_before": value_before,
                "resume_value_after": expected_fields[2],
                "tail_dispatched": dispatched,
                "callback_stack_pointer": 0xFF00 if dispatched else None,
                "defined_flags": expected_flags,
            }
        )

    return vectors


def near_noop_vectors(module: str, entry: int) -> list[dict[str, object]]:
    image = load_image(module)
    if image[entry : entry + 1] != b"\xc3":
        raise AssertionError(f"{module}:{entry:#x}: expected one-byte near RET")

    stack_segment = 0x9000
    return_address = 0xF000
    flag_cases = (0x0202, 0x0AD7, 0x0646)
    flag_masks = {
        "cf": 0x0001,
        "pf": 0x0004,
        "af": 0x0010,
        "zf": 0x0040,
        "sf": 0x0080,
        "if": 0x0200,
        "df": 0x0400,
        "of": 0x0800,
    }
    vectors = []

    for case_index, initial_flags in enumerate(flag_cases):
        stack_sentinel = bytes.fromhex("5aa596698778")
        initial = {
            "eax": 0xA1A1BEEF + case_index,
            "ebx": 0xB2B22345 + case_index,
            "ecx": 0xC3C33456 + case_index,
            "edx": 0xD4D44567 + case_index,
            "esi": 0xE5E55678 + case_index,
            "edi": 0xF6F66789 + case_index,
            "ebp": 0x9797789A + case_index,
            "sp": 0xFF00,
            "ds": 0x4400,
            "es": 0x4800,
            "fs": 0x4C00,
            "gs": 0x2C00,
            "ss": stack_segment,
            "flags": initial_flags,
        }
        machine = execute(
            image,
            entry,
            return_address,
            initial,
            [
                (0, return_address, b"\xcc"),
                (
                    stack_segment,
                    0xFF00,
                    struct.pack("<H", return_address) + stack_sentinel,
                ),
            ],
        )
        expected_registers = dict(initial)
        del expected_registers["flags"]
        expected_registers["sp"] = 0xFF02
        for register, expected in expected_registers.items():
            actual = machine.reg_read(REGISTERS[register])
            if actual != expected:
                raise AssertionError(
                    f"{module}:{entry:#x} case {case_index}: "
                    f"{register}={actual:#x}, expected={expected:#x}"
                )
        if machine.reg_read(UC_X86_REG_CS) != 0:
            raise AssertionError(f"{module}:{entry:#x}: near return CS changed")
        if bytes(machine.mem_read(stack_segment * 16 + 0xFF02, 6)) != stack_sentinel:
            raise AssertionError(f"{module}:{entry:#x}: stack sentinel changed")

        flags_after = machine.reg_read(UC_X86_REG_EFLAGS)
        expected_flags = {
            flag: bool(initial_flags & mask) for flag, mask in flag_masks.items()
        }
        actual_flags = {
            flag: bool(flags_after & mask) for flag, mask in flag_masks.items()
        }
        if actual_flags != expected_flags:
            raise AssertionError(
                f"{module}:{entry:#x} case {case_index}: "
                f"flags={actual_flags}, expected={expected_flags}"
            )

        vectors.append(
            {
                "name": f"flags_{initial_flags:04x}",
                "module": module,
                "entry": entry,
                "stack_bytes_consumed": 2,
                "defined_flags": expected_flags,
            }
        )

    return vectors


def manu3_api_entry_vectors() -> list[dict[str, object]]:
    module = "manu3"
    entry = 0x0000
    image = load_image(module)
    expected_hash = "9d5ca45567f31b131e58d4532c14fe288d957a3136ce4e25e1363e28de3ac8a5"
    if hashlib.sha256(image[entry : entry + 289]).hexdigest() != expected_hash:
        raise AssertionError(f"{module}:{entry:#x}: recovered 289-byte body changed")

    callees = (0x0181, 0x019B, 0x0270, 0x0549, 0x06F6)
    patched_image = bytearray(image)
    for callee in callees:
        patched_image[callee] = 0xC3

    stack_segment = 0x9000
    return_address = 0xF000
    return_segment = 0x0000
    initial_data_segment = 0x3000
    initial_extra_segment = 0x3800
    initial_fs_segment = 0x4000
    active_segment = 0x5000
    geometry_segment = 0x7000
    request_offset = 0x8000
    state_offset = 0x24AE
    vectors = []

    def signed_word(value: int) -> int:
        value &= 0xFFFF
        return value if value < 0x8000 else value - 0x10000

    def signed_dword(value: int) -> int:
        value &= 0xFFFFFFFF
        return value if value < 0x80000000 else value - 0x100000000

    def multiply_dword(left: int, right: int) -> int:
        return ((left & 0xFFFFFFFF) * (right & 0xFFFFFFFF)) & 0xFFFFFFFF

    def divide_signed(dividend: int, divisor: int) -> tuple[int, int]:
        left = signed_dword(dividend)
        right = signed_dword(divisor)
        quotient = abs(left) // abs(right)
        if (left < 0) != (right < 0):
            quotient = -quotient
        remainder = left - quotient * right
        return quotient & 0xFFFFFFFF, remainder & 0xFFFFFFFF

    def subtract_flags_32(
        left: int, right: int, initial_flags: int
    ) -> dict[str, bool]:
        left &= 0xFFFFFFFF
        right &= 0xFFFFFFFF
        result = (left - right) & 0xFFFFFFFF
        return {
            "cf": left < right,
            "pf": (result & 0xFF).bit_count() % 2 == 0,
            "af": (left & 0x0F) < (right & 0x0F),
            "zf": result == 0,
            "sf": bool(result & 0x80000000),
            "if": bool(initial_flags & 0x0200),
            "df": bool(initial_flags & 0x0400),
            "of": bool(((left ^ right) & (left ^ result)) & 0x80000000),
        }

    def shift_right_flags_32(
        value: int, result: int, initial_flags: int
    ) -> dict[str, bool]:
        result &= 0xFFFFFFFF
        return {
            "cf": bool(value & 0x00000080),
            "pf": (result & 0xFF).bit_count() % 2 == 0,
            "zf": result == 0,
            "sf": bool(result & 0x80000000),
            "if": bool(initial_flags & 0x0200),
            "df": bool(initial_flags & 0x0400),
        }

    flag_masks = {
        "cf": 0x0001,
        "pf": 0x0004,
        "af": 0x0010,
        "zf": 0x0040,
        "sf": 0x0080,
        "if": 0x0200,
        "df": 0x0400,
        "of": 0x0800,
    }

    data_delta = 0x6000
    work_deltas = (0x0800, 0x1000, 0x1800)
    inactive_data_segment = data_delta
    inactive_work_segments = []
    segment = inactive_data_segment
    for delta in work_deltas:
        segment = (segment + delta) & 0xFFFF
        inactive_work_segments.append(segment)
    inactive_directory = bytearray(
        ((offset * 23 + 9) & 0xFF) for offset in range(18)
    )
    struct.pack_into("<HHH", inactive_directory, 0x0C, *work_deltas)
    inactive_expected = bytearray(inactive_directory)
    struct.pack_into("<HHH", inactive_expected, 0x02, *inactive_work_segments)
    continuation_before = bytes.fromhex("1021324354657687")
    continuation_expected = (
        continuation_before[:2]
        + struct.pack("<H", 0x0AE0)
        + continuation_before[4:]
    )
    initial_flags = 0x0A93
    initial = {
        "eax": 0xA1A1BEEF,
        "ebx": 0xB2B22345,
        "ecx": 0xC3C33456,
        "edx": 0xD4D44567,
        "esi": 0xE5E55678,
        "edi": 0xF6F66789,
        "ebp": 0x97978000,
        "sp": 0xFF00,
        "ds": initial_data_segment,
        "es": initial_extra_segment,
        "fs": initial_fs_segment,
        "gs": 0x4800,
        "ss": stack_segment,
        "flags": initial_flags,
    }
    stack_sentinel = bytes.fromhex("5aa596698778")
    machine = execute(
        bytes(patched_image),
        entry,
        return_address,
        initial,
        [
            (0, 0x1368, struct.pack("<HH", data_delta, 0)),
            (0, return_address, b"\xcc"),
            (inactive_data_segment, 0, bytes(inactive_directory)),
            (
                inactive_work_segments[-1],
                0x067C,
                continuation_before,
            ),
            (
                stack_segment,
                0xFF00,
                struct.pack("<HH", return_address, return_segment)
                + stack_sentinel,
            ),
        ],
        return_segment=return_segment,
    )
    expected_registers = dict(initial)
    del expected_registers["flags"]
    expected_registers["eax"] = (
        initial["eax"] & 0xFFFF0000
    ) | inactive_work_segments[-1]
    expected_registers["ecx"] &= 0xFFFF0000
    expected_registers["es"] = inactive_work_segments[-1]
    expected_registers["fs"] = inactive_data_segment
    expected_registers["sp"] = 0xFF04
    for register, expected in expected_registers.items():
        actual = machine.reg_read(REGISTERS[register])
        if actual != expected:
            raise AssertionError(
                f"{module}:{entry:#x} inactive_init: "
                f"{register}={actual:#x}, expected={expected:#x}"
            )
    if bytes(machine.mem_read(inactive_data_segment * 16, 18)) != inactive_expected:
        raise AssertionError(f"{module}:{entry:#x} inactive_init: directory differs")
    if bytes(
        machine.mem_read(inactive_work_segments[-1] * 16 + 0x067C, 8)
    ) != continuation_expected:
        raise AssertionError(f"{module}:{entry:#x} inactive_init: continuation differs")
    if bytes(machine.mem_read(stack_segment * 16 + 0xFF04, 6)) != stack_sentinel:
        raise AssertionError(f"{module}:{entry:#x} inactive_init: stack changed")
    expected_flags = add_flags_16(
        inactive_work_segments[-2], work_deltas[-1], initial_flags
    )
    flags_after = machine.reg_read(UC_X86_REG_EFLAGS)
    actual_flags = {
        flag: bool(flags_after & flag_masks[flag]) for flag in expected_flags
    }
    if actual_flags != expected_flags:
        raise AssertionError(
            f"{module}:{entry:#x} inactive_init: "
            f"flags={actual_flags}, expected={expected_flags}"
        )
    vectors.append(
        {
            "name": "inactive_initializes_overlay",
            "module": module,
            "entry": entry,
            "data_segment": inactive_data_segment,
            "work_segments": inactive_work_segments,
            "ordered_callees": [],
            "far_stack_bytes_consumed": 4,
            "defined_flags": expected_flags,
        }
    )

    zero_matrix = ((0, 0, 0), (0, 0, 0), (0, 0, 0))
    cases = (
        (
            "centered_no_selector",
            (160, 100),
            0,
            0,
            zero_matrix,
            (0, 0, 256),
            (0, 0, 0),
        ),
        (
            "selector_mask_and_translation",
            (10, 20),
            0x0023,
            0x1234,
            zero_matrix,
            (100, -50, 256),
            (0, 0, 0),
        ),
        (
            "matrix_and_signed_coordinates",
            (-120, 250),
            0x001F,
            0x00FF,
            ((2, -3, 4), (-5, 6, -7), (8, -9, 10)),
            (400, -300, 1024),
            ((-11), 13, -17),
        ),
        (
            "zero_depth_preserves_centers",
            (0, 0),
            0x0020,
            0x000F,
            zero_matrix,
            (123, 456, 0),
            (1, 2, 3),
        ),
        (
            "negative_depth_preserves_centers",
            (32767, -32768),
            0xFFFF,
            0x0010,
            zero_matrix,
            (-1, 1, -256),
            (-1, -1, -1),
        ),
        (
            "cursor_and_camera_wrap",
            (-1, -32768),
            1,
            0xFFF0,
            zero_matrix,
            (0, 0, 512),
            (0, 0, 0),
        ),
        (
            "maximum_window_offset",
            (-32768, 32767),
            0x0042,
            0xFFFF,
            zero_matrix,
            (-400, 600, 256),
            (0, 0, 0),
        ),
    )

    for case_index, (
        name,
        cursor,
        selector,
        window_offset,
        matrix,
        translation,
        reference,
    ) in enumerate(cases, start=1):
        active_before = bytearray(
            ((offset * 37 + case_index * 19 + 11) & 0xFF)
            for offset in range(0x10000)
        )
        active_expected = bytearray(active_before)
        geometry_before = bytearray(
            ((offset * 13 + case_index * 29 + 7) & 0xFF)
            for offset in range(0x10000)
        )
        request = struct.pack(
            "<hhHH", cursor[0], cursor[1], selector, window_offset
        )
        initial_pitch = (0x1100 + case_index * 0x1111) & 0xFFFF
        initial_yaw = (0x2200 + case_index * 0x2222) & 0xFFFF
        centers_before = (
            (0x12345678 + case_index * 0x01010101) & 0xFFFFFFFF,
            (0x89ABCDEF + case_index * 0x01010101) & 0xFFFFFFFF,
        )
        struct.pack_into("<H", active_before, 0x0002, geometry_segment)
        struct.pack_into("<HH", active_before, 0x23E2, initial_pitch, initial_yaw)
        struct.pack_into("<II", active_before, 0x223E, *centers_before)
        for row in range(3):
            for column in range(3):
                struct.pack_into(
                    "<I",
                    active_before,
                    state_offset + 0x12 + (row * 3 + column) * 4,
                    matrix[row][column] & 0xFFFFFFFF,
                )
        struct.pack_into(
            "<III",
            active_before,
            state_offset + 0x36,
            *(value & 0xFFFFFFFF for value in translation),
        )
        struct.pack_into("<hhh", geometry_before, 0x02AC, *reference)
        active_expected[:] = active_before

        cursor_words = (cursor[0] & 0xFFFF, cursor[1] & 0xFFFF)
        framebuffer_segment = (0xA000 + (window_offset >> 4)) & 0xFFFF
        struct.pack_into("<HH", active_expected, 0x001A, *cursor_words)
        struct.pack_into("<H", active_expected, 0x0018, framebuffer_segment)
        yaw_delta = ((cursor_words[0] - 0x00A0) << 1) & 0xFFFF
        pitch_delta = ((cursor_words[1] - 0x0064) << 1) & 0xFFFF
        adjusted_pitch = (initial_pitch + pitch_delta) & 0xFFFF
        adjusted_yaw = (initial_yaw + yaw_delta) & 0xFFFF

        object_values = tuple(value & 0xFFFFFFFF for value in reference)
        depth_accumulator = 0
        for column in range(3):
            depth_accumulator += multiply_dword(
                matrix[2][column], object_values[column]
            )
        depth_accumulator = (
            depth_accumulator + (translation[2] & 0xFFFFFFFF)
        ) & 0xFFFFFFFF
        depth = (signed_dword(depth_accumulator) >> 8) & 0xFFFFFFFF

        center_x, center_y = centers_before
        final_eax = multiply_dword(matrix[2][2], object_values[2])
        final_edx = 0xD4D44567 + case_index
        final_ebp = object_values[2]
        if signed_dword(depth) > 0:
            y_accumulator = 0
            x_accumulator = 0
            for column in range(3):
                y_accumulator += multiply_dword(
                    matrix[1][column], object_values[column]
                )
                x_accumulator += multiply_dword(
                    matrix[0][column], object_values[column]
                )
            y_accumulator = (
                y_accumulator + (translation[1] & 0xFFFFFFFF)
            ) & 0xFFFFFFFF
            x_accumulator = (
                x_accumulator + (translation[0] & 0xFFFFFFFF)
            ) & 0xFFFFFFFF
            y_quotient, _y_remainder = divide_signed(y_accumulator, depth)
            x_quotient, x_remainder = divide_signed(x_accumulator, depth)
            center_y = (
                (signed_word(cursor_words[1]) & 0xFFFFFFFF) + y_quotient
            ) & 0xFFFFFFFF
            center_x = (
                (signed_word(cursor_words[0]) & 0xFFFFFFFF) - x_quotient
            ) & 0xFFFFFFFF
            struct.pack_into("<II", active_expected, 0x223E, center_x, center_y)
            final_eax = x_quotient
            final_edx = x_remainder
            final_ebp = center_x

        initial_flags = 0x0A93 | (0x0400 if case_index & 1 else 0)
        initial = {
            "eax": 0xA1A1BEEF + case_index,
            "ebx": 0xB2B22345 + case_index,
            "ecx": 0xC3C33456 + case_index,
            "edx": 0xD4D44567 + case_index,
            "esi": 0xE5E55678 + case_index,
            "edi": 0xF6F66789 + case_index,
            "ebp": 0x97978000,
            "sp": 0xFF00,
            "ds": initial_data_segment,
            "es": initial_extra_segment,
            "fs": initial_fs_segment,
            "gs": 0x4800,
            "ss": stack_segment,
            "flags": initial_flags,
        }
        call_entries: list[dict[str, object]] = []

        def code_handler(
            machine: Uc, address: int, _size: int, _data: object
        ) -> None:
            if address not in callees:
                return
            sp = machine.reg_read(UC_X86_REG_SP)
            call_entries.append(
                {
                    "callee": address,
                    "return_ip": struct.unpack(
                        "<H", machine.mem_read(stack_segment * 16 + sp, 2)
                    )[0],
                    "sp": sp,
                    "ds": machine.reg_read(UC_X86_REG_DS),
                    "es": machine.reg_read(UC_X86_REG_ES),
                    "fs": machine.reg_read(UC_X86_REG_FS),
                    "cursor": struct.unpack(
                        "<HH",
                        machine.mem_read(active_segment * 16 + 0x001A, 4),
                    ),
                    "framebuffer_segment": struct.unpack(
                        "<H",
                        machine.mem_read(active_segment * 16 + 0x0018, 2),
                    )[0],
                    "view": struct.unpack(
                        "<HH",
                        machine.mem_read(active_segment * 16 + 0x23E2, 4),
                    ),
                    "centers": struct.unpack(
                        "<II",
                        machine.mem_read(active_segment * 16 + 0x223E, 8),
                    ),
                }
            )

        machine = execute(
            bytes(patched_image),
            entry,
            return_address,
            initial,
            [
                (0, 0x136A, struct.pack("<H", active_segment)),
                (0, return_address, b"\xcc"),
                (active_segment, 0, bytes(active_before)),
                (geometry_segment, 0, bytes(geometry_before)),
                (initial_data_segment, request_offset, b"\x10" * 8),
                (initial_extra_segment, request_offset, b"\x20" * 8),
                (initial_fs_segment, request_offset, b"\x30" * 8),
                (stack_segment, request_offset, request),
                (
                    stack_segment,
                    0xFF00,
                    struct.pack("<HH", return_address, return_segment)
                    + stack_sentinel,
                ),
            ],
            code_handler=code_handler,
            return_segment=return_segment,
        )

        common_early = {
            "ds": active_segment,
            "es": active_segment,
            "fs": active_segment,
            "cursor": cursor_words,
            "framebuffer_segment": framebuffer_segment,
            "view": (initial_pitch, initial_yaw),
            "centers": centers_before,
        }
        expected_calls = []
        masked_selector = selector & 0x001F
        if masked_selector != 0:
            expected_calls.append(
                {
                    "callee": 0x0181,
                    "return_ip": 0x0031,
                    "sp": 0xFEFC,
                    **common_early,
                }
            )
        expected_calls.append(
            {
                "callee": 0x019B,
                "return_ip": 0x0034,
                "sp": 0xFEFC,
                **common_early,
            }
        )
        expected_calls.append(
            {
                "callee": 0x0270,
                "return_ip": 0x0058,
                "sp": 0xFEF8,
                **{
                    **common_early,
                    "view": (adjusted_pitch, adjusted_yaw),
                },
            }
        )
        for callee, return_ip in ((0x0549, 0x011C), (0x06F6, 0x011F)):
            expected_calls.append(
                {
                    "callee": callee,
                    "return_ip": return_ip,
                    "sp": 0xFEFC,
                    **{
                        **common_early,
                        "es": geometry_segment,
                        "centers": (center_x, center_y),
                    },
                }
            )
        if call_entries != expected_calls:
            raise AssertionError(
                f"{module}:{entry:#x} {name}: calls={call_entries}, "
                f"expected={expected_calls}"
            )

        expected_registers = {
            "eax": final_eax,
            "ebx": object_values[0],
            "ecx": object_values[1],
            "edx": final_edx,
            "esi": depth,
            "edi": (initial["edi"] & 0xFFFF0000) | state_offset,
            "ebp": final_ebp,
            "sp": 0xFF04,
            "ds": initial_data_segment,
            "es": geometry_segment,
            "fs": active_segment,
            "gs": initial["gs"],
            "ss": stack_segment,
        }
        for register, expected in expected_registers.items():
            actual = machine.reg_read(REGISTERS[register])
            if actual != expected:
                raise AssertionError(
                    f"{module}:{entry:#x} {name}: "
                    f"{register}={actual:#x}, expected={expected:#x}"
                )

        actual_active = bytes(machine.mem_read(active_segment * 16, 0x10000))
        if actual_active != active_expected:
            raise AssertionError(f"{module}:{entry:#x} {name}: active memory differs")
        actual_geometry = bytes(
            machine.mem_read(geometry_segment * 16, 0x10000)
        )
        if actual_geometry != geometry_before:
            raise AssertionError(f"{module}:{entry:#x} {name}: geometry changed")
        if bytes(machine.mem_read(stack_segment * 16 + request_offset, 8)) != request:
            raise AssertionError(f"{module}:{entry:#x} {name}: request changed")
        if bytes(machine.mem_read(stack_segment * 16 + 0xFF04, 6)) != stack_sentinel:
            raise AssertionError(f"{module}:{entry:#x} {name}: stack sentinel changed")

        if signed_dword(depth) > 0:
            expected_flags = subtract_flags_32(
                signed_word(cursor_words[0]), final_eax, initial_flags
            )
        else:
            expected_flags = shift_right_flags_32(
                depth_accumulator, depth, initial_flags
            )
        flags_after = machine.reg_read(UC_X86_REG_EFLAGS)
        actual_flags = {
            flag: bool(flags_after & flag_masks[flag]) for flag in expected_flags
        }
        if actual_flags != expected_flags:
            raise AssertionError(
                f"{module}:{entry:#x} {name}: "
                f"flags={actual_flags}, expected={expected_flags}"
            )

        vectors.append(
            {
                "name": name,
                "module": module,
                "entry": entry,
                "cursor": list(cursor),
                "selector": selector,
                "masked_selector": masked_selector,
                "framebuffer_window_offset": window_offset,
                "framebuffer_segment": framebuffer_segment,
                "initial_view": [initial_pitch, initial_yaw],
                "adjusted_view": [adjusted_pitch, adjusted_yaw],
                "restored_view": [initial_pitch, initial_yaw],
                "matrix": [list(row) for row in matrix],
                "translation": list(translation),
                "reference": list(reference),
                "depth": signed_dword(depth),
                "screen_center_before": [
                    signed_dword(centers_before[0]),
                    signed_dword(centers_before[1]),
                ],
                "screen_center": [signed_dword(center_x), signed_dword(center_y)],
                "ordered_callees": [call["callee"] for call in expected_calls],
                "far_stack_bytes_consumed": 4,
                "defined_flags": expected_flags,
            }
        )

    return vectors


def manu3_matrix_build_vectors() -> list[dict[str, object]]:
    module = "manu3"
    entry = 0x0270
    image = load_image(module)
    expected_hash = "18e03847b76f4b4898b7de6cf8431d4373fba4862f362b1d6aa48395b95b5b89"
    if hashlib.sha256(image[entry : entry + 729]).hexdigest() != expected_hash:
        raise AssertionError(f"{module}:{entry:#x}: recovered 729-byte body changed")

    active_segment = 0x5000
    extra_segment = 0x6800
    game_segment = 0x7800
    stack_segment = 0x9000
    return_address = 0xF000
    state_base = 0x2394
    state_size = 0x005E
    cases = (
        ("single_zero_angles", 1, ((0x0000, 0x0000, 0x0000, 0),)),
        ("single_mixed_angles", 1, ((0x0014, 0x02A0, 0x07FC, 1234),)),
        ("angle_masking", 1, ((0xF013, 0xA2A3, 0x77FF, -2222),)),
        ("negative_radial_extreme", 1, ((0x0FFC, 0x0800, 0x0400, -32768),)),
        ("positive_radial_extreme", 1, ((0x0554, 0x0AA8, 0x0CC0, 32767),)),
        (
            "two_state_hierarchy",
            2,
            ((0x0100, 0x0200, 0x0300, 511), (0x0444, 0x0888, 0x0CCC, -777)),
        ),
        (
            "two_state_overflow",
            2,
            ((0x0FF8, 0x0004, 0x07FC, -1), (0x0ABC, 0x0120, 0x0F00, 1)),
        ),
    )
    vectors = []

    def signed_word(value: int) -> int:
        value &= 0xFFFF
        return value if value < 0x8000 else value - 0x10000

    def signed_dword(value: int) -> int:
        value &= 0xFFFFFFFF
        return value if value < 0x80000000 else value - 0x100000000

    def read_word(buffer: bytearray, offset: int) -> int:
        offset &= 0xFFFF
        return buffer[offset] | (buffer[offset + 1] << 8)

    def write_word(buffer: bytearray, offset: int, value: int) -> None:
        offset &= 0xFFFF
        buffer[offset : offset + 2] = (value & 0xFFFF).to_bytes(2, "little")

    def read_dword(buffer: bytearray, offset: int) -> int:
        offset &= 0xFFFF
        return int.from_bytes(buffer[offset : offset + 4], "little")

    def write_dword(buffer: bytearray, offset: int, value: int) -> None:
        offset &= 0xFFFF
        buffer[offset : offset + 4] = (value & 0xFFFFFFFF).to_bytes(4, "little")

    def multiply(left: int, right: int) -> int:
        return ((left & 0xFFFFFFFF) * (right & 0xFFFFFFFF)) & 0xFFFFFFFF

    def add(left: int, right: int) -> int:
        return (left + right) & 0xFFFFFFFF

    def subtract(left: int, right: int) -> int:
        return (left - right) & 0xFFFFFFFF

    def negate(value: int) -> int:
        return (-value) & 0xFFFFFFFF

    def sar(value: int, count: int) -> int:
        return (signed_dword(value) >> count) & 0xFFFFFFFF

    def trig(buffer: bytearray, angle: int) -> tuple[int, int]:
        offset = angle & 0x0FFC
        return (
            signed_word(read_word(buffer, offset + 0x26)),
            signed_word(read_word(buffer, offset + 0x28)),
        )

    def build_rotation(
        buffer: bytearray, angle_0: int, angle_1: int, angle_2: int
    ) -> list[list[int]]:
        angle_0 &= 0x0FFC
        angle_1 &= 0x0FFC
        angle_2 &= 0x0FFC
        matrix = [[0, 0, 0] for _ in range(3)]
        _a0_component_0, a0_component_1 = trig(buffer, angle_0)
        matrix[1][2] = negate((a0_component_1 << 1) & 0xFFFFFFFF)

        first = trig(buffer, angle_0 - angle_1 - angle_2)
        second = trig(buffer, angle_0 + angle_1 + angle_2)
        base = trig(buffer, angle_1 + angle_2)
        value_0 = add(sar(subtract(first[0], second[0]), 1), base[1])
        value_1 = add(sar(add(first[1], second[1]), 1), base[0])
        matrix[0][1] = value_0
        matrix[2][0] = negate(value_0)
        matrix[0][0] = value_1
        matrix[2][1] = value_1

        first = trig(buffer, angle_0 - angle_1 + angle_2)
        second = trig(buffer, angle_0 + angle_1 - angle_2)
        base = trig(buffer, angle_1 - angle_2)
        value_0 = sar(subtract(first[0], second[0]), 1)
        value_1 = sar(add(first[1], second[1]), 1)
        adjustment_0 = subtract(base[1], value_0)
        adjustment_1 = subtract(base[0], value_1)
        matrix[0][1] = subtract(matrix[0][1], adjustment_0)
        matrix[2][0] = subtract(matrix[2][0], adjustment_0)
        matrix[0][0] = add(matrix[0][0], adjustment_1)
        matrix[2][1] = subtract(matrix[2][1], adjustment_1)

        first = trig(buffer, angle_2 + angle_0)
        second = trig(buffer, angle_2 - angle_0)
        matrix[1][1] = add(first[0], second[0])
        matrix[1][0] = negate(add(first[1], second[1]))

        first = trig(buffer, angle_1 + angle_0)
        second = trig(buffer, angle_1 - angle_0)
        matrix[2][2] = add(first[0], second[0])
        matrix[0][2] = add(first[1], second[1])
        return matrix

    def model_state(buffer: bytearray, state_offset: int) -> list[list[int]]:
        angle_0 = read_word(buffer, state_offset + 0x4E) & 0x0FFC
        angle_1 = read_word(buffer, state_offset + 0x50) & 0x0FFC
        angle_2 = read_word(buffer, state_offset + 0x52) & 0x0FFC
        write_word(buffer, 0x0020, angle_1)
        write_word(buffer, 0x0022, angle_0)
        write_word(buffer, 0x0024, angle_2)
        write_word(buffer, 0x2248, state_offset)
        rotation = build_rotation(buffer, angle_0, angle_1, angle_2)
        for row in range(3):
            for column in range(3):
                write_dword(buffer, 0x2250 + (row * 3 + column) * 4, rotation[row][column])

        radial = signed_word(read_word(buffer, state_offset + 0x54))
        for axis, matrix_value in ((0, rotation[0][2]), (2, rotation[2][2])):
            product = multiply(matrix_value, radial)
            value = sar(product, 16)
            write_dword(
                buffer,
                state_offset + 0x42 + axis * 4,
                add(read_dword(buffer, state_offset + 0x42 + axis * 4), value),
            )
        product = multiply(rotation[1][2], radial)
        rounded = add(sar(product, 16), (product >> 15) & 1)
        write_dword(
            buffer,
            state_offset + 0x46,
            add(read_dword(buffer, state_offset + 0x46), rounded),
        )

        parent_offset = read_word(buffer, state_offset)
        local = [
            signed_word(read_word(buffer, state_offset + 0x42 + axis * 4))
            for axis in range(3)
        ]
        for row in (2, 1, 0):
            accumulator = 0
            for column in range(3):
                accumulator = add(
                    accumulator,
                    multiply(
                        read_dword(buffer, parent_offset + 0x12 + (row * 3 + column) * 4),
                        local[column],
                    ),
                )
            accumulator = add(
                accumulator, read_dword(buffer, parent_offset + 0x36 + row * 4)
            )
            write_dword(buffer, state_offset + 0x36 + row * 4, accumulator)

        for row in range(3):
            parent_1 = read_dword(buffer, parent_offset + 0x12 + (row * 3 + 1) * 4)
            parent_2 = read_dword(buffer, parent_offset + 0x12 + (row * 3 + 2) * 4)
            for column in range(3):
                accumulator = multiply(
                    read_dword(buffer, parent_offset + 0x12 + row * 12),
                    rotation[0][column],
                )
                accumulator = add(
                    accumulator, multiply(parent_1, rotation[1][column])
                )
                accumulator = add(
                    accumulator, multiply(parent_2, rotation[2][column])
                )
                write_dword(
                    buffer,
                    state_offset + 0x12 + (row * 3 + column) * 4,
                    sar(accumulator, 15),
                )
        return rotation

    flag_masks = {
        "cf": 0x0001,
        "pf": 0x0004,
        "af": 0x0010,
        "zf": 0x0040,
        "sf": 0x0080,
        "if": 0x0200,
        "df": 0x0400,
        "of": 0x0800,
    }

    for case_index, (name, state_count, states) in enumerate(cases):
        active_before = bytearray(
            ((offset * 37 + case_index * 19 + 11) & 0xFF)
            for offset in range(0x10000)
        )
        for trig_index in range(1024):
            component_0 = signed_word(trig_index * 197 + case_index * 991 + 17)
            component_1 = signed_word(trig_index * 389 + case_index * 577 + 91)
            struct.pack_into(
                "<hh", active_before, 0x0026 + trig_index * 4, component_0, component_1
            )
        write_word(active_before, 0x22F2, state_count)

        parent_offsets = (0x1800, 0x1900)
        for parent_index, parent_offset in enumerate(parent_offsets):
            for row in range(3):
                for column in range(3):
                    value = signed_dword(
                        0x10203040
                        + parent_index * 0x11111111
                        + row * 0x01020304
                        - column * 0x00112233
                    )
                    write_dword(
                        active_before,
                        parent_offset + 0x12 + (row * 3 + column) * 4,
                        value,
                    )
                write_dword(
                    active_before,
                    parent_offset + 0x36 + row * 4,
                    0x70000000 + parent_index * 0x08080808 + row * 0x1234567,
                )

        state_offsets = []
        for state_index, (angle_0, angle_1, angle_2, radial) in enumerate(states):
            state_offset = state_base + state_index * state_size
            state_offsets.append(state_offset)
            parent_offset = (
                state_base if state_index == 1 else parent_offsets[state_index]
            )
            write_word(active_before, state_offset, parent_offset)
            write_word(active_before, state_offset + 0x4E, angle_0)
            write_word(active_before, state_offset + 0x50, angle_1)
            write_word(active_before, state_offset + 0x52, angle_2)
            write_word(active_before, state_offset + 0x54, radial)
            for axis in range(3):
                write_dword(
                    active_before,
                    state_offset + 0x42 + axis * 4,
                    (
                        0x7FFF0001
                        + state_index * 0x80808080
                        + axis * 0x1111FFFE
                    ),
                )

        active_expected = bytearray(active_before)
        rotations = []
        for state_offset in state_offsets:
            rotations.append(model_state(active_expected, state_offset))
            remaining = (read_word(active_expected, 0x224A) - 1) & 0xFFFF
            write_word(active_expected, 0x224A, remaining)
        write_word(active_expected, 0x224A, 0)

        initial_flags = 0x0A93 | (0x0400 if case_index & 1 else 0)
        initial = {
            "eax": 0xA1A1BEEF + case_index,
            "ebx": 0xB2B22345 + case_index,
            "ecx": 0xC3C33456 + case_index,
            "edx": 0xD4D44567 + case_index,
            "esi": 0xE5E55678 + case_index,
            "edi": 0xF6F66789 + case_index,
            "ebp": 0x9797789A + case_index,
            "sp": 0xFF00,
            "ds": active_segment,
            "es": extra_segment,
            "fs": active_segment,
            "gs": game_segment,
            "ss": stack_segment,
            "flags": initial_flags,
        }
        active_decoy = bytes(
            (offset * 11 + case_index * 31 + 5) & 0xFF
            for offset in range(0x10000)
        )
        stack_sentinel = bytes.fromhex("5aa596698778")
        phases: list[dict[str, int]] = []

        def code_handler(
            machine: Uc, address: int, _size: int, _data: object
        ) -> None:
            if address in (0x0279, 0x0477, 0x0548):
                phases.append(
                    {
                        "address": address,
                        "di": machine.reg_read(UC_X86_REG_EDI) & 0xFFFF,
                        "remaining": struct.unpack(
                            "<H",
                            machine.mem_read(active_segment * 16 + 0x224A, 2),
                        )[0],
                    }
                )

        machine = execute(
            image,
            entry,
            return_address,
            initial,
            [
                (0, return_address, b"\xcc"),
                (active_segment, 0, bytes(active_before)),
                (extra_segment, 0, active_decoy),
                (game_segment, 0, active_decoy[::-1]),
                (
                    stack_segment,
                    0xFF00,
                    struct.pack("<H", return_address) + stack_sentinel,
                ),
            ],
            code_handler=code_handler,
            max_instructions=10000,
        )

        actual_active = bytes(machine.mem_read(active_segment * 16, 0x10000))
        if actual_active != active_expected:
            for offset, (actual, expected) in enumerate(
                zip(actual_active, active_expected)
            ):
                if actual != expected:
                    raise AssertionError(
                        f"{module}:{entry:#x} {name}: active byte {offset:#x} "
                        f"is {actual:#x}, expected {expected:#x}"
                    )
            raise AssertionError(f"{module}:{entry:#x} {name}: active memory differs")
        if bytes(machine.mem_read(extra_segment * 16, 0x10000)) != active_decoy:
            raise AssertionError(f"{module}:{entry:#x} {name}: ES decoy changed")
        if bytes(machine.mem_read(game_segment * 16, 0x10000)) != active_decoy[::-1]:
            raise AssertionError(f"{module}:{entry:#x} {name}: GS decoy changed")

        final_state = state_offsets[-1]
        final_parent = read_word(active_expected, final_state)
        final_rotation = rotations[-1]
        parent_row_2 = [
            read_dword(active_expected, final_parent + 0x12 + (2 * 3 + column) * 4)
            for column in range(3)
        ]
        final_accumulator = 0
        for index in range(3):
            final_accumulator = add(
                final_accumulator,
                multiply(parent_row_2[index], final_rotation[index][2]),
            )
        final_result = sar(final_accumulator, 15)
        final_local_y = signed_word(read_word(active_expected, final_state + 0x46))
        expected_registers = {
            "eax": multiply(final_rotation[2][2], parent_row_2[2]),
            "ebx": parent_row_2[1],
            "ecx": (0xFFFF0000 if final_local_y < 0 else 0),
            "edx": parent_row_2[2],
            "esi": (initial["esi"] & 0xFFFF0000)
            | ((final_parent + 0x36) & 0xFFFF),
            "edi": (initial["edi"] & 0xFFFF0000) | final_state,
            "ebp": final_result,
            "sp": 0xFF02,
            "ds": active_segment,
            "es": extra_segment,
            "fs": active_segment,
            "gs": game_segment,
            "ss": stack_segment,
        }
        for register, expected in expected_registers.items():
            actual = machine.reg_read(REGISTERS[register])
            if actual != expected:
                raise AssertionError(
                    f"{module}:{entry:#x} {name}: "
                    f"{register}={actual:#x}, expected={expected:#x}"
                )

        expected_phases = []
        for state_index, state_offset in enumerate(state_offsets):
            expected_phases.extend(
                (
                    {
                        "address": 0x0279,
                        "di": 0x2336 if state_index == 0 else state_offsets[state_index - 1],
                        "remaining": state_count - state_index,
                    },
                    {
                        "address": 0x0477,
                        "di": state_offset,
                        "remaining": state_count - state_index,
                    },
                )
            )
        expected_phases.append(
            {"address": 0x0548, "di": final_state, "remaining": 0}
        )
        if phases != expected_phases:
            raise AssertionError(
                f"{module}:{entry:#x} {name}: phases={phases}, "
                f"expected={expected_phases}"
            )

        expected_flags = {
            "cf": final_state + 0x36 > 0xFFFF,
            "pf": True,
            "af": False,
            "zf": True,
            "sf": False,
            "if": bool(initial_flags & 0x0200),
            "df": bool(initial_flags & 0x0400),
            "of": False,
        }
        flags_after = machine.reg_read(UC_X86_REG_EFLAGS)
        actual_flags = {
            flag: bool(flags_after & flag_masks[flag]) for flag in expected_flags
        }
        if actual_flags != expected_flags:
            raise AssertionError(
                f"{module}:{entry:#x} {name}: "
                f"flags={actual_flags}, expected={expected_flags}"
            )
        if bytes(machine.mem_read(stack_segment * 16 + 0xFF02, 6)) != stack_sentinel:
            raise AssertionError(f"{module}:{entry:#x} {name}: stack changed")

        vectors.append(
            {
                "name": name,
                "module": module,
                "entry": entry,
                "state_count": state_count,
                "state_offsets": state_offsets,
                "internal_transform_fallthroughs": state_count,
                "back_edges": state_count - 1,
                "final_rotation_matrix": [
                    signed_dword(value) for row in final_rotation for value in row
                ],
                "states_after": [
                    {
                        "matrix": [
                            signed_dword(
                                read_dword(
                                    active_expected,
                                    state_offset + 0x12 + (row * 3 + column) * 4,
                                )
                            )
                            for row in range(3)
                            for column in range(3)
                        ],
                        "translation": [
                            signed_dword(
                                read_dword(
                                    active_expected,
                                    state_offset + 0x36 + axis * 4,
                                )
                            )
                            for axis in range(3)
                        ],
                        "local_position": [
                            signed_dword(
                                read_dword(
                                    active_expected,
                                    state_offset + 0x42 + axis * 4,
                                )
                            )
                            for axis in range(3)
                        ],
                    }
                    for state_offset in state_offsets
                ],
                "defined_flags": expected_flags,
            }
        )

    return vectors


def manu3_init_protocol_vectors() -> list[dict[str, object]]:
    module = "manu3"
    entry = 0x0121
    image = load_image(module)
    expected_hash = "53ee04799c1a04e8fa75a5da3c3003e16a4c3de7a8b51ec0ab5c519b9363a0f7"
    if hashlib.sha256(image[entry : entry + 47]).hexdigest() != expected_hash:
        raise AssertionError(f"{module}:{entry:#x}: recovered 47-byte body changed")

    cases = (
        ("ordinary", 0x1000, 0x2000, (0x2000, 0x2000, 0x2000)),
        ("zero_work_deltas", 0x2000, 0x3000, (0x0000, 0x0000, 0x0000)),
        ("data_segment_wrap", 0xE000, 0x5000, (0x2000, 0x2000, 0x2000)),
        ("cumulative_wrap", 0x4000, 0x5000, (0x6000, 0x3000, 0x6000)),
        ("maximum_deltas", 0x6000, 0x3000, (0xFFFF, 0xFFFF, 0xFFFF)),
        ("final_zero_segment", 0x8000, 0x5000, (0x1000, 0x1000, 0x1000)),
    )
    initial_data_segment = 0x0100
    initial_extra_segment = 0x0200
    initial_fs_segment = 0x0300
    game_segment = 0x0400
    stack_segment = 0xA000
    return_segment = 0xF000
    return_address = 0xF000
    vectors = []

    for case_index, (
        name,
        code_segment,
        data_delta,
        work_deltas,
    ) in enumerate(cases):
        data_segment = (code_segment + data_delta) & 0xFFFF
        work_segments = []
        segment = data_segment
        for delta in work_deltas:
            segment = (segment + delta) & 0xFFFF
            work_segments.append(segment)
        final_segment = work_segments[-1]

        directory_before = bytearray(
            ((offset * 37 + case_index * 19 + 11) & 0xFF)
            for offset in range(18)
        )
        struct.pack_into("<HHH", directory_before, 0x0C, *work_deltas)
        directory_expected = bytearray(directory_before)
        struct.pack_into("<HHH", directory_expected, 0x02, *work_segments)
        continuation_before = bytes.fromhex("a1b2c3d4e5f60718")
        continuation_expected = (
            continuation_before[:2]
            + struct.pack("<H", 0x0AE0)
            + continuation_before[4:]
        )

        initial_flags = 0x0A93 | (0x0400 if case_index & 1 else 0)
        initial = {
            "eax": 0xA1A1BEEF + case_index,
            "ebx": 0xB2B22345 + case_index,
            "ecx": 0xC3C33456 + case_index,
            "edx": 0xD4D44567 + case_index,
            "esi": 0xE5E55678 + case_index,
            "edi": 0xF6F66789 + case_index,
            "ebp": 0x9797789A + case_index,
            "sp": 0xFEFE,
            "ds": initial_data_segment,
            "es": initial_extra_segment,
            "fs": initial_fs_segment,
            "gs": game_segment,
            "ss": stack_segment,
            "flags": initial_flags,
        }
        stack_sentinel = bytes.fromhex("5aa596698778")
        decoys = (
            bytes.fromhex("1021324354657687"),
            bytes.fromhex("89a9bacbdcedfe0f"),
            bytes.fromhex("f0e0d0c0b0a09080"),
        )
        code_window = bytearray(image[0x1360:0x1370])
        struct.pack_into("<H", code_window, 0x08, data_delta)
        struct.pack_into("<H", code_window, 0x0A, data_segment)

        machine = execute(
            image,
            entry,
            return_address,
            initial,
            [
                (code_segment, 0x1368, struct.pack("<HH", data_delta, 0xA55A)),
                (return_segment, return_address, b"\xcc"),
                (data_segment, 0, bytes(directory_before)),
                (final_segment, 0x067C, continuation_before),
                (initial_data_segment, 0x8000, decoys[0]),
                (initial_extra_segment, 0x8000, decoys[1]),
                (initial_fs_segment, 0x8000, decoys[2]),
                (
                    stack_segment,
                    0xFEFE,
                    struct.pack(
                        "<HHH",
                        initial_data_segment,
                        return_address,
                        return_segment,
                    )
                    + stack_sentinel,
                ),
            ],
            code_segment=code_segment,
            return_segment=return_segment,
        )

        expected_registers = dict(initial)
        del expected_registers["flags"]
        expected_registers["eax"] = (
            initial["eax"] & 0xFFFF0000
        ) | final_segment
        expected_registers["es"] = final_segment
        expected_registers["fs"] = data_segment
        expected_registers["sp"] = 0xFF04
        for register, expected in expected_registers.items():
            actual = machine.reg_read(REGISTERS[register])
            if actual != expected:
                raise AssertionError(
                    f"{module}:{entry:#x} {name}: "
                    f"{register}={actual:#x}, expected={expected:#x}"
                )
        if machine.reg_read(UC_X86_REG_CS) != return_segment:
            raise AssertionError(f"{module}:{entry:#x} {name}: far return CS differs")

        actual_directory = bytes(machine.mem_read(data_segment * 16, 18))
        if actual_directory != directory_expected:
            raise AssertionError(
                f"{module}:{entry:#x} {name}: relocation directory differs"
            )
        actual_continuation = bytes(
            machine.mem_read(final_segment * 16 + 0x067C, 8)
        )
        if actual_continuation != continuation_expected:
            raise AssertionError(
                f"{module}:{entry:#x} {name}: span continuation differs"
            )
        actual_code_window = bytes(
            machine.mem_read(code_segment * 16 + 0x1360, 16)
        )
        if actual_code_window != code_window:
            raise AssertionError(
                f"{module}:{entry:#x} {name}: code globals differ"
            )
        for segment_value, expected_decoy in zip(
            (initial_data_segment, initial_extra_segment, initial_fs_segment),
            decoys,
        ):
            actual_decoy = bytes(
                machine.mem_read(segment_value * 16 + 0x8000, 8)
            )
            if actual_decoy != expected_decoy:
                raise AssertionError(
                    f"{module}:{entry:#x} {name}: segment decoy changed"
                )
        if bytes(machine.mem_read(stack_segment * 16 + 0xFF04, 6)) != stack_sentinel:
            raise AssertionError(f"{module}:{entry:#x} {name}: stack sentinel changed")

        expected_flags = add_flags_16(
            work_segments[-2], work_deltas[-1], initial_flags
        )
        flag_masks = {
            "cf": 0x0001,
            "pf": 0x0004,
            "af": 0x0010,
            "zf": 0x0040,
            "sf": 0x0080,
            "if": 0x0200,
            "df": 0x0400,
            "of": 0x0800,
        }
        flags_after = machine.reg_read(UC_X86_REG_EFLAGS)
        actual_flags = {
            flag: bool(flags_after & flag_masks[flag]) for flag in expected_flags
        }
        if actual_flags != expected_flags:
            raise AssertionError(
                f"{module}:{entry:#x} {name}: flags={actual_flags}, "
                f"expected={expected_flags}"
            )

        vectors.append(
            {
                "name": name,
                "module": module,
                "entry": entry,
                "code_segment": code_segment,
                "data_delta": data_delta,
                "data_segment": data_segment,
                "work_deltas": list(work_deltas),
                "work_segments": work_segments,
                "span_continuation_offset": 0x0AE0,
                "data_segment_restored": True,
                "far_stack_bytes_consumed": 6,
                "defined_flags": expected_flags,
            }
        )

    return vectors


def manu3_frame_step_vectors() -> list[dict[str, object]]:
    module = "manu3"
    entry = 0x0150
    callees = (0x019B, 0x0270, 0x0549, 0x06F6)
    return_ips = (0x0171, 0x0174, 0x0177, 0x017A)
    image = load_image(module)
    expected_hash = "5b722c6d62fdc873ebb82a18e20efcbd82febab4f0e954f6bdfab7b805fc09af"
    if hashlib.sha256(image[entry : entry + 44]).hexdigest() != expected_hash:
        raise AssertionError(f"{module}:{entry:#x}: recovered 44-byte body changed")

    patched_image = bytearray(image)
    for callee in callees:
        patched_image[callee] = 0xC3

    cases = (
        ("inactive", 0x0000, 0x1234),
        ("zero_offset", 0x2400, 0x0000),
        ("subparagraph", 0x4400, 0x000F),
        ("one_paragraph", 0x6400, 0x0010),
        ("high_offset", 0x8400, 0xFFF0),
        ("maximum_offset", 0xA400, 0xFFFF),
        ("high_bit_segment", 0xC400, 0xA55A),
    )
    initial_data_segment = 0xE400
    initial_extra_segment = 0x1400
    initial_fs_segment = 0xF400
    stack_segment = 0xD400
    return_address = 0xF000
    vectors = []

    for case_index, (name, active_segment, window_offset) in enumerate(cases):
        stack_sentinel = bytes.fromhex("5aa596698778")
        active_before = bytearray(
            ((offset * 37 + case_index * 19 + 11) & 0xFF)
            for offset in range(0x10000)
        )
        active_expected = bytearray(active_before)
        framebuffer_segment = (0xA000 + (window_offset >> 4)) & 0xFFFF
        if active_segment != 0:
            struct.pack_into("<H", active_expected, 0x0018, framebuffer_segment)

        initial_flags = 0x0A93 | (0x0400 if case_index & 1 else 0)
        initial = {
            "eax": 0xA1A1BEEF + case_index,
            "ebx": 0xB2B22345 + case_index,
            "ecx": 0xC3C33456 + case_index,
            "edx": 0xD4D44567 + case_index,
            "esi": 0xE5E55678 + case_index,
            "edi": 0xF6F66789 + case_index,
            "ebp": 0x9797789A + case_index,
            "sp": 0xFF00,
            "ds": initial_data_segment,
            "es": initial_extra_segment,
            "fs": initial_fs_segment,
            "gs": 0x3400,
            "ss": stack_segment,
            "flags": initial_flags,
        }
        initial_data_before = bytes.fromhex("1021324354657687")
        initial_extra_before = bytes.fromhex("89a9bacbdcedfe0f")
        initial_fs_before = bytes.fromhex("f0e0d0c0b0a09080")
        call_entries: list[dict[str, int]] = []

        def code_handler(
            machine: Uc, address: int, _size: int, _data: object
        ) -> None:
            if address in callees:
                call_index = callees.index(address)
                call_entries.append(
                    {
                        "callee": address,
                        "return_ip": struct.unpack(
                            "<H",
                            machine.mem_read(stack_segment * 16 + 0xFEFC, 2),
                        )[0],
                        "sp": machine.reg_read(UC_X86_REG_SP),
                        "ds": machine.reg_read(UC_X86_REG_DS),
                        "es": machine.reg_read(UC_X86_REG_ES),
                        "fs": machine.reg_read(UC_X86_REG_FS),
                        "framebuffer_segment": struct.unpack(
                            "<H",
                            machine.mem_read(active_segment * 16 + 0x0018, 2),
                        )[0],
                        "call_index": call_index,
                    }
                )

        memory = [
            (0, 0x136A, struct.pack("<H", active_segment)),
            (0, return_address, b"\xcc"),
            (initial_data_segment, 0x0018, initial_data_before),
            (initial_extra_segment, 0x0018, initial_extra_before),
            (initial_fs_segment, 0x0018, initial_fs_before),
            (stack_segment, 0x20CE, struct.pack("<H", window_offset)),
            (
                stack_segment,
                0xFF00,
                struct.pack("<HH", return_address, 0) + stack_sentinel,
            ),
        ]
        if active_segment != 0:
            memory.append((active_segment, 0, bytes(active_before)))

        machine = execute(
            bytes(patched_image),
            entry,
            return_address,
            initial,
            memory,
            code_handler=code_handler,
        )

        if active_segment == 0:
            expected_calls: list[dict[str, int]] = []
        else:
            expected_calls = [
                {
                    "callee": callee,
                    "return_ip": return_ips[call_index],
                    "sp": 0xFEFC,
                    "ds": active_segment,
                    "es": active_segment,
                    "fs": active_segment,
                    "framebuffer_segment": framebuffer_segment,
                    "call_index": call_index,
                }
                for call_index, callee in enumerate(callees)
            ]
        if call_entries != expected_calls:
            raise AssertionError(
                f"{module}:{entry:#x} {name}: calls={call_entries}, "
                f"expected={expected_calls}"
            )

        expected_registers = dict(initial)
        del expected_registers["flags"]
        expected_registers["ecx"] = (
            initial["ecx"] & 0xFFFF0000
        ) | active_segment
        expected_registers["sp"] = 0xFF04
        if active_segment == 0:
            expected_flags = _logical_flags_16(0, initial_flags)
        else:
            expected_registers["eax"] = (
                initial["eax"] & 0xFFFF0000
            ) | framebuffer_segment
            expected_registers["es"] = active_segment
            expected_registers["fs"] = active_segment
            expected_flags = add_flags_8(
                window_offset >> 12, 0xA0, initial_flags
            )

        for register, expected in expected_registers.items():
            actual = machine.reg_read(REGISTERS[register])
            if actual != expected:
                raise AssertionError(
                    f"{module}:{entry:#x} {name}: "
                    f"{register}={actual:#x}, expected={expected:#x}"
                )
        if machine.reg_read(UC_X86_REG_CS) != 0:
            raise AssertionError(f"{module}:{entry:#x} {name}: far return CS changed")

        if active_segment != 0:
            actual_active = bytes(machine.mem_read(active_segment * 16, 0x10000))
            if actual_active != active_expected:
                raise AssertionError(
                    f"{module}:{entry:#x} {name}: active data segment changed"
                )
        if (
            bytes(machine.mem_read(initial_data_segment * 16 + 0x0018, 8))
            != initial_data_before
        ):
            raise AssertionError(f"{module}:{entry:#x} {name}: initial DS changed")
        if (
            bytes(machine.mem_read(initial_extra_segment * 16 + 0x0018, 8))
            != initial_extra_before
        ):
            raise AssertionError(f"{module}:{entry:#x} {name}: initial ES changed")
        if (
            bytes(machine.mem_read(initial_fs_segment * 16 + 0x0018, 8))
            != initial_fs_before
        ):
            raise AssertionError(f"{module}:{entry:#x} {name}: initial FS changed")
        if bytes(machine.mem_read(stack_segment * 16 + 0xFF04, 6)) != stack_sentinel:
            raise AssertionError(f"{module}:{entry:#x} {name}: stack sentinel changed")
        if (
            struct.unpack(
                "<H", machine.mem_read(stack_segment * 16 + 0x20CE, 2)
            )[0]
            != window_offset
        ):
            raise AssertionError(f"{module}:{entry:#x} {name}: SS input changed")
        if struct.unpack("<H", machine.mem_read(0x136A, 2))[0] != active_segment:
            raise AssertionError(f"{module}:{entry:#x} {name}: CS segment word changed")

        flag_masks = {
            "cf": 0x0001,
            "pf": 0x0004,
            "af": 0x0010,
            "zf": 0x0040,
            "sf": 0x0080,
            "if": 0x0200,
            "df": 0x0400,
            "of": 0x0800,
        }
        flags_after = machine.reg_read(UC_X86_REG_EFLAGS)
        actual_flags = {
            flag: bool(flags_after & flag_masks[flag]) for flag in expected_flags
        }
        if actual_flags != expected_flags:
            raise AssertionError(
                f"{module}:{entry:#x} {name}: flags={actual_flags}, "
                f"expected={expected_flags}"
            )

        vectors.append(
            {
                "name": name,
                "module": module,
                "entry": entry,
                "active_data_segment": active_segment,
                "framebuffer_window_offset": window_offset,
                "framebuffer_segment": (
                    framebuffer_segment if active_segment != 0 else None
                ),
                "ordered_callees": list(callees) if active_segment != 0 else [],
                "segments_installed": active_segment != 0,
                "far_stack_bytes_consumed": 4,
                "defined_flags": expected_flags,
            }
        )

    return vectors


def manu3_anim_select_entry_vectors() -> list[dict[str, object]]:
    module = "manu3"
    entry = 0x017C
    callee = 0x0181
    image = load_image(module)
    expected_hash = "5ca59432c288b57ddf6d9c6a31b795f0fdf2b9dee14c83df53eb84133420e2f8"
    if hashlib.sha256(image[entry : entry + 4]).hexdigest() != expected_hash:
        raise AssertionError(f"{module}:{entry:#x}: recovered 4-byte body changed")

    patched_image = bytearray(image)
    patched_image[callee] = 0xC3
    stack_segment = 0x9000
    return_address = 0xF000
    selectors = (0x0000, 0x001F, 0x0020, 0xFFFF)
    vectors = []

    for case_index, selector in enumerate(selectors):
        stack_sentinel = bytes.fromhex("5aa596698778")
        initial = {
            "eax": 0xA1A1BEEF + case_index,
            "ebx": 0xB2B20000 | selector,
            "ecx": 0xC3C33456 + case_index,
            "edx": 0xD4D44567 + case_index,
            "esi": 0xE5E55678 + case_index,
            "edi": 0xF6F66789 + case_index,
            "ebp": 0x9797789A + case_index,
            "sp": 0xFF00,
            "ds": 0x4400,
            "es": 0x4800,
            "fs": 0x4C00,
            "gs": 0x2C00,
            "ss": stack_segment,
            "flags": 0x0A93 | (0x0400 if case_index & 1 else 0),
        }
        callee_entries: list[dict[str, int]] = []

        def code_handler(
            machine: Uc, address: int, _size: int, _data: object
        ) -> None:
            if address == callee:
                callee_entries.append(
                    {
                        "bx": machine.reg_read(UC_X86_REG_EBX) & 0xFFFF,
                        "sp": machine.reg_read(UC_X86_REG_SP),
                        "return_ip": struct.unpack(
                            "<H",
                            machine.mem_read(stack_segment * 16 + 0xFEFE, 2),
                        )[0],
                    }
                )

        machine = execute(
            bytes(patched_image),
            entry,
            return_address,
            initial,
            [
                (0, return_address, b"\xcc"),
                (
                    stack_segment,
                    0xFF00,
                    struct.pack("<HH", return_address, 0) + stack_sentinel,
                ),
            ],
            code_handler=code_handler,
        )
        expected_entry = {"bx": selector, "sp": 0xFEFE, "return_ip": 0x017F}
        if callee_entries != [expected_entry]:
            raise AssertionError(
                f"{module}:{entry:#x} selector_{selector:04x}: "
                f"callee entries={callee_entries}, expected={[expected_entry]}"
            )

        expected_registers = dict(initial)
        del expected_registers["flags"]
        expected_registers["sp"] = 0xFF04
        for register, expected in expected_registers.items():
            actual = machine.reg_read(REGISTERS[register])
            if actual != expected:
                raise AssertionError(
                    f"{module}:{entry:#x} selector_{selector:04x}: "
                    f"{register}={actual:#x}, expected={expected:#x}"
                )
        if machine.reg_read(UC_X86_REG_CS) != 0:
            raise AssertionError(f"{module}:{entry:#x}: far return CS changed")
        if bytes(machine.mem_read(stack_segment * 16 + 0xFF04, 6)) != stack_sentinel:
            raise AssertionError(f"{module}:{entry:#x}: stack sentinel changed")
        flags_after = machine.reg_read(UC_X86_REG_EFLAGS)
        if (flags_after & 0x0ED5) != (initial["flags"] & 0x0ED5):
            raise AssertionError(f"{module}:{entry:#x}: flags changed")

        vectors.append(
            {
                "name": f"selector_{selector:04x}",
                "module": module,
                "entry": entry,
                "selector": selector,
                "near_callee": callee,
                "near_return_ip": 0x017F,
                "far_stack_bytes_consumed": 4,
            }
        )

    return vectors


def manu3_anim_select_vectors() -> list[dict[str, object]]:
    module = "manu3"
    entry = 0x0181
    constructor = 0x01DF
    image = load_image(module)
    expected_hash = "2c95a4b6fd3aaae13c30793487b21286b267e557b3b8ab231caf4802fde54c61"
    if hashlib.sha256(image[entry : entry + 26]).hexdigest() != expected_hash:
        raise AssertionError(f"{module}:{entry:#x}: recovered 26-byte body changed")

    patched_image = bytearray(image)
    patched_image[constructor] = 0xC3
    cases = (
        ("first", 0x0000, 0x3000, 0x0123),
        ("last", 0x001F, 0x3400, 0xFEDC),
        ("masked_zero", 0x0020, 0x3800, 0x8000),
        ("masked_last", 0xFFFF, 0xFFC0, 0x0080),
    )
    data_segment = 0x4400
    stack_segment = 0x9000
    return_address = 0xF000
    vectors = []

    for case_index, (name, selector, table_offset, relative_offset) in enumerate(cases):
        stack_sentinel = bytes.fromhex("5aa596698778")
        data_before = bytearray(
            ((offset * 37 + case_index * 19 + 11) & 0xFF)
            for offset in range(0x10000)
        )
        table_entry = (table_offset + ((selector & 0x1F) * 2)) & 0xFFFF
        struct.pack_into("<H", data_before, 0x2306, table_offset)
        struct.pack_into("<H", data_before, table_entry, relative_offset)
        data_expected = bytearray(data_before)
        script_offset = (table_offset + relative_offset) & 0xFFFF
        struct.pack_into("<H", data_expected, 0x102C, 0)
        struct.pack_into("<H", data_expected, 0x102E, script_offset)
        decoy = bytes(
            ((offset * 13 + case_index * 29 + 7) & 0xFF)
            for offset in range(0x10000)
        )

        initial = {
            "eax": 0xA1A1BEEF + case_index,
            "ebx": 0xB2B20000 | selector,
            "ecx": 0xC3C33456 + case_index,
            "edx": 0xD4D44567 + case_index,
            "esi": 0xE5E55678 + case_index,
            "edi": 0xF6F66789 + case_index,
            "ebp": 0x9797789A + case_index,
            "sp": 0xFF00,
            "ds": data_segment,
            "es": 0x6000,
            "fs": 0x4C00,
            "gs": 0x7000,
            "ss": stack_segment,
            "flags": 0x0A93 | (0x0400 if case_index & 1 else 0),
        }
        constructor_entries: list[dict[str, int]] = []

        def code_handler(
            machine: Uc, address: int, _size: int, _data: object
        ) -> None:
            if address == constructor:
                constructor_entries.append(
                    {
                        "bx": machine.reg_read(UC_X86_REG_EBX) & 0xFFFF,
                        "di": machine.reg_read(UC_X86_REG_EDI) & 0xFFFF,
                        "sp": machine.reg_read(UC_X86_REG_SP),
                    }
                )

        machine = execute(
            bytes(patched_image),
            entry,
            return_address,
            initial,
            [
                (0, return_address, b"\xcc"),
                (0x6000, 0, decoy),
                (0x7000, 0, decoy),
                (data_segment, 0, bytes(data_before)),
                (
                    stack_segment,
                    0xFF00,
                    struct.pack("<H", return_address)
                    + stack_sentinel,
                ),
            ],
            code_handler=code_handler,
        )
        expected_entry = {"bx": 0x1032, "di": script_offset, "sp": 0xFF00}
        if constructor_entries != [expected_entry]:
            raise AssertionError(
                f"{module}:{entry:#x} {name}: constructor entries="
                f"{constructor_entries}, expected={[expected_entry]}"
            )

        expected_registers = dict(initial)
        del expected_registers["flags"]
        expected_registers["ebx"] = (initial["ebx"] & 0xFFFF0000) | 0x1032
        expected_registers["edi"] = (initial["edi"] & 0xFFFF0000) | script_offset
        expected_registers["sp"] = 0xFF02
        for register, expected in expected_registers.items():
            actual = machine.reg_read(REGISTERS[register])
            if actual != expected:
                raise AssertionError(
                    f"{module}:{entry:#x} {name}: "
                    f"{register}={actual:#x}, expected={expected:#x}"
                )
        if bytes(machine.mem_read(data_segment * 16, 0x10000)) != data_expected:
            raise AssertionError(f"{module}:{entry:#x} {name}: data segment differs")
        if bytes(machine.mem_read(0x6000 * 16, 0x10000)) != decoy:
            raise AssertionError(f"{module}:{entry:#x} {name}: ES decoy changed")
        if bytes(machine.mem_read(0x7000 * 16, 0x10000)) != decoy:
            raise AssertionError(f"{module}:{entry:#x} {name}: GS decoy changed")
        if bytes(machine.mem_read(stack_segment * 16 + 0xFF02, 6)) != stack_sentinel:
            raise AssertionError(f"{module}:{entry:#x} {name}: stack sentinel changed")

        expected_flags = add_flags_16(table_offset, relative_offset, initial["flags"])
        flags_after = machine.reg_read(UC_X86_REG_EFLAGS)
        flag_masks = {
            "cf": 0x0001,
            "pf": 0x0004,
            "af": 0x0010,
            "zf": 0x0040,
            "sf": 0x0080,
            "if": 0x0200,
            "df": 0x0400,
            "of": 0x0800,
        }
        actual_flags = {
            flag: bool(flags_after & mask) for flag, mask in flag_masks.items()
        }
        if actual_flags != expected_flags:
            raise AssertionError(
                f"{module}:{entry:#x} {name}: flags={actual_flags}, "
                f"expected={expected_flags}"
            )

        vectors.append(
            {
                "name": name,
                "module": module,
                "entry": entry,
                "selector": selector,
                "masked_selector": selector & 0x1F,
                "table_offset": table_offset,
                "relative_offset": relative_offset,
                "script_offset": script_offset,
                "constructor_tail_jump": constructor,
                "defined_flags": expected_flags,
            }
        )

    return vectors


def _signed_16(value: int) -> int:
    value &= 0xFFFF
    return value - 0x10000 if value & 0x8000 else value


def _logical_flags_16(result: int, initial_flags: int) -> dict[str, bool]:
    result &= 0xFFFF
    return {
        "cf": False,
        "pf": (result & 0xFF).bit_count() % 2 == 0,
        "zf": result == 0,
        "sf": bool(result & 0x8000),
        "if": bool(initial_flags & 0x0200),
        "df": bool(initial_flags & 0x0400),
        "of": False,
    }


def manu3_tween_step_vectors() -> list[dict[str, object]]:
    module = "manu3"
    entry = 0x019B
    constructor = 0x01DF
    image = load_image(module)
    expected_hash = "9072490ce643cd0fb2f7f955bf33ac5ffd75d4d3be30942b725a43991855a42d"
    if hashlib.sha256(image[entry : entry + 163]).hexdigest() != expected_hash:
        raise AssertionError(f"{module}:{entry:#x}: recovered 163-byte body changed")

    patched_image = bytearray(image)
    patched_image[constructor] = 0xC3
    cases = (
        {
            "name": "paused_high_phase",
            "phase": 0x0107,
            "records": (
                (0x4000, 3, 0x5000, 0x12345678, 0x00102030, 0xA55A),
            ),
        },
        {
            "name": "paused_negative_high_phase",
            "phase": 0xFF00,
            "records": (
                (0x4020, 0, 0x5010, 0x87654321, 0xFFEEDDCC, 0x5AA5),
            ),
        },
        {"name": "empty", "phase": 0x0007, "records": ()},
        {
            "name": "one_live_wrapping_accumulator",
            "phase": 0x00A5,
            "records": (
                (0x4040, 1, 0x5020, 0xFFFE8000, 0x00018001, 0x1357),
            ),
        },
        {
            "name": "one_expired",
            "phase": 0x0001,
            "records": (
                (0x4060, 0, 0x5030, 0x80011234, 0x7FFFFFFF, 0x2468),
            ),
        },
        {
            "name": "first_expired_last_swapped_and_live",
            "phase": 0x0002,
            "records": (
                (0x4080, 0, 0x5040, 0x11112222, 0x33334444, 0xAAAA),
                (0x40A0, 2, 0x5050, 0xFFFF0001, 0x0001FFFF, 0xBBBB),
            ),
        },
        {
            "name": "last_expired_after_live",
            "phase": 0x0003,
            "records": (
                (0x40C0, 0x8000, 0x5060, 0x7FFF8000, 0x80010000, 0xCCCC),
                (0x40E0, 0, 0x5070, 0xFEDCBA98, 0x01020304, 0xDDDD),
            ),
        },
        {
            "name": "middle_expired_replacement_processed",
            "phase": 0x0004,
            "records": (
                (0x4100, 1, 0x5080, 0x0000FFFF, 0x00000002, 0x1111),
                (0x4120, 0, 0x5090, 0xFFFF8000, 0x11111111, 0x2222),
                (0x4140, 0x7FFF, 0x50A0, 0x80000000, 0xFFFFFFFF, 0x3333),
            ),
        },
    )
    data_segment = 0x4400
    initial_data_segment = 0x6000
    extra_segment = 0x7000
    stack_segment = 0x9000
    return_address = 0xF000
    active_base = 0x1032
    vectors = []

    for case_index, case in enumerate(cases):
        stack_sentinel = bytes.fromhex("5aa596698778")
        data_before = bytearray(
            ((offset * 37 + case_index * 19 + 11) & 0xFF)
            for offset in range(0x10000)
        )
        records = case["records"]
        phase = int(case["phase"])
        active_end = active_base + len(records) * 2
        struct.pack_into("<H", data_before, 0x102C, phase)
        struct.pack_into("<H", data_before, 0x1030, active_end)
        for record_index, record_values in enumerate(records):
            (
                record_offset,
                counter,
                target_offset,
                accumulator,
                step,
                target_before,
            ) = record_values
            struct.pack_into(
                "<HHHII",
                data_before,
                record_offset,
                counter,
                0x9000 + record_index,
                target_offset,
                accumulator,
                step,
            )
            struct.pack_into("<H", data_before, target_offset, target_before)
            struct.pack_into(
                "<H", data_before, active_base + record_index * 2, record_offset
            )

        data_expected = bytearray(data_before)
        slot_values = [int(record[0]) for record in records]
        record_metadata = {
            int(record[0]): {
                "counter_before": int(record[1]),
                "target_offset": int(record[2]),
                "accumulator_before": int(record[3]),
                "step": int(record[4]),
            }
            for record in records
        }
        initial_flags = 0x0A93 | (0x0400 if case_index & 1 else 0)
        initial = {
            "eax": 0xA1A1BEEF + case_index,
            "ebx": 0xB2B22345 + case_index,
            "ecx": 0xC3C33456 + case_index,
            "edx": 0xD4D44567 + case_index,
            "esi": 0xE5E55678 + case_index,
            "edi": 0xF6F66789 + case_index,
            "ebp": 0x9797789A + case_index,
            "sp": 0xFF00,
            "ds": initial_data_segment,
            "es": extra_segment,
            "fs": data_segment,
            "gs": 0x7800,
            "ss": stack_segment,
            "flags": initial_flags,
        }
        expected_registers = dict(initial)
        del expected_registers["flags"]
        expected_registers["ds"] = data_segment
        expected_registers["sp"] = 0xFF02
        constructor_expected: list[dict[str, int]] = []
        record_results = []

        if phase & 0xFF00:
            expected_flags = _logical_flags_16(phase & 0xFF00, initial_flags)
        else:
            cursor_index = 0
            end_index = len(slot_values)
            eax = initial["eax"]
            edi = initial["edi"]
            ebp = initial["ebp"]
            while cursor_index != end_index:
                record_offset = slot_values[cursor_index]
                metadata = record_metadata[record_offset]
                counter_before = struct.unpack_from(
                    "<H", data_expected, record_offset
                )[0]
                accumulator_before = struct.unpack_from(
                    "<I", data_expected, record_offset + 6
                )[0]
                target_offset = int(metadata["target_offset"])
                value = (accumulator_before >> 16) & 0xFFFF
                struct.pack_into("<H", data_expected, target_offset, value)
                counter_after = (counter_before - 1) & 0xFFFF
                struct.pack_into("<H", data_expected, record_offset, counter_after)
                edi = (edi & 0xFFFF0000) | record_offset
                ebp = (ebp & 0xFFFF0000) | target_offset
                eax = (eax & 0xFFFF0000) | value
                expired = bool(counter_after & 0x8000)

                if expired:
                    end_index -= 1
                    replacement = slot_values[end_index]
                    slot_values[end_index] = record_offset
                    slot_values[cursor_index] = replacement
                    struct.pack_into(
                        "<H",
                        data_expected,
                        active_base + end_index * 2,
                        record_offset,
                    )
                    struct.pack_into(
                        "<H",
                        data_expected,
                        active_base + cursor_index * 2,
                        replacement,
                    )
                    edi = (edi & 0xFFFF0000) | replacement
                    accumulator_after = accumulator_before
                else:
                    step = int(metadata["step"])
                    accumulator_after = (accumulator_before + step) & 0xFFFFFFFF
                    struct.pack_into(
                        "<I", data_expected, record_offset + 6, accumulator_after
                    )
                    eax = step
                    cursor_index += 1

                record_results.append(
                    {
                        "record_offset": record_offset,
                        "target_offset": target_offset,
                        "published_value": _signed_16(value),
                        "counter_before": counter_before,
                        "counter_after": counter_after,
                        "accumulator_before": accumulator_before,
                        "accumulator_after": accumulator_after,
                        "expired": expired,
                    }
                )

            final_cursor = active_base + cursor_index * 2
            final_end = active_base + end_index * 2
            expected_registers["eax"] = eax
            expected_registers["ebx"] = (
                initial["ebx"] & 0xFFFF0000
            ) | final_end
            expected_registers["esi"] = (
                initial["esi"] & 0xFFFF0000
            ) | final_cursor
            expected_registers["edi"] = edi
            expected_registers["ebp"] = ebp
            expected_flags = sub_flags_16(final_cursor, final_end, initial_flags)
            constructor_expected = [
                {
                    "bx": final_end,
                    "si": final_cursor,
                    "sp": 0xFF00,
                    "ds": data_segment,
                }
            ]

        decoy = bytes(
            ((offset * 13 + case_index * 29 + 7) & 0xFF)
            for offset in range(0x10000)
        )
        constructor_entries: list[dict[str, int]] = []

        def code_handler(
            machine: Uc, address: int, _size: int, _data: object
        ) -> None:
            if address == constructor:
                constructor_entries.append(
                    {
                        "bx": machine.reg_read(UC_X86_REG_EBX) & 0xFFFF,
                        "si": machine.reg_read(UC_X86_REG_ESI) & 0xFFFF,
                        "sp": machine.reg_read(UC_X86_REG_SP),
                        "ds": machine.reg_read(UC_X86_REG_DS),
                    }
                )

        machine = execute(
            bytes(patched_image),
            entry,
            return_address,
            initial,
            [
                (0, return_address, b"\xcc"),
                (initial_data_segment, 0, decoy),
                (extra_segment, 0, decoy),
                (data_segment, 0, bytes(data_before)),
                (
                    stack_segment,
                    0xFF00,
                    struct.pack("<H", return_address) + stack_sentinel,
                ),
            ],
            code_handler=code_handler,
        )
        if constructor_entries != constructor_expected:
            raise AssertionError(
                f"{module}:{entry:#x} {case['name']}: constructor entries="
                f"{constructor_entries}, expected={constructor_expected}"
            )
        for register, expected in expected_registers.items():
            actual = machine.reg_read(REGISTERS[register])
            if actual != expected:
                raise AssertionError(
                    f"{module}:{entry:#x} {case['name']}: "
                    f"{register}={actual:#x}, expected={expected:#x}"
                )
        actual_data = bytes(machine.mem_read(data_segment * 16, 0x10000))
        if actual_data != data_expected:
            difference = next(
                offset
                for offset, (actual, expected) in enumerate(
                    zip(actual_data, data_expected)
                )
                if actual != expected
            )
            raise AssertionError(
                f"{module}:{entry:#x} {case['name']}: data differs at "
                f"{difference:#06x}: actual={actual_data[difference]:#04x}, "
                f"expected={data_expected[difference]:#04x}"
            )
        if bytes(machine.mem_read(initial_data_segment * 16, 0x10000)) != decoy:
            raise AssertionError(
                f"{module}:{entry:#x} {case['name']}: initial DS decoy changed"
            )
        if bytes(machine.mem_read(extra_segment * 16, 0x10000)) != decoy:
            raise AssertionError(
                f"{module}:{entry:#x} {case['name']}: ES decoy changed"
            )
        if bytes(machine.mem_read(stack_segment * 16 + 0xFF02, 6)) != stack_sentinel:
            raise AssertionError(
                f"{module}:{entry:#x} {case['name']}: stack sentinel changed"
            )

        flag_masks = {
            "cf": 0x0001,
            "pf": 0x0004,
            "af": 0x0010,
            "zf": 0x0040,
            "sf": 0x0080,
            "if": 0x0200,
            "df": 0x0400,
            "of": 0x0800,
        }
        flags_after = machine.reg_read(UC_X86_REG_EFLAGS)
        actual_flags = {
            flag: bool(flags_after & flag_masks[flag]) for flag in expected_flags
        }
        if actual_flags != expected_flags:
            raise AssertionError(
                f"{module}:{entry:#x} {case['name']}: flags={actual_flags}, "
                f"expected={expected_flags}"
            )

        vectors.append(
            {
                "name": case["name"],
                "module": module,
                "entry": entry,
                "phase": phase,
                "active_end_before": active_end,
                "constructor_active_end": (
                    constructor_expected[0]["bx"] if constructor_expected else None
                ),
                "active_slots_after": slot_values,
                "record_steps": record_results,
                "ds_loaded_from_fs": True,
                "constructor_tail_jump": bool(constructor_expected),
                "defined_flags": expected_flags,
            }
        )

    return vectors


def manu3_entity_project_vectors() -> list[dict[str, object]]:
    module = "manu3"
    entry = 0x0549
    image = load_image(module)
    expected_hash = "b8b58b8148911c130bebdc2455fea987390e6c8f291d8521527f58596dbb41f7"
    if hashlib.sha256(image[entry : entry + 368]).hexdigest() != expected_hash:
        raise AssertionError(f"{module}:{entry:#x}: recovered 368-byte body changed")

    zero_matrix = ((0, 0, 0), (0, 0, 0), (0, 0, 0))
    cases = (
        ("ordinary_centered", (10, 5), zero_matrix, (100, -50, 256), ((0, 0, 0),)),
        ("zero_depth_rejected", (0, 0), zero_matrix, (20, 30, 0), ((1, 2, 3),)),
        (
            "negative_depth_rejected",
            (0, 0),
            zero_matrix,
            (20, 30, -256),
            ((-1, -2, -3),),
        ),
        ("left_clip", (0, 0), zero_matrix, (-1, 0, 256), ((0, 0, 0),)),
        ("left_clamp", (0, 0), zero_matrix, (-40, 0, 256), ((0, 0, 0),)),
        ("right_clip", (0, 0), zero_matrix, (320, 0, 256), ((0, 0, 0),)),
        ("right_clamp", (0, 0), zero_matrix, (360, 0, 256), ((0, 0, 0),)),
        ("top_clip", (0, 0), zero_matrix, (0, 1, 256), ((0, 0, 0),)),
        ("top_clamp", (0, 0), zero_matrix, (0, 100, 256), ((0, 0, 0),)),
        ("bottom_clip", (0, 0), zero_matrix, (0, -200, 256), ((0, 0, 0),)),
        ("bottom_clamp", (0, 0), zero_matrix, (0, -300, 256), ((0, 0, 0),)),
        (
            "matrix_vertex_traversal",
            (0, 0),
            ((1, 0, 0), (0, 1, 0), (0, 0, 0)),
            (0, 0, 256),
            ((10, -20, 7), (319, -199, -8), (-1, 1, 9)),
        ),
    )
    active_segment = 0x6000
    geometry_segment = 0x4400
    extra_segment = 0x7000
    game_segment = 0x7800
    stack_segment = 0x9000
    return_address = 0xF000
    state_offset = 0x2394
    vectors = []

    def read_word(buffer: bytearray, offset: int) -> int:
        offset &= 0xFFFF
        return buffer[offset] | (buffer[offset + 1] << 8)

    def write_word(buffer: bytearray, offset: int, value: int) -> None:
        offset &= 0xFFFF
        buffer[offset] = value & 0xFF
        buffer[offset + 1] = (value >> 8) & 0xFF

    def read_dword(buffer: bytearray, offset: int) -> int:
        offset &= 0xFFFF
        return int.from_bytes(buffer[offset : offset + 4], "little")

    def write_dword(buffer: bytearray, offset: int, value: int) -> None:
        offset &= 0xFFFF
        buffer[offset : offset + 4] = (value & 0xFFFFFFFF).to_bytes(4, "little")

    def signed_word(value: int) -> int:
        value &= 0xFFFF
        return value if value < 0x8000 else value - 0x10000

    def signed_dword(value: int) -> int:
        value &= 0xFFFFFFFF
        return value if value < 0x80000000 else value - 0x100000000

    def multiply_dword(left: int, right: int) -> int:
        return ((left & 0xFFFFFFFF) * (right & 0xFFFFFFFF)) & 0xFFFFFFFF

    def divide_signed(dividend: int, divisor: int) -> tuple[int, int]:
        left = signed_dword(dividend)
        right = signed_dword(divisor)
        quotient = abs(left) // abs(right)
        if (left < 0) != (right < 0):
            quotient = -quotient
        remainder = left - quotient * right
        return quotient & 0xFFFFFFFF, remainder & 0xFFFFFFFF

    def model_project(
        active: bytearray,
        geometry: bytearray,
        initial: dict[str, int],
    ) -> tuple[dict[str, int], dict[str, bool]]:
        eax = initial["eax"]
        ebx = initial["ebx"]
        ecx = initial["ecx"]
        edx = initial["edx"]
        esi = initial["esi"]
        edi = (initial["edi"] & 0xFFFF0000) | 0x2336
        ebp = initial["ebp"]
        outer_count = read_word(active, 0x22F2)
        ecx = (ecx & 0xFFFF0000) | outer_count
        outer_iterations = outer_count if outer_count != 0 else 0x10000
        final_vertex_add_left = 0

        for _state_index in range(outer_iterations):
            saved_outer_count = ecx & 0xFFFF
            edi = (edi & 0xFFFF0000) | (((edi & 0xFFFF) + 0x005E) & 0xFFFF)
            current_state = edi & 0xFFFF
            inner_count = read_word(active, current_state + 2)
            ecx = (ecx & 0xFFFF0000) | inner_count
            esi = (esi & 0xFFFF0000) | read_word(active, current_state + 6)
            write_word(active, 0x224A, inner_count)
            write_word(active, 0x224E, 0)
            inner_iterations = inner_count if inner_count != 0 else 0x10000

            for _vertex_index in range(inner_iterations):
                vertex = esi & 0xFFFF
                write_word(geometry, vertex + 0x12, 0x8000)
                ebx = signed_word(read_word(geometry, vertex + 4)) & 0xFFFFFFFF
                ecx = signed_word(read_word(geometry, vertex + 6)) & 0xFFFFFFFF
                edx = signed_word(read_word(geometry, vertex + 8)) & 0xFFFFFFFF

                eax = multiply_dword(read_dword(active, current_state + 0x2A), ebx)
                ebp = eax
                eax = multiply_dword(read_dword(active, current_state + 0x2E), ecx)
                ebp = (ebp + eax) & 0xFFFFFFFF
                eax = multiply_dword(read_dword(active, current_state + 0x32), edx)
                ebp = (ebp + eax) & 0xFFFFFFFF
                ebp = (
                    ebp + read_dword(active, current_state + 0x3E)
                ) & 0xFFFFFFFF
                ebp = (signed_dword(ebp) >> 8) & 0xFFFFFFFF
                write_dword(geometry, vertex + 0x0E, ebp)

                if signed_dword(ebp) > 0:
                    eax = multiply_dword(
                        read_dword(active, current_state + 0x1E), ebx
                    )
                    dot = eax
                    eax = multiply_dword(
                        read_dword(active, current_state + 0x22), ecx
                    )
                    dot = (dot + eax) & 0xFFFFFFFF
                    eax = multiply_dword(
                        read_dword(active, current_state + 0x26), edx
                    )
                    saved_screen_y = (
                        eax + read_dword(active, current_state + 0x3A) + dot
                    ) & 0xFFFFFFFF

                    eax = multiply_dword(
                        read_dword(active, current_state + 0x12), ebx
                    )
                    dot = eax
                    eax = multiply_dword(
                        read_dword(active, current_state + 0x16), ecx
                    )
                    dot = (dot + eax) & 0xFFFFFFFF
                    eax = multiply_dword(
                        read_dword(active, current_state + 0x1A), edx
                    )
                    eax = (
                        eax + read_dword(active, current_state + 0x36) + dot
                    ) & 0xFFFFFFFF
                    ecx &= 0xFFFF0000
                    eax, edx = divide_signed(eax, ebp)
                    ebx = eax
                    eax, edx = divide_signed(saved_screen_y, ebp)
                    eax = (-eax) & 0xFFFFFFFF
                    ebx = (ebx + read_dword(active, 0x223E)) & 0xFFFFFFFF

                    if signed_dword(ebx) < 0:
                        ecx = (ecx & 0xFFFFFF00) | 0x01
                        if signed_dword(ebx) <= -40:
                            ebx = (ebx & 0xFFFF0000) | 0xFFD9
                    if signed_dword(ebx) >= 320:
                        ecx = (ecx & 0xFFFFFF00) | 0x02
                        if signed_dword(ebx) >= 360:
                            ebx = (ebx & 0xFFFF0000) | 0x0167

                    eax = (eax + read_dword(active, 0x2242)) & 0xFFFFFFFF
                    if signed_dword(eax) < 0:
                        ecx = (ecx & 0xFFFFFF00) | ((ecx | 0x04) & 0xFF)
                        if signed_dword(eax) <= -100:
                            eax = (eax & 0xFFFF0000) | 0xFF9D
                    if signed_dword(eax) >= 200:
                        ecx = (ecx & 0xFFFFFF00) | ((ecx | 0x08) & 0xFF)
                        if signed_dword(eax) >= 300:
                            eax = (eax & 0xFFFF0000) | 0x012B

                    write_word(geometry, vertex + 0x12, ecx)
                    write_word(geometry, vertex + 0x0A, ebx)
                    write_word(geometry, vertex + 0x0C, eax)

                final_vertex_add_left = esi & 0xFFFF
                esi = (esi & 0xFFFF0000) | (
                    ((esi & 0xFFFF) + 0x0014) & 0xFFFF
                )
                remaining = (read_word(active, 0x224A) - 1) & 0xFFFF
                write_word(active, 0x224A, remaining)

            ecx = (ecx & 0xFFFF0000) | saved_outer_count
            outer_count = ((ecx & 0xFFFF) - 1) & 0xFFFF
            ecx = (ecx & 0xFFFF0000) | outer_count

        copy_count = read_word(active, 0x22FE)
        ecx = (ecx & 0xFFFF0000) | copy_count
        final_copy_add_left = 0
        data_segment = active_segment
        if copy_count != 0:
            data_segment = geometry_segment
            edi = (edi & 0xFFFF0000) | read_word(active, 0x22FA)
            for _copy_index in range(copy_count):
                destination = edi & 0xFFFF
                esi = (esi & 0xFFFF0000) | read_word(geometry, destination + 4)
                source = esi & 0xFFFF
                eax = read_dword(geometry, source + 0x0A)
                ebx = read_dword(geometry, source + 0x0E)
                edx = (edx & 0xFFFF0000) | read_word(geometry, source + 0x12)
                write_dword(geometry, destination + 0x0A, eax)
                write_dword(geometry, destination + 0x0E, ebx)
                write_word(geometry, destination + 0x12, edx)
                final_copy_add_left = destination
                edi = (edi & 0xFFFF0000) | ((destination + 0x14) & 0xFFFF)
                ecx = (ecx & 0xFFFF0000) | (((ecx & 0xFFFF) - 1) & 0xFFFF)

        expected_registers = {
            "eax": eax,
            "ebx": ebx,
            "ecx": ecx,
            "edx": edx,
            "esi": esi,
            "edi": edi,
            "ebp": ebp,
            "sp": 0xFF02,
            "ds": data_segment,
            "es": geometry_segment,
            "fs": active_segment,
            "gs": game_segment,
            "ss": stack_segment,
        }
        if copy_count != 0:
            expected_flags = add_flags_16(
                final_copy_add_left, 0x14, initial["flags"]
            )
        else:
            expected_flags = {
                "cf": final_vertex_add_left + 0x14 > 0xFFFF,
                "pf": True,
                "af": False,
                "zf": True,
                "sf": False,
                "if": bool(initial["flags"] & 0x0200),
                "df": bool(initial["flags"] & 0x0400),
                "of": False,
            }
        return expected_registers, expected_flags

    flag_masks = {
        "cf": 0x0001,
        "pf": 0x0004,
        "af": 0x0010,
        "zf": 0x0040,
        "sf": 0x0080,
        "if": 0x0200,
        "df": 0x0400,
        "of": 0x0800,
    }

    def run_case(
        name: str,
        case_index: int,
        center: tuple[int, int],
        matrix: tuple[tuple[int, int, int], ...],
        translation: tuple[int, int, int],
        vertices: tuple[tuple[int, int, int], ...],
        vertex_count: int | None = None,
        max_instructions: int = 10000,
        copy_count: int = 0,
        additional_state: tuple[
            tuple[tuple[int, int, int], ...],
            tuple[int, int, int],
            tuple[tuple[int, int, int], ...],
        ]
        | None = None,
    ) -> dict[str, object]:
        active_before = bytearray(
            ((offset * 37 + case_index * 19 + 11) & 0xFF)
            for offset in range(0x10000)
        )
        geometry_before = bytearray(
            ((offset * 13 + case_index * 29 + 7) & 0xFF)
            for offset in range(0x10020)
        )
        state_specs = [(matrix, translation, vertices)]
        if additional_state is not None:
            state_specs.append(additional_state)
        projected_offsets = []
        write_word(active_before, 0x0002, geometry_segment)
        write_dword(active_before, 0x223E, center[0])
        write_dword(active_before, 0x2242, center[1])
        write_word(active_before, 0x22F2, len(state_specs))
        write_word(active_before, 0x22FA, 0x3000)
        write_word(active_before, 0x22FE, copy_count)
        for state_index, (state_matrix, state_translation, state_vertices) in enumerate(
            state_specs
        ):
            current_state = state_offset + state_index * 0x005E
            vertex_base = 0x1000 + state_index * 0x0400
            current_count = len(state_vertices)
            if state_index == 0 and vertex_count is not None:
                current_count = vertex_count
            write_word(active_before, current_state + 2, current_count)
            write_word(active_before, current_state + 6, vertex_base)
            for row in range(3):
                for column in range(3):
                    write_dword(
                        active_before,
                        current_state + 0x12 + row * 0x0C + column * 4,
                        state_matrix[row][column],
                    )
            for component, value in enumerate(state_translation):
                write_dword(
                    active_before,
                    current_state + 0x36 + component * 4,
                    value,
                )
            for vertex_index, coordinates in enumerate(state_vertices):
                offset = vertex_base + vertex_index * 0x14
                projected_offsets.append(offset)
                for component, value in enumerate(coordinates):
                    write_word(
                        geometry_before, offset + 4 + component * 2, value
                    )
        if copy_count:
            for copy_index in range(copy_count):
                destination = 0x3000 + copy_index * 0x14
                source = 0x4000 + copy_index * 0x20
                write_word(geometry_before, destination + 4, source)
                write_dword(
                    geometry_before,
                    source + 0x0A,
                    0x22001100 + copy_index * 0x01010101,
                )
                write_dword(
                    geometry_before, source + 0x0E, 0x12345678 + copy_index
                )
                write_word(geometry_before, source + 0x12, 0x0041 + copy_index)

        active_expected = bytearray(active_before)
        geometry_expected = bytearray(geometry_before)
        initial_flags = 0x0A93 | (0x0400 if case_index & 1 else 0)
        initial = {
            "eax": 0xA1A10000 | ((0xBEEF + case_index) & 0xFFFF),
            "ebx": 0xB2B20000 | ((0x2345 + case_index) & 0xFFFF),
            "ecx": 0xC3C30000 | ((0x3456 + case_index) & 0xFFFF),
            "edx": 0xD4D40000 | ((0x4567 + case_index) & 0xFFFF),
            "esi": 0xE5E50000 | ((0x5678 + case_index) & 0xFFFF),
            "edi": 0xF6F60000 | ((0x6789 + case_index) & 0xFFFF),
            "ebp": 0x97970000 | ((0x789A + case_index) & 0xFFFF),
            "sp": 0xFF00,
            "ds": active_segment,
            "es": extra_segment,
            "fs": active_segment,
            "gs": game_segment,
            "ss": stack_segment,
            "flags": initial_flags,
        }
        expected_registers, expected_flags = model_project(
            active_expected, geometry_expected, initial
        )
        stack_sentinel = bytes.fromhex("5aa596698778")
        decoy = bytes.fromhex("112233445566778899aabbccddeeff00")
        machine = execute(
            image,
            entry,
            return_address,
            initial,
            [
                (0, return_address, b"\xcc"),
                (active_segment, 0, bytes(active_before)),
                (geometry_segment, 0, bytes(geometry_before)),
                (extra_segment, 0x0100, decoy),
                (game_segment, 0x0100, decoy),
                (
                    stack_segment,
                    0xFF00,
                    struct.pack("<H", return_address) + stack_sentinel,
                ),
            ],
            max_instructions=max_instructions,
        )
        for segment, expected, label in (
            (active_segment, active_expected, "active"),
            (geometry_segment, geometry_expected, "geometry"),
        ):
            actual = bytes(machine.mem_read(segment * 16, len(expected)))
            if actual != bytes(expected):
                differences = [
                    (offset, actual[offset], expected[offset])
                    for offset in range(len(expected))
                    if actual[offset] != expected[offset]
                ][:8]
                raise AssertionError(
                    f"{module}:{entry:#x} {name}: {label} differs at {differences}"
                )
        for segment in (extra_segment, game_segment):
            if bytes(machine.mem_read(segment * 16 + 0x0100, len(decoy))) != decoy:
                raise AssertionError(
                    f"{module}:{entry:#x} {name}: decoy {segment:#x} changed"
                )
        if bytes(machine.mem_read(stack_segment * 16 + 0xFF02, 6)) != stack_sentinel:
            raise AssertionError(f"{module}:{entry:#x} {name}: stack changed")
        for register, expected in expected_registers.items():
            actual = machine.reg_read(REGISTERS[register])
            if actual != expected:
                raise AssertionError(
                    f"{module}:{entry:#x} {name}: "
                    f"{register}={actual:#x}, expected={expected:#x}"
                )
        flags_after = machine.reg_read(UC_X86_REG_EFLAGS)
        actual_flags = {
            flag: bool(flags_after & mask) for flag, mask in flag_masks.items()
        }
        if actual_flags != expected_flags:
            raise AssertionError(
                f"{module}:{entry:#x} {name}: "
                f"flags={actual_flags}, expected={expected_flags}"
            )

        return {
            "name": name,
            "module": module,
            "entry": entry,
            "state_count": len(state_specs),
            "vertex_count": len(vertices) if vertex_count is None else vertex_count,
            "iterations": (
                len(vertices)
                if vertex_count is None
                else vertex_count if vertex_count != 0 else 0x10000
            ),
            "projected_vertices": [
                {
                    "offset": offset,
                    "screen_x": signed_word(read_word(geometry_expected, offset + 0x0A)),
                    "screen_y": signed_word(read_word(geometry_expected, offset + 0x0C)),
                    "depth": signed_dword(read_dword(geometry_expected, offset + 0x0E)),
                    "clip_flags": read_word(geometry_expected, offset + 0x12),
                }
                for offset in projected_offsets
            ],
            "copy_count": copy_count,
            "defined_flags": expected_flags,
        }

    for case_index, (name, center, matrix, translation, vertices) in enumerate(cases):
        vectors.append(
            run_case(name, case_index, center, matrix, translation, vertices)
        )
    vectors.append(
        run_case(
            "two_projection_states",
            len(cases),
            (3, 4),
            zero_matrix,
            (7, -8, 256),
            ((0, 0, 0),),
            additional_state=(zero_matrix, (34, -36, 512), ((0, 0, 0),)),
        )
    )
    vectors.append(
        run_case(
            "copy_tail_two_vertices",
            len(cases) + 1,
            (0, 0),
            zero_matrix,
            (0, 0, 0),
            ((0, 0, 0),),
            copy_count=2,
        )
    )
    vectors.append(
        run_case(
            "zero_vertex_count_wraps_65536_iterations",
            len(cases) + 2,
            (0, 0),
            zero_matrix,
            (0, 0, -256),
            ((0, 0, 0),),
            vertex_count=0,
            max_instructions=2000000,
        )
    )
    return vectors


def manu3_face_builder_next_vectors() -> list[dict[str, object]]:
    module = "manu3"
    entry = 0x06F6
    bucket_sort = 0x0700
    image = load_image(module)
    expected_hash = "ca58b810bd257232784dea4ec18b70600fd2d014ef7fea532fbf7c810973ac29"
    if hashlib.sha256(image[entry : entry + 10]).hexdigest() != expected_hash:
        raise AssertionError(f"{module}:{entry:#x}: recovered 10-byte body changed")

    patched_image = bytearray(image)
    patched_image[bucket_sort] = 0xC3
    cases = (
        ("both_zero", 0x0000, 0x0000),
        ("ordinary", 0x4400, 0x6400),
        ("same_segment", 0x4800, 0x4800),
        ("geometry_high", 0x8000, 0x4C00),
        ("raster_high", 0x5000, 0x8000),
        ("maximum", 0xFFFF, 0xFFFF),
    )
    data_segment = 0x6000
    stack_segment = 0x9000
    return_address = 0xF000
    vectors = []

    for case_index, (name, geometry_segment, raster_segment) in enumerate(cases):
        stack_sentinel = bytes.fromhex("5aa596698778")
        data_before = bytearray(
            ((offset * 37 + case_index * 19 + 11) & 0xFF)
            for offset in range(8)
        )
        struct.pack_into("<HHH", data_before, 0x02, geometry_segment, 0xA55A, raster_segment)
        decoys = (
            bytes.fromhex("1021324354657687"),
            bytes.fromhex("89a9bacbdcedfe0f"),
        )
        initial_flags = 0x0A93 | (0x0400 if case_index & 1 else 0)
        initial = {
            "eax": 0xA1A1BEEF + case_index,
            "ebx": 0xB2B22345 + case_index,
            "ecx": 0xC3C33456 + case_index,
            "edx": 0xD4D44567 + case_index,
            "esi": 0xE5E55678 + case_index,
            "edi": 0xF6F66789 + case_index,
            "ebp": 0x9797789A + case_index,
            "sp": 0xFF00,
            "ds": 0x2400,
            "es": 0x2800,
            "fs": data_segment,
            "gs": 0x2C00,
            "ss": stack_segment,
            "flags": initial_flags,
        }
        sort_entries: list[dict[str, int]] = []

        def code_handler(
            machine: Uc, address: int, _size: int, _data: object
        ) -> None:
            if address == bucket_sort:
                sort_entries.append(
                    {
                        "ds": machine.reg_read(UC_X86_REG_DS),
                        "es": machine.reg_read(UC_X86_REG_ES),
                        "fs": machine.reg_read(UC_X86_REG_FS),
                        "sp": machine.reg_read(UC_X86_REG_SP),
                    }
                )

        machine = execute(
            bytes(patched_image),
            entry,
            return_address,
            initial,
            [
                (0, return_address, b"\xcc"),
                (data_segment, 0, bytes(data_before)),
                (initial["ds"], 0x0100, decoys[0]),
                (initial["es"], 0x0100, decoys[1]),
                (
                    stack_segment,
                    0xFF00,
                    struct.pack("<H", return_address) + stack_sentinel,
                ),
            ],
            code_handler=code_handler,
        )
        expected_entry = {
            "ds": geometry_segment,
            "es": raster_segment,
            "fs": data_segment,
            "sp": 0xFF00,
        }
        if sort_entries != [expected_entry]:
            raise AssertionError(
                f"{module}:{entry:#x} {name}: sort entries={sort_entries}, "
                f"expected={[expected_entry]}"
            )

        expected_registers = dict(initial)
        del expected_registers["flags"]
        expected_registers["ds"] = geometry_segment
        expected_registers["es"] = raster_segment
        expected_registers["sp"] = 0xFF02
        for register, expected in expected_registers.items():
            actual = machine.reg_read(REGISTERS[register])
            if actual != expected:
                raise AssertionError(
                    f"{module}:{entry:#x} {name}: "
                    f"{register}={actual:#x}, expected={expected:#x}"
                )
        if bytes(machine.mem_read(data_segment * 16, 8)) != data_before:
            raise AssertionError(f"{module}:{entry:#x} {name}: FS directory changed")
        if bytes(machine.mem_read(initial["ds"] * 16 + 0x0100, 8)) != decoys[0]:
            raise AssertionError(f"{module}:{entry:#x} {name}: DS decoy changed")
        if bytes(machine.mem_read(initial["es"] * 16 + 0x0100, 8)) != decoys[1]:
            raise AssertionError(f"{module}:{entry:#x} {name}: ES decoy changed")
        if bytes(machine.mem_read(stack_segment * 16 + 0xFF02, 6)) != stack_sentinel:
            raise AssertionError(f"{module}:{entry:#x} {name}: stack sentinel changed")
        flags_after = machine.reg_read(UC_X86_REG_EFLAGS)
        if (flags_after & 0x0ED5) != (initial_flags & 0x0ED5):
            raise AssertionError(f"{module}:{entry:#x} {name}: flags changed")

        vectors.append(
            {
                "name": name,
                "module": module,
                "entry": entry,
                "geometry_segment": geometry_segment,
                "raster_segment": raster_segment,
                "bucket_sort_fallthrough": bucket_sort,
                "fallthrough_stack_unchanged": True,
            }
        )

    return vectors


def manu3_face_bucket_sort_vectors() -> list[dict[str, object]]:
    module = "manu3"
    entry = 0x0700
    renderer = 0x0775
    image = load_image(module)
    expected_hash = "7c332d7f4ed8cddf1dc6289e33919c57f0109bddda0a68301cf67eb912eaaa32"
    if hashlib.sha256(image[entry : entry + 117]).hexdigest() != expected_hash:
        raise AssertionError(f"{module}:{entry:#x}: recovered 117-byte body changed")

    patched_image = bytearray(image)
    patched_image[renderer] = 0xC3
    cases = (
        {
            "name": "common_clip_rejected",
            "faces": (((10, 0x0001), (20, 0x0003), (30, 0x0005)),),
            "bucket_heads": {},
        },
        {
            "name": "already_lowest",
            "faces": (((10, 0), (20, 0), (30, 0)),),
            "bucket_heads": {0x069A: 0x7111},
        },
        {
            "name": "rotate_vertex_2_on_x_tie",
            "faces": (((10, 0), (30, 0), (10, 0)),),
            "bucket_heads": {0x069A: 0x7222},
        },
        {
            "name": "rotate_vertex_1",
            "faces": (((25, 0), (5, 0), (20, 0)),),
            "bucket_heads": {0x0690: 0x7333},
        },
        {
            "name": "negative_x_clamps_bucket",
            "faces": (((-5, 0), (4, 0), (6, 0)),),
            "bucket_heads": {0x0686: 0x7444},
        },
        {
            "name": "first_span_rejected",
            "faces": (((0, 0), (400, 0), (100, 0)),),
            "bucket_heads": {},
        },
        {
            "name": "second_span_rejected",
            "faces": (((0, 0), (100, 0), (400, 0)),),
            "bucket_heads": {},
        },
        {
            "name": "doubled_x_sign_clamps_bucket",
            "faces": (((0x4000, 0), (0x4001, 0), (0x4002, 0)),),
            "bucket_heads": {0x0686: 0x7555},
        },
        {
            "name": "three_faces_prepend_same_bucket",
            "faces": (
                ((12, 0), (20, 0), (24, 0)),
                ((12, 0), (18, 0), (28, 0)),
                ((12, 0), (16, 0), (32, 0)),
            ),
            "bucket_heads": {0x069E: 0x7666},
        },
    )
    geometry_segment = 0x4400
    active_segment = 0x6000
    raster_segment = 0x7000
    stack_segment = 0x9000
    return_address = 0xF000
    face_list_offset = 0x1000
    vectors = []

    def read_word(buffer: bytearray, offset: int) -> int:
        return struct.unpack_from("<H", buffer, offset)[0]

    def write_word(buffer: bytearray, offset: int, value: int) -> None:
        struct.pack_into("<H", buffer, offset, value & 0xFFFF)

    def signed_word(value: int) -> int:
        return value if value < 0x8000 else value - 0x10000

    def model_sort(
        geometry: bytearray,
        raster: bytearray,
        count: int,
        start: int,
        initial: dict[str, int],
    ) -> tuple[dict[str, int], dict[str, bool]]:
        ax = initial["eax"] & 0xFFFF
        bx = initial["ebx"] & 0xFFFF
        cx = count
        dx = initial["edx"] & 0xFFFF
        si = start
        di = initial["edi"] & 0xFFFF
        bp = initial["ebp"] & 0xFFFF
        iterations = count if count != 0 else 0x10000

        for _iteration in range(iterations):
            bx = read_word(geometry, si + 2)
            di = read_word(geometry, si + 4)
            ax = read_word(geometry, bx + 0x12)
            bp = read_word(geometry, si + 6)
            ax &= read_word(geometry, di + 0x12)
            ax &= read_word(geometry, bp + 0x12)
            if ax == 0:
                saved_count = cx
                ax = read_word(geometry, bx + 0x0A)
                dx = read_word(geometry, di + 0x0A)
                cx = read_word(geometry, bp + 0x0A)
                if signed_word(dx) > signed_word(cx):
                    if signed_word(ax) >= signed_word(cx):
                        bp, bx = bx, bp
                        cx, ax = ax, cx
                        bp, di = di, bp
                        dx, cx = cx, dx
                        write_word(geometry, si + 2, bx)
                        write_word(geometry, si + 4, di)
                        write_word(geometry, si + 6, bp)
                elif signed_word(ax) > signed_word(dx):
                    bp, bx = bx, bp
                    cx, ax = ax, cx
                    di, bx = bx, di
                    dx, ax = ax, dx
                    write_word(geometry, si + 2, bx)
                    write_word(geometry, si + 4, di)
                    write_word(geometry, si + 6, bp)

                dx = (dx - ax) & 0xFFFF
                cx = (cx - ax) & 0xFFFF
                if dx < 0x0190 and cx < 0x0190:
                    ax = (ax + ax) & 0xFFFF
                    di = 0x0686
                    if signed_word(ax) >= 0:
                        di = (di + ax) & 0xFFFF
                    bx = read_word(raster, di)
                    write_word(raster, di, si)
                    write_word(geometry, si, bx)
                cx = saved_count

            add_left = si
            si = (si + 8) & 0xFFFF
            cx = (cx - 1) & 0xFFFF

        low_words = {
            "eax": ax,
            "ebx": bx,
            "ecx": cx,
            "edx": dx,
            "esi": si,
            "edi": di,
            "ebp": bp,
        }
        expected_registers = {
            register: (initial[register] & 0xFFFF0000) | value
            for register, value in low_words.items()
        }
        expected_registers.update(
            {
                "sp": 0xFF02,
                "ds": geometry_segment,
                "es": raster_segment,
                "fs": active_segment,
                "gs": initial["gs"],
                "ss": stack_segment,
            }
        )
        return expected_registers, add_flags_16(add_left, 8, initial["flags"])

    flag_masks = {
        "cf": 0x0001,
        "pf": 0x0004,
        "af": 0x0010,
        "zf": 0x0040,
        "sf": 0x0080,
        "if": 0x0200,
        "df": 0x0400,
        "of": 0x0800,
    }

    for case_index, case in enumerate(cases):
        faces = case["faces"]
        count = len(faces)
        geometry_before = bytearray(
            ((offset * 13 + case_index * 29 + 7) & 0xFF)
            for offset in range(0x10000)
        )
        raster_before = bytearray(
            ((offset * 31 + case_index * 17 + 3) & 0xFF)
            for offset in range(0x10000)
        )
        active_before = bytearray(
            ((offset * 37 + case_index * 19 + 11) & 0xFF)
            for offset in range(0x2400)
        )
        write_word(active_before, 0x2300, face_list_offset)
        write_word(active_before, 0x2304, count)

        for face_index, vertices in enumerate(faces):
            face_offset = face_list_offset + face_index * 8
            vertex_offsets = tuple(
                0x3000 + face_index * 0x0080 + vertex_index * 0x0020
                for vertex_index in range(3)
            )
            struct.pack_into(
                "<HHHH",
                geometry_before,
                face_offset,
                0x9000 + face_index,
                *vertex_offsets,
            )
            for vertex_offset, (screen_y, clip_flags) in zip(
                vertex_offsets, vertices
            ):
                write_word(geometry_before, vertex_offset + 0x0A, screen_y)
                write_word(geometry_before, vertex_offset + 0x12, clip_flags)
        for bucket_offset, head in case["bucket_heads"].items():
            write_word(raster_before, bucket_offset, head)

        geometry_expected = bytearray(geometry_before)
        raster_expected = bytearray(raster_before)
        initial_flags = 0x0A93 | (0x0400 if case_index & 1 else 0)
        initial = {
            "eax": 0xA1A10000 | ((0xBEEF + case_index) & 0xFFFF),
            "ebx": 0xB2B20000 | ((0x2345 + case_index) & 0xFFFF),
            "ecx": 0xC3C30000 | ((0x3456 + case_index) & 0xFFFF),
            "edx": 0xD4D40000 | ((0x4567 + case_index) & 0xFFFF),
            "esi": 0xE5E50000 | ((0x5678 + case_index) & 0xFFFF),
            "edi": 0xF6F60000 | ((0x6789 + case_index) & 0xFFFF),
            "ebp": 0x97970000 | ((0x789A + case_index) & 0xFFFF),
            "sp": 0xFF00,
            "ds": geometry_segment,
            "es": raster_segment,
            "fs": active_segment,
            "gs": 0x7800,
            "ss": stack_segment,
            "flags": initial_flags,
        }
        expected_registers, expected_flags = model_sort(
            geometry_expected,
            raster_expected,
            count,
            face_list_offset,
            initial,
        )
        stack_sentinel = bytes.fromhex("5aa596698778")
        renderer_entries: list[dict[str, int]] = []

        def code_handler(
            machine: Uc, address: int, _size: int, _data: object
        ) -> None:
            if address == renderer:
                renderer_entries.append(
                    {
                        "ds": machine.reg_read(UC_X86_REG_DS),
                        "es": machine.reg_read(UC_X86_REG_ES),
                        "fs": machine.reg_read(UC_X86_REG_FS),
                        "sp": machine.reg_read(UC_X86_REG_SP),
                    }
                )

        machine = execute(
            bytes(patched_image),
            entry,
            return_address,
            initial,
            [
                (0, return_address, b"\xcc"),
                (geometry_segment, 0, bytes(geometry_before)),
                (active_segment, 0, bytes(active_before)),
                (raster_segment, 0, bytes(raster_before)),
                (
                    stack_segment,
                    0xFF00,
                    struct.pack("<H", return_address) + stack_sentinel,
                ),
            ],
            code_handler=code_handler,
        )
        expected_entry = {
            "ds": geometry_segment,
            "es": raster_segment,
            "fs": active_segment,
            "sp": 0xFF00,
        }
        if renderer_entries != [expected_entry]:
            raise AssertionError(
                f"{module}:{entry:#x} {case['name']}: "
                f"renderer entries={renderer_entries}, expected={[expected_entry]}"
            )
        for segment, expected, label in (
            (geometry_segment, geometry_expected, "geometry"),
            (active_segment, active_before, "active"),
            (raster_segment, raster_expected, "raster"),
        ):
            actual = bytes(machine.mem_read(segment * 16, len(expected)))
            if actual != bytes(expected):
                differences = [
                    (offset, actual[offset], expected[offset])
                    for offset in range(len(expected))
                    if actual[offset] != expected[offset]
                ][:8]
                raise AssertionError(
                    f"{module}:{entry:#x} {case['name']}: "
                    f"{label} differs at {differences}"
                )
        if bytes(machine.mem_read(stack_segment * 16 + 0xFF02, 6)) != stack_sentinel:
            raise AssertionError(
                f"{module}:{entry:#x} {case['name']}: stack sentinel changed"
            )
        for register, expected in expected_registers.items():
            actual = machine.reg_read(REGISTERS[register])
            if actual != expected:
                raise AssertionError(
                    f"{module}:{entry:#x} {case['name']}: "
                    f"{register}={actual:#x}, expected={expected:#x}"
                )
        flags_after = machine.reg_read(UC_X86_REG_EFLAGS)
        actual_flags = {
            flag: bool(flags_after & mask) for flag, mask in flag_masks.items()
        }
        if actual_flags != expected_flags:
            raise AssertionError(
                f"{module}:{entry:#x} {case['name']}: "
                f"flags={actual_flags}, expected={expected_flags}"
            )

        face_words = [
            struct.unpack_from("<HHHH", geometry_expected, face_list_offset + i * 8)
            for i in range(count)
        ]
        vectors.append(
            {
                "name": case["name"],
                "module": module,
                "entry": entry,
                "face_count": count,
                "faces_before": [
                    {
                        "vertices": [
                            {
                                "screen_x": screen_x,
                                "clip_flags": clip_flags,
                            }
                            for screen_x, clip_flags in vertices
                        ]
                    }
                    for vertices in faces
                ],
                "faces_after": [
                    {
                        "offset": face_list_offset + i * 8,
                        "link": words[0],
                        "vertices": list(words[1:]),
                    }
                    for i, words in enumerate(face_words)
                ],
                "bucket_heads_after": {
                    f"0x{offset:04x}": read_word(raster_expected, offset)
                    for offset in case["bucket_heads"]
                },
                "renderer_fallthrough": renderer,
                "defined_flags": expected_flags,
            }
        )

    geometry_before = bytearray(
        ((offset * 13 + 0xA7) & 0xFF) for offset in range(0x10000)
    )
    for face_offset in range(0, 0x10000, 8):
        write_word(geometry_before, face_offset + 2, 0x8000)
        write_word(geometry_before, face_offset + 4, 0x8000)
        write_word(geometry_before, face_offset + 6, 0x8000)
    active_before = bytearray(
        ((offset * 37 + 0x3D) & 0xFF) for offset in range(0x2400)
    )
    write_word(active_before, 0x2300, 0)
    write_word(active_before, 0x2304, 0)
    raster_before = bytearray(
        ((offset * 31 + 0x51) & 0xFF) for offset in range(0x10000)
    )
    initial_flags = 0x0E93
    initial = {
        "eax": 0xA1A1BEEF,
        "ebx": 0xB2B22345,
        "ecx": 0xC3C33456,
        "edx": 0xD4D44567,
        "esi": 0xE5E55678,
        "edi": 0xF6F66789,
        "ebp": 0x9797789A,
        "sp": 0xFF00,
        "ds": geometry_segment,
        "es": raster_segment,
        "fs": active_segment,
        "gs": 0x7800,
        "ss": stack_segment,
        "flags": initial_flags,
    }
    geometry_expected = bytearray(geometry_before)
    raster_expected = bytearray(raster_before)
    expected_registers, expected_flags = model_sort(
        geometry_expected, raster_expected, 0, 0, initial
    )
    stack_sentinel = bytes.fromhex("5aa596698778")
    renderer_entries = []

    def zero_code_handler(
        machine: Uc, address: int, _size: int, _data: object
    ) -> None:
        if address == renderer:
            renderer_entries.append(address)

    machine = execute(
        bytes(patched_image),
        entry,
        return_address,
        initial,
        [
            (0, return_address, b"\xcc"),
            (geometry_segment, 0, bytes(geometry_before)),
            (active_segment, 0, bytes(active_before)),
            (raster_segment, 0, bytes(raster_before)),
            (
                stack_segment,
                0xFF00,
                struct.pack("<H", return_address) + stack_sentinel,
            ),
        ],
        max_instructions=1000000,
        code_handler=zero_code_handler,
    )
    if renderer_entries != [renderer]:
        raise AssertionError(
            f"{module}:{entry:#x} zero_count: renderer entries={renderer_entries}"
        )
    for segment, expected, label in (
        (geometry_segment, geometry_expected, "geometry"),
        (active_segment, active_before, "active"),
        (raster_segment, raster_expected, "raster"),
    ):
        actual = bytes(machine.mem_read(segment * 16, len(expected)))
        if actual != bytes(expected):
            raise AssertionError(f"{module}:{entry:#x} zero_count: {label} changed")
    if bytes(machine.mem_read(stack_segment * 16 + 0xFF02, 6)) != stack_sentinel:
        raise AssertionError(f"{module}:{entry:#x} zero_count: stack sentinel changed")
    for register, expected in expected_registers.items():
        actual = machine.reg_read(REGISTERS[register])
        if actual != expected:
            raise AssertionError(
                f"{module}:{entry:#x} zero_count: "
                f"{register}={actual:#x}, expected={expected:#x}"
            )
    flags_after = machine.reg_read(UC_X86_REG_EFLAGS)
    actual_flags = {
        flag: bool(flags_after & mask) for flag, mask in flag_masks.items()
    }
    if actual_flags != expected_flags:
        raise AssertionError(
            f"{module}:{entry:#x} zero_count: "
            f"flags={actual_flags}, expected={expected_flags}"
        )
    vectors.append(
        {
            "name": "zero_count_wraps_65536_iterations",
            "module": module,
            "entry": entry,
            "face_count": 0,
            "faces_before": [],
            "iterations": 0x10000,
            "face_cursor_after": expected_registers["esi"] & 0xFFFF,
            "renderer_fallthrough": renderer,
            "defined_flags": expected_flags,
        }
    )

    return vectors


def manu3_face_activate_vectors() -> list[dict[str, object]]:
    module = "manu3"
    entry = 0x0D7D
    gradient = 0x0D93
    image = load_image(module)
    expected_hash = "2b309ac3e61e3280e4874c293d95ac9706bd0aa1151450d61ef8c32fef31287b"
    if hashlib.sha256(image[entry : entry + 22]).hexdigest() != expected_hash:
        raise AssertionError(f"{module}:{entry:#x}: recovered 22-byte body changed")

    patched_image = bytearray(image)
    patched_image[gradient] = 0xC3
    cases = (
        ("inactive", 0x3000, 0x0000, 0x1111, 0x2222, 0x3333),
        ("ordinary", 0x3200, 0x4100, 0x1200, 0x3400, 0x5600),
        ("active_high_bit", 0x3400, 0x8000, 0x0000, 0x8000, 0xFFFF),
        ("face_offset_wrap", 0xFFFC, 0x4300, 0xA55A, 0x5AA5, 0x7FFF),
        ("maximum_active", 0x3600, 0xFFFF, 0xFFFF, 0x0001, 0xFFFE),
        ("mixed", 0x3800, 0x0001, 0x1357, 0x2468, 0xACE0),
    )
    data_segment = 0x4400
    face_segment = 0x6000
    extra_segment = 0x7000
    game_segment = 0x7800
    stack_segment = 0x9000
    return_address = 0xF000
    vectors = []

    for case_index, (name, face_offset, raster, v0, v1, v2) in enumerate(cases):
        stack_sentinel = bytes.fromhex("5aa596698778")
        data_before = bytearray(
            ((offset * 37 + case_index * 19 + 11) & 0xFF)
            for offset in range(0x10000)
        )
        face_before = bytearray(
            ((offset * 13 + case_index * 29 + 7) & 0xFF)
            for offset in range(0x10000)
        )
        struct.pack_into("<H", data_before, 0x0908, raster)
        for relative_offset, value in ((2, v0), (4, v1), (6, v2)):
            encoded = struct.pack("<H", value)
            field_offset = (face_offset + relative_offset) & 0xFFFF
            for byte_index, byte in enumerate(encoded):
                face_before[(field_offset + byte_index) & 0xFFFF] = byte

        initial_flags = 0x0A93 | (0x0400 if case_index & 1 else 0)
        initial = {
            "eax": 0xA1A1BEEF + case_index,
            "ebx": 0xB2B22345 + case_index,
            "ecx": 0xC3C33456 + case_index,
            "edx": 0xD4D44567 + case_index,
            "esi": 0xE5E50000 | face_offset,
            "edi": 0xF6F66789 + case_index,
            "ebp": 0x9797789A + case_index,
            "sp": 0xFF00,
            "ds": data_segment,
            "es": face_segment,
            "fs": extra_segment,
            "gs": game_segment,
            "ss": stack_segment,
            "flags": initial_flags,
        }
        gradient_entries: list[dict[str, int]] = []

        def code_handler(
            machine: Uc, address: int, _size: int, _data: object
        ) -> None:
            if address == gradient:
                gradient_entries.append(
                    {
                        "bx": machine.reg_read(UC_X86_REG_EBX) & 0xFFFF,
                        "di": machine.reg_read(UC_X86_REG_EDI) & 0xFFFF,
                        "bp": machine.reg_read(UC_X86_REG_EBP) & 0xFFFF,
                        "si": machine.reg_read(UC_X86_REG_ESI) & 0xFFFF,
                        "ds": machine.reg_read(UC_X86_REG_DS),
                        "es": machine.reg_read(UC_X86_REG_ES),
                        "sp": machine.reg_read(UC_X86_REG_SP),
                    }
                )

        decoy = bytes(
            ((offset * 31 + case_index * 17 + 5) & 0xFF)
            for offset in range(0x10000)
        )
        machine = execute(
            bytes(patched_image),
            entry,
            return_address,
            initial,
            [
                (0, return_address, b"\xcc"),
                (extra_segment, 0, decoy),
                (game_segment, 0, decoy),
                (data_segment, 0, bytes(data_before)),
                (face_segment, 0, bytes(face_before)),
                (
                    stack_segment,
                    0xFF00,
                    struct.pack("<H", return_address) + stack_sentinel,
                ),
            ],
            code_handler=code_handler,
        )
        expected_gradient_entries = (
            []
            if raster == 0
            else [
                {
                    "bx": v0,
                    "di": v1,
                    "bp": v2,
                    "si": raster,
                    "ds": data_segment,
                    "es": face_segment,
                    "sp": 0xFF00,
                }
            ]
        )
        if gradient_entries != expected_gradient_entries:
            raise AssertionError(
                f"{module}:{entry:#x} {name}: gradient entries="
                f"{gradient_entries}, expected={expected_gradient_entries}"
            )

        expected_registers = dict(initial)
        del expected_registers["flags"]
        expected_registers["ebx"] = (initial["ebx"] & 0xFFFF0000) | v0
        expected_registers["edi"] = (initial["edi"] & 0xFFFF0000) | v1
        expected_registers["ebp"] = (initial["ebp"] & 0xFFFF0000) | v2
        expected_registers["esi"] = (initial["esi"] & 0xFFFF0000) | raster
        expected_registers["sp"] = 0xFF02
        for register, expected in expected_registers.items():
            actual = machine.reg_read(REGISTERS[register])
            if actual != expected:
                raise AssertionError(
                    f"{module}:{entry:#x} {name}: "
                    f"{register}={actual:#x}, expected={expected:#x}"
                )
        if bytes(machine.mem_read(data_segment * 16, 0x10000)) != data_before:
            raise AssertionError(f"{module}:{entry:#x} {name}: DS data changed")
        if bytes(machine.mem_read(face_segment * 16, 0x10000)) != face_before:
            raise AssertionError(f"{module}:{entry:#x} {name}: ES face data changed")
        if bytes(machine.mem_read(extra_segment * 16, 0x10000)) != decoy:
            raise AssertionError(f"{module}:{entry:#x} {name}: FS decoy changed")
        if bytes(machine.mem_read(game_segment * 16, 0x10000)) != decoy:
            raise AssertionError(f"{module}:{entry:#x} {name}: GS decoy changed")
        if bytes(machine.mem_read(stack_segment * 16 + 0xFF02, 6)) != stack_sentinel:
            raise AssertionError(f"{module}:{entry:#x} {name}: stack sentinel changed")

        expected_flags = _logical_flags_16(raster, initial_flags)
        flag_masks = {
            "cf": 0x0001,
            "pf": 0x0004,
            "zf": 0x0040,
            "sf": 0x0080,
            "if": 0x0200,
            "df": 0x0400,
            "of": 0x0800,
        }
        flags_after = machine.reg_read(UC_X86_REG_EFLAGS)
        actual_flags = {
            flag: bool(flags_after & flag_masks[flag]) for flag in expected_flags
        }
        if actual_flags != expected_flags:
            raise AssertionError(
                f"{module}:{entry:#x} {name}: flags={actual_flags}, "
                f"expected={expected_flags}"
            )

        vectors.append(
            {
                "name": name,
                "module": module,
                "entry": entry,
                "face_offset": face_offset,
                "vertex_offsets": [v0, v1, v2],
                "active_raster_offset": raster,
                "gradient_tail_entered": raster != 0,
                "inactive_return_offset": 0x0848 if raster == 0 else None,
                "defined_flags": expected_flags,
            }
        )

    return vectors


def _manu3_gradient_reference(
    screen: tuple[tuple[int, int], tuple[int, int], tuple[int, int]],
    texture: tuple[tuple[int, int], tuple[int, int], tuple[int, int]],
    depth: tuple[int, int, int],
    work_segment: int,
    reciprocal_table: tuple[int, ...],
    max_face_width: int = 0x0190,
    advance_secondary: int = 0x0CCA,
    advance_switch: int = 0x0D19,
    advance_remove: int = 0x0D5E,
) -> tuple[bool, dict[int, int]]:
    def u16(value: int) -> int:
        return value & 0xFFFF

    def i16(value: int) -> int:
        value &= 0xFFFF
        return value - 0x10000 if value & 0x8000 else value

    def u32(value: int) -> int:
        return value & 0xFFFFFFFF

    def i32(value: int) -> int:
        value &= 0xFFFFFFFF
        return value - 0x100000000 if value & 0x80000000 else value

    def add32(left: int, right: int) -> int:
        return i32(left + right)

    def sub32(left: int, right: int) -> int:
        return i32(left - right)

    def mul_low(left: int, right: int) -> int:
        return i32(left * right)

    def mul_q16(left: int, right: int) -> int:
        return i32(i32(left) * i32(right) >> 16)

    def divide(left: int, right: int) -> int:
        quotient = abs(left) // abs(right)
        if (left < 0) != (right < 0):
            quotient = -quotient
        return i32(quotient)

    def reciprocal(width: int) -> int:
        return reciprocal_table[width]

    def packed(pair: tuple[int, int]) -> int:
        return u16(pair[0]) | (u16(pair[1]) << 16)

    def word_base(value: int) -> int:
        return u16(value << 8)

    def word_add(left: int, right: int) -> int:
        return i16(u16(left) + u16(right))

    values: dict[int, int] = {}

    def put16(offset: int, value: int) -> None:
        values[offset] = u16(value)

    def put32(offset: int, value: int) -> None:
        values[offset] = u32(value)

    screen_value = tuple(packed(pair) for pair in screen)
    texture_value = tuple(packed(pair) for pair in texture)
    x_0, x_1, x_2 = (u16(pair[0]) for pair in screen)
    width_1 = u16(x_1 - x_0)
    width_2 = u16(x_2 - x_0)
    clipping_mode = 0

    if width_1 == 0:
        if width_2 == 0 or width_2 >= max_face_width:
            return False, values
        vertical_span = u16(screen[1][1] - screen[0][1])
        if i16(vertical_span) <= 0 or vertical_span >= max_face_width:
            return False, values

        reciprocal_1 = reciprocal(vertical_span)
        reciprocal_2 = reciprocal(width_2)
        remaining = i16(width_2 - 1)
        edge_0_step = mul_low(i16(screen[2][1] - screen[0][1]), reciprocal_2)
        edge_0_position = add32(i32(screen_value[0] & 0xFFFF0000), edge_0_step >> 1)
        edge_1_step = mul_low(i16(screen[2][1] - screen[1][1]), reciprocal_2)
        edge_1_position = add32(i32(screen_value[1] & 0xFFFF0000), edge_1_step >> 1)

        delta_1 = mul_low(i16(texture[1][0] - texture[0][0]), reciprocal_1)
        delta_2 = mul_low(i16(texture[2][0] - texture[0][0]), reciprocal_2)
        texture_du = i16(delta_1 >> 8)
        texture_u_step = i16(delta_2 >> 8)
        texture_u = word_add(word_base(texture[0][0]), i32(delta_2) >> 9)

        delta_1 = mul_low(u16(texture[1][1]) - u16(texture[0][1]), reciprocal_1)
        delta_2 = mul_low(u16(texture[2][1]) - u16(texture[0][1]), reciprocal_2)
        texture_dv = i16(delta_1 >> 8)
        texture_v_step = i16(delta_2 >> 8)
        texture_v = word_add(word_base(texture[0][1]), i32(delta_2) >> 9)

        depth_step = mul_q16(sub32(depth[2], depth[0]), reciprocal_2)
        depth_position = add32(depth[0], depth_step >> 1)
        depth_gradient = mul_q16(sub32(depth[1], depth[0]), reciprocal_1)
        advance_offset = advance_remove
    else:
        if width_2 == 0:
            return False, values

        reciprocal_1 = reciprocal(width_1)
        reciprocal_2 = reciprocal(width_2)
        remaining = i16(width_2 - 1)
        put16(0x2E, remaining)
        edge_1_step = mul_low(i16(screen[1][1] - screen[0][1]), reciprocal_1)
        edge_0_step = mul_low(i16(screen[2][1] - screen[0][1]), reciprocal_2)
        area = sub32(edge_0_step, edge_1_step)
        if area >= 0:
            return False, values
        denominator = -(area >> 8)
        edge_0_position = add32(i32(screen_value[0] & 0xFFFF0000), edge_0_step >> 1)
        edge_1_position = add32(i32(screen_value[0] & 0xFFFF0000), edge_1_step >> 1)

        delta_1 = mul_low(i16(texture[1][0] - texture[0][0]), reciprocal_1)
        delta_2 = mul_low(i16(texture[2][0] - texture[0][0]), reciprocal_2)
        texture_du = i16(divide(sub32(delta_1, delta_2), denominator))
        texture_u_step = i16(delta_2 >> 8)
        texture_u = word_add(word_base(texture[0][0]), texture_u_step >> 1)

        delta_1 = mul_low(u16(texture[1][1]) - u16(texture[0][1]), reciprocal_1)
        delta_2 = mul_low(u16(texture[2][1]) - u16(texture[0][1]), reciprocal_2)
        texture_dv = i16(divide(sub32(delta_1, delta_2), denominator))
        texture_v_step = i16(delta_2 >> 8)
        texture_v = word_add(word_base(texture[0][1]), texture_v_step >> 1)

        depth_step = mul_q16(sub32(depth[2], depth[0]), reciprocal_2)
        depth_position = add32(depth[0], depth_step >> 1)
        delta_1 = mul_low(sub32(depth[1], depth[0]), reciprocal_1)
        delta_2 = mul_low(sub32(depth[2], depth[0]), reciprocal_2)
        depth_gradient = divide(sub32(delta_1, delta_2), denominator) >> 8

        x_difference = i16(x_1 - x_2)
        if x_difference > 0:
            secondary_width = u16(x_1 - x_2)
            secondary_reciprocal = reciprocal(secondary_width)
            if i16(x_2) < 0:
                clipped_columns = u16(-x_2)
                remaining = i16(x_1 - 1)
                texture_u_step = i16(
                    mul_low(
                        i16(texture[1][0] - texture[2][0]),
                        secondary_reciprocal,
                    )
                    >> 8
                )
                texture_v_step = i16(
                    mul_low(
                        i16(texture[1][1] - texture[2][1]),
                        secondary_reciprocal,
                    )
                    >> 8
                )
                texture_u = word_add(
                    word_base(texture[2][0]),
                    u16(texture_u_step) * clipped_columns,
                )
                texture_v = word_add(
                    word_base(texture[2][1]),
                    u16(texture_v_step) * clipped_columns,
                )
                edge_0_step = mul_low(
                    i16(screen[1][1] - screen[2][1]),
                    secondary_reciprocal,
                )
                edge_0_position = add32(
                    i32(screen_value[2] & 0xFFFF0000),
                    mul_low(edge_0_step, clipped_columns),
                )
                depth_step = mul_q16(sub32(depth[1], depth[2]), secondary_reciprocal)
                depth_position = add32(depth[2], mul_low(depth_step, clipped_columns))
                clipped_columns = u16(-x_0)
                edge_1_position = add32(
                    edge_1_position, mul_low(edge_1_step, clipped_columns)
                )
                advance_offset = advance_remove
                clipping_mode = 1
            else:
                put16(0x30, secondary_width - 1)
                secondary_texture_u_step = i16(
                    mul_low(
                        i16(texture[1][0] - texture[2][0]),
                        secondary_reciprocal,
                    )
                    >> 8
                )
                secondary_texture_v_step = i16(
                    mul_low(
                        i16(texture[1][1] - texture[2][1]),
                        secondary_reciprocal,
                    )
                    >> 8
                )
                put16(
                    0x46,
                    word_add(
                        word_base(texture[2][0]),
                        secondary_texture_u_step >> 1,
                    ),
                )
                put16(
                    0x48,
                    word_add(
                        word_base(texture[2][1]),
                        secondary_texture_v_step >> 1,
                    ),
                )
                put16(0x4E, secondary_texture_u_step)
                put16(0x50, secondary_texture_v_step)
                secondary_edge_step = mul_low(
                    i16(screen[1][1] - screen[2][1]),
                    secondary_reciprocal,
                )
                put32(0x36, secondary_edge_step)
                put32(
                    0x32,
                    add32(
                        i32(screen_value[2] & 0xFFFF0000),
                        secondary_edge_step >> 1,
                    ),
                )
                secondary_depth_step = mul_q16(
                    sub32(depth[1], depth[2]), secondary_reciprocal
                )
                put32(0x3E, secondary_depth_step)
                put32(0x3A, add32(depth[2], secondary_depth_step >> 1))
                advance_offset = advance_secondary
        elif x_difference < 0:
            secondary_width = u16(x_2 - x_1)
            secondary_reciprocal = reciprocal(secondary_width)
            if i16(x_1) < 0:
                clipped_columns = u16(-x_1)
                edge_1_step = mul_low(
                    i16(screen[2][1] - screen[1][1]),
                    secondary_reciprocal,
                )
                edge_1_position = add32(
                    i32(screen_value[1] & 0xFFFF0000),
                    mul_low(edge_1_step, clipped_columns),
                )
                advance_offset = advance_remove
                clipping_mode = 2
            else:
                remaining = i16(u16(remaining) - secondary_width)
                put16(0x30, secondary_width - 1)
                secondary_edge_step = mul_low(
                    i16(screen[2][1] - screen[1][1]),
                    secondary_reciprocal,
                )
                put32(0x36, secondary_edge_step)
                put32(
                    0x32,
                    add32(
                        i32(screen_value[1] & 0xFFFF0000),
                        secondary_edge_step >> 1,
                    ),
                )
                advance_offset = advance_switch
        else:
            advance_offset = advance_remove

    texture_segment = u16(work_segment + ((texture_value[0] >> 24) << 12))
    if i16(x_0) < 0 and clipping_mode != 1:
        clipped_columns = u16(-x_0)
        remaining = i16(u16(remaining) - clipped_columns)
        edge_0_position = add32(edge_0_position, mul_low(edge_0_step, clipped_columns))
        if clipping_mode != 2:
            edge_1_position = add32(
                edge_1_position, mul_low(edge_1_step, clipped_columns)
            )
        depth_position = add32(depth_position, mul_low(depth_step, clipped_columns))
        texture_u = word_add(texture_u, u16(texture_u_step) * clipped_columns)
        texture_v = word_add(texture_v, u16(texture_v_step) * clipped_columns)

    put32(0x08, edge_0_position)
    put32(0x0C, edge_0_step)
    put32(0x18, edge_1_position)
    put32(0x1C, edge_1_step)
    put32(0x20, depth_position)
    put32(0x24, depth_step)
    put32(0x28, depth_gradient)
    put16(0x2C, advance_offset)
    put16(0x2E, remaining)
    put16(0x42, texture_u)
    put16(0x44, texture_v)
    put16(0x4A, texture_u_step)
    put16(0x4C, texture_v_step)
    put16(0x52, texture_du)
    put16(0x54, texture_dv)
    put16(0x56, texture_segment)
    return True, values


def face_gradient_vectors(
    module: str,
    entry: int,
    body_hash: str,
    layout: dict[str, int],
    extra_cases: tuple[dict[str, object], ...] = (),
) -> list[dict[str, object]]:
    image = load_image(module)
    if hashlib.sha256(image[entry : entry + 1514]).hexdigest() != body_hash:
        raise AssertionError(f"{module}:{entry:#x}: recovered gradient body changed")

    cases = (
        {
            "name": "inactive",
            "active": False,
            "screen": ((10, 20), (80, 100), (50, 30)),
        },
        {
            "name": "backface_rejected",
            "screen": ((10, 20), (80, 25), (50, 100)),
        },
        {
            "name": "right_edge_switch",
            "screen": ((10, 20), (80, 100), (50, 30)),
            "list_mode": "after_existing",
        },
        {
            "name": "right_edge_clipped",
            "screen": ((-100, 20), (80, 120), (-20, 30)),
        },
        {
            "name": "left_edge_switch",
            "screen": ((10, 20), (50, 100), (100, 30)),
        },
        {
            "name": "left_edge_clipped",
            "screen": ((-100, 20), (-20, 100), (80, 30)),
        },
        {
            "name": "equal_secondary_x",
            "screen": ((10, 20), (80, 100), (80, 30)),
        },
        {
            "name": "vertical_first_edge",
            "screen": ((10, 20), (10, 100), (100, 30)),
        },
        {
            "name": "zero_second_width_rejected",
            "screen": ((10, 20), (80, 100), (10, 30)),
        },
        {
            "name": "vertical_direction_rejected",
            "screen": ((10, 100), (10, 20), (100, 30)),
        },
    ) + extra_cases
    texture = ((0x0123, 0x1234), (0x2345, 0x3456), (0x4567, 0x5678))
    depth = (0x10203040, -0x1234567, 0x30405060)
    data_segment = 0x4000
    geometry_segment = 0x6000
    globals_segment = 0x7000
    stack_segment = 0x9000
    return_address = 0xF000
    face_offset = 0x1000
    vertex_offsets = (0x1100, 0x1200, 0x1300)
    raster_offset = 0x2000
    free_offset = 0x205A
    head_offset = layout["active_head"]
    tail_offset = layout["active_tail"]
    existing_offset = 0x2100
    work_segment = 0x3210
    raster_data_offset = layout["reciprocal_data"]
    reciprocal_table = tuple(
        struct.unpack_from("<I", image, raster_data_offset + width * 4)[0]
        for width in range(layout["max_face_width"])
    )
    vectors = []

    for case_index, case in enumerate(cases):
        screen = case["screen"]
        active = bool(case.get("active", True))
        accepted, writes = _manu3_gradient_reference(
            screen,
            texture,
            depth,
            work_segment,
            reciprocal_table,
            layout["max_face_width"],
            layout["advance_secondary"],
            layout["advance_switch"],
            layout["advance_remove"],
        )
        if not active:
            accepted = False
            writes = {}

        data_before = bytearray(
            ((offset * 37 + case_index * 11 + 5) & 0xFF) for offset in range(0x10000)
        )
        geometry_before = bytearray(
            ((offset * 13 + case_index * 17 + 7) & 0xFF) for offset in range(0x10000)
        )
        globals_before = bytearray(
            ((offset * 19 + case_index * 23 + 3) & 0xFF) for offset in range(0x10000)
        )
        struct.pack_into("<HHHH", geometry_before, face_offset, 0, *vertex_offsets)
        for vertex_offset, position, coordinate, depth_value in zip(
            vertex_offsets, screen, texture, depth
        ):
            struct.pack_into(
                "<HH", geometry_before, vertex_offset, coordinate[0], coordinate[1]
            )
            struct.pack_into(
                "<I",
                geometry_before,
                vertex_offset + 0x0A,
                (position[0] & 0xFFFF) | ((position[1] & 0xFFFF) << 16),
            )
            struct.pack_into(
                "<I", geometry_before, vertex_offset + 0x0E, depth_value & 0xFFFFFFFF
            )

        reciprocal_size = layout["max_face_width"] * 4
        data_before[:reciprocal_size] = image[
            raster_data_offset : raster_data_offset + reciprocal_size
        ]
        struct.pack_into(
            "<H", data_before, layout["free_head"], raster_offset if active else 0
        )
        struct.pack_into("<H", data_before, raster_offset, free_offset)
        struct.pack_into("<H", data_before, head_offset, tail_offset)
        struct.pack_into("<I", data_before, tail_offset + 0x08, 0x7FFFFFFF)
        struct.pack_into("<I", data_before, tail_offset + 0x0C, 0x7FFFFFFF)
        struct.pack_into("<H", data_before, tail_offset + 0x10, head_offset)
        if accepted and case.get("list_mode") == "after_existing":
            struct.pack_into("<H", data_before, head_offset, existing_offset)
            struct.pack_into("<H", data_before, existing_offset, tail_offset)
            struct.pack_into("<H", data_before, existing_offset + 0x10, head_offset)
            struct.pack_into(
                "<I", data_before, existing_offset + 0x08, writes[0x08] - 1
            )
            struct.pack_into("<I", data_before, existing_offset + 0x0C, 0)
            struct.pack_into("<H", data_before, tail_offset + 0x10, existing_offset)
        struct.pack_into("<H", globals_before, 0x0004, work_segment)

        record_expected = bytearray(data_before[raster_offset : raster_offset + 0x5A])
        for offset, value in writes.items():
            if offset in {
                0x08,
                0x0C,
                0x18,
                0x1C,
                0x20,
                0x24,
                0x28,
                0x32,
                0x36,
                0x3A,
                0x3E,
            }:
                struct.pack_into("<I", record_expected, offset, value)
            else:
                struct.pack_into("<H", record_expected, offset, value)
        expected_active = raster_offset if active else 0
        expected_head = tail_offset
        expected_tail_previous = head_offset
        expected_existing_next = struct.unpack_from("<H", data_before, existing_offset)[
            0
        ]
        if accepted:
            expected_active = free_offset
            expected_head = raster_offset
            expected_tail_previous = raster_offset
            struct.pack_into("<H", record_expected, 0x00, tail_offset)
            struct.pack_into("<H", record_expected, 0x10, head_offset)
            if case.get("list_mode") == "after_existing":
                expected_head = existing_offset
                expected_existing_next = raster_offset
                struct.pack_into("<H", record_expected, 0x10, existing_offset)

        data_expected = bytearray(data_before)
        data_expected[raster_offset : raster_offset + 0x5A] = record_expected
        if accepted:
            struct.pack_into("<H", data_expected, layout["free_head"], free_offset)
            struct.pack_into("<H", data_expected, tail_offset + 0x10, raster_offset)
            if case.get("list_mode") == "after_existing":
                struct.pack_into("<H", data_expected, existing_offset, raster_offset)
            else:
                struct.pack_into("<H", data_expected, head_offset, raster_offset)

        initial = {
            "esi": 0xA5A50000 | face_offset,
            "sp": 0xFF00,
            "ds": data_segment,
            "es": geometry_segment,
            "fs": globals_segment,
            "gs": 0x7800,
            "ss": stack_segment,
            "flags": 0x0202,
        }
        stack_sentinel = bytes.fromhex("5aa59669")
        machine = execute(
            image,
            entry,
            return_address,
            initial,
            [
                (0, return_address, b"\xcc"),
                (data_segment, 0, bytes(data_before)),
                (geometry_segment, 0, bytes(geometry_before)),
                (globals_segment, 0, bytes(globals_before)),
                (
                    stack_segment,
                    0xFF00,
                    struct.pack("<H", return_address) + stack_sentinel,
                ),
            ],
            max_instructions=5000,
        )

        actual_record = bytes(machine.mem_read(data_segment * 16 + raster_offset, 0x5A))
        if actual_record != bytes(record_expected):
            differing = [
                offset
                for offset, (actual, expected) in enumerate(
                    zip(actual_record, record_expected)
                )
                if actual != expected
            ]
            raise AssertionError(
                f"{module}:{entry:#x} {case['name']}: raster differs at "
                f"{differing[:16]} with "
                f"actual={[actual_record[offset] for offset in differing[:16]]} "
                f"expected={[record_expected[offset] for offset in differing[:16]]}"
            )
        actual_active = struct.unpack(
            "<H", machine.mem_read(data_segment * 16 + layout["free_head"], 2)
        )[0]
        actual_head = struct.unpack(
            "<H", machine.mem_read(data_segment * 16 + head_offset, 2)
        )[0]
        actual_tail_previous = struct.unpack(
            "<H", machine.mem_read(data_segment * 16 + tail_offset + 0x10, 2)
        )[0]
        actual_existing_next = struct.unpack(
            "<H", machine.mem_read(data_segment * 16 + existing_offset, 2)
        )[0]
        if (
            actual_active,
            actual_head,
            actual_tail_previous,
            actual_existing_next,
        ) != (
            expected_active,
            expected_head,
            expected_tail_previous,
            expected_existing_next,
        ):
            raise AssertionError(
                f"{module}:{entry:#x} {case['name']}: list state "
                f"{(actual_active, actual_head, actual_tail_previous, actual_existing_next)} != "
                f"{(expected_active, expected_head, expected_tail_previous, expected_existing_next)}"
            )
        if bytes(machine.mem_read(geometry_segment * 16, 0x10000)) != bytes(
            geometry_before
        ):
            raise AssertionError(
                f"{module}:{entry:#x} {case['name']}: geometry memory changed"
            )
        if bytes(machine.mem_read(globals_segment * 16, 0x10000)) != bytes(
            globals_before
        ):
            raise AssertionError(
                f"{module}:{entry:#x} {case['name']}: FS globals changed"
            )
        actual_data = bytes(machine.mem_read(data_segment * 16, 0x10000))
        if actual_data != bytes(data_expected):
            differing = [
                offset
                for offset, (actual, expected) in enumerate(
                    zip(actual_data, data_expected)
                )
                if actual != expected
            ]
            scratch_ranges = (
                (layout["scratch_low_start"], layout["scratch_low_end"]),
                (layout["scratch_high_start"], layout["scratch_high_end"]),
            )
            unexpected = [
                offset
                for offset in differing
                if not any(start <= offset < end for start, end in scratch_ranges)
            ]
            if unexpected:
                raise AssertionError(
                    f"{module}:{entry:#x} {case['name']}: raster memory differs "
                    f"outside volatile scratch at {unexpected[:16]}"
                )
        for register, expected in (
            (UC_X86_REG_DS, data_segment),
            (UC_X86_REG_ES, geometry_segment),
            (UC_X86_REG_FS, globals_segment),
            (UC_X86_REG_GS, initial["gs"]),
            (UC_X86_REG_SS, stack_segment),
            (UC_X86_REG_SP, 0xFF02),
        ):
            actual = machine.reg_read(register)
            if actual != expected:
                raise AssertionError(
                    f"{module}:{entry:#x} {case['name']}: "
                    f"segment/stack register {register}={actual:#x}, "
                    f"expected={expected:#x}"
                )
        if bytes(machine.mem_read(stack_segment * 16 + 0xFF02, 4)) != stack_sentinel:
            raise AssertionError(
                f"{module}:{entry:#x} {case['name']}: stack sentinel changed"
            )

        vectors.append(
            {
                "name": case["name"],
                "module": module,
                "entry": entry,
                "screen": [list(pair) for pair in screen],
                "accepted": accepted,
                "advance_offset": writes.get(0x2C),
                "remaining": writes.get(0x2E),
                "record_sha256": hashlib.sha256(actual_record).hexdigest(),
            }
        )

    return vectors


def manu3_face_gradient_vectors() -> list[dict[str, object]]:
    return face_gradient_vectors(
        "manu3",
        0x0D7D,
        "823c014f74d7371b875944a9ae293253654327074a745ec5605fdea15c3aa1a5",
        {
            "free_head": 0x0908,
            "active_head": 0x0964,
            "active_tail": 0x0A18,
            "max_face_width": 0x0190,
            "advance_secondary": 0x0CCA,
            "advance_switch": 0x0D19,
            "advance_remove": 0x0D5E,
            "reciprocal_data": 0x0A280,
            "scratch_low_start": 0x061C,
            "scratch_low_end": 0x0634,
            "scratch_high_start": 0x0670,
            "scratch_high_end": 0x067E,
        },
    )


def alien_face_activate_vectors(
    module: str,
    entry: int,
    body_hash: str,
    layout: dict[str, int],
) -> list[dict[str, object]]:
    return face_gradient_vectors(
        module,
        entry,
        body_hash,
        layout,
        (
            {
                "name": "maximum_vertical_width_accepted",
                "screen": ((10, 20), (10, 519), (509, 30)),
            },
            {
                "name": "horizontal_width_limit_rejected",
                "screen": ((10, 20), (10, 100), (510, 30)),
            },
            {
                "name": "vertical_span_limit_rejected",
                "screen": ((10, 20), (10, 520), (509, 30)),
            },
        ),
    )


def _signed_divide(dividend: int, divisor: int) -> tuple[int, int]:
    quotient = abs(dividend) // divisor
    if dividend < 0:
        quotient = -quotient
    return quotient, dividend - quotient * divisor


def manu3_full_renderer_vectors() -> list[dict[str, object]]:
    module = "manu3"
    entry = 0x0700
    renderer_entry = 0x0775
    face_activate = 0x0D7D
    image = load_image(module)
    expected_hash = "a687e5cd5b80445d293f096ee73c2952ad61052dc1613636d24d53fdc1484161"
    if hashlib.sha256(image[entry:face_activate]).hexdigest() != expected_hash:
        raise AssertionError(
            f"{module}:{entry:#x}: recovered 1661-byte renderer body changed"
        )

    globals_segment = 0x3000
    geometry_segment = 0x5000
    raster_segment = 0x7000
    texture_segment = 0x9000
    mode_x_segment = 0xA000
    linear_segment = 0xB000
    stack_segment = 0xC000
    return_address = 0xF000
    face_offset = 0x1000
    record_offset = 0x0A72
    head_offset = 0x0964
    middle_offset = 0x09BE

    def u16(value: int) -> int:
        return value & 0xFFFF

    def u32(value: int) -> int:
        return value & 0xFFFFFFFF

    def i16(value: int) -> int:
        value &= 0xFFFF
        return value if value < 0x8000 else value - 0x10000

    def write_word(buffer: bytearray, offset: int, value: int) -> None:
        struct.pack_into("<H", buffer, offset, u16(value))

    def write_dword(buffer: bytearray, offset: int, value: int) -> None:
        struct.pack_into("<I", buffer, offset, u32(value))

    def initialize_raster() -> bytearray:
        raster = bytearray(0x10000)
        write_word(raster, 0x067E, 0x0AE0)
        return raster

    vectors: list[dict[str, object]] = []

    raster_before = initialize_raster()
    globals_before = bytearray(0x10000)
    write_word(globals_before, 0x0002, geometry_segment)
    write_word(globals_before, 0x0006, raster_segment)
    initial = {
        "eax": 0xA1A1BEEF,
        "ebx": 0xB2B22345,
        "ecx": 0xC3C33456,
        "edx": 0xD4D44567,
        "esi": 0xE5E55678,
        "edi": 0xF6F66789,
        "ebp": 0x9797789A,
        "sp": 0xFF00,
        "ds": geometry_segment,
        "es": raster_segment,
        "fs": globals_segment,
        "gs": 0x2800,
        "ss": stack_segment,
        "flags": 0x0A93,
    }
    stack_sentinel = bytes.fromhex("5aa596698778")
    outputs: list[tuple[int, int, int]] = []

    def output_handler(
        _machine: Uc, port: int, size: int, value: int
    ) -> None:
        outputs.append((port, size, value))

    machine = execute(
        image,
        renderer_entry,
        return_address,
        initial,
        [
            (0, return_address, b"\xcc"),
            (globals_segment, 0, bytes(globals_before)),
            (raster_segment, 0, bytes(raster_before)),
            (
                stack_segment,
                0xFF00,
                struct.pack("<H", return_address) + stack_sentinel,
            ),
        ],
        max_instructions=10000,
        output_handler=output_handler,
    )
    if outputs:
        raise AssertionError(f"{module}:{renderer_entry:#x}: empty frame wrote VGA")
    raster_after = bytes(machine.mem_read(raster_segment * 16, 0x10000))
    if struct.unpack_from("<HH", raster_after, 0x0680) != (0, 0):
        raise AssertionError(f"{module}:{renderer_entry:#x}: empty frame cursor differs")
    if struct.unpack_from("<H", raster_after, 0x0684)[0] != 0x0686:
        raise AssertionError(f"{module}:{renderer_entry:#x}: empty bucket cursor differs")
    if struct.unpack_from("<H", raster_after, 0x0908)[0] != record_offset:
        raise AssertionError(f"{module}:{renderer_entry:#x}: free-list head differs")
    for index in range(200):
        offset = record_offset + index * 0x5A
        expected_next = 0 if index == 199 else offset + 0x5A
        if struct.unpack_from("<H", raster_after, offset)[0] != expected_next:
            raise AssertionError(
                f"{module}:{renderer_entry:#x}: pool link {index} differs"
            )
    vectors.append(
        {
            "name": "empty_bucket_table",
            "module": module,
            "entry": renderer_entry,
            "pool_records": 200,
            "free_head": record_offset,
            "bucket_cursor": 0x0686,
            "vga_outputs": [],
        }
    )

    cases = (
        {
            "name": "mode_x_plane_one",
            "column": 5,
            "continuation": 0x0AE0,
            "remaining": 0,
            "edge_0": 2 << 16,
            "edge_1": 6 << 16,
            "texture_u": 0x1200,
            "texture_v": 0x3400,
            "texture_du": 0x0100,
            "texture_dv": 0x0200,
        },
        {
            "name": "linear_three_columns_with_edge_steps",
            "column": 2,
            "continuation": 0x0BD6,
            "remaining": 2,
            "edge_0": 1 << 16,
            "edge_1": 4 << 16,
            "edge_0_step": 1 << 16,
            "edge_1_step": 1 << 16,
            "texture_u": 0x1000,
            "texture_v": 0x2000,
            "texture_u_step": 0x0300,
            "texture_v_step": -0x0100,
            "texture_du": 0x0080,
            "texture_dv": 0x0100,
        },
        {
            "name": "linear_negative_top_clip",
            "column": 0,
            "continuation": 0x0BD6,
            "remaining": 0,
            "edge_0": -2 << 16,
            "edge_1": 3 << 16,
            "texture_u": 0x2200,
            "texture_v": 0x1100,
            "texture_du": 0x0100,
            "texture_dv": 0x0080,
            "depth_gradient": 0x00018000,
        },
        {
            "name": "four_plane_sparse_columns",
            "column": 0,
            "continuation": 0x0AA4,
            "remaining": 4,
            "edge_0": 2 << 16,
            "edge_1": 4 << 16,
            "texture_u": 0x0800,
            "texture_v": 0x1800,
            "texture_u_step": 0x0100,
            "texture_du": 0x0080,
            "texture_dv": 0x0040,
        },
        {
            "name": "secondary_left_edge_switch",
            "column": 0,
            "continuation": 0x0BD6,
            "remaining": 0,
            "edge_0": 1 << 16,
            "edge_1": 5 << 16,
            "texture_u": 0x1000,
            "texture_v": 0x2000,
            "texture_du": 0x0100,
            "advance": 0x0CCA,
            "secondary_remaining": 1,
            "secondary_edge": 2 << 16,
            "secondary_texture_u": 0x4000,
            "secondary_texture_v": 0x3000,
            "secondary_texture_u_step": 0x0100,
        },
        {
            "name": "secondary_right_edge_switch",
            "column": 0,
            "continuation": 0x0BD6,
            "remaining": 0,
            "edge_0": 1 << 16,
            "edge_1": 4 << 16,
            "edge_0_step": 1 << 16,
            "texture_u": 0x1400,
            "texture_v": 0x2800,
            "texture_u_step": 0x0100,
            "texture_du": 0x0080,
            "advance": 0x0D19,
            "secondary_remaining": 1,
            "secondary_edge": 5 << 16,
            "secondary_edge_step": 1 << 16,
        },
    )
    texture = bytes((offset * 37 + 11) & 0xFF for offset in range(0x10000))

    for case_index, case in enumerate(cases):
        column = int(case["column"])
        continuation = int(case["continuation"])
        geometry_before = bytearray(0x10000)
        raster_before = initialize_raster()
        globals_before = bytearray(0x10000)
        mode_x_expected = bytearray(0x10000)
        linear_expected = bytearray(0x10000)
        vertex_offsets = (0x3000, 0x3020, 0x3040)

        write_word(globals_before, 0x0002, geometry_segment)
        write_word(globals_before, 0x0006, raster_segment)
        write_word(globals_before, 0x0014, linear_segment)
        write_word(globals_before, 0x0018, mode_x_segment)
        write_word(globals_before, 0x2300, face_offset)
        write_word(globals_before, 0x2304, 1)
        write_word(raster_before, 0x067E, continuation)
        write_word(geometry_before, face_offset, 0xA55A)
        for field_offset, vertex_offset in zip((2, 4, 6), vertex_offsets):
            write_word(geometry_before, face_offset + field_offset, vertex_offset)
        for vertex_index, vertex_offset in enumerate(vertex_offsets):
            write_word(geometry_before, vertex_offset + 0x0A, column + vertex_index)
            write_word(geometry_before, vertex_offset + 0x12, 0)

        state = {
            "remaining": int(case["remaining"]),
            "edge_0": u32(int(case["edge_0"])),
            "edge_0_step": u32(int(case.get("edge_0_step", 0))),
            "edge_1": u32(int(case["edge_1"])),
            "edge_1_step": u32(int(case.get("edge_1_step", 0))),
            "depth": 0x01000000,
            "depth_step": 0,
            "depth_gradient": u32(int(case.get("depth_gradient", 0))),
            "texture_u": u16(int(case["texture_u"])),
            "texture_v": u16(int(case["texture_v"])),
            "texture_u_step": u16(int(case.get("texture_u_step", 0))),
            "texture_v_step": u16(int(case.get("texture_v_step", 0))),
            "texture_du": u16(int(case.get("texture_du", 0))),
            "texture_dv": u16(int(case.get("texture_dv", 0))),
            "advance": int(case.get("advance", 0x0D5E)),
            "secondary_remaining": int(case.get("secondary_remaining", 0)),
            "secondary_edge": u32(int(case.get("secondary_edge", 0))),
            "secondary_edge_step": u32(
                int(case.get("secondary_edge_step", 0))
            ),
            "secondary_depth": 0x01000000,
            "secondary_depth_step": 0,
            "secondary_texture_u": u16(
                int(case.get("secondary_texture_u", 0))
            ),
            "secondary_texture_v": u16(
                int(case.get("secondary_texture_v", 0))
            ),
            "secondary_texture_u_step": u16(
                int(case.get("secondary_texture_u_step", 0))
            ),
            "secondary_texture_v_step": u16(
                int(case.get("secondary_texture_v_step", 0))
            ),
        }
        expected_outputs: list[tuple[int, int, int]] = []
        expected_pixels: list[dict[str, int]] = []
        active_column = column
        active = True
        while active:
            draw = continuation != 0x0AA4 or (active_column & 3) == 0
            if draw:
                if continuation == 0x0BD6:
                    target = linear_expected
                    base = active_column
                    stride = 320
                else:
                    target = mode_x_expected
                    base = active_column >> 2
                    stride = 80
                    mask = (
                        0x0F02
                        if continuation == 0x0AA4
                        else 0x0002 | (0x0100 << (active_column & 3))
                    )
                    expected_outputs.append((0x03C4, 2, mask))
                start_y = max(i16(state["edge_0"] >> 16), 0)
                end_y = i16(state["edge_1"] >> 16)
                relative = start_y - i16(state["edge_0"] >> 16)
                texture_u = u16(
                    state["texture_u"] + state["texture_du"] * relative
                )
                texture_v = u16(
                    state["texture_v"] + state["texture_dv"] * relative
                )
                for y in range(start_y, end_y):
                    texture_offset = (texture_u >> 8) | (texture_v & 0xFF00)
                    value = texture[texture_offset]
                    output_offset = u16(base + y * stride)
                    target[output_offset] = value
                    expected_pixels.append(
                        {"x": active_column, "y": y, "value": value}
                    )
                    texture_u = u16(texture_u + state["texture_du"])
                    texture_v = u16(texture_v + state["texture_dv"])

            state["remaining"] = i16(state["remaining"] - 1)
            if state["remaining"] >= 0:
                state["texture_u"] = u16(
                    state["texture_u"] + state["texture_u_step"]
                )
                state["texture_v"] = u16(
                    state["texture_v"] + state["texture_v_step"]
                )
                state["edge_0"] = u32(state["edge_0"] + state["edge_0_step"])
                state["edge_1"] = u32(state["edge_1"] + state["edge_1_step"])
                state["depth"] = u32(state["depth"] + state["depth_step"])
            elif state["advance"] == 0x0CCA:
                state["edge_0"] = state["secondary_edge"]
                state["edge_0_step"] = state["secondary_edge_step"]
                state["depth"] = state["secondary_depth"]
                state["depth_step"] = state["secondary_depth_step"]
                state["texture_u"] = state["secondary_texture_u"]
                state["texture_v"] = state["secondary_texture_v"]
                state["texture_u_step"] = state["secondary_texture_u_step"]
                state["texture_v_step"] = state["secondary_texture_v_step"]
                state["remaining"] = state["secondary_remaining"]
                state["advance"] = 0x0D5E
                state["edge_1"] = u32(state["edge_1"] + state["edge_1_step"])
            elif state["advance"] == 0x0D19:
                state["edge_0"] = u32(state["edge_0"] + state["edge_0_step"])
                state["depth"] = u32(state["depth"] + state["depth_step"])
                state["texture_u"] = u16(
                    state["texture_u"] + state["texture_u_step"]
                )
                state["texture_v"] = u16(
                    state["texture_v"] + state["texture_v_step"]
                )
                state["edge_1"] = state["secondary_edge"]
                state["edge_1_step"] = state["secondary_edge_step"]
                state["remaining"] = state["secondary_remaining"]
                state["advance"] = 0x0D5E
            else:
                active = False
            active_column += 1

        patched_image = bytearray(image)
        patched_image[face_activate] = 0xC3
        activations: list[int] = []
        clipped_sort_keys: list[int] = []
        outputs = []

        def code_handler(
            machine: Uc, address: int, _size: int, _data: object
        ) -> None:
            if address == 0x0960:
                clipped_sort_keys.append(
                    struct.unpack(
                        "<I",
                        machine.mem_read(
                            raster_segment * 16 + record_offset + 4, 4
                        ),
                    )[0]
                )
                return
            if address != face_activate:
                return
            activations.append(machine.reg_read(UC_X86_REG_ESI) & 0xFFFF)
            raster_base = raster_segment * 16
            next_free = struct.unpack(
                "<H", machine.mem_read(raster_base + record_offset, 2)
            )[0]
            record_data = bytearray(0x5A)
            write_word(record_data, 0x00, middle_offset)
            write_dword(record_data, 0x08, state_initial["edge_0"])
            write_dword(record_data, 0x0C, state_initial["edge_0_step"])
            write_word(record_data, 0x10, head_offset)
            write_dword(record_data, 0x18, state_initial["edge_1"])
            write_dword(record_data, 0x1C, state_initial["edge_1_step"])
            write_dword(record_data, 0x20, state_initial["depth"])
            write_dword(record_data, 0x24, state_initial["depth_step"])
            write_dword(record_data, 0x28, state_initial["depth_gradient"])
            write_word(record_data, 0x2C, state_initial["advance"])
            write_word(record_data, 0x2E, state_initial["remaining"])
            write_word(
                record_data, 0x30, state_initial["secondary_remaining"]
            )
            write_dword(record_data, 0x32, state_initial["secondary_edge"])
            write_dword(
                record_data, 0x36, state_initial["secondary_edge_step"]
            )
            write_dword(record_data, 0x3A, state_initial["secondary_depth"])
            write_dword(
                record_data, 0x3E, state_initial["secondary_depth_step"]
            )
            write_word(record_data, 0x42, state_initial["texture_u"])
            write_word(record_data, 0x44, state_initial["texture_v"])
            write_word(
                record_data, 0x46, state_initial["secondary_texture_u"]
            )
            write_word(
                record_data, 0x48, state_initial["secondary_texture_v"]
            )
            write_word(record_data, 0x4A, state_initial["texture_u_step"])
            write_word(record_data, 0x4C, state_initial["texture_v_step"])
            write_word(
                record_data, 0x4E, state_initial["secondary_texture_u_step"]
            )
            write_word(
                record_data, 0x50, state_initial["secondary_texture_v_step"]
            )
            write_word(record_data, 0x52, state_initial["texture_du"])
            write_word(record_data, 0x54, state_initial["texture_dv"])
            write_word(record_data, 0x56, texture_segment)
            machine.mem_write(raster_base + record_offset, bytes(record_data))
            machine.mem_write(
                raster_base + 0x0908, struct.pack("<H", next_free)
            )
            machine.mem_write(
                raster_base + head_offset,
                struct.pack("<H", record_offset),
            )
            machine.mem_write(
                raster_base + middle_offset + 0x10,
                struct.pack("<H", record_offset),
            )

        state_initial = dict(state)
        state_initial.update(
            {
                "remaining": int(case["remaining"]),
                "edge_0": u32(int(case["edge_0"])),
                "edge_0_step": u32(int(case.get("edge_0_step", 0))),
                "edge_1": u32(int(case["edge_1"])),
                "edge_1_step": u32(int(case.get("edge_1_step", 0))),
                "depth": 0x01000000,
                "depth_step": 0,
                "depth_gradient": u32(int(case.get("depth_gradient", 0))),
                "texture_u": u16(int(case["texture_u"])),
                "texture_v": u16(int(case["texture_v"])),
                "texture_u_step": u16(int(case.get("texture_u_step", 0))),
                "texture_v_step": u16(int(case.get("texture_v_step", 0))),
                "texture_du": u16(int(case.get("texture_du", 0))),
                "texture_dv": u16(int(case.get("texture_dv", 0))),
                "advance": int(case.get("advance", 0x0D5E)),
                "secondary_remaining": int(
                    case.get("secondary_remaining", 0)
                ),
                "secondary_edge": u32(int(case.get("secondary_edge", 0))),
                "secondary_edge_step": u32(
                    int(case.get("secondary_edge_step", 0))
                ),
                "secondary_depth": 0x01000000,
                "secondary_depth_step": 0,
                "secondary_texture_u": u16(
                    int(case.get("secondary_texture_u", 0))
                ),
                "secondary_texture_v": u16(
                    int(case.get("secondary_texture_v", 0))
                ),
                "secondary_texture_u_step": u16(
                    int(case.get("secondary_texture_u_step", 0))
                ),
                "secondary_texture_v_step": u16(
                    int(case.get("secondary_texture_v_step", 0))
                ),
            }
        )
        initial = {
            "eax": 0xA1A1BEEF + case_index,
            "ebx": 0xB2B22345 + case_index,
            "ecx": 0xC3C33456 + case_index,
            "edx": 0xD4D44567 + case_index,
            "esi": 0xE5E55678 + case_index,
            "edi": 0xF6F66789 + case_index,
            "ebp": 0x9797789A + case_index,
            "sp": 0xFF00,
            "ds": geometry_segment,
            "es": raster_segment,
            "fs": globals_segment,
            "gs": 0x2800,
            "ss": stack_segment,
            "flags": 0x0A93,
        }
        machine = execute(
            bytes(patched_image),
            entry,
            return_address,
            initial,
            [
                (0, return_address, b"\xcc"),
                (globals_segment, 0, bytes(globals_before)),
                (geometry_segment, 0, bytes(geometry_before)),
                (raster_segment, 0, bytes(raster_before)),
                (texture_segment, 0, texture),
                (mode_x_segment, 0, bytes(0x10000)),
                (linear_segment, 0, bytes(0x10000)),
                (
                    stack_segment,
                    0xFF00,
                    struct.pack("<H", return_address) + stack_sentinel,
                ),
            ],
            max_instructions=500000,
            output_handler=output_handler,
            code_handler=code_handler,
        )
        if activations != [face_offset]:
            raise AssertionError(
                f"{module}:{entry:#x} {case['name']}: activations={activations}"
            )
        expected_clipped_sort_keys = []
        if i16(state_initial["edge_0"] >> 16) < 0:
            expected_clipped_sort_keys.append(
                u32(
                    state_initial["depth"]
                    - i16(state_initial["edge_0"] >> 16)
                    * state_initial["depth_gradient"]
                )
            )
        if clipped_sort_keys != expected_clipped_sort_keys:
            raise AssertionError(
                f"{module}:{entry:#x} {case['name']}: clipped sort keys="
                f"{clipped_sort_keys}, expected={expected_clipped_sort_keys}"
            )
        if outputs != expected_outputs:
            raise AssertionError(
                f"{module}:{entry:#x} {case['name']}: outputs={outputs}, "
                f"expected={expected_outputs}"
            )
        actual_mode_x = bytes(machine.mem_read(mode_x_segment * 16, 0x10000))
        actual_linear = bytes(machine.mem_read(linear_segment * 16, 0x10000))
        if actual_mode_x != bytes(mode_x_expected):
            differences = [
                (offset, actual_mode_x[offset], mode_x_expected[offset])
                for offset in range(0x10000)
                if actual_mode_x[offset] != mode_x_expected[offset]
            ][:8]
            raise AssertionError(
                f"{module}:{entry:#x} {case['name']}: mode-X differs at {differences}"
            )
        if actual_linear != bytes(linear_expected):
            differences = [
                (offset, actual_linear[offset], linear_expected[offset])
                for offset in range(0x10000)
                if actual_linear[offset] != linear_expected[offset]
            ][:8]
            raise AssertionError(
                f"{module}:{entry:#x} {case['name']}: linear differs at {differences}"
            )
        raster_after = bytes(machine.mem_read(raster_segment * 16, 0x10000))
        geometry_after = bytes(machine.mem_read(geometry_segment * 16, 0x10000))
        if struct.unpack_from("<H", geometry_after, face_offset)[0] != 0:
            raise AssertionError(
                f"{module}:{entry:#x} {case['name']}: face bucket link differs"
            )
        if struct.unpack_from("<H", raster_after, 0x0908)[0] != record_offset:
            raise AssertionError(
                f"{module}:{entry:#x} {case['name']}: record was not freed"
            )
        if struct.unpack_from("<H", raster_after, head_offset)[0] != middle_offset:
            raise AssertionError(
                f"{module}:{entry:#x} {case['name']}: active head differs"
            )
        head_skip_boundary = (
            struct.unpack_from("<H", raster_after, head_offset + 2)[0],
            struct.unpack_from("<H", raster_after, head_offset + 6)[0],
        )
        expected_head_boundary = (
            0x0974
            if i16(state_initial["edge_0"] >> 16) < 0
            else record_offset
        )
        if head_skip_boundary != (1, expected_head_boundary):
            raise AssertionError(
                f"{module}:{entry:#x} {case['name']}: head skip boundary="
                f"{head_skip_boundary}"
            )
        if struct.unpack_from("<H", raster_after, middle_offset + 0x10)[0] != head_offset:
            raise AssertionError(
                f"{module}:{entry:#x} {case['name']}: active previous differs"
            )
        if struct.unpack_from("<H", raster_after, 0x0680)[0] != 0x013F:
            raise AssertionError(
                f"{module}:{entry:#x} {case['name']}: final column differs"
            )
        vectors.append(
            {
                "name": case["name"],
                "module": module,
                "entry": entry,
                "bucket_column": column,
                "continuation": continuation,
                "active_columns": active_column - column,
                "pixels": expected_pixels,
                "vga_outputs": [list(item) for item in expected_outputs],
                "clipped_sort_keys": clipped_sort_keys,
                "record_returned_to_free_list": True,
            }
        )

    return vectors

def alien_full_renderer_vectors(
    module: str,
    entry: int,
    face_activate: int,
    body_hash: str,
    layout: dict[str, int],
) -> list[dict[str, object]]:
    renderer_entry = entry
    image = load_image(module)
    body_size = face_activate - entry
    if hashlib.sha256(image[entry:face_activate]).hexdigest() != body_hash:
        raise AssertionError(
            f"{module}:{entry:#x}: recovered {body_size}-byte renderer body changed"
        )

    globals_segment = 0x3000
    geometry_segment = 0x5000
    raster_segment = 0x7000
    texture_segment = 0x9000
    mode_x_segment = 0xA000
    linear_segment = 0xB000
    stack_segment = 0xC000
    return_address = 0xF000
    face_offset = 0x1000
    record_offset = layout["raster_pool"]
    head_offset = layout["active_head"]
    middle_offset = layout["active_middle"]

    def u16(value: int) -> int:
        return value & 0xFFFF

    def u32(value: int) -> int:
        return value & 0xFFFFFFFF

    def i16(value: int) -> int:
        value &= 0xFFFF
        return value if value < 0x8000 else value - 0x10000

    def write_word(buffer: bytearray, offset: int, value: int) -> None:
        struct.pack_into("<H", buffer, offset, u16(value))

    def write_dword(buffer: bytearray, offset: int, value: int) -> None:
        struct.pack_into("<I", buffer, offset, u32(value))

    def initialize_raster() -> bytearray:
        raster = bytearray(0x10000)
        write_word(
            raster,
            layout["render_continuation"],
            layout["render_mode_x"],
        )
        return raster

    vectors: list[dict[str, object]] = []

    raster_before = initialize_raster()
    globals_before = bytearray(0x10000)
    write_word(globals_before, 0x0002, geometry_segment)
    write_word(globals_before, 0x0006, raster_segment)
    initial = {
        "eax": 0xA1A1BEEF,
        "ebx": 0xB2B22345,
        "ecx": 0xC3C33456,
        "edx": 0xD4D44567,
        "esi": 0xE5E55678,
        "edi": 0xF6F66789,
        "ebp": 0x9797789A,
        "sp": 0xFF00,
        "ds": geometry_segment,
        "es": raster_segment,
        "fs": globals_segment,
        "gs": 0x2800,
        "ss": stack_segment,
        "flags": 0x0A93,
    }
    stack_sentinel = bytes.fromhex("5aa596698778")
    outputs: list[tuple[int, int, int]] = []

    def output_handler(
        _machine: Uc, port: int, size: int, value: int
    ) -> None:
        outputs.append((port, size, value))

    machine = execute(
        image,
        renderer_entry,
        return_address,
        initial,
        [
            (0, return_address, b"\xcc"),
            (globals_segment, 0, bytes(globals_before)),
            (raster_segment, 0, bytes(raster_before)),
            (
                stack_segment,
                0xFF00,
                struct.pack("<H", return_address) + stack_sentinel,
            ),
        ],
        max_instructions=10000,
        output_handler=output_handler,
    )
    if outputs:
        raise AssertionError(f"{module}:{renderer_entry:#x}: empty frame wrote VGA")
    raster_after = bytes(machine.mem_read(raster_segment * 16, 0x10000))
    if struct.unpack_from("<HH", raster_after, layout["column"]) != (0, 0):
        raise AssertionError(f"{module}:{renderer_entry:#x}: empty frame cursor differs")
    if struct.unpack_from("<H", raster_after, layout["bucket_cursor"])[0] != layout["bucket_heads"]:
        raise AssertionError(f"{module}:{renderer_entry:#x}: empty bucket cursor differs")
    if struct.unpack_from("<H", raster_after, layout["free_head"])[0] != record_offset:
        raise AssertionError(f"{module}:{renderer_entry:#x}: free-list head differs")
    for index in range(layout["pool_count"]):
        offset = record_offset + index * 0x5A
        expected_next = 0 if index == layout["pool_count"] - 1 else offset + 0x5A
        if struct.unpack_from("<H", raster_after, offset)[0] != expected_next:
            raise AssertionError(
                f"{module}:{renderer_entry:#x}: pool link {index} differs"
            )
    vectors.append(
        {
            "name": "empty_bucket_table",
            "module": module,
            "entry": renderer_entry,
            "pool_records": layout["pool_count"],
            "free_head": record_offset,
            "bucket_cursor": layout["bucket_heads"],
            "vga_outputs": [],
        }
    )

    cases = (
        {
            "name": "mode_x_plane_one",
            "column": 5,
            "continuation": layout["render_mode_x"],
            "remaining": 0,
            "edge_0": 2 << 16,
            "edge_1": 6 << 16,
            "texture_u": 0x1200,
            "texture_v": 0x3400,
            "texture_du": 0x0100,
            "texture_dv": 0x0200,
        },
        {
            "name": "linear_three_columns_with_edge_steps",
            "column": 2,
            "continuation": layout["render_linear"],
            "remaining": 2,
            "edge_0": 1 << 16,
            "edge_1": 4 << 16,
            "edge_0_step": 1 << 16,
            "edge_1_step": 1 << 16,
            "texture_u": 0x1000,
            "texture_v": 0x2000,
            "texture_u_step": 0x0300,
            "texture_v_step": -0x0100,
            "texture_du": 0x0080,
            "texture_dv": 0x0100,
        },
        {
            "name": "linear_negative_top_clip",
            "column": 0,
            "continuation": layout["render_linear"],
            "remaining": 0,
            "edge_0": -2 << 16,
            "edge_1": 3 << 16,
            "texture_u": 0x2200,
            "texture_v": 0x1100,
            "texture_du": 0x0100,
            "texture_dv": 0x0080,
            "depth_gradient": 0x00018000,
        },
        {
            "name": "four_plane_sparse_columns",
            "column": 0,
            "continuation": layout["render_four_planes"],
            "remaining": 4,
            "edge_0": 2 << 16,
            "edge_1": 4 << 16,
            "texture_u": 0x0800,
            "texture_v": 0x1800,
            "texture_u_step": 0x0100,
            "texture_du": 0x0080,
            "texture_dv": 0x0040,
        },
        {
            "name": "secondary_left_edge_switch",
            "column": 0,
            "continuation": layout["render_linear"],
            "remaining": 0,
            "edge_0": 1 << 16,
            "edge_1": 5 << 16,
            "texture_u": 0x1000,
            "texture_v": 0x2000,
            "texture_du": 0x0100,
            "advance": layout["advance_secondary"],
            "secondary_remaining": 1,
            "secondary_edge": 2 << 16,
            "secondary_texture_u": 0x4000,
            "secondary_texture_v": 0x3000,
            "secondary_texture_u_step": 0x0100,
        },
        {
            "name": "secondary_right_edge_switch",
            "column": 0,
            "continuation": layout["render_linear"],
            "remaining": 0,
            "edge_0": 1 << 16,
            "edge_1": 4 << 16,
            "edge_0_step": 1 << 16,
            "texture_u": 0x1400,
            "texture_v": 0x2800,
            "texture_u_step": 0x0100,
            "texture_du": 0x0080,
            "advance": layout["advance_switch"],
            "secondary_remaining": 1,
            "secondary_edge": 5 << 16,
            "secondary_edge_step": 1 << 16,
        },
    )
    texture = bytes((offset * 37 + 11) & 0xFF for offset in range(0x10000))

    for case_index, case in enumerate(cases):
        column = int(case["column"])
        continuation = int(case["continuation"])
        geometry_before = bytearray(0x10000)
        raster_before = initialize_raster()
        globals_before = bytearray(0x10000)
        mode_x_expected = bytearray(0x10000)
        linear_expected = bytearray(0x10000)
        vertex_offsets = (0x3000, 0x3020, 0x3040)

        write_word(globals_before, 0x0002, geometry_segment)
        write_word(globals_before, 0x0006, raster_segment)
        write_word(globals_before, 0x0024, linear_segment)
        write_word(globals_before, 0x0028, mode_x_segment)
        write_word(
            raster_before,
            layout["render_continuation"],
            continuation,
        )
        write_word(
            raster_before,
            layout["bucket_heads"] + column * 2,
            face_offset,
        )
        write_word(geometry_before, face_offset, 0)
        for field_offset, vertex_offset in zip((2, 4, 6), vertex_offsets):
            write_word(geometry_before, face_offset + field_offset, vertex_offset)
        for vertex_index, vertex_offset in enumerate(vertex_offsets):
            write_word(geometry_before, vertex_offset + 0x0A, column + vertex_index)
            write_word(geometry_before, vertex_offset + 0x12, 0)

        state = {
            "remaining": int(case["remaining"]),
            "edge_0": u32(int(case["edge_0"])),
            "edge_0_step": u32(int(case.get("edge_0_step", 0))),
            "edge_1": u32(int(case["edge_1"])),
            "edge_1_step": u32(int(case.get("edge_1_step", 0))),
            "depth": 0x01000000,
            "depth_step": 0,
            "depth_gradient": u32(int(case.get("depth_gradient", 0))),
            "texture_u": u16(int(case["texture_u"])),
            "texture_v": u16(int(case["texture_v"])),
            "texture_u_step": u16(int(case.get("texture_u_step", 0))),
            "texture_v_step": u16(int(case.get("texture_v_step", 0))),
            "texture_du": u16(int(case.get("texture_du", 0))),
            "texture_dv": u16(int(case.get("texture_dv", 0))),
            "advance": int(case.get("advance", layout["advance_remove"])),
            "secondary_remaining": int(case.get("secondary_remaining", 0)),
            "secondary_edge": u32(int(case.get("secondary_edge", 0))),
            "secondary_edge_step": u32(
                int(case.get("secondary_edge_step", 0))
            ),
            "secondary_depth": 0x01000000,
            "secondary_depth_step": 0,
            "secondary_texture_u": u16(
                int(case.get("secondary_texture_u", 0))
            ),
            "secondary_texture_v": u16(
                int(case.get("secondary_texture_v", 0))
            ),
            "secondary_texture_u_step": u16(
                int(case.get("secondary_texture_u_step", 0))
            ),
            "secondary_texture_v_step": u16(
                int(case.get("secondary_texture_v_step", 0))
            ),
        }
        expected_outputs: list[tuple[int, int, int]] = []
        expected_pixels: list[dict[str, int]] = []
        active_column = column
        active = True
        while active:
            draw = continuation != layout["render_four_planes"] or (active_column & 3) == 0
            if draw:
                if continuation == layout["render_linear"]:
                    target = linear_expected
                    base = active_column
                    stride = 320
                else:
                    target = mode_x_expected
                    base = active_column >> 2
                    stride = 80
                    mask = (
                        0x0F02
                        if continuation == layout["render_four_planes"]
                        else 0x0002 | (0x0100 << (active_column & 3))
                    )
                    expected_outputs.append((0x03C4, 2, mask))
                start_y = max(i16(state["edge_0"] >> 16), 0)
                end_y = i16(state["edge_1"] >> 16)
                relative = start_y - i16(state["edge_0"] >> 16)
                texture_u = u16(
                    state["texture_u"] + state["texture_du"] * relative
                )
                texture_v = u16(
                    state["texture_v"] + state["texture_dv"] * relative
                )
                for y in range(start_y, end_y):
                    texture_offset = (texture_u >> 8) | (texture_v & 0xFF00)
                    value = texture[texture_offset]
                    output_offset = u16(base + y * stride)
                    target[output_offset] = value
                    expected_pixels.append(
                        {"x": active_column, "y": y, "value": value}
                    )
                    texture_u = u16(texture_u + state["texture_du"])
                    texture_v = u16(texture_v + state["texture_dv"])

            state["remaining"] = i16(state["remaining"] - 1)
            if state["remaining"] >= 0:
                state["texture_u"] = u16(
                    state["texture_u"] + state["texture_u_step"]
                )
                state["texture_v"] = u16(
                    state["texture_v"] + state["texture_v_step"]
                )
                state["edge_0"] = u32(state["edge_0"] + state["edge_0_step"])
                state["edge_1"] = u32(state["edge_1"] + state["edge_1_step"])
                state["depth"] = u32(state["depth"] + state["depth_step"])
            elif state["advance"] == layout["advance_secondary"]:
                state["edge_0"] = state["secondary_edge"]
                state["edge_0_step"] = state["secondary_edge_step"]
                state["depth"] = state["secondary_depth"]
                state["depth_step"] = state["secondary_depth_step"]
                state["texture_u"] = state["secondary_texture_u"]
                state["texture_v"] = state["secondary_texture_v"]
                state["texture_u_step"] = state["secondary_texture_u_step"]
                state["texture_v_step"] = state["secondary_texture_v_step"]
                state["remaining"] = state["secondary_remaining"]
                state["advance"] = layout["advance_remove"]
                state["edge_1"] = u32(state["edge_1"] + state["edge_1_step"])
            elif state["advance"] == layout["advance_switch"]:
                state["edge_0"] = u32(state["edge_0"] + state["edge_0_step"])
                state["depth"] = u32(state["depth"] + state["depth_step"])
                state["texture_u"] = u16(
                    state["texture_u"] + state["texture_u_step"]
                )
                state["texture_v"] = u16(
                    state["texture_v"] + state["texture_v_step"]
                )
                state["edge_1"] = state["secondary_edge"]
                state["edge_1_step"] = state["secondary_edge_step"]
                state["remaining"] = state["secondary_remaining"]
                state["advance"] = layout["advance_remove"]
            else:
                active = False
            active_column += 1

        patched_image = bytearray(image)
        patched_image[face_activate] = 0xC3
        activations: list[int] = []
        clipped_sort_keys: list[int] = []
        outputs = []

        def code_handler(
            machine: Uc, address: int, _size: int, _data: object
        ) -> None:
            if address == layout["clipped_sort_hook"]:
                clipped_sort_keys.append(
                    struct.unpack(
                        "<I",
                        machine.mem_read(
                            raster_segment * 16 + record_offset + 4, 4
                        ),
                    )[0]
                )
                return
            if address != face_activate:
                return
            activations.append(machine.reg_read(UC_X86_REG_ESI) & 0xFFFF)
            raster_base = raster_segment * 16
            next_free = struct.unpack(
                "<H", machine.mem_read(raster_base + record_offset, 2)
            )[0]
            record_data = bytearray(0x5A)
            write_word(record_data, 0x00, middle_offset)
            write_dword(record_data, 0x08, state_initial["edge_0"])
            write_dword(record_data, 0x0C, state_initial["edge_0_step"])
            write_word(record_data, 0x10, head_offset)
            write_dword(record_data, 0x18, state_initial["edge_1"])
            write_dword(record_data, 0x1C, state_initial["edge_1_step"])
            write_dword(record_data, 0x20, state_initial["depth"])
            write_dword(record_data, 0x24, state_initial["depth_step"])
            write_dword(record_data, 0x28, state_initial["depth_gradient"])
            write_word(record_data, 0x2C, state_initial["advance"])
            write_word(record_data, 0x2E, state_initial["remaining"])
            write_word(
                record_data, 0x30, state_initial["secondary_remaining"]
            )
            write_dword(record_data, 0x32, state_initial["secondary_edge"])
            write_dword(
                record_data, 0x36, state_initial["secondary_edge_step"]
            )
            write_dword(record_data, 0x3A, state_initial["secondary_depth"])
            write_dword(
                record_data, 0x3E, state_initial["secondary_depth_step"]
            )
            write_word(record_data, 0x42, state_initial["texture_u"])
            write_word(record_data, 0x44, state_initial["texture_v"])
            write_word(
                record_data, 0x46, state_initial["secondary_texture_u"]
            )
            write_word(
                record_data, 0x48, state_initial["secondary_texture_v"]
            )
            write_word(record_data, 0x4A, state_initial["texture_u_step"])
            write_word(record_data, 0x4C, state_initial["texture_v_step"])
            write_word(
                record_data, 0x4E, state_initial["secondary_texture_u_step"]
            )
            write_word(
                record_data, 0x50, state_initial["secondary_texture_v_step"]
            )
            write_word(record_data, 0x52, state_initial["texture_du"])
            write_word(record_data, 0x54, state_initial["texture_dv"])
            write_word(record_data, 0x56, texture_segment)
            machine.mem_write(raster_base + record_offset, bytes(record_data))
            machine.mem_write(
                raster_base + layout["free_head"], struct.pack("<H", next_free)
            )
            machine.mem_write(
                raster_base + head_offset,
                struct.pack("<H", record_offset),
            )
            machine.mem_write(
                raster_base + middle_offset + 0x10,
                struct.pack("<H", record_offset),
            )

        state_initial = dict(state)
        state_initial.update(
            {
                "remaining": int(case["remaining"]),
                "edge_0": u32(int(case["edge_0"])),
                "edge_0_step": u32(int(case.get("edge_0_step", 0))),
                "edge_1": u32(int(case["edge_1"])),
                "edge_1_step": u32(int(case.get("edge_1_step", 0))),
                "depth": 0x01000000,
                "depth_step": 0,
                "depth_gradient": u32(int(case.get("depth_gradient", 0))),
                "texture_u": u16(int(case["texture_u"])),
                "texture_v": u16(int(case["texture_v"])),
                "texture_u_step": u16(int(case.get("texture_u_step", 0))),
                "texture_v_step": u16(int(case.get("texture_v_step", 0))),
                "texture_du": u16(int(case.get("texture_du", 0))),
                "texture_dv": u16(int(case.get("texture_dv", 0))),
                "advance": int(case.get("advance", layout["advance_remove"])),
                "secondary_remaining": int(
                    case.get("secondary_remaining", 0)
                ),
                "secondary_edge": u32(int(case.get("secondary_edge", 0))),
                "secondary_edge_step": u32(
                    int(case.get("secondary_edge_step", 0))
                ),
                "secondary_depth": 0x01000000,
                "secondary_depth_step": 0,
                "secondary_texture_u": u16(
                    int(case.get("secondary_texture_u", 0))
                ),
                "secondary_texture_v": u16(
                    int(case.get("secondary_texture_v", 0))
                ),
                "secondary_texture_u_step": u16(
                    int(case.get("secondary_texture_u_step", 0))
                ),
                "secondary_texture_v_step": u16(
                    int(case.get("secondary_texture_v_step", 0))
                ),
            }
        )
        initial = {
            "eax": 0xA1A1BEEF + case_index,
            "ebx": 0xB2B22345 + case_index,
            "ecx": 0xC3C33456 + case_index,
            "edx": 0xD4D44567 + case_index,
            "esi": 0xE5E55678 + case_index,
            "edi": 0xF6F66789 + case_index,
            "ebp": 0x9797789A + case_index,
            "sp": 0xFF00,
            "ds": geometry_segment,
            "es": raster_segment,
            "fs": globals_segment,
            "gs": 0x2800,
            "ss": stack_segment,
            "flags": 0x0A93,
        }
        machine = execute(
            bytes(patched_image),
            entry,
            return_address,
            initial,
            [
                (0, return_address, b"\xcc"),
                (globals_segment, 0, bytes(globals_before)),
                (geometry_segment, 0, bytes(geometry_before)),
                (raster_segment, 0, bytes(raster_before)),
                (texture_segment, 0, texture),
                (mode_x_segment, 0, bytes(0x10000)),
                (linear_segment, 0, bytes(0x10000)),
                (
                    stack_segment,
                    0xFF00,
                    struct.pack("<H", return_address) + stack_sentinel,
                ),
            ],
            max_instructions=500000,
            output_handler=output_handler,
            code_handler=code_handler,
        )
        if activations != [face_offset]:
            raise AssertionError(
                f"{module}:{entry:#x} {case['name']}: activations={activations}"
            )
        expected_clipped_sort_keys = []
        if i16(state_initial["edge_0"] >> 16) < 0:
            expected_clipped_sort_keys.append(
                u32(
                    state_initial["depth"]
                    - i16(state_initial["edge_0"] >> 16)
                    * state_initial["depth_gradient"]
                )
            )
        if clipped_sort_keys != expected_clipped_sort_keys:
            raise AssertionError(
                f"{module}:{entry:#x} {case['name']}: clipped sort keys="
                f"{clipped_sort_keys}, expected={expected_clipped_sort_keys}"
            )
        if outputs != expected_outputs:
            raise AssertionError(
                f"{module}:{entry:#x} {case['name']}: outputs={outputs}, "
                f"expected={expected_outputs}"
            )
        actual_mode_x = bytes(machine.mem_read(mode_x_segment * 16, 0x10000))
        actual_linear = bytes(machine.mem_read(linear_segment * 16, 0x10000))
        if actual_mode_x != bytes(mode_x_expected):
            differences = [
                (offset, actual_mode_x[offset], mode_x_expected[offset])
                for offset in range(0x10000)
                if actual_mode_x[offset] != mode_x_expected[offset]
            ][:8]
            raise AssertionError(
                f"{module}:{entry:#x} {case['name']}: mode-X differs at {differences}"
            )
        if actual_linear != bytes(linear_expected):
            differences = [
                (offset, actual_linear[offset], linear_expected[offset])
                for offset in range(0x10000)
                if actual_linear[offset] != linear_expected[offset]
            ][:8]
            raise AssertionError(
                f"{module}:{entry:#x} {case['name']}: linear differs at {differences}"
            )
        raster_after = bytes(machine.mem_read(raster_segment * 16, 0x10000))
        geometry_after = bytes(machine.mem_read(geometry_segment * 16, 0x10000))
        if struct.unpack_from("<H", geometry_after, face_offset)[0] != 0:
            raise AssertionError(
                f"{module}:{entry:#x} {case['name']}: face bucket link differs"
            )
        if struct.unpack_from("<H", raster_after, layout["free_head"])[0] != record_offset:
            raise AssertionError(
                f"{module}:{entry:#x} {case['name']}: record was not freed"
            )
        if struct.unpack_from("<H", raster_after, head_offset)[0] != middle_offset:
            raise AssertionError(
                f"{module}:{entry:#x} {case['name']}: active head differs"
            )
        head_skip_boundary = (
            struct.unpack_from("<H", raster_after, head_offset + 2)[0],
            struct.unpack_from("<H", raster_after, head_offset + 6)[0],
        )
        expected_head_boundary = (
            head_offset + 0x10
            if i16(state_initial["edge_0"] >> 16) < 0
            else record_offset
        )
        if head_skip_boundary != (1, expected_head_boundary):
            raise AssertionError(
                f"{module}:{entry:#x} {case['name']}: head skip boundary="
                f"{head_skip_boundary}"
            )
        if struct.unpack_from("<H", raster_after, middle_offset + 0x10)[0] != head_offset:
            raise AssertionError(
                f"{module}:{entry:#x} {case['name']}: active previous differs"
            )
        if struct.unpack_from("<H", raster_after, layout["column"])[0] != 0x013F:
            raise AssertionError(
                f"{module}:{entry:#x} {case['name']}: final column differs"
            )
        vectors.append(
            {
                "name": case["name"],
                "module": module,
                "entry": entry,
                "bucket_column": column,
                "continuation": continuation,
                "active_columns": active_column - column,
                "pixels": expected_pixels,
                "vga_outputs": [list(item) for item in expected_outputs],
                "clipped_sort_keys": clipped_sort_keys,
                "record_returned_to_free_list": True,
            }
        )

    return vectors



def manu3_active_renderer_vectors() -> list[dict[str, object]]:
    module = "manu3"
    entry = 0x0700
    image = load_image(module)
    globals_segment = 0x3000
    geometry_segment = 0x5000
    raster_segment = 0x7000
    texture_segment = 0x9000
    linear_segment = 0xB000
    stack_segment = 0xC000
    return_address = 0xF000
    face_offset = 0x1000
    vertex_offsets = (0x1100, 0x1200, 0x1300)
    screen = ((10, 20), (10, 24), (12, 20))
    texture_coordinates = (
        (0x0123, 0x1234),
        (0x2345, 0x3456),
        (0x4567, 0x5678),
    )
    depth = (0x10203040, -0x01234567, 0x30405060)

    globals_before = bytearray(0x10000)
    geometry_before = bytearray(0x10000)
    raster_before = bytearray(0x10000)
    texture_before = bytes(
        (offset * 37 + 11) & 0xFF for offset in range(0x10000)
    )
    linear_before = bytes(0x10000)

    struct.pack_into("<H", globals_before, 0x0002, geometry_segment)
    struct.pack_into(
        "<H",
        globals_before,
        0x0004,
        (texture_segment - 0x2000) & 0xFFFF,
    )
    struct.pack_into("<H", globals_before, 0x0006, raster_segment)
    struct.pack_into("<H", globals_before, 0x0014, linear_segment)
    struct.pack_into("<H", globals_before, 0x2300, face_offset)
    struct.pack_into("<H", globals_before, 0x2304, 1)

    struct.pack_into("<HHHH", geometry_before, face_offset, 0, *vertex_offsets)
    for vertex_offset, position, coordinate, depth_value in zip(
        vertex_offsets,
        screen,
        texture_coordinates,
        depth,
    ):
        struct.pack_into("<HH", geometry_before, vertex_offset, *coordinate)
        struct.pack_into(
            "<I",
            geometry_before,
            vertex_offset + 0x0A,
            (position[0] & 0xFFFF) | ((position[1] & 0xFFFF) << 16),
        )
        struct.pack_into(
            "<I", geometry_before, vertex_offset + 0x0E, depth_value & 0xFFFFFFFF
        )
        struct.pack_into("<H", geometry_before, vertex_offset + 0x12, 0)

    raster_data_offset = 0xA280
    raster_payload = image[raster_data_offset:]
    raster_before[: len(raster_payload)] = raster_payload
    struct.pack_into("<H", raster_before, 0x067E, 0x0BD6)

    outputs: list[tuple[int, int, int]] = []

    def output_handler(
        _machine: Uc, port: int, size: int, value: int
    ) -> None:
        outputs.append((port, size, value))

    machine = execute(
        image,
        entry,
        return_address,
        {
            "eax": 0xA1A1BEEF,
            "ebx": 0xB2B22345,
            "ecx": 0xC3C33456,
            "edx": 0xD4D44567,
            "esi": 0xE5E55678,
            "edi": 0xF6F66789,
            "ebp": 0x9797789A,
            "sp": 0xFF00,
            "ds": geometry_segment,
            "es": raster_segment,
            "fs": globals_segment,
            "gs": 0x2800,
            "ss": stack_segment,
            "flags": 0x0202,
        },
        [
            (0, return_address, b"\xcc"),
            (globals_segment, 0, bytes(globals_before)),
            (geometry_segment, 0, bytes(geometry_before)),
            (raster_segment, 0, bytes(raster_before)),
            (texture_segment, 0, texture_before),
            (linear_segment, 0, linear_before),
            (
                stack_segment,
                0xFF00,
                struct.pack("<H", return_address) + bytes.fromhex("5aa59669"),
            ),
        ],
        max_instructions=250000,
        output_handler=output_handler,
    )
    if outputs:
        raise AssertionError(f"{module}:{entry:#x}: linear renderer wrote VGA")
    geometry_after = bytes(machine.mem_read(geometry_segment * 16, 0x10000))
    if geometry_after != bytes(geometry_before):
        raise AssertionError(f"{module}:{entry:#x}: active geometry changed")
    raster_after = bytes(machine.mem_read(raster_segment * 16, 0x10000))
    free_head = struct.unpack_from("<H", raster_after, 0x0908)[0]
    if free_head != 0x0A72:
        raise AssertionError(
            f"{module}:{entry:#x}: active record was not returned: {free_head:#x}"
        )
    framebuffer = bytes(machine.mem_read(linear_segment * 16, 320 * 200))
    nonzero_pixels = sum(value != 0 for value in framebuffer)
    if nonzero_pixels == 0:
        column = struct.unpack_from("<H", raster_after, 0x0680)[0]
        active_head = struct.unpack_from("<H", raster_after, 0x0964)[0]
        record_words = struct.unpack_from("<8H", raster_after, 0x0A72)
        raise AssertionError(
            f"{module}:{entry:#x}: active triangle drew no pixels; "
            f"column={column:#x}, active_head={active_head:#x}, "
            f"record={record_words}"
        )

    return [
        {
            "name": "active_vertical_edge_linear",
            "module": module,
            "entry": entry,
            "screen": [list(pair) for pair in screen],
            "texture_coordinates": [
                list(pair) for pair in texture_coordinates
            ],
            "depth": list(depth),
            "framebuffer_sha256": hashlib.sha256(framebuffer).hexdigest(),
            "nonzero_pixels": nonzero_pixels,
            "free_head": free_head,
            "vga_outputs": [],
        }
    ]


def manu3_tween_constructor_vectors() -> list[dict[str, object]]:
    module = "manu3"
    entry = 0x01DF
    image = load_image(module)
    expected_hash = "bda522f4e9b3ec9663a2568a6a45ec969621599ed2546d6531fbf61fb495409d"
    if hashlib.sha256(image[entry : entry + 145]).hexdigest() != expected_hash:
        raise AssertionError(f"{module}:{entry:#x}: recovered 145-byte body changed")

    cases = (
        {
            "name": "phase_mismatch",
            "script": 0x3000,
            "phase": 0xAB02,
            "active": 0x1032,
            "specs": ((5, 3, 0x5000, 100),),
            "records": (),
        },
        {
            "name": "empty_initial",
            "script": 0x3100,
            "phase": 7,
            "active": 0x1032,
            "specs": ((0, 0, 0, 0),),
            "records": (),
        },
        {
            "name": "empty_after_active_phase_wrap",
            "script": 0x3180,
            "phase": 0xFFFF,
            "active": 0x1100,
            "specs": ((0, 0, 0, 0),),
            "records": (),
        },
        {
            "name": "one_positive_then_end",
            "script": 0x3200,
            "phase": 1,
            "active": 0x1032,
            "specs": ((4, 1, 0x5000, 300), (0, 0, 0, 0)),
            "records": ((0x4000, 100),),
        },
        {
            "name": "one_negative_then_next_phase",
            "script": 0x3300,
            "phase": 0xCD01,
            "active": 0x1032,
            "specs": ((3, 1, 0x5010, -200), (5, 2, 0x5020, 400)),
            "records": ((0x4020, 100),),
        },
        {
            "name": "two_records_delta_wrap",
            "script": 0x3400,
            "phase": 4,
            "active": 0x1032,
            "specs": (
                (1, 4, 0x5030, 0x7FFF),
                (255, 4, 0x5040, -0x8000),
                (6, 5, 0x5050, 0),
            ),
            "records": ((0x4040, -0x8000), (0x4060, 0x7FFF)),
        },
        {
            "name": "script_wrap",
            "script": 0xFFF8,
            "phase": 6,
            "active": 0x1032,
            "specs": ((2, 6, 0x5060, 11), (0, 0, 0, 0)),
            "records": ((0x4080, -9),),
        },
        {
            "name": "active_cursor_wrap",
            "script": 0x3500,
            "phase": 8,
            "active": 0xFFFE,
            "specs": ((7, 8, 0x5070, -300), (0, 0, 0, 0)),
            "records": ((0x40A0, 900),),
        },
    )
    data_segment = 0x4400
    stack_segment = 0x9000
    return_address = 0xF000
    vectors = []

    for case_index, case in enumerate(cases):
        stack_sentinel = bytes.fromhex("5aa596698778")
        data_before = bytearray(
            ((offset * 37 + case_index * 19 + 11) & 0xFF)
            for offset in range(0x10000)
        )
        script = int(case["script"])
        phase = int(case["phase"])
        active = int(case["active"])
        specs = case["specs"]
        records = case["records"]
        struct.pack_into("<H", data_before, 0x001A, 0x00B7 + case_index)
        struct.pack_into("<H", data_before, 0x102C, phase)
        struct.pack_into("<H", data_before, 0x102E, script)
        struct.pack_into("<H", data_before, 0x1030, 0xA55A)
        struct.pack_into("<H", data_before, 0x223A, 0xB66B)
        struct.pack_into("<H", data_before, 0x223C, 0xC77C)
        struct.pack_into("<H", data_before, 0x23E2, 0x1357 + case_index)
        struct.pack_into("<H", data_before, 0x23E4, 0x2468 + case_index)

        for spec_index, (count, spec_phase, target, end_value) in enumerate(specs):
            spec_offset = (script + spec_index * 8) & 0xFFFF
            encoded = struct.pack(
                "<BBHHH",
                count,
                spec_phase,
                0x9000 + spec_index,
                target,
                end_value & 0xFFFF,
            )
            for byte_index, value in enumerate(encoded):
                data_before[(spec_offset + byte_index) & 0xFFFF] = value
        for record_index, (record_offset, current) in enumerate(records):
            pointer_offset = (active + record_index * 2) & 0xFFFF
            struct.pack_into("<H", data_before, pointer_offset, record_offset)
            target_offset = int(specs[record_index][2])
            struct.pack_into("<H", data_before, target_offset, current & 0xFFFF)

        data_expected = bytearray(data_before)
        final_script = script
        final_active = active
        generated_records = []
        processed = 0
        while processed < len(specs):
            count, spec_phase, target_offset, end_value = specs[processed]
            if count == 0 or spec_phase != (phase & 0xFF):
                break
            record_offset, current = records[processed]
            delta = _signed_16((end_value & 0xFFFF) - (current & 0xFFFF))
            step, remainder = _signed_divide(delta * 65536, count)
            accumulator = ((current & 0xFFFF) << 16) + step
            accumulator &= 0xFFFFFFFF
            struct.pack_into("<H", data_expected, record_offset + 4, target_offset)
            struct.pack_into("<I", data_expected, record_offset + 10, step & 0xFFFFFFFF)
            struct.pack_into("<H", data_expected, record_offset, count - 1)
            struct.pack_into("<I", data_expected, record_offset + 6, accumulator)
            final_script = (final_script + 8) & 0xFFFF
            final_active = (final_active + 2) & 0xFFFF
            generated_records.append(
                {
                    "record_offset": record_offset,
                    "target_offset": target_offset,
                    "counter": count - 1,
                    "step": step,
                    "accumulator": _signed_16(accumulator >> 16) * 65536
                    + (accumulator & 0xFFFF),
                    "remainder": remainder,
                }
            )
            processed += 1

        count, next_phase, _target, _end = specs[processed]
        next_header = count | (next_phase << 8)
        struct.pack_into("<H", data_expected, 0x102E, final_script)
        struct.pack_into("<H", data_expected, 0x1030, final_active)
        initial_flags = 0x0A93 | (0x0400 if case_index & 1 else 0)
        if count != 0:
            struct.pack_into("<H", data_expected, 0x102C, (phase + 1) & 0xFFFF)
            expected_flags = add_flags_16(phase, 1, initial_flags)
            expected_flags["cf"] = next_phase < (phase & 0xFF)
            final_path = "phase_advance"
        elif final_active == 0x1032:
            cursor_x = struct.unpack_from("<H", data_before, 0x001A)[0]
            cursor_delta = ((cursor_x - 0x00A0) << 1) & 0xFFFF
            view_yaw = struct.unpack_from("<H", data_before, 0x23E4)[0]
            view_pitch = struct.unpack_from("<H", data_before, 0x23E2)[0]
            finished_yaw = (view_yaw - cursor_delta) & 0xFFFF
            struct.pack_into("<H", data_expected, 0x223C, finished_yaw)
            struct.pack_into("<H", data_expected, 0x223A, view_pitch)
            struct.pack_into("<H", data_expected, 0x102C, 0x0100)
            expected_flags = sub_flags_16(view_yaw, cursor_delta, initial_flags)
            final_path = "completed_without_active_records"
        else:
            struct.pack_into("<H", data_expected, 0x102C, (phase + 1) & 0xFFFF)
            expected_flags = add_flags_16(phase, 1, initial_flags)
            expected_flags["cf"] = final_active < 0x1032
            final_path = "end_after_active_records"

        initial = {
            "eax": 0xA1A1BEEF + case_index,
            "ebx": 0xB2B20000 | active,
            "ecx": 0xC3C33456 + case_index,
            "edx": 0xD4D44567 + case_index,
            "esi": 0xE5E55678 + case_index,
            "edi": 0xF6F66789 + case_index,
            "ebp": 0x9797789A + case_index,
            "sp": 0xFF00,
            "ds": data_segment,
            "es": 0x6000,
            "fs": 0x4C00,
            "gs": 0x7000,
            "ss": stack_segment,
            "flags": initial_flags,
        }
        decoy = bytes(
            ((offset * 13 + case_index * 29 + 7) & 0xFF)
            for offset in range(0x10000)
        )
        machine = execute(
            image,
            entry,
            return_address,
            initial,
            [
                (0, return_address, b"\xcc"),
                (0x6000, 0, decoy),
                (0x7000, 0, decoy),
                (data_segment, 0, bytes(data_before)),
                (
                    stack_segment,
                    0xFF00,
                    struct.pack("<H", return_address)
                    + stack_sentinel,
                ),
            ],
        )

        expected_registers = dict(initial)
        del expected_registers["flags"]
        expected_registers["esi"] = (initial["esi"] & 0xFFFF0000) | final_script
        expected_registers["ecx"] = next_header
        expected_registers["ebx"] = (initial["ebx"] & 0xFFFF0000) | final_active
        expected_registers["sp"] = 0xFF02
        if generated_records:
            last = generated_records[-1]
            expected_registers["edi"] = (
                initial["edi"] & 0xFFFF0000
            ) | int(last["record_offset"])
            expected_registers["eax"] = int(last["step"]) & 0xFFFFFFFF
            expected_registers["edx"] = int(last["remainder"]) & 0xFFFFFFFF
            expected_registers["ebp"] = int(last["accumulator"]) & 0xFFFFFFFF
        if final_path == "completed_without_active_records":
            expected_registers["eax"] = (
                initial["eax"] & 0xFFFF0000
            ) | struct.unpack_from("<H", data_before, 0x23E2)[0]
            expected_registers["ecx"] = struct.unpack_from(
                "<H", data_expected, 0x223C
            )[0]
        for register, expected in expected_registers.items():
            actual = machine.reg_read(REGISTERS[register])
            if actual != expected:
                raise AssertionError(
                    f"{module}:{entry:#x} {case['name']}: "
                    f"{register}={actual:#x}, expected={expected:#x}"
                )
        actual_data = bytes(machine.mem_read(data_segment * 16, 0x10000))
        if actual_data != data_expected:
            difference = next(
                offset
                for offset, (actual, expected) in enumerate(
                    zip(actual_data, data_expected)
                )
                if actual != expected
            )
            raise AssertionError(
                f"{module}:{entry:#x} {case['name']}: data differs at "
                f"{difference:#06x}: actual={actual_data[difference]:#04x}, "
                f"expected={data_expected[difference]:#04x}"
            )
        if bytes(machine.mem_read(0x6000 * 16, 0x10000)) != decoy:
            raise AssertionError(
                f"{module}:{entry:#x} {case['name']}: ES decoy changed"
            )
        if bytes(machine.mem_read(0x7000 * 16, 0x10000)) != decoy:
            raise AssertionError(
                f"{module}:{entry:#x} {case['name']}: GS decoy changed"
            )
        if bytes(machine.mem_read(stack_segment * 16 + 0xFF02, 6)) != stack_sentinel:
            raise AssertionError(
                f"{module}:{entry:#x} {case['name']}: stack sentinel changed"
            )

        flag_masks = {
            "cf": 0x0001,
            "pf": 0x0004,
            "af": 0x0010,
            "zf": 0x0040,
            "sf": 0x0080,
            "if": 0x0200,
            "df": 0x0400,
            "of": 0x0800,
        }
        flags_after = machine.reg_read(UC_X86_REG_EFLAGS)
        actual_flags = {
            flag: bool(flags_after & mask) for flag, mask in flag_masks.items()
        }
        if actual_flags != expected_flags:
            raise AssertionError(
                f"{module}:{entry:#x} {case['name']}: flags={actual_flags}, "
                f"expected={expected_flags}"
            )

        vectors.append(
            {
                "name": case["name"],
                "module": module,
                "entry": entry,
                "script_offset_before": script,
                "script_offset_after": final_script,
                "phase_before": phase,
                "phase_after": struct.unpack_from("<H", data_expected, 0x102C)[0],
                "active_cursor_before": active,
                "active_cursor_after": final_active,
                "processed_records": generated_records,
                "final_path": final_path,
                "defined_flags": expected_flags,
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


def alien_slot1_callback_head_vectors(
    module: str,
    entry: int,
    body_size: int,
    body_hash: str,
    active_offset: int,
    selection_offset: int,
    selected_offset: int,
    finish_callback: int,
    camera_callback: int,
    pulse_updates: tuple[tuple[int, int], ...],
    clear_active_on_selection: bool,
) -> list[dict[str, object]]:
    """Exercise every branch in the slot-1 wave callback head."""
    image = load_image(module)
    actual_hash = hashlib.sha256(image[entry : entry + body_size]).hexdigest()
    if actual_hash != body_hash:
        raise AssertionError(
            f"{module}:{entry:#x}: recovered {body_size}-byte body changed"
        )

    data_segment = 0x5000
    extra_segment = 0x7000
    fs_segment = 0x9000
    game_segment = 0xA000
    stack_segment = 0xB000
    state = 0x4000
    context = 0x3000
    return_address = 0xF000
    stack_sentinel = bytes.fromhex("5aa596698778")
    selected_state = 0x3456
    cases = (
        ("inactive_no_selection", 0, 0, 0x0010),
        ("inactive_selected_unclamped", 0, 2, 0x0010),
        ("inactive_selected_clamped", 0, 2, 0x007A),
        ("active_camera_handoff", 1, 2, 0xA55A),
    )

    def put_u16(memory: bytearray, offset: int, value: int) -> None:
        struct.pack_into("<H", memory, offset, value & 0xFFFF)

    def put_i16(memory: bytearray, offset: int, value: int) -> None:
        struct.pack_into("<h", memory, offset, value)

    def put_u32(memory: bytearray, offset: int, value: int) -> None:
        struct.pack_into("<I", memory, offset, value & 0xFFFFFFFF)

    def get_u16(memory: bytearray, offset: int) -> int:
        return struct.unpack_from("<H", memory, offset)[0]

    def get_u32(memory: bytearray, offset: int) -> int:
        return struct.unpack_from("<I", memory, offset)[0]

    vectors: list[dict[str, object]] = []
    for case_index, (name, active, selection, delta) in enumerate(cases):
        code_before = bytearray(image)
        code_expected = bytearray(image)
        put_u16(code_before, active_offset, active)
        put_u16(code_expected, active_offset, active)
        put_u16(code_before, selection_offset, selection)
        put_u16(code_expected, selection_offset, selection)
        put_u16(code_before, selected_offset, selected_state)
        put_u16(code_expected, selected_offset, selected_state)
        put_u16(code_before, 0x0099, delta)
        put_u16(code_expected, 0x0099, delta)
        # The active path tail-jumps to a separately recovered routine. A RET
        # at that routine's entry preserves the original head's tail boundary.
        code_before[camera_callback] = 0xC3
        code_expected[camera_callback] = 0xC3

        data_before = bytearray(
            (offset * 29 + case_index * 17 + 3) & 0xFF
            for offset in range(0x10000)
        )
        data_expected = bytearray(data_before)
        for field, value in (
            (0x00, 0x1111),
            (0x0E, 0x2222),
            (0x4E, 0x3333),
            (0x50, 0x4444),
            (0x52, 0xFFF0),
        ):
            put_u16(data_before, state + field, value)
            put_u16(data_expected, state + field, value)
        for field, value in (
            (0x42, 0x11223344),
            (0x46, 0x55667788),
            (0x4A, 0x99AABBCC),
        ):
            put_u32(data_before, state + field, value)
            put_u32(data_expected, state + field, value)
        for offset, _amount in pulse_updates:
            value = 0x10203040 + offset
            put_u32(data_before, offset, value)
            put_u32(data_expected, offset, value)
        put_u16(data_before, 0x001E, 0x7777)
        put_u16(data_expected, 0x001E, 0x7777)
        for offset, value in ((0x22EC, -123), (0x22F0, 456), (0x22F4, -32768)):
            put_i16(data_before, offset, value)
            put_i16(data_expected, offset, value)

        if active == 0:
            put_u16(data_expected, state + 0x4E, 0)
            put_u16(data_expected, state + 0x50, 0x0800)
            put_u16(data_expected, state + 0x52, 0x0025)
            if selection & 2:
                put_u16(code_expected, 0x0099, min(delta + 8, 0x007F))
                put_u16(data_expected, state, selected_state)
                put_u16(data_expected, state + 0x0E, finish_callback)
                put_u16(code_expected, selection_offset, 0)
                for offset, amount in pulse_updates:
                    put_u32(data_expected, offset, get_u32(data_expected, offset) - amount)
                if clear_active_on_selection:
                    put_u16(code_expected, active_offset, 0)
                put_u16(data_expected, 0x001E, 4)
        else:
            put_u16(code_expected, selection_offset, 0)
            for offset, amount in pulse_updates:
                put_u32(data_expected, offset, get_u32(data_expected, offset) - amount)
            put_u16(data_expected, state, 0x22A8)
            for field, source in ((0x42, 0x22EC), (0x46, 0x22F0), (0x4A, 0x22F4)):
                signed_value = struct.unpack_from("<h", data_before, source)[0]
                put_u32(data_expected, state + field, -signed_value)

        initial = {
            "eax": 0xA1A11111 + case_index,
            "ebx": 0xB2B22222 + case_index,
            "ecx": 0xC3C33333 + case_index,
            "edx": 0xD4D44444 + case_index,
            "esi": 0xE5E50000 | state,
            "edi": 0xF6F60000 | context,
            "ebp": 0x97975555 + case_index,
            "sp": 0xFF00,
            "ds": data_segment,
            "es": extra_segment,
            "fs": fs_segment,
            "gs": game_segment,
            "ss": stack_segment,
            "flags": 0x0293 | (0x0400 if case_index & 1 else 0),
        }
        extra_before = bytes(
            (offset * 13 + case_index + 7) & 0xFF for offset in range(0x10000)
        )
        fs_before = bytes(
            (offset * 11 + case_index + 9) & 0xFF for offset in range(0x10000)
        )
        game_before = bytes(
            (offset * 7 + case_index + 5) & 0xFF for offset in range(0x10000)
        )
        machine = execute(
            bytes(code_before),
            entry,
            return_address,
            initial,
            [
                (data_segment, 0, bytes(data_before)),
                (extra_segment, 0, extra_before),
                (fs_segment, 0, fs_before),
                (game_segment, 0, game_before),
                (
                    stack_segment,
                    0xFF00,
                    struct.pack("<H", return_address) + stack_sentinel,
                ),
            ],
        )
        actual_code = bytes(machine.mem_read(0, len(image)))
        actual_data = bytes(machine.mem_read(data_segment * 16, 0x10000))
        if actual_code != bytes(code_expected):
            differences = [
                (offset, actual_code[offset], code_expected[offset])
                for offset in range(len(image))
                if actual_code[offset] != code_expected[offset]
            ][:8]
            raise AssertionError(
                f"{module}:{entry:#x} {name}: code differs at {differences}"
            )
        if actual_data != bytes(data_expected):
            differences = [
                (offset, actual_data[offset], data_expected[offset])
                for offset in range(0x10000)
                if actual_data[offset] != data_expected[offset]
            ][:8]
            raise AssertionError(
                f"{module}:{entry:#x} {name}: data differs at {differences}"
            )
        for segment, expected in (
            (extra_segment, extra_before),
            (fs_segment, fs_before),
            (game_segment, game_before),
        ):
            if bytes(machine.mem_read(segment * 16, 0x10000)) != expected:
                raise AssertionError(
                    f"{module}:{entry:#x} {name}: decoy {segment:#x} changed"
                )
        if machine.reg_read(UC_X86_REG_SP) != 0xFF02:
            raise AssertionError(f"{module}:{entry:#x} {name}: stack changed")
        if bytes(machine.mem_read(stack_segment * 16 + 0xFF02, 6)) != stack_sentinel:
            raise AssertionError(f"{module}:{entry:#x} {name}: stack sentinel changed")

        vectors.append(
            {
                "name": name,
                "module": module,
                "entry": entry,
                "active_before": active,
                "active_after": get_u16(code_expected, active_offset),
                "selection_before": selection,
                "selection_after": get_u16(code_expected, selection_offset),
                "delta_before": delta,
                "delta_after": get_u16(code_expected, 0x0099),
                "owner_after": get_u16(data_expected, state),
                "callback_after": get_u16(data_expected, state + 0x0E),
                "countdown_after": get_u16(data_expected, 0x001E),
                "code_sha256": hashlib.sha256(code_expected).hexdigest(),
                "data_sha256": hashlib.sha256(data_expected).hexdigest(),
            }
        )

    return vectors


def alien_slot3_callback_vectors(
    module: str,
    kind: str,
    entry: int,
    body_size: int,
    body_hash: str,
    timer_offset: int,
    resume_countdown_offset: int,
    resume_state_offset: int,
    ring_offset: int,
    initial_callback: int,
    followup_callback: int,
    initial_position: tuple[int, int, int],
) -> list[dict[str, object]]:
    image = load_image(module)
    actual_hash = hashlib.sha256(image[entry : entry + body_size]).hexdigest()
    if actual_hash != body_hash:
        raise AssertionError(
            f"{module}:{entry:#x}: recovered {body_size}-byte body changed"
        )

    data_segment = 0x5000
    extra_segment = 0x7000
    fs_segment = 0x9000
    game_segment = 0xA000
    stack_segment = 0xB000
    state = 0x4000
    context = 0x3000
    return_address = 0xF000
    stack_sentinel = bytes.fromhex("5aa596698778")

    if kind == "restart":
        cases = (
            ("zero", 0x0000, 0x0000),
            ("ordinary", 0x0198, 0x1234),
            ("ring_end", 0x03FC, 0xFFFF),
            ("borrowed_rotate", 0x0008, 0x0004),
        )
    elif kind == "resume":
        cases = (
            ("ring_zero", 0x0000, 0x1234),
            ("ring_middle", 0x0198, 0xA55A),
            ("ring_end", 0x03FC, 0xFFFF),
        )
    elif kind == "capture":
        cases = (
            ("capture_zero_ring", 0x0000, 0x1234),
            ("capture_middle_ring", 0x0198, 0xA55A),
            ("capture_end_ring", 0x03FC, 0xFFFF),
        )
    elif kind == "ring_zero":
        cases = (
            ("timer_blocks", 0x0198, 1),
            ("advance_zero", 0x0000, 0),
            ("advance_middle", 0x0198, 0),
            ("advance_wrap", 0x03F8, 0),
        )
    else:
        raise ValueError(f"unsupported slot-3 callback kind: {kind}")

    def put_bytes(memory: bytearray, offset: int, value: bytes) -> None:
        for index, byte in enumerate(value):
            memory[(offset + index) & 0xFFFF] = byte

    def put_u16(memory: bytearray, offset: int, value: int) -> None:
        put_bytes(memory, offset, struct.pack("<H", value & 0xFFFF))

    def put_u32(memory: bytearray, offset: int, value: int) -> None:
        put_bytes(memory, offset, struct.pack("<I", value & 0xFFFFFFFF))

    def get_u16(memory: bytearray, offset: int) -> int:
        return memory[offset & 0xFFFF] | (memory[(offset + 1) & 0xFFFF] << 8)

    vectors: list[dict[str, object]] = []
    for case_index, (name, ring_cursor, parameter) in enumerate(cases):
        data_before = bytearray(
            (offset * 29 + case_index * 17 + 3) & 0xFF
            for offset in range(0x10000)
        )
        data_expected = bytearray(data_before)
        code_before = bytearray(image)
        code_expected = bytearray(code_before)
        code_before[return_address] = 0xCC
        code_expected[return_address] = 0xCC

        callback_before = (0xA100 + case_index) & 0xFFFF
        put_u16(data_before, state + 0x0E, callback_before)
        put_u16(data_before, state + 0x5A, ring_cursor)
        put_u16(data_expected, state + 0x0E, callback_before)
        put_u16(data_expected, state + 0x5A, ring_cursor)
        for field, value in zip(
            (0x42, 0x46, 0x4A),
            (0x11223344, 0x55667788, 0x99AABBCC),
        ):
            put_u32(data_before, state + field, value)
            put_u32(data_expected, state + field, value)
        for field, value in zip(
            (0x4E, 0x50, 0x52, 0x54, 0x56, 0x5C),
            (0x1111, 0x2222, 0x3333, 0x4444, 0x5555, 0x6666),
        ):
            put_u16(data_before, state + field, value)
            put_u16(data_expected, state + field, value)

        if kind == "restart":
            put_u16(data_before, 0x105C, parameter)
            put_u16(data_expected, 0x105C, parameter)
            random_after = (
                ((parameter >> 3) | (parameter << 13))
                - ((parameter >> 2) & 1)
            ) & 0xFFFF
            put_u16(data_expected, 0x105C, random_after)
            put_u16(data_expected, state + 0x0E, initial_callback)
            put_u16(data_expected, state + 0x52, 0)
            put_u16(data_expected, state + 0x54, 8)
            put_u16(data_expected, state + 0x56, 0x1E)
            put_u16(data_expected, state + 0x5C, random_after)
            put_u16(code_expected, ring_offset + ring_cursor + 4, 8)
            put_u16(code_expected, ring_offset + ring_cursor + 6, 0)
        elif kind == "resume":
            for field, value in zip((0x42, 0x46, 0x4A), initial_position):
                put_u32(data_expected, state + field, value)
            for field in (0x4E, 0x50, 0x52, 0x54):
                put_u16(data_expected, state + field, 0)
            put_u16(data_expected, state + 0x0E, followup_callback)
            for field, value in ((0, 0), (2, 0), (4, 0), (6, 2)):
                put_u16(code_expected, ring_offset + ring_cursor + field, value)
        elif kind == "capture":
            for field, value in zip((0x42, 0x46, 0x4A), initial_position):
                put_u32(data_expected, state + field, value)
            for field in (0x4E, 0x50, 0x52, 0x54):
                put_u16(data_expected, state + field, 0)
            put_u16(code_expected, resume_countdown_offset, 0x12)
            put_u16(code_expected, resume_state_offset, state)
        else:
            put_u16(code_before, timer_offset, parameter)
            put_u16(code_expected, timer_offset, parameter)
            if parameter == 0:
                ring_after = (ring_cursor + 8) & 0x03FC
                put_u16(data_expected, state + 0x5A, ring_after)
                for field in (0, 2, 4, 6):
                    put_u16(code_expected, ring_offset + ring_after + field, 0)

        initial = {
            "eax": 0xA1A10000 | ((0x1111 + case_index) & 0xFFFF),
            "ebx": 0xB2B20000 | ((0x2222 + case_index) & 0xFFFF),
            "ecx": 0xC3C30000 | ((0x3333 + case_index) & 0xFFFF),
            "edx": 0xD4D40000 | ((0x4444 + case_index) & 0xFFFF),
            "esi": 0xE5E50000 | state,
            "edi": 0xF6F60000 | context,
            "ebp": 0x97970000 | ring_cursor,
            "sp": 0xFF00,
            "ds": data_segment,
            "es": extra_segment,
            "fs": fs_segment,
            "gs": game_segment,
            "ss": stack_segment,
            "flags": 0x0293 | (0x0400 if case_index & 1 else 0),
        }
        extra_before = bytes(
            (offset * 13 + case_index + 7) & 0xFF for offset in range(0x10000)
        )
        fs_before = bytes(
            (offset * 11 + case_index + 9) & 0xFF for offset in range(0x10000)
        )
        game_before = bytes(
            (offset * 7 + case_index + 5) & 0xFF for offset in range(0x10000)
        )
        machine = execute(
            bytes(code_before),
            entry,
            return_address,
            initial,
            [
                (data_segment, 0, bytes(data_before)),
                (extra_segment, 0, extra_before),
                (fs_segment, 0, fs_before),
                (game_segment, 0, game_before),
                (
                    stack_segment,
                    0xFF00,
                    struct.pack("<H", return_address) + stack_sentinel,
                ),
            ],
        )
        actual_data = bytes(machine.mem_read(data_segment * 16, 0x10000))
        actual_code = bytes(machine.mem_read(0, len(image)))
        if actual_data != bytes(data_expected):
            differences = [
                (offset, actual_data[offset], data_expected[offset])
                for offset in range(0x10000)
                if actual_data[offset] != data_expected[offset]
            ][:8]
            raise AssertionError(
                f"{module}:{entry:#x} {name}: data differs at {differences}"
            )
        if actual_code != bytes(code_expected):
            differences = [
                (offset, actual_code[offset], code_expected[offset])
                for offset in range(len(image))
                if actual_code[offset] != code_expected[offset]
            ][:8]
            raise AssertionError(
                f"{module}:{entry:#x} {name}: code differs at {differences}"
            )
        for segment, expected in (
            (extra_segment, extra_before),
            (fs_segment, fs_before),
            (game_segment, game_before),
        ):
            if bytes(machine.mem_read(segment * 16, 0x10000)) != expected:
                raise AssertionError(
                    f"{module}:{entry:#x} {name}: decoy {segment:#x} changed"
                )
        if machine.reg_read(UC_X86_REG_SP) != 0xFF02:
            raise AssertionError(f"{module}:{entry:#x} {name}: stack changed")
        if bytes(machine.mem_read(stack_segment * 16 + 0xFF02, 6)) != stack_sentinel:
            raise AssertionError(f"{module}:{entry:#x} {name}: stack sentinel changed")

        vectors.append(
            {
                "name": name,
                "module": module,
                "entry": entry,
                "kind": kind,
                "ring_before": ring_cursor,
                "ring_after": get_u16(data_expected, state + 0x5A),
                "callback_before": callback_before,
                "callback_after": get_u16(data_expected, state + 0x0E),
                "position_after": [
                    struct.unpack_from("<I", data_expected, state + field)[0]
                    for field in (0x42, 0x46, 0x4A)
                ],
                "motion_after": [
                    get_u16(data_expected, state + field)
                    for field in (0x4E, 0x50, 0x52, 0x54, 0x56, 0x5C)
                ],
                "resume_countdown_after": struct.unpack_from(
                    "<H", code_expected, resume_countdown_offset
                )[0],
                "resume_state_after": struct.unpack_from(
                    "<H", code_expected, resume_state_offset
                )[0],
                "data_sha256": hashlib.sha256(data_expected).hexdigest(),
                "code_sha256": hashlib.sha256(code_expected).hexdigest(),
            }
        )

    return vectors


def main() -> int:
    global _COVERAGE_RECORDER
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--check", action="store_true", help="fail unless committed vectors are current"
    )
    parser.add_argument(
        "--only-slot3-callbacks",
        action="store_true",
        help=(
            "generate or check only the recovered alien slot-1 callback heads "
            "and slot-3 callbacks"
        ),
    )
    parser.add_argument(
        "--xdb-dir",
        type=Path,
        help="directory containing amer/croolis/manu3/scrut XDB images",
    )
    parser.add_argument(
        "--require-complete-coverage",
        action="store_true",
        help="reject all incomplete direct-oracle branch coverage",
    )
    parser.add_argument(
        "--allow-unreviewed-coverage",
        action="store_true",
        help="write a report before its changed missing-edge set is reviewed",
    )
    args = parser.parse_args()

    if args.xdb_dir is not None:
        for module in IMAGE_PATHS:
            IMAGE_PATHS[module] = args.xdb_dir.resolve() / f"{module}.xdb"

    if not args.only_slot3_callbacks:
        canonical_images = {module: load_image(module) for module in IMAGE_PATHS}
        image_sizes = {
            len(image): module for module, image in canonical_images.items()
        }
        if len(image_sizes) != len(canonical_images):
            raise SystemExit("XDB image sizes do not uniquely identify all modules")
        _COVERAGE_RECORDER = CoverageRecorder(image_sizes, canonical_images)

    VECTOR_ROOT.mkdir(parents=True, exist_ok=True)
    for (
        module,
        entry,
        body_size,
        body_hash,
        active_offset,
        selection_offset,
        selected_offset,
        finish_callback,
        camera_callback,
        pulse_updates,
        clear_active_on_selection,
    ) in (
        (
            "amer", 0x0B37, 153,
            "99050300f8178ba956418cd222a091f5e9f5e2857f374d4a0ea0927aea9c42ff",
            0x1648, 0x0B2F, 0x0B33, 0x0BD0, 0x0C5D,
            ((0x2594, 0x1E), (0x25F2, 0x23)), False,
        ),
        (
            "croolis", 0x0B78, 172,
            "b31f5bd9edba9484f09f381b769138430c73c30375fdf4a09a000a6d39be513c",
            0x16A0, 0x0B70, 0x0B74, 0x0C24, 0x0CB5,
            ((0x2536, 0x19), (0x2594, 0x1E), (0x25F2, 0x23)), True,
        ),
        (
            "scrut", 0x0B78, 160,
            "866abe2a41ca054d817fe1087b3d101a04de02c0c324cb695badd2499d5bf815",
            0x168E, 0x0B70, 0x0B74, 0x0C18, 0x0CA3,
            ((0x2594, 0x1E), (0x25F2, 0x23)), True,
        ),
    ):
        update_vector(
            VECTOR_ROOT / f"xdb_{module}_func_{entry:04x}_natural.json",
            alien_slot1_callback_head_vectors(
                module,
                entry,
                body_size,
                body_hash,
                active_offset,
                selection_offset,
                selected_offset,
                finish_callback,
                camera_callback,
                pulse_updates,
                clear_active_on_selection,
            ),
            args.check,
        )
    for (
        module,
        timer_offset,
        resume_countdown_offset,
        resume_state_offset,
        ring_offset,
        initial_callback,
        followup_callback,
        initial_position,
        routines,
    ) in (
        (
            "amer",
            0x0B31,
            0x0D5F,
            0x0D61,
            0x0D63,
            0x12B3,
            0x1614,
            (0, 0x06A4, 0),
            (
                ("restart", 0x1558, 50, "2a1e8de52d2f3196361f13995fa21caf300fe036a58630bfd9d085138f4c491d"),
                ("resume", 0x158A, 81, "023c0d65c25447bdd89901d8a22fe08e5b4bc885f025bbb330d0f810ae96c332"),
                ("capture", 0x15DB, 57, "5203cdb5019a7ebb327a92120bf26347b26fef3da785235379f48f0b46135a40"),
                ("ring_zero", 0x1614, 52, "0778ff3e9ee060dfd17aba0e58538e744f76ce3d6cf44419509d3e19616df88f"),
            ),
        ),
        (
            "croolis",
            0x0B72,
            0x0DB7,
            0x0DB9,
            0x0DBB,
            0x130B,
            0x166C,
            (0, 0x06A4, 0),
            (
                ("restart", 0x15B0, 50, "bb668de9705df4c3eb4586efee3a9c7924993d3c98a63311a6f6980df5460739"),
                ("resume", 0x15E2, 81, "a177b430a47f03da75c8706811233e23fb0c92511a0fd89fef2fa966246199be"),
                ("capture", 0x1633, 57, "592a5c7d078bee64223c401b2fdc131862c057151c9de27f6f425981a21a7de2"),
                ("ring_zero", 0x166C, 52, "4c3455654656d4a75e669da99de2f4d1522968313870307fd469726c4b100d05"),
            ),
        ),
        (
            "scrut",
            0x0B72,
            0x0DA5,
            0x0DA7,
            0x0DA9,
            0x12F9,
            0x165A,
            (0x06A4, 0, 0),
            (
                ("restart", 0x159E, 50, "09d3cf58a7fef37a34eb3f352a57cad79c16c452e044f2c53daee13c7a9ce39f"),
                ("resume", 0x15D0, 81, "f7f05bbfd9b9ef9344a73b91f483df4cc9927b6bfd2f5aea3ac2093dd03c57f3"),
                ("capture", 0x1621, 57, "b8da15e8b430e9e7a8a8995767caba0bf80dcc79bf4abde228023143ea31c966"),
                ("ring_zero", 0x165A, 52, "b7eef2effd0ae190399c62fabf7137013f83d44d492c9cdd152b03a9a9d5924d"),
            ),
        ),
    ):
        for kind, entry, body_size, body_hash in routines:
            update_vector(
                VECTOR_ROOT / f"xdb_{module}_func_{entry:04x}_natural.json",
                alien_slot3_callback_vectors(
                    module,
                    kind,
                    entry,
                    body_size,
                    body_hash,
                    timer_offset,
                    resume_countdown_offset,
                    resume_state_offset,
                    ring_offset,
                    initial_callback,
                    followup_callback,
                    initial_position,
                ),
                args.check,
            )
    if args.only_slot3_callbacks:
        return 0
    update_vector(
        VECTOR_ROOT / "xdb_manu3_func_0000_natural.json",
        manu3_api_entry_vectors(),
        args.check,
    )
    update_vector(
        VECTOR_ROOT / "xdb_manu3_func_0270_natural.json",
        manu3_matrix_build_vectors(),
        args.check,
    )
    update_vector(
        VECTOR_ROOT / "xdb_manu3_func_0121_natural.json",
        manu3_init_protocol_vectors(),
        args.check,
    )
    update_vector(
        VECTOR_ROOT / "xdb_manu3_func_0150_natural.json",
        manu3_frame_step_vectors(),
        args.check,
    )
    update_vector(
        VECTOR_ROOT / "xdb_manu3_func_017c_natural.json",
        manu3_anim_select_entry_vectors(),
        args.check,
    )
    update_vector(
        VECTOR_ROOT / "xdb_manu3_func_0181_natural.json",
        manu3_anim_select_vectors(),
        args.check,
    )
    update_vector(
        VECTOR_ROOT / "xdb_manu3_func_019b_natural.json",
        manu3_tween_step_vectors(),
        args.check,
    )
    update_vector(
        VECTOR_ROOT / "xdb_manu3_func_01df_natural.json",
        manu3_tween_constructor_vectors(),
        args.check,
    )
    update_vector(
        VECTOR_ROOT / "xdb_manu3_func_0549_natural.json",
        manu3_entity_project_vectors(),
        args.check,
    )
    update_vector(
        VECTOR_ROOT / "xdb_manu3_func_06f6_natural.json",
        manu3_face_builder_next_vectors(),
        args.check,
    )
    update_vector(
        VECTOR_ROOT / "xdb_manu3_func_0700_natural.json",
        manu3_face_bucket_sort_vectors(),
        args.check,
    )
    update_vector(
        VECTOR_ROOT / "xdb_manu3_func_0700_renderer_natural.json",
        manu3_full_renderer_vectors(),
        args.check,
    )
    update_vector(
        VECTOR_ROOT / "xdb_manu3_func_0700_active_natural.json",
        manu3_active_renderer_vectors(),
        args.check,
    )
    update_vector(
        VECTOR_ROOT / "xdb_manu3_func_0d7d_natural.json",
        manu3_face_activate_vectors(),
        args.check,
    )
    update_vector(
        VECTOR_ROOT / "xdb_manu3_func_0d7d_gradient_natural.json",
        manu3_face_gradient_vectors(),
        args.check,
    )
    for module, entry in (
        ("amer", 0x02F0),
        ("croolis", 0x0305),
        ("scrut", 0x0305),
    ):
        update_vector(
            VECTOR_ROOT / f"xdb_{module}_func_{entry:04x}_natural.json",
            vga_clear_and_sync_vectors(module, entry),
            args.check,
        )
    for module, entry in (
        ("amer", 0x0336),
        ("croolis", 0x034B),
        ("scrut", 0x034B),
    ):
        update_vector(
            VECTOR_ROOT / f"xdb_{module}_func_{entry:04x}_natural.json",
            mouse_bounds_vectors(module, entry),
            args.check,
        )
    for module, entry, body_size, body_hash in (
        (
            "amer",
            0x0223,
            205,
            "66cc8762f57e7fe55e6dd95eb82acb977851b95c46ed0b75a9665a39a8ef9a59",
        ),
        (
            "croolis",
            0x022A,
            219,
            "33edea2995d9e02b635337e879349eb351b468967e0d54a930b662861ddeb375",
        ),
        (
            "scrut",
            0x022A,
            219,
            "33edea2995d9e02b635337e879349eb351b468967e0d54a930b662861ddeb375",
        ),
    ):
        update_vector(
            VECTOR_ROOT / f"xdb_{module}_func_{entry:04x}_natural.json",
            mouse_camera_step_vectors(module, entry, body_size, body_hash),
            args.check,
        )
    for module, entry in (
        ("amer", 0x0347),
        ("croolis", 0x035C),
        ("scrut", 0x035C),
    ):
        update_vector(
            VECTOR_ROOT / f"xdb_{module}_func_{entry:04x}_natural.json",
            mouse_position_vectors(module, entry),
            args.check,
        )
    for module, entry in (
        ("amer", 0x0958),
        ("croolis", 0x0999),
        ("scrut", 0x0999),
    ):
        update_vector(
            VECTOR_ROOT / f"xdb_{module}_func_{entry:04x}_natural.json",
            wrap_positions_vectors(module, entry),
            args.check,
        )
    for (
        module,
        entry,
        block_start,
        body_hash,
        selection_state_offset,
        selected_state_offset,
        current_sample_offset,
        publish_initial_state,
    ) in (
        (
            "amer",
            0x09EF,
            0x09B4,
            "c9402aa2a09e57ad0510f1c63fb3ab4e600639e2f58e722aa9e8a5537588b8ab",
            0x0B2F,
            0x0B33,
            0x0B35,
            False,
        ),
        (
            "croolis",
            0x0A30,
            0x09F5,
            "2441bda45c8759bdefa8c9ab22f585ab3a21da48ee45209bf65953b3c28aee86",
            0x0B70,
            0x0B74,
            0x0B76,
            False,
        ),
        (
            "scrut",
            0x0A35,
            0x09F5,
            "7bc6cebc553f911f03f8aecabe977e7abee802da76edf969433c7c3995a01946",
            0x0B70,
            0x0B74,
            0x0B76,
            True,
        ),
    ):
        update_vector(
            VECTOR_ROOT / f"xdb_{module}_func_{entry:04x}_natural.json",
            alien_slot1_wave_update_or_init_vectors(
                module,
                entry,
                block_start,
                body_hash,
                selection_state_offset,
                selected_state_offset,
                current_sample_offset,
                publish_initial_state,
            ),
            args.check,
        )
    for (
        module,
        entry,
        block_start,
        timer_offset,
        generation_offset,
        cursor_offset,
        ring_offset,
        initial_callback,
        generic_callback,
        initial_position,
    ) in (
        (
            "amer",
            0x1286,
            0x1163,
            0x0B31,
            0x0D5B,
            0x0D5D,
            0x0D63,
            0x12B3,
            0x1414,
            (0, 0x06A4, 0),
        ),
        (
            "croolis",
            0x12DE,
            0x11BB,
            0x0B72,
            0x0DB3,
            0x0DB5,
            0x0DBB,
            0x130B,
            0x146C,
            (0, 0x06A4, 0),
        ),
        (
            "scrut",
            0x12CC,
            0x11A9,
            0x0B72,
            0x0DA1,
            0x0DA3,
            0x0DA9,
            0x12F9,
            0x145A,
            (0x06A4, 0, 0),
        ),
    ):
        update_vector(
            VECTOR_ROOT / f"xdb_{module}_func_{entry:04x}_natural.json",
            alien_slot3_update_or_init_vectors(
                module,
                entry,
                block_start,
                timer_offset,
                generation_offset,
                cursor_offset,
                ring_offset,
                initial_callback,
                generic_callback,
                initial_position,
            ),
            args.check,
        )
    for module, entry, wrap_entry in (
        ("amer", 0x0925, 0x0958),
        ("croolis", 0x0966, 0x0999),
        ("scrut", 0x0966, 0x0999),
    ):
        update_vector(
            VECTOR_ROOT / f"xdb_{module}_func_{entry:04x}_natural.json",
            alien_slot10_bounds_then_wrap_vectors(module, entry, wrap_entry),
            args.check,
        )
    for (
        module,
        entry,
        body_size,
        body_hash,
        seed_offset,
        seed_step,
        initial_callback,
    ) in (
        (
            "amer",
            0x164C,
            60,
            "76e7934fa691d1e8042a8e1ca78b50085c911d0e2da12cb828c86344cdb92ec1",
            None,
            0,
            0x1692,
        ),
        (
            "croolis",
            0x16A4,
            121,
            "6da3c5d246e0202a486757484c491c49745638c5c36ad1fef6361d56d1346377",
            0x16A2,
            0x00FA,
            0x1727,
        ),
        (
            "scrut",
            0x1692,
            127,
            "93ab5fc7e70f4185f3af1c80581ad143fe9a5835c8a1ae937b70c347bf03615f",
            0x1690,
            0x012C,
            0x171B,
        ),
    ):
        update_vector(
            VECTOR_ROOT / f"xdb_{module}_func_{entry:04x}_natural.json",
            alien_slot2_dispatch_or_init_vectors(
                module,
                entry,
                body_size,
                body_hash,
                seed_offset,
                seed_step,
                initial_callback,
            ),
            args.check,
        )
    update_vector(
        VECTOR_ROOT / "xdb_amer_func_18d3_natural.json",
        amer_slot2_return_update_vectors(0x18D3),
        args.check,
    )
    update_vector(
        VECTOR_ROOT / "xdb_amer_func_1a5c_natural.json",
        amer_slot2_steer_update_vectors(0x1A5C),
        args.check,
    )
    for module, entry in (
        ("amer", 0x2027),
        ("croolis", 0x206C),
        ("scrut", 0x212C),
    ):
        update_vector(
            VECTOR_ROOT / f"xdb_{module}_func_{entry:04x}_natural.json",
            alien_transform_and_project_vectors(module, entry),
            args.check,
        )
    for (
        module,
        body_hash,
        data_delta_slot,
        data_segment_slot,
        continuation_offset,
        continuation_target,
    ) in (
        (
            "amer",
            "ed2dc63683f89af79b8bb92c96b4302553c81e0055233fe3f7ebd1478e174af3",
            0x3275,
            0x3277,
            0x0944,
            0x28D0,
        ),
        (
            "croolis",
            "6fb4e4318577167cf011c7028aad0c8a16224cd78ef7cf43e395998acc82c584",
            0x32E5,
            0x32E7,
            0x0946,
            0x2940,
        ),
        (
            "scrut",
            "7d04c0a35e82fbf020b08f8d7278e90f2f552717cc658572fab77cb889b307e7",
            0x33A5,
            0x33A7,
            0x0946,
            0x2A00,
        ),
    ):
        update_vector(
            VECTOR_ROOT / f"xdb_{module}_func_0000_natural.json",
            alien_api_entry_vectors(
                module,
                body_hash,
                data_delta_slot,
                data_segment_slot,
                continuation_offset,
                continuation_target,
            ),
            args.check,
        )
    for (
        module,
        body_size,
        body_hash,
        data_segment_slot,
        direct_calls,
        clears_control_latch,
    ) in (
        (
            "amer",
            384,
            "d9ac4420d0879158c8023912cc10a07f16931b23f561ad3b8534011d54b8c47e",
            0x3277,
            {
                "mouse_camera": 0x0223,
                "vga_clear": 0x02F0,
                "mouse_bounds": 0x0336,
                "mouse_position": 0x0347,
                "primary_mesh": 0x059B,
                "starfield": 0x0734,
                "camera_matrix": 0x1DD8,
                "transform": 0x2027,
                "bucket_faces": 0x24CF,
            },
            False,
        ),
        (
            "croolis",
            391,
            "861c7aaf490071e6521ee1fba7a894577745e3a360ec466fdacf424789141289",
            0x32E7,
            {
                "mouse_camera": 0x022A,
                "vga_clear": 0x0305,
                "mouse_bounds": 0x034B,
                "mouse_position": 0x035C,
                "primary_mesh": 0x05DC,
                "starfield": 0x0775,
                "camera_matrix": 0x1E1D,
                "transform": 0x206C,
                "bucket_faces": 0x2514,
            },
            True,
        ),
        (
            "scrut",
            391,
            "2578e41247079f68c34bd74d9552e5eaffe72eb964092451be422b7d21960c07",
            0x33A7,
            {
                "mouse_camera": 0x022A,
                "vga_clear": 0x0305,
                "mouse_bounds": 0x034B,
                "mouse_position": 0x035C,
                "primary_mesh": 0x05DC,
                "starfield": 0x0775,
                "camera_matrix": 0x1EDD,
                "transform": 0x212C,
                "bucket_faces": 0x25D4,
            },
            True,
        ),
    ):
        update_vector(
            VECTOR_ROOT / f"xdb_{module}_func_00a3_natural.json",
            alien_main_vectors(
                module,
                0x00A3,
                body_size,
                body_hash,
                data_segment_slot,
                direct_calls,
                clears_control_latch,
            ),
            args.check,
        )
    for (
        module,
        entry,
        body_hash,
        data_segment_slot,
        shade_table_offset,
        seed_offset,
        remaining_offset,
        cursors_offset,
        matrix_offset,
        camera_cells_offset,
        records_offset,
    ) in (
        (
            "amer",
            0x0734,
            "c20927be684fe47460ff868324a7f228fa927e1b00a4a61d8770bb700343e601",
            0x3277,
            0x07D4,
            0x08D4,
            0x08D8,
            0x08DA,
            0x0D4A,
            0x0D7A,
            0x1F38,
        ),
        (
            "croolis",
            0x0775,
            "d4d6f353d6eeb8dbecafed87f13994b317139a2b7dcbec524640c0abb9817f4f",
            0x32E7,
            0x07D6,
            0x08D6,
            0x08DA,
            0x08DC,
            0x0D4C,
            0x0D7C,
            0x1F3A,
        ),
        (
            "scrut",
            0x0775,
            "84a6560b66ea7cf94afd831458f7417f6b59a861de9b516f913628c65061d821",
            0x33A7,
            0x07D6,
            0x08D6,
            0x08DA,
            0x08DC,
            0x0D4C,
            0x0D7C,
            0x1F3A,
        ),
    ):
        update_vector(
            VECTOR_ROOT / f"xdb_{module}_func_{entry:04x}_natural.json",
            alien_starfield_vectors(
                module,
                entry,
                body_hash,
                data_segment_slot,
                shade_table_offset,
                seed_offset,
                remaining_offset,
                cursors_offset,
                matrix_offset,
                camera_cells_offset,
                records_offset,
            ),
            args.check,
        )
    for module, entry, body_hash, renderer_entry, bucket_base in (
        (
            "amer",
            0x059B,
            "e8b8889034e477f80bc9bb2a2cc0c3877220804facb2c40371e03acac5a9e744",
            0x2572,
            0x094C,
        ),
        (
            "croolis",
            0x05DC,
            "9acc15fa9730c092561bf9b60ef6aeede36f3f4491cbd5131b9a53ff491bed09",
            0x25D6,
            0x094E,
        ),
        (
            "scrut",
            0x05DC,
            "e30b562918452a15fd9eeecf6b06ca502bd49f1223f313416b4ef2d05baf359b",
            0x2696,
            0x094E,
        ),
    ):
        update_vector(
            VECTOR_ROOT / f"xdb_{module}_func_{entry:04x}_natural.json",
            alien_primary_mesh_vectors(
                module,
                entry,
                body_hash,
                renderer_entry,
                bucket_base,
            ),
            args.check,
        )
    for (
        module,
        entry,
        body_size,
        body_hash,
        continuation,
        bucket_base,
        per_context_signal,
    ) in (
        (
            "amer",
            0x24CF,
            163,
            "e784f6305eb359e3b85baaf4a5c87d0600db42cea9525f094cde2ee2ccc0bcc2",
            0x2572,
            0x094C,
            False,
        ),
        (
            "croolis",
            0x2514,
            194,
            "ef8eb9a19208f2e1446c47d2783b68c4e903587f2b3c8cc553c4ad4acc28c628",
            0x25D6,
            0x094E,
            True,
        ),
        (
            "scrut",
            0x25D4,
            194,
            "ef8eb9a19208f2e1446c47d2783b68c4e903587f2b3c8cc553c4ad4acc28c628",
            0x2696,
            0x094E,
            True,
        ),
    ):
        update_vector(
            VECTOR_ROOT / f"xdb_{module}_func_{entry:04x}_natural.json",
            alien_face_bucket_vectors(
                module,
                entry,
                body_size,
                body_hash,
                continuation,
                bucket_base,
                per_context_signal,
            ),
            args.check,
        )
    for module, entry, face_activate, body_hash, layout in (
        (
            "amer",
            0x2572,
            0x2B6D,
            "51afd4217130dba15453b8a3bcd2bc417abc05067e546e292c12a227f70dcd57",
            {
                "free_head": 0x0BCE,
                "column": 0x0946,
                "bucket_cursor": 0x094A,
                "bucket_heads": 0x094C,
                "render_continuation": 0x0944,
                "active_head": 0x0C2A,
                "active_middle": 0x0C84,
                "raster_pool": 0x0D38,
                "pool_count": 0x0258,
                "render_four_planes": 0x28A1,
                "render_mode_x": 0x28D0,
                "render_linear": 0x29C6,
                "advance_secondary": 0x2ABA,
                "advance_switch": 0x2B09,
                "advance_remove": 0x2B4E,
                "clipped_sort_hook": 0x275D,
            },
        ),
        (
            "croolis",
            0x25D6,
            0x2BDD,
            "88ec45d0b9294a277feccd8e804e3b73f79bd078e147d599921df0de64ce35ab",
            {
                "free_head": 0x0BD0,
                "column": 0x0948,
                "bucket_cursor": 0x094C,
                "bucket_heads": 0x094E,
                "render_continuation": 0x0946,
                "active_head": 0x0C2C,
                "active_middle": 0x0C86,
                "raster_pool": 0x0D3A,
                "pool_count": 0x0258,
                "render_four_planes": 0x2905,
                "render_mode_x": 0x2940,
                "render_linear": 0x2A36,
                "advance_secondary": 0x2B2A,
                "advance_switch": 0x2B79,
                "advance_remove": 0x2BBE,
                "clipped_sort_hook": 0x27C1,
            },
        ),
        (
            "scrut",
            0x2696,
            0x2C9D,
            "1e798885570673359062749747c6271d91cb1de8f3d4ab9cbc5fa1541de9eeeb",
            {
                "free_head": 0x0BD0,
                "column": 0x0948,
                "bucket_cursor": 0x094C,
                "bucket_heads": 0x094E,
                "render_continuation": 0x0946,
                "active_head": 0x0C2C,
                "active_middle": 0x0C86,
                "raster_pool": 0x0D3A,
                "pool_count": 0x0258,
                "render_four_planes": 0x29C5,
                "render_mode_x": 0x2A00,
                "render_linear": 0x2AF6,
                "advance_secondary": 0x2BEA,
                "advance_switch": 0x2C39,
                "advance_remove": 0x2C7E,
                "clipped_sort_hook": 0x2881,
            },
        ),
    ):
        update_vector(
            VECTOR_ROOT / f"xdb_{module}_func_{entry:04x}_natural.json",
            alien_full_renderer_vectors(
                module,
                entry,
                face_activate,
                body_hash,
                layout,
            ),
            args.check,
        )
    for module, entry, body_hash, layout in (
        (
            "amer",
            0x2B6D,
            "92d3573f9bd1b2b3d79e3a1179f00c075fe633903d28ec02be7b5e8ba3dac38d",
            {
                "free_head": 0x0BCE,
                "active_head": 0x0C2A,
                "active_tail": 0x0CDE,
                "max_face_width": 0x01F4,
                "advance_secondary": 0x2ABA,
                "advance_switch": 0x2B09,
                "advance_remove": 0x2B4E,
                "reciprocal_data": 0x33100,
                "scratch_low_start": 0x08E2,
                "scratch_low_end": 0x08FA,
                "scratch_high_start": 0x0936,
                "scratch_high_end": 0x0944,
            },
        ),
        (
            "croolis",
            0x2BDD,
            "84ca972abc64d3f32329ea41ce675c13e657b44ceabc9dbc5ae8c9f61b498bc8",
            {
                "free_head": 0x0BD0,
                "active_head": 0x0C2C,
                "active_tail": 0x0CE0,
                "max_face_width": 0x01F4,
                "advance_secondary": 0x2B2A,
                "advance_switch": 0x2B79,
                "advance_remove": 0x2BBE,
                "reciprocal_data": 0x311E0,
                "scratch_low_start": 0x08E4,
                "scratch_low_end": 0x08FC,
                "scratch_high_start": 0x0938,
                "scratch_high_end": 0x0946,
            },
        ),
        (
            "scrut",
            0x2C9D,
            "bd9371a018942ec432ea695d4046a06902cac4f1e4e21a8231667d4fb5722ff0",
            {
                "free_head": 0x0BD0,
                "active_head": 0x0C2C,
                "active_tail": 0x0CE0,
                "max_face_width": 0x01F4,
                "advance_secondary": 0x2BEA,
                "advance_switch": 0x2C39,
                "advance_remove": 0x2C7E,
                "reciprocal_data": 0x30EF0,
                "scratch_low_start": 0x08E4,
                "scratch_low_end": 0x08FC,
                "scratch_high_start": 0x0938,
                "scratch_high_end": 0x0946,
            },
        ),
    ):
        update_vector(
            VECTOR_ROOT / f"xdb_{module}_func_{entry:04x}_natural.json",
            alien_face_activate_vectors(module, entry, body_hash, layout),
            args.check,
        )
    for module, entry in (
        ("amer", 0x1DD8),
        ("croolis", 0x1E1D),
        ("scrut", 0x1EDD),
    ):
        update_vector(
            VECTOR_ROOT / f"xdb_{module}_func_{entry:04x}_natural.json",
            alien_camera_matrix_update_vectors(module, entry),
            args.check,
        )
    for module, entry in (
        ("amer", 0x0355),
        ("croolis", 0x036A),
        ("scrut", 0x036A),
    ):
        update_vector(
            VECTOR_ROOT / f"xdb_{module}_func_{entry:04x}_natural.json",
            alien_slot7_palette_update_vectors(module, entry),
            args.check,
        )
    for module, entry, cursor_offset in (
        ("amer", 0x0B0F, 0x1BC2),
        ("croolis", 0x0B50, 0x1B2E),
        ("scrut", 0x0B55, 0x1BE3),
    ):
        update_vector(
            VECTOR_ROOT / f"xdb_{module}_func_{entry:04x}_natural.json",
            anchor_state_vectors(module, entry, cursor_offset),
            args.check,
        )
    for module, entry in (
        ("amer", 0x0B1F),
        ("croolis", 0x0B60),
    ):
        update_vector(
            VECTOR_ROOT / f"xdb_{module}_func_{entry:04x}_natural.json",
            apply_delta_vectors(module, entry),
            args.check,
        )
    update_vector(
        VECTOR_ROOT / "xdb_scrut_func_0b65_natural.json",
        lower_state_vectors("scrut", 0x0B65, 0x1BE3),
        args.check,
    )
    for module, entry in (
        ("amer", 0x1B5F),
        ("croolis", 0x1ACB),
        ("scrut", 0x1B80),
    ):
        update_vector(
            VECTOR_ROOT / f"xdb_{module}_func_{entry:04x}_natural.json",
            sample_delta_vectors(module, entry, False),
            args.check,
        )
    for module, entry in (
        ("amer", 0x1B8F),
        ("croolis", 0x1AFB),
        ("scrut", 0x1BB0),
    ):
        update_vector(
            VECTOR_ROOT / f"xdb_{module}_func_{entry:04x}_natural.json",
            sample_delta_vectors(module, entry, True),
            args.check,
        )
    for module, entry, initial_resume in (
        ("amer", 0x1BEA, 0x1C34),
        ("croolis", 0x1B46, 0x1B85),
        ("scrut", 0x1BFB, 0x1C45),
    ):
        update_vector(
            VECTOR_ROOT / f"xdb_{module}_func_{entry:04x}_natural.json",
            resume_or_init_vectors(module, entry, initial_resume),
            args.check,
        )
    for module, entry in (
        ("amer", 0x1DD6),
        ("croolis", 0x1D27),
        ("scrut", 0x1DE7),
    ):
        update_vector(
            VECTOR_ROOT / f"xdb_{module}_func_{entry:04x}_natural.json",
            near_noop_vectors(module, entry),
            args.check,
        )
    assert _COVERAGE_RECORDER is not None
    coverage_rows = build_coverage_report(
        REPO_ROOT,
        XDB_MANIFEST,
        {module: load_image(module) for module in IMAGE_PATHS},
        _COVERAGE_RECORDER,
    )
    update_coverage_report(XDB_COVERAGE_REPORT, coverage_rows, args.check)
    if args.require_complete_coverage:
        require_complete_direct_coverage(coverage_rows)
    elif not args.allow_unreviewed_coverage:
        require_reviewed_direct_coverage(coverage_rows, XDB_COVERAGE_REVIEWS)
    direct_count = sum(
        "oracle_verified" in row["oracle_status"] for row in coverage_rows
    )
    pending_count = len(coverage_rows) - direct_count
    print(
        f"OK: {XDB_COVERAGE_REPORT.relative_to(REPO_ROOT)} "
        f"({direct_count} direct, {pending_count} pending)"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
