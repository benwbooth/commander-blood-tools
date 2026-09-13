#!/usr/bin/env python3
"""Build a recursive-descent call graph for a 16-bit DOS MZ executable.

The graph starts at the MZ entrypoint. Direct near and relocated far calls add
functions, while direct jumps and conditional branches extend the current
function's control-flow walk. Register and memory-indirect transfers, plus the
segment-zero far calls used by the games' external runtime boundary, remain in
the ``indirect`` list for separate resolution.

The JSON schema intentionally matches ``re/func_graph.json`` so existing
analysis tools can consume graphs generated for either game.
"""

from __future__ import annotations

import argparse
import collections
import json
import os
from pathlib import Path
import sys
from typing import TypeAlias

_HERE = os.path.dirname(os.path.abspath(__file__))
sys.path[:] = [
    path for path in sys.path if os.path.abspath(path or os.curdir) != _HERE
]

import capstone  # noqa: E402
from capstone.x86 import (  # noqa: E402
    X86_INS_CALL,
    X86_INS_IRET,
    X86_INS_JMP,
    X86_INS_LCALL,
    X86_INS_LJMP,
    X86_INS_RET,
    X86_INS_RETF,
    X86_OP_IMM,
)

sys.path.insert(0, _HERE)

from mzfile import DEFAULT_BIN, MZ  # noqa: E402

sys.path[:] = [
    path for path in sys.path if os.path.abspath(path or os.curdir) != _HERE
]


IndirectSite: TypeAlias = list[str | int]


class FunctionGraphBuilder:
    def __init__(self, mz: MZ):
        self.mz = mz
        self.disassembler = capstone.Cs(
            capstone.CS_ARCH_X86, capstone.CS_MODE_16
        )
        self.disassembler.detail = True
        self.callgraph: dict[int, set[int]] = {}
        self.function_instructions: dict[int, set[int]] = {}
        self.function_segments: dict[int, set[int]] = collections.defaultdict(set)
        self.instruction_segments: dict[tuple[int, int], int] = {}
        self.indirect: list[IndirectSite] = []

    def in_image(self, file_offset: int) -> bool:
        return self.mz.header_size <= file_offset < self.mz.image_total

    def one_instruction(self, file_offset: int):
        end = min(self.mz.image_total, file_offset + 16)
        return next(
            self.disassembler.disasm(
                self.mz.data[file_offset:end], file_offset, count=1
            ),
            None,
        )

    @staticmethod
    def immediate(operands) -> int | None:
        if len(operands) == 1 and operands[0].type == X86_OP_IMM:
            return int(operands[0].imm)
        return None

    def far_target(self, instruction) -> tuple[int, int] | None:
        operands = instruction.operands
        if len(operands) != 2 or any(op.type != X86_OP_IMM for op in operands):
            return None
        segment = int(operands[0].imm)
        offset = int(operands[1].imm)
        if segment == 0:
            return None
        segment_word = self.mz.file_to_image(
            instruction.address + instruction.size - 2
        )
        if segment_word not in self.mz.reloc_image_offsets:
            return None
        target = self.mz.segoff_to_file(segment, offset)
        return (target, segment) if self.in_image(target) else None

    def record_indirect(self, instruction) -> None:
        self.indirect.append(
            [hex(instruction.address), instruction.mnemonic, instruction.op_str]
        )

    def add_callee(self, caller: int, target: int, segment: int) -> None:
        self.callgraph[caller].add(target)
        self.walk_function(target, segment)

    def walk_function(self, entry: int, code_segment: int) -> None:
        self.function_segments[entry].add(code_segment)
        if entry in self.callgraph:
            return

        self.callgraph[entry] = set()
        self.function_instructions[entry] = set()
        pending_blocks = [(entry, code_segment)]
        visited = self.function_instructions[entry]

        while pending_blocks:
            pc, block_segment = pending_blocks.pop()
            while self.in_image(pc) and pc not in visited:
                visited.add(pc)
                self.instruction_segments[(entry, pc)] = block_segment
                instruction = self.one_instruction(pc)
                if instruction is None:
                    break

                operands = instruction.operands
                next_pc = pc + instruction.size

                if instruction.id == X86_INS_CALL:
                    target = self.immediate(operands)
                    if target is not None and self.in_image(target):
                        self.add_callee(entry, target, block_segment)
                    else:
                        self.record_indirect(instruction)
                elif instruction.id == X86_INS_LCALL:
                    far_target = self.far_target(instruction)
                    if far_target is not None:
                        target, target_segment = far_target
                        self.add_callee(entry, target, target_segment)
                    else:
                        self.record_indirect(instruction)
                elif instruction.id in (X86_INS_RET, X86_INS_RETF, X86_INS_IRET):
                    break
                elif instruction.id == X86_INS_JMP:
                    target = self.immediate(operands)
                    if target is not None and self.in_image(target):
                        pending_blocks.append((target, block_segment))
                    else:
                        self.record_indirect(instruction)
                    break
                elif instruction.id == X86_INS_LJMP:
                    far_target = self.far_target(instruction)
                    if far_target is not None:
                        target, target_segment = far_target
                        pending_blocks.append((target, target_segment))
                    else:
                        self.record_indirect(instruction)
                    break
                elif capstone.CS_GRP_JUMP in instruction.groups:
                    target = self.immediate(operands)
                    if target is not None and self.in_image(target):
                        pending_blocks.append((target, block_segment))
                    else:
                        self.record_indirect(instruction)

                pc = next_pc

    def build(self) -> dict[str, object]:
        self.walk_function(self.mz.entry_file, self.mz.e_cs)
        functions = sorted(self.callgraph)
        graph = {
            str(caller): sorted(callees)
            for caller, callees in sorted(self.callgraph.items())
        }
        return {
            "funcs": functions,
            "leaves": [function for function in functions if not graph[str(function)]],
            "callgraph": graph,
            "indirect": self.indirect,
        }


def build_function_graph(executable: str | Path) -> dict[str, object]:
    return FunctionGraphBuilder(MZ(str(executable))).build()


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "executable",
        nargs="?",
        type=Path,
        default=Path(DEFAULT_BIN),
        help="DOS MZ executable (default: re/bin/BLOODPRG.EXE)",
    )
    parser.add_argument(
        "-o",
        "--output",
        type=Path,
        help="write JSON to this path instead of stdout",
    )
    return parser.parse_args()


def main() -> None:
    args = parse_args()
    graph = build_function_graph(args.executable)
    encoded = json.dumps(graph, indent=2) + "\n"
    if args.output is None:
        sys.stdout.write(encoded)
    else:
        args.output.write_text(encoded)


if __name__ == "__main__":
    main()
