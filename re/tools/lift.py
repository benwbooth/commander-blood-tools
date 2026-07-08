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
    if m in ("ret", "retf", "nop"):
        return [f"// {m}"]
    if m == "clc": return ["m.regs.cf = false;"]
    if m == "stc": return ["m.regs.cf = true;"]
    if m == "cld": return ["m.regs.df = false;"]
    if m == "std": return ["m.regs.df = true;"]
    if m == "cbw": return ["let __s = m.regs.al() as i8 as i16 as u16; m.regs.set_ax(__s);"]
    if m in ("cli", "sti"): return [f"// {m} (IF not modelled)"]
    if m in ("shl", "sal", "shr"):
        a, _ = opval(op[0]); cnt, _ = opval(op[1])
        if op[0].size == 2:
            h = "shl16" if m in ("shl", "sal") else "shr16"
            return [f"let __r = m.regs.{h}(({a}) as u16, ({cnt}) as u8);"] + opset(op[0], "__r")
        raise NotImplementedError(f"{m} size {op[0].size}")
    if m in ("inc", "dec"):
        a, _ = opval(op[0])
        if op[0].size == 2:
            h = "inc16" if m == "inc" else "dec16"
            return [f"let __r = m.regs.{h}(({a}) as u16);"] + opset(op[0], "__r")
        raise NotImplementedError(f"{m} size {op[0].size}")
    if m in ("les", "lds", "lfs", "lgs", "lss"):
        dst = insn.reg_name(op[0].reg)
        seg, off, _ = mem_addr(insn, op[1])
        segreg = {"les": "es", "lds": "ds", "lfs": "fs", "lgs": "gs", "lss": "ss"}[m]
        return [f"let __o = m.read16({seg}, {off});",
                f"let __s = m.read16({seg}, ({off}).wrapping_add(2));",
                wr(dst, "__o"), f"m.regs.{segreg} = __s;"]
    if m == "xchg":
        if op[0].type == capstone.x86.X86_OP_REG and op[1].type == capstone.x86.X86_OP_REG:
            a = insn.reg_name(op[0].reg); b = insn.reg_name(op[1].reg)
            av, _ = opval(op[0]); bv, _ = opval(op[1])
            return [f"let __t = {av};"] + opset(op[0], bv) + opset(op[1], "__t".replace("__t", f"({{}})".format("__t")))
        raise NotImplementedError("xchg mem")
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
    if m == "push":
        o = op[0]
        if o.size == 2:
            v, _ = opval(o)
            return ["m.regs.set_sp(m.regs.sp().wrapping_sub(2));",
                    f"m.write16(m.regs.ss, m.regs.sp(), ({v}) as u16);"]
        raise NotImplementedError(f"push size {o.size}")
    if m == "pop":
        o = op[0]
        if o.size == 2:
            return ["let __v = m.read16(m.regs.ss, m.regs.sp());",
                    "m.regs.set_sp(m.regs.sp().wrapping_add(2));"] + opset(o, "(__v) as u16")
        raise NotImplementedError(f"pop size {o.size}")
    if m == "mov":
        src, _ = opval(op[1])
        castsz = {1: "u8", 2: "u16", 4: "u32"}[op[0].size]
        return opset(op[0], f"({src}) as {castsz}")
    if m in ("add", "xor", "sub", "and", "or"):
        a, _ = opval(op[0]); b, _ = opval(op[1]); sz = op[0].size
        if sz == 2:
            h = {"add": "add16", "xor": "xor16", "sub": "sub16", "and": "and16", "or": "or16"}[m]
            return [f"let __r = m.regs.{h}(({a}) as u16, ({b}) as u16);"] + opset(op[0], "__r")
        if sz == 1:
            h = {"add": "add8", "xor": "xor8", "sub": "sub8", "and": "and8", "or": "or8"}[m]
            return [f"let __r = m.regs.{h}(({a}) as u8, ({b}) as u8);"] + opset(op[0], "__r")
        raise NotImplementedError(f"{m} size {sz}")
    if m in ("cmp", "test"):
        a, _ = opval(op[0]); b, _ = opval(op[1]); sz = op[0].size
        suf = {1: "8", 2: "16"}.get(sz)
        if suf:
            cast = {1: "u8", 2: "u16"}[sz]
            return [f"m.regs.{m}{suf}(({a}) as {cast}, ({b}) as {cast});"]
        raise NotImplementedError(f"{m} size {sz}")
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



