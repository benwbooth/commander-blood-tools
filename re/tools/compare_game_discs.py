#!/usr/bin/env python3
"""Compare extracted original discs without changing game inputs or runtime code.

Archive layout follows commander-blood-formats/src/archive.rs. Hash equality
proves byte identity only; cbvm round trips do not prove sequel VM semantics.
"""

import argparse
from collections import Counter
import hashlib
import json
from pathlib import Path
import re
import struct
import subprocess

DIRECTORY_START = 2
DIRECTORY_LIMIT = 65536
DIRECTORY_ENTRY = struct.Struct("<16siiB")
NATIVE_SUFFIXES = {".XDB", ".DRV", ".EXE", ".COM"}

# File offsets established from each original's native dispatch and skip paths.
# Never apply Commander offsets to another build merely because it is an MZ.
VM_LAYOUTS = {
    "7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823":
        (0x142D0, 0x14338, 0x53A0, 52, 0x56A6, bytes.fromhex("bf b0 6e")),
    "4b65ffca3e113a1826371e3436177861640a1b7aae24caafebb4c2f7aa467834":
        (0x16A78, 0x16AEA, 0x5820, 57, 0x5B65, bytes.fromhex("bf 88 72")),
}


def native_vm_table(data):
    layout = VM_LAYOUTS.get(digest(data))
    if layout is None:
        return None
    table, descriptors, code_base, count, probe, expected = layout
    assert data[probe:probe + len(expected)] == expected
    assert table + count * 2 == descriptors
    rows = []
    for index in range(count):
        offset = struct.unpack_from("<H", data, table + index * 2)[0]
        rows.append({"opcode": 160 + index,
                     "handler_file_offset": code_base + offset if offset else None,
                     "skip_descriptor": list(data[descriptors + index * 2:descriptors + index * 2 + 2])})
    return {"handler_table_file_offset": table,
            "skip_table_file_offset": descriptors, "rows": rows}


def archive_entries(data):
    entries = {}
    for offset in range(DIRECTORY_START, min(DIRECTORY_LIMIT, len(data)), DIRECTORY_ENTRY.size):
        if data[offset] == 0:
            return entries
        if offset + DIRECTORY_ENTRY.size > min(DIRECTORY_LIMIT, len(data)):
            raise ValueError("truncated archive directory")
        raw, size, position, _reserved = DIRECTORY_ENTRY.unpack_from(data, offset)
        if b"\0" not in raw:
            raise ValueError("unterminated archive name")
        name = raw.split(b"\0", 1)[0].decode("ascii").upper()
        if size < 0 or position < 0 or position + size > len(data):
            raise ValueError(f"invalid archive extent: {name}")
        payload = data[position:position + size]
        # Commander repeats directory records. Collapse only identical content;
        # differing payloads require an explicit native lookup-order analysis.
        if name in entries and entries[name] != payload:
            raise ValueError(f"conflicting duplicate archive name: {name}")
        entries[name] = payload
    raise ValueError("missing archive directory terminator")


def digest(data):
    return hashlib.sha256(data).hexdigest()


def describe(data):
    return {"bytes": len(data), "sha256": digest(data)}


def compare(left, right):
    shared = left.keys() & right.keys()
    identical = sorted(n for n in shared if left[n] == right[n])
    left_hashes = {digest(data) for data in left.values()}
    reused = sorted(n for n, data in right.items() if digest(data) in left_hashes)
    return {
        "commander_count": len(left), "sequel_count": len(right),
        "commander_payload_bytes": sum(map(len, left.values())),
        "sequel_payload_bytes": sum(map(len, right.values())),
        "same_name_identical": identical,
        "same_name_changed": sorted(shared - set(identical)),
        "commander_only": sorted(left.keys() - right.keys()),
        "sequel_only": sorted(right.keys() - left.keys()),
        "sequel_reused_any_name": reused,
        "sequel_reused_payload_bytes": sum(len(right[n]) for n in reused),
        "commander_extensions": dict(sorted(Counter(Path(n).suffix for n in left).items())),
        "sequel_extensions": dict(sorted(Counter(Path(n).suffix for n in right).items())),
    }


