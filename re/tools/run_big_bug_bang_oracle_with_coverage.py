#!/usr/bin/env python3
"""Run one BBB oracle and record the original instructions it executes."""

from __future__ import annotations

import argparse
import atexit
import hashlib
import json
from pathlib import Path
import runpy
import sys
from typing import Any

_HERE = Path(__file__).resolve().parent
sys.path[:] = [path for path in sys.path if Path(path or ".").resolve() != _HERE]

import capstone  # noqa: E402
import unicorn  # noqa: E402


REPO_ROOT = Path(__file__).resolve().parents[2]
SEQUEL_PATH = REPO_ROOT / "output/big-bug-bang/disc/BLOOD2PG.EXE"
COMMANDER_PATH = REPO_ROOT / "re/bin/BLOODPRG.EXE"
SEQUEL_HEADER_SIZE = 0x800
COMMANDER_HEADER_SIZE = 0x600
DYNAMIC_ENTRYPOINTS = {0xE0ED}


def image_signature(data: bytes, image: bytes, header_size: int) -> int | None:
    """Return the file-offset bias when *data* is a complete mapped image."""
    for source_offset in (0, header_size):
        candidate = image[source_offset:]
        if len(data) != len(candidate):
            continue
        if data[:64] == candidate[:64] and data[-64:] == candidate[-64:]:
            return source_offset
    return None


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("oracle", type=Path)
    parser.add_argument("coverage", type=Path)
    parser.add_argument("oracle_args", nargs=argparse.REMAINDER)
    args = parser.parse_args()

    sequel = SEQUEL_PATH.read_bytes()
    commander = COMMANDER_PATH.read_bytes()
    graph = json.loads(
        (REPO_ROOT / "re/big_bug_bang_expanded_func_graph.json").read_text()
    )
    known_entries = set(graph["funcs"]) | DYNAMIC_ENTRYPOINTS
    entered: set[int] = set()
    executed: set[int] = set()
    original_uc = unicorn.Uc
    decoder = capstone.Cs(capstone.CS_ARCH_X86, capstone.CS_MODE_16)
    decoder.detail = True

    class CoverageMachine:
        def __init__(self, *uc_args: Any, **uc_kwargs: Any) -> None:
            self._machine = original_uc(*uc_args, **uc_kwargs)
            self._bbb_file_bias: int | None = None
            self._emulation_start: int | None = None
            self._last_original: tuple[int, bytes] | None = None
            self._machine.hook_add(unicorn.UC_HOOK_CODE, self._record_bbb_instruction)

        def __getattr__(self, name: str) -> Any:
            return getattr(self._machine, name)

        def mem_write(self, address: int, data: bytes | bytearray) -> None:
            raw = bytes(data)
            sequel_source = image_signature(raw, sequel, SEQUEL_HEADER_SIZE)
            commander_source = image_signature(raw, commander, COMMANDER_HEADER_SIZE)
            if sequel_source is not None and commander_source is None:
                self._bbb_file_bias = sequel_source - address
            elif commander_source is not None and sequel_source is None:
                self._bbb_file_bias = None
            self._machine.mem_write(address, raw)

        def emu_start(self, begin: int, *emu_args: Any, **emu_kwargs: Any) -> None:
            self._emulation_start = begin
            self._last_original = None
            self._machine.emu_start(begin, *emu_args, **emu_kwargs)

        def _record_bbb_instruction(
            self, machine: unicorn.Uc, address: int, size: int, _user_data: Any
        ) -> None:
            if self._bbb_file_bias is None:
                self._last_original = None
                return
            file_offset = address + self._bbb_file_bias
            expected = sequel[file_offset : file_offset + size]
            try:
                current = bytes(machine.mem_read(address, size))
            except unicorn.UcError:
                self._last_original = None
                return
            if not (
                SEQUEL_HEADER_SIZE <= file_offset < len(sequel) and current == expected
            ):
                self._last_original = None
                return

            executed.add(file_offset)
            if file_offset in known_entries and (
                address == self._emulation_start
                or self._arrived_by_original_control_transfer(address)
            ):
                entered.add(file_offset)
            self._emulation_start = None
            self._last_original = (address, current)

        def _arrived_by_original_control_transfer(self, address: int) -> bool:
            if self._last_original is None:
                return False
            previous_address, previous_bytes = self._last_original
            if previous_address + len(previous_bytes) == address:
                return False
            instruction = next(decoder.disasm(previous_bytes, previous_address), None)
            if instruction is None:
                return False
            return not (
                capstone.CS_GRP_RET in instruction.groups
                or instruction.mnemonic == "iret"
            )

    unicorn.Uc = CoverageMachine

    def write_coverage() -> None:
        entries = sorted(known_entries.intersection(entered))
        args.coverage.parent.mkdir(parents=True, exist_ok=True)
        args.coverage.write_text(
            json.dumps(
                {
                    "oracle": str(args.oracle.resolve().relative_to(REPO_ROOT)),
                    "oracle_sha256": hashlib.sha256(
                        args.oracle.read_bytes()
                    ).hexdigest(),
                    "executed_instruction_count": len(executed),
                    "entered_entrypoints": [f"0x{entry:04x}" for entry in entries],
                },
                indent=2,
            )
            + "\n"
        )

    atexit.register(write_coverage)
    sys.argv = [str(args.oracle), *args.oracle_args]
    runpy.run_path(str(args.oracle), run_name="__main__")


if __name__ == "__main__":
    main()
