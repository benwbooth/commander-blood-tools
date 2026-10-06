#!/usr/bin/env python3
"""Replay original BBB allocation code from a read-only normal-UI capture.

Only this private Unicorn copy is modified. No live guest memory is written.
The report retains hashes and a short numeric probe, not whole game resources.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import struct
from pathlib import Path

from unicorn import UC_ARCH_X86, UC_HOOK_CODE, UC_MODE_16, Uc
from unicorn.x86_const import (
    UC_X86_REG_AX, UC_X86_REG_CS, UC_X86_REG_DS, UC_X86_REG_EBP,
    UC_X86_REG_ES, UC_X86_REG_FS, UC_X86_REG_GS, UC_X86_REG_SP, UC_X86_REG_SS,
)

EXECUTABLE_SHA256 = "4b65ffca3e113a1826371e3436177861640a1b7aae24caafebb4c2f7aa467834"
RELEASE_ORDER = ("deb", "cod", "bas", "dic")


def digest(data):
    return hashlib.sha256(data).hexdigest()


def replay(capture, before_name, after_name, resources, offset, extent, numeric_name):
    report_bytes = (capture / "capture.json").read_bytes()
    report = json.loads(report_bytes)
    assert report["executable_sha256"] == EXECUTABLE_SHA256
    before = next(s for s in report["samples"] if s.get("guest_dump") == before_name)
    after = next(s for s in report["samples"] if s.get("guest_dump") == after_name)
    initial = (capture / before_name).read_bytes()
    actual = (capture / after_name).read_bytes()
    assert len(initial) == len(actual) == 0x100000
    assert before["global_segment"] == after["global_segment"]
    assert before["catalog_segment"] == after["catalog_segment"]
    globals_segment, catalog_segment = before["global_segment"], before["catalog_segment"]
    base = globals_segment * 16 - (0xF7F0 - 0x800)
    machine = Uc(UC_ARCH_X86, UC_MODE_16)
    machine.mem_map(0, 0x100000)
    machine.mem_write(0, initial)
    returned = []

    def stop(cpu, address, _size, _context):
        if address == 0x200:
            returned.append(True)
            cpu.emu_stop()

    machine.hook_add(UC_HOOK_CODE, stop)

    # Observe the unmodified profile loader's actual release order, stopping
    # before its first file read. The allocator replay below must use that order.
    loader = Uc(UC_ARCH_X86, UC_MODE_16)
    loader.mem_map(0, 0x100000)
    loader.mem_write(0, initial)
    for register, value in ((UC_X86_REG_CS, base // 16 + 0x4E1),
                            (UC_X86_REG_DS, globals_segment), (UC_X86_REG_ES, globals_segment),
                            (UC_X86_REG_GS, globals_segment), (UC_X86_REG_FS, catalog_segment),
                            (UC_X86_REG_SS, globals_segment), (UC_X86_REG_SP, 0xFF00),
                            (UC_X86_REG_AX, after["profile"])):
        loader.reg_write(register, value)
    released_by_loader, load_requested = [], []

    def observe_loader(cpu, address, _size, _context):
        if address == base + 0x5700 - 0x800:
            released_by_loader.append(cpu.reg_read(UC_X86_REG_AX))
        if address == base + 0x2BFB - 0x800:
            load_requested.append(cpu.reg_read(UC_X86_REG_AX))
            cpu.emu_stop()

    loader.hook_add(UC_HOOK_CODE, observe_loader)
    loader.emu_start(base + 0x5820 - 0x800, 0, count=1000000)
    assert released_by_loader == [before["bindings"][role]["handle"] for role in RELEASE_ORDER]
    assert load_requested == [after["bindings"]["deb"]["handle"]]

    def call(entry, identity, size=None):
        returned.clear()
        for register, value in (
            (UC_X86_REG_CS, base // 16 + 0x4E1),
            (UC_X86_REG_DS, globals_segment), (UC_X86_REG_ES, globals_segment),
            (UC_X86_REG_GS, globals_segment), (UC_X86_REG_FS, catalog_segment),
            (UC_X86_REG_SS, globals_segment), (UC_X86_REG_SP, 0xFF00),
            (UC_X86_REG_AX, identity),
        ):
            machine.reg_write(register, value)
        if size is not None:
            machine.reg_write(UC_X86_REG_EBP, size)
        machine.mem_write(globals_segment * 16 + 0xFF00, struct.pack("<HH", 0x200, 0))
        machine.emu_start(base + entry - 0x800, 0, count=1000000)
        assert returned == [True], hex(entry)

    released = []
    for role in RELEASE_ORDER:
        binding = before["bindings"][role]
        resident = next(r for r in before["resident_resources"] if r["id"] == binding["handle"])
        released.append(dict(role=role, resource=resident["id"],
                             allocation_size=resident["allocated_bytes"]))
        call(0x5700, binding["handle"])

    loaded = []
    seen = set()
    for role in RELEASE_ORDER:
        binding = after["bindings"][role]
        identity = binding["handle"]
        if role == "bas" and identity not in binding["owners"]:
            assert binding["linear"] == after["bindings"]["cod"]["linear"]
            continue
        if identity in seen:
            continue
        seen.add(identity)
        resident = next(r for r in after["resident_resources"] if r["id"] == identity)
        source = resources / resident["name"].upper()
        data = source.read_bytes()
        call(0x5610, identity, len(data))
        segment, flags, size = struct.unpack("<HHI", machine.mem_read(catalog_segment * 16 + identity * 8, 8))
        assert segment * 16 == binding["linear"]
        assert size == resident["allocated_bytes"] == (len(data) + 15) & ~15
        assert flags == 3
        machine.mem_write(segment * 16, data)
        loaded.append(dict(role=role, resource=identity, filename=source.name,
                           file_size=len(data), allocation_size=size, sha256=digest(data)))

    dictionary = after["bindings"]["dic"]["linear"]
    allocation_size = next(r["allocation_size"] for r in loaded if r["role"] == "dic")
    assert allocation_size <= offset < extent
    predicted = bytes(machine.mem_read(dictionary + allocation_size, extent - allocation_size))
    observed = actual[dictionary + allocation_size:dictionary + extent]
    assert predicted == observed, "original allocation replay differs from captured pool tail"
    suffix = observed[offset - allocation_size:].split(b"\0", 1)[0] + b"\0"
    assert len(suffix) <= 32, "probe should be a short, terminated numeric suffix"
    # Identify the immutable original companion that supplied these exact bytes.
    provenance = []
    for role in ("bas", "dic"):
        name = f"SCRIPT{before['profile'] + 1}.{role.upper()}"
        data = (resources / name).read_bytes()
        start = 8 if role == "dic" else 0
        while (found := data.find(suffix, start)) >= 0:
            address = before["bindings"][role]["linear"] + found
            if initial[address:address + len(suffix)] == suffix:
                provenance.append(dict(resource=before["bindings"][role]["handle"],
                                       filename=name, offset=found, sha256=digest(data)))
            start = found + 1
    assert len(provenance) == 1, provenance
    numeric_image = (capture / numeric_name).read_bytes()
    numeric_sample = next(s for s in report["samples"] if s.get("guest_dump") == numeric_name)
    assert numeric_sample["bindings"] == after["bindings"]
    assert numeric_image[dictionary + offset:dictionary + offset + len(suffix)] == suffix
    globals_address = globals_segment * 16
    word_offset, word_segment = struct.unpack_from("<HH", numeric_image, globals_address + 0x6B1A)
    word_address = word_segment * 16 + word_offset
    words = []
    for index in range(256):
        value = struct.unpack_from("<H", numeric_image, word_address + index * 2)[0]
        words.append(value)
        if value in (0, 65535):
            break
    assert words[-1] in (0, 65535) and offset in words
    hash_cases = []
    for scratch in (b"", b"20"):
        cpu = Uc(UC_ARCH_X86, UC_MODE_16)
        cpu.mem_map(0, 0x100000)
        cpu.mem_write(0, numeric_image)
        cpu.mem_write(dictionary + 1, scratch + b"\0")
        for field, value in ((0xCE7, 1), (0xCE8, 0), (0xF47, 1), (0xF48, 0), (0xF49, 0)):
            cpu.mem_write(globals_address + field, bytes([value]))
        for register, value in ((UC_X86_REG_CS, base // 16 + 0xC5C),
                                (UC_X86_REG_DS, globals_segment), (UC_X86_REG_GS, globals_segment),
                                (UC_X86_REG_SS, globals_segment), (UC_X86_REG_SP, 0xFF00)):
            cpu.reg_write(register, value)
        done = []

        def stop_hash(machine, address, _size, _context):
            if address == base + 0xD05C - 0x800:
                done.append(True)
                machine.emu_stop()

        cpu.hook_add(UC_HOOK_CODE, stop_hash)
        cpu.emu_start(base + 0xCF73 - 0x800, 0, count=10000)
        assert done == [True]
        seed = struct.unpack("<H", cpu.mem_read(globals_address + 0xE5F, 2))[0]
        assert bytes(cpu.mem_read(globals_address + 0xF47, 2)) == b"\0\1"
        hash_cases.append(dict(scratch=list(scratch), seed=seed))
    numeric_words = [list(numeric_image[dictionary + value:].split(b"\0", 1)[0])
                     for value in words[:-1] if value not in (1, offset)]
    return dict(
        scope="original allocator replay of captured normal profile transition, not a playthrough",
        executable_sha256=EXECUTABLE_SHA256, capture_sha256=digest(report_bytes),
        recorder_sha256=report["recorder_sha256"],
        before=dict(sample=before_name, sha256=digest(initial), profile=before["profile"]),
        after=dict(sample=after_name, sha256=digest(actual), profile=after["profile"]),
        released=released, loaded=loaded,
        observed_profile_loader_releases=released_by_loader,
        tail=dict(start=allocation_size, end=extent, sha256=digest(observed), matches=True),
        probe=dict(offset=offset, bytes=list(suffix), provenance=provenance[0]),
        numeric=dict(sample=numeric_name, sha256=digest(numeric_image),
                     encoded_words=words, dictionary_words=numeric_words,
                     cases=hash_cases, scope="controlled hash inputs; not observed call timing"),
    )


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("capture", type=Path)
    parser.add_argument("resources", type=Path)
    parser.add_argument("output", type=Path)
    parser.add_argument("--before", required=True)
    parser.add_argument("--after", required=True)
    parser.add_argument("--offset", type=int, required=True)
    parser.add_argument("--extent", type=int, required=True)
    parser.add_argument("--numeric-sample", required=True)
    args = parser.parse_args()
    result = replay(args.capture, args.before, args.after, args.resources, args.offset, args.extent, args.numeric_sample)
    args.output.write_text(json.dumps(result, indent=2) + "\n")
    print(json.dumps(dict(tail=result["tail"], probe=result["probe"]), indent=2))


if __name__ == "__main__":
    main()