def mz_header(data):
    if data[:2] != b"MZ":
        return None
    words = struct.unpack_from("<14H", data)
    header_bytes = words[4] * 16
    return {"header_bytes": header_bytes, "relocations": words[3],
            "entry_cs": words[11], "entry_ip": words[10],
            "entry_file_offset": header_bytes + words[11] * 16 + words[10]}


def inventory(directory):
    loose = {str(p.relative_to(directory)).upper(): p.read_bytes()
             for p in sorted(directory.rglob("*"))
             if p.is_file() and p.name.upper() != "BLOOD.DAT"}
    data = (directory / "BLOOD.DAT").read_bytes()
    return loose, archive_entries(data), describe(data)


def vm_probes(cbvm, directory, output):
    output.mkdir(parents=True, exist_ok=True)
    results = []
    for path in sorted(directory.glob("SCRIPT*.*")):
        if path.suffix not in {".COD", ".BAS"}:
            continue
        process = subprocess.run([
            str(cbvm), "disassemble", path.suffix[1:].lower(), str(path),
            str(path.with_suffix(".DIC")), str(output / (path.name + ".blood")),
        ], capture_output=True, text=True)
        result = {"file": path.name, "exit_code": process.returncode,
                  "output": (process.stdout + process.stderr).strip()}
        counts = re.search(r"(\d+) semantic span\(s\), (\d+) semantic byte\(s\), (\d+) raw byte\(s\)", process.stdout)
        if counts:
            result.update(zip(("semantic_spans", "semantic_bytes", "raw_bytes"), map(int, counts.groups())))
        results.append(result)
    process = subprocess.run([
        str(cbvm), "decompile-descript", str(directory / "DESCRIPT.DES"),
        str(output / "DESCRIPT.descript"),
    ], capture_output=True, text=True)
    results.append({"file": "DESCRIPT.DES", "exit_code": process.returncode,
                    "output": (process.stdout + process.stderr).strip()})
    return results


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("commander", type=Path)
    parser.add_argument("sequel", type=Path)
    parser.add_argument("output", type=Path)
    parser.add_argument("--cbvm", type=Path)
    args = parser.parse_args()
    old_loose, old_archive, old_dat = inventory(args.commander)
    new_loose, new_archive, new_dat = inventory(args.sequel)
    report = {"scope": "Original-disc byte comparison, not runtime compatibility",
              "archive_inputs": {"commander": old_dat, "sequel": new_dat},
              "loose": compare(old_loose, new_loose),
              "archive": compare(old_archive, new_archive), "native": {}}
    report["native_vm_tables"] = {
        label: native_vm_table(data)
        for label, data in [("commander", old_loose["BLOODPRG.EXE"]),
                            ("sequel", new_loose["BLOOD2PG.EXE"])]
    }
    for label, loose, archive in [("commander", old_loose, old_archive), ("sequel", new_loose, new_archive)]:
        report["native"][label] = {
            origin + "/" + name: {**describe(data), "mz": mz_header(data)}
            for origin, group in [("disc", loose), ("archive", archive)]
            for name, data in group.items() if Path(name).suffix in NATIVE_SUFFIXES
        }
    if args.cbvm:
        report["vm_probes"] = vm_probes(args.cbvm.resolve(), args.sequel.resolve(), args.output.parent / "vm-probes")
        report["commander_vm_control_probes"] = vm_probes(
            args.cbvm.resolve(), args.commander.resolve(), args.output.parent / "commander-vm-probes")
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(report, indent=2) + "\n")
    for kind in ("loose", "archive"):
        item = report[kind]
        print(f"{kind}: {item['sequel_count']} sequel entries; "
              f"{len(item['same_name_identical'])} same-name identical; "
              f"{len(item['same_name_changed'])} same-name changed; "
              f"{len(item['sequel_only'])} sequel-only")
    print(args.output)


if __name__ == "__main__":
    main()
