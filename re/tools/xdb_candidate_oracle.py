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

from unicorn import UC_ARCH_X86, UC_HOOK_CODE, UC_MODE_16, Uc, UcError
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
