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
