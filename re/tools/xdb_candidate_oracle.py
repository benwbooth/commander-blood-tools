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
import json
import struct
from pathlib import Path

from unicorn import UC_ARCH_X86, UC_HOOK_CODE, UC_HOOK_INTR, UC_MODE_16, Uc, UcError
from unicorn.x86_const import (
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
) -> Uc:
    machine = Uc(UC_ARCH_X86, UC_MODE_16)
    machine.mem_map(0, 0x300000)
    machine.mem_write(0, image)
    machine.reg_write(UC_X86_REG_CS, 0)
    for name, value in registers.items():
        machine.reg_write(REGISTERS[name], value)
    for segment, offset, data in memory:
        machine.mem_write(segment * 16 + offset, data)

    returned = []

    def stop_at_return(
        machine: Uc, address: int, _size: int, _data: object
    ) -> None:
        if address == return_address:
            returned.append(address)
            machine.emu_stop()

    machine.hook_add(UC_HOOK_CODE, stop_at_return)
    if interrupt_handler is not None:
        machine.hook_add(UC_HOOK_INTR, interrupt_handler)
    try:
        machine.emu_start(entry, 0x2FFFF0, count=1000)
    except UcError as error:
        raise RuntimeError(
            f"{entry:#x}: execution failed at "
            f"{machine.reg_read(UC_X86_REG_CS):#x}:"
            f"{machine.reg_read(UC_X86_REG_SP):#x}"
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
        ("manu3", 0x0848),
        ("scrut", 0x1DE7),
    ):
        update_vector(
            VECTOR_ROOT / f"xdb_{module}_func_{entry:04x}_natural.json",
            near_noop_vectors(module, entry),
            args.check,
        )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
