#!/usr/bin/env python3
"""Targeted 16-bit/386 disassembler for BLOODPRG.EXE.

Usage:
    python3 re/tools/dis.py <file_off_hex> [n_insns]
    python3 re/tools/dis.py SEG:OFF [n_insns]
    python3 re/tools/dis.py --img <image_off_hex> [n_insns]

Decodes in 16-bit mode (CS_MODE_16); capstone honours the 0x66/0x67
operand/address-size prefixes the 386 code uses. Addresses are shown as file
offsets so they can be fed straight back in. Labels from labels.csv are shown
inline for the instruction address and appended as comments when an operand's
immediate/target matches a known label or relocation.
"""
import os
import sys

# This file is named dis.py, which shadows the stdlib 'dis' module that
# capstone -> inspect imports. Drop our own dir from sys.path[0] before
# importing capstone, then restore it for the local mzfile import.
_here = os.path.dirname(os.path.abspath(__file__))
if sys.path and os.path.abspath(sys.path[0]) == _here:
    sys.path.pop(0)

import capstone

sys.path.insert(0, _here)
from mzfile import MZ, load_labels


def parse_addr(mz, s):
    s = s.strip()
    if s.lower().startswith("--img"):
        return None
    if ":" in s and not s.lower().startswith("0x"):
        seg, off = s.split(":")
        return mz.segoff_to_file(int(seg, 16), int(off, 16))
    return int(s, 16)


def main():
    args = sys.argv[1:]
    if not args:
        print(__doc__)
        return
    mz = MZ()
    _, labels_file = load_labels()

    img_mode = False
    if args[0] == "--img":
        img_mode = True
        args = args[1:]

    if img_mode:
        file_off = mz.image_to_file(int(args[0], 16))
    else:
        file_off = parse_addr(mz, args[0])
    n = int(args[1]) if len(args) > 1 else 40

    md = capstone.Cs(capstone.CS_ARCH_X86, capstone.CS_MODE_16)
    md.detail = True

    # Disassemble a generous window from the file; capstone addresses == file off.
    window = mz.data[file_off:file_off + max(n * 12, 256)]
    count = 0
    for insn in md.disasm(window, file_off):
        if count >= n:
            break
        count += 1
        lbl = labels_file.get(insn.address)
        label_str = ""
        if lbl:
            label_str = f"  ; <{lbl[0]}>" + (f" {lbl[1]}" if lbl[1] else "")

        # Annotate referenced addresses / relocation operands.
        notes = []
        for op in insn.operands:
            if op.type == capstone.x86.X86_OP_IMM:
                tgt = op.imm
                # Near branch/call: capstone already resolved to file-off space.
                if insn.group(capstone.x86.X86_GRP_JUMP) or insn.group(capstone.x86.X86_GRP_CALL):
                    t = labels_file.get(tgt)
                    if t:
                        notes.append(f"-> {t[0]}")
        # Flag relocated segment immediates (image offset of operand bytes).
        img_off = mz.file_to_image(insn.address)
        for k in range(insn.size):
            if (img_off + k) in mz.reloc_image_offsets:
                notes.append("reloc")
                break

        note_str = ""
        if notes:
            note_str = "  ; " + " ".join(dict.fromkeys(notes))

        b = insn.bytes.hex()
        print(f"{insn.address:#08x}: {b:<20} {insn.mnemonic:<7} {insn.op_str}"
              f"{label_str}{note_str}")


if __name__ == "__main__":
    main()