# --- control-flow (branch/loop) lifting: basic blocks -> `loop { match blk { ... } }` ---
JCC = {
    "je": "m.regs.zf", "jz": "m.regs.zf", "jne": "!m.regs.zf", "jnz": "!m.regs.zf",
    "jb": "m.regs.cf", "jc": "m.regs.cf", "jnae": "m.regs.cf",
    "jae": "!m.regs.cf", "jnb": "!m.regs.cf", "jnc": "!m.regs.cf",
    "jbe": "(m.regs.cf || m.regs.zf)", "jna": "(m.regs.cf || m.regs.zf)",
    "ja": "(!m.regs.cf && !m.regs.zf)", "jnbe": "(!m.regs.cf && !m.regs.zf)",
    "jl": "(m.regs.sf != m.regs.of)", "jnge": "(m.regs.sf != m.regs.of)",
    "jge": "(m.regs.sf == m.regs.of)", "jnl": "(m.regs.sf == m.regs.of)",
    "jle": "(m.regs.zf || (m.regs.sf != m.regs.of))", "jng": "(m.regs.zf || (m.regs.sf != m.regs.of))",
    "jg": "(!m.regs.zf && (m.regs.sf == m.regs.of))", "jnle": "(!m.regs.zf && (m.regs.sf == m.regs.of))",
    "js": "m.regs.sf", "jns": "!m.regs.sf", "jo": "m.regs.of", "jno": "!m.regs.of",
    "jp": "m.regs.pf", "jpe": "m.regs.pf", "jnp": "!m.regs.pf", "jpo": "!m.regs.pf",
}

def lift_cfg(off, name):
    # 1) linear-sweep collect instructions until we've covered all reachable blocks
    insns = {}
    leaders = {off}
    worklist = [off]
    end = off + 400
    while worklist:
        a = worklist.pop()
        if a in insns or not (0x600 <= a < 0xd000):
            continue
        for i in MD.disasm(D[a:end], a):
            if i.address in insns:
                break
            insns[i.address] = i
            mn = i.mnemonic
            if mn in ("ret", "retf", "iret"):
                break
            if mn == "jmp" and i.op_str.startswith("0x"):
                t = int(i.op_str, 16); leaders.add(t); worklist.append(t); break
            if mn == "jmp":
                break  # indirect jmp - stop (handled as TODO)
            if (mn in JCC or mn == "loop") and i.op_str.startswith("0x"):
                t = int(i.op_str, 16); leaders.add(t); worklist.append(t)
                leaders.add(i.address + i.size)  # fallthrough is a leader
            # fallthrough continues
    addrs = sorted(insns)
    # 2) split into blocks at leaders
    blocks = {}  # leader -> [insns]
    cur = None
    for a in addrs:
        if a in leaders:
            cur = a; blocks[cur] = []
        blocks[cur].append(insns[a])
    # next leader after a block (fallthrough target)
    sorted_leaders = sorted(blocks)
    nxt = {sorted_leaders[k]: (sorted_leaders[k+1] if k+1 < len(sorted_leaders) else None)
           for k in range(len(sorted_leaders))}
    # 3) emit
    out = [f"pub fn {name}(m: &mut Machine) {{", f"    let mut __blk: u32 = 0x{off:x};",
           "    loop {", "        match __blk {"]
    for lead in sorted_leaders:
        out.append(f"            0x{lead:x} => {{")
        blk = blocks[lead]
        terminated = False
        for i in blk[:-1]:
            out.append(f"                // 0x{i.address:05x}: {i.mnemonic} {i.op_str}")
            try:
                for l in emit(i):
                    out.append("                " + l)
            except NotImplementedError as e:
                out.append(f"                return; // TODO(lifter): {e}"); terminated = True; break
        if terminated:
            out.append("            }"); continue
        last = blk[-1]
        out.append(f"                // 0x{last.address:05x}: {last.mnemonic} {last.op_str}")
        mn = last.mnemonic
        if mn in ("ret", "retf", "iret"):
            out.append("                return;")
        elif mn == "jmp" and last.op_str.startswith("0x"):
            out.append(f"                __blk = 0x{int(last.op_str,16):x};")
        elif mn in JCC and last.op_str.startswith("0x"):
            t = int(last.op_str, 16); fall = nxt[lead]
            out.append(f"                if {JCC[mn]} {{ __blk = 0x{t:x}; }} else {{ __blk = 0x{fall:x}; }}")
        elif mn == "loop" and last.op_str.startswith("0x"):
            t = int(last.op_str, 16); fall = nxt[lead]
            out.append("                let __c = m.regs.cx().wrapping_sub(1); m.regs.set_cx(__c);")
            out.append(f"                if __c != 0 {{ __blk = 0x{t:x}; }} else {{ __blk = 0x{fall:x}; }}")
        else:
            # non-terminator last insn (block ends because next addr is a leader): lift + fallthrough
            try:
                for l in emit(last):
                    out.append("                " + l)
                out.append(f"                __blk = 0x{nxt[lead]:x};")
            except NotImplementedError as e:
                out.append(f"                return; // TODO(lifter): {e}")
        out.append("            }")
    out.append("            _ => unreachable!(),")
    out += ["        }", "    }", "}"]
    return "\n".join(out)


if __name__ == "__main__":
    fn = lift_cfg if (len(sys.argv) > 3 and sys.argv[3] == "cfg") else lift
    print(fn(int(sys.argv[1], 16), sys.argv[2]))