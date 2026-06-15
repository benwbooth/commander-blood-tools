#!/usr/bin/env python3
"""Fingerprint a VM opcode handler: linearly disassemble from its entry until a
return, and report the far calls (lcall seg:off), near calls, and notable
data references (gs:/DS offsets, immediates that match known DS labels).

This identifies which subsystem each handler drives (audio, HNM, palette, etc.)
without reading every instruction by hand.

Usage:
    python3 re/tools/analyze_handler.py <file_off_hex> [max_bytes]
    python3 re/tools/analyze_handler.py --table     # analyze all 52 handlers
"""
import os
import struct
import sys

_here = os.path.dirname(os.path.abspath(__file__))
if sys.path and os.path.abspath(sys.path[0]) == _here:
    sys.path.pop(0)
import capstone

sys.path.insert(0, _here)
from mzfile import MZ, load_labels

TABLE_FILE_OFF = 0x142D0
SEG_04DA_FILE_BASE = 0x053A0
SEG_BASES = [0x0000, 0x008b, 0x01ce, 0x0299, 0x04b9, 0x04da, 0x071e,
             0x0971, 0x0a9a, 0x0b1b, 0x0bbf, 0x0ce2]
SEG_NAME = {0x0299: "render", 0x04da: "vm", 0x071e: "dispatch_seg",
            0x0bbf: "FSdata", 0x0ce2: "DSdata"}


def analyze(mz, md, lbl_file, foff, max_bytes=400):
    far_calls, near_calls, datarefs = [], [], []
    window = mz.data[foff:foff + max_bytes]
    end = foff
    for insn in md.disasm(window, foff):
        m = insn.mnemonic
        if m in ("ret", "retf"):
            end = insn.address
            break
        if m == "lcall":
            # operand "0xSEG, 0xOFF"
            far_calls.append(insn.op_str)
        elif m == "call":
            for op in insn.operands:
                if op.type == capstone.x86.X86_OP_IMM:
                    t = op.imm
                    lab = lbl_file.get(t)
                    near_calls.append(f"{t:#x}" + (f"<{lab[0]}>" if lab else ""))
        # data refs: any memory operand disp that matches a DS label
        for op in insn.operands:
            if op.type == capstone.x86.X86_OP_MEM and op.mem.disp:
                d = op.mem.disp & 0xFFFF
                datarefs.append(d)
        end = insn.address + insn.size
    return far_calls, near_calls, datarefs, end


def seg_of(img):
    cand = [b for b in SEG_BASES if b * 16 <= img]
    return max(cand) if cand else 0


def main():
    mz = MZ()
    _, lbl_file = load_labels()
    md = capstone.Cs(capstone.CS_ARCH_X86, capstone.CS_MODE_16)
    md.detail = True

    if sys.argv[1:] and sys.argv[1] == "--table":
        print("op   handler   end     far_calls (seg:off -> subsystem)")
        for i in range(0x34):
            op = 0xA0 + i
            near = struct.unpack_from("<H", mz.data, TABLE_FILE_OFF + i * 2)[0]
            foff = SEG_04DA_FILE_BASE + near
            fc, nc, dr, end = analyze(mz, md, lbl_file, foff)
            tags = []
            for s in fc:
                tags.append(s)
            print(f"0x{op:02x} {foff:#08x} {end:#08x}  "
                  + ("; ".join(tags) if tags else "(no far calls)"))
        return

    foff = int(sys.argv[1], 16)
    max_bytes = int(sys.argv[2]) if len(sys.argv) > 2 else 400
    fc, nc, dr, end = analyze(mz, md, lbl_file, foff, max_bytes)
    print(f"handler {foff:#08x} .. {end:#08x} ({end-foff} bytes)")
    print("far calls:")
    for s in fc:
        # resolve seg:off -> file
        try:
            seg, off = [int(x, 16) for x in s.replace(" ", "").split(",")]
            tgt = mz.segoff_to_file(seg, off)
            print(f"  lcall {s}  -> file {tgt:#08x}")
        except Exception:
            print(f"  lcall {s}")
    print("near calls:", ", ".join(nc) if nc else "(none)")
    notable = sorted(set(d for d in dr if d >= 0x1000))
    print("data offsets:", " ".join(f"{d:#06x}" for d in notable[:40]))


if __name__ == "__main__":
    main()
