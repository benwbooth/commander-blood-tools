#!/usr/bin/env python3
"""Automated instruction lifter for path B: translate a LINEAR (no internal branch) DOS function
to a Rust `fn(m: &mut Machine)` operating on the Machine, using the verified flag helpers. The
oracle then checks the emitted lift bit-exact, so any translation error is caught. Grows opcode
coverage incrementally; control-flow (branches/loops) is a later extension.

Usage (PYTHONSAFEPATH=1): re/tools/lift.py <file_offset_hex> <rust_fn_name>"""
import capstone, sys

D = open("re/bin/BLOODPRG.EXE", "rb").read()
MD = capstone.Cs(capstone.CS_ARCH_X86, capstone.CS_MODE_16)
MD.detail = True

W = {"ax", "bx", "cx", "dx", "si", "di", "bp", "sp"}
B = {"al", "ah", "bl", "bh", "cl", "ch", "dl", "dh"}
E = {"eax", "ebx", "ecx", "edx", "esi", "edi", "ebp", "esp"}
SEG = {"cs", "ds", "es", "ss", "fs", "gs"}

def rd(reg):  # read a register as a Rust expr
    if reg in W: return f"m.regs.{reg}()"
    if reg in B: return f"m.regs.{reg}()"
    if reg in E: return f"m.regs.{reg}"
    if reg in SEG: return f"m.regs.{reg}"
    raise NotImplementedError(f"read reg {reg}")

def wr(reg, expr):  # write a register
    if reg in W: return f"m.regs.set_{reg}({expr});"
    if reg in B: return f"m.regs.set_{reg}({expr});"
    if reg in E: return f"m.regs.{reg} = {expr};"
    if reg in SEG: return f"m.regs.{reg} = {expr};"
    raise NotImplementedError(f"write reg {reg}")

def mem_addr(insn, opnd):
    """Return (seg_expr, off_expr, size) for a memory operand."""
    m = opnd.mem
    seg = "m.regs.ds"
    # segment override
    if insn.prefix and insn.prefix[1]:
        ov = {0x2e: "cs", 0x36: "ss", 0x3e: "ds", 0x26: "es", 0x64: "fs", 0x65: "gs"}.get(insn.prefix[1])
        if ov: seg = f"m.regs.{ov}"
    parts = []
    if m.base != 0:
        b = insn.reg_name(m.base)
        if b == "bp": seg = "m.regs.ss"  # bp defaults to SS
        parts.append(rd(b))
    if m.index != 0:
        parts.append(rd(insn.reg_name(m.index)))
    if m.disp != 0 or not parts:
        parts.append(f"0x{m.disp & 0xffff:x}")
    off = parts[0] if len(parts) == 1 else "(" + ".wrapping_add(".join(parts) + ")" * (len(parts) - 1)
    if len(parts) > 1:
        off = parts[0]
        for p in parts[1:]:
            off = f"{off}.wrapping_add({p})"
    size = opnd.size
    return seg, off, size

def emit(insn):
    """Emit Rust lines for one linear instruction. Returns list[str] or raises NotImplementedError."""
    m, op = insn.mnemonic, insn.operands
    if m in ("push", "pop"):
        return [f"// {m} {insn.op_str} (balanced -> register preserved)"]
    if m in ("ret", "retf", "nop"):
        return [f"// {m}"]
    if m == "clc": return ["m.regs.cf = false;"]
    if m == "stc": return ["m.regs.cf = true;"]
    if m == "cld": return ["m.regs.df = false;"]
    if m == "std": return ["m.regs.df = true;"]
    if m == "cbw": return ["let __s = m.regs.al() as i8 as i16 as u16; m.regs.set_ax(__s);"]
    # generic 2-operand read of src -> expr; dst write
    def opval(o):
        if o.type == capstone.x86.X86_OP_REG:
            return rd(insn.reg_name(o.reg)), o.size
        if o.type == capstone.x86.X86_OP_IMM:
            return f"0x{o.imm & 0xffffffff:x}", o.size
        if o.type == capstone.x86.X86_OP_MEM:
            seg, off, sz = mem_addr(insn, o)
            rdfn = {1: "read8", 2: "read16", 4: "read32"}[sz]
            return f"m.{rdfn}({seg}, {off})", sz
        raise NotImplementedError("operand type")
    def opset(o, expr):
        if o.type == capstone.x86.X86_OP_REG:
            return [wr(insn.reg_name(o.reg), expr)]
        if o.type == capstone.x86.X86_OP_MEM:
            seg, off, sz = mem_addr(insn, o)
            wrfn = {1: "write8", 2: "write16", 4: "write32"}[sz]
            return [f"m.{wrfn}({seg}, {off}, {expr});"]
        raise NotImplementedError("dst")
    if m == "mov":
        src, _ = opval(op[1])
        castsz = {1: "u8", 2: "u16", 4: "u32"}[op[0].size]
        return opset(op[0], f"({src}) as {castsz}")
    if m in ("add", "xor", "sub", "and", "or"):
        a, _ = opval(op[0]); b, _ = opval(op[1])
        helper = {"add": "add16", "xor": "xor16", "sub": "sub16", "and": "and16", "or": "or16"}[m]
        if op[0].size != 2:
            raise NotImplementedError(f"{m} size {op[0].size}")
        return [f"let __r = m.regs.{helper}(({a}) as u16, ({b}) as u16);"] + opset(op[0], "__r")
    if m in ("cmp", "test"):
        a, _ = opval(op[0]); b, _ = opval(op[1])
        if op[0].size == 1:
            return [f"m.regs.{('cmp8' if m=='cmp' else 'test8')}(({a}) as u8, ({b}) as u8);"]
        raise NotImplementedError(f"{m} size {op[0].size}")
    raise NotImplementedError(f"opcode {m} ({insn.op_str})")

def lift(off, name):
    lines = [f"pub fn {name}(m: &mut Machine) {{"]
    for insn in MD.disasm(D[off:off + 200], off):
        lines.append(f"    // 0x{insn.address:05x}: {insn.mnemonic} {insn.op_str}")
        try:
            for l in emit(insn):
                lines.append("    " + l)
        except NotImplementedError as e:
            lines.append(f"    // TODO(lifter): {e}")
        if insn.mnemonic in ("ret", "retf"):
            break
    lines.append("}")
    return "\n".join(lines)

if __name__ == "__main__":
    print(lift(int(sys.argv[1], 16), sys.argv[2]))
