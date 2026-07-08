#!/usr/bin/env python3
"""Automated instruction lifter for path B: translate a LINEAR (no internal branch) DOS function
to a Rust `fn(m: &mut Machine)` operating on the Machine, using the verified flag helpers. The
oracle then checks the emitted lift bit-exact, so any translation error is caught. Grows opcode
coverage incrementally; control-flow (branches/loops) is a later extension.

Usage (PYTHONSAFEPATH=1): re/tools/lift.py <file_offset_hex> <rust_fn_name>"""
import capstone, sys

# Offsets of already-lifted functions that a `call` may compose against. Only NEAR-ret callees
# belong here (a retf callee executing mid-run trips the Unicorn read-hook bug in the oracle).
# Populated by scan_clean/gen_batch before lifting non-leaf functions.
AVAILABLE = set()

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
    # An explicit segment-override prefix ALWAYS wins over the default (including the
    # bp/ebp -> SS default below), so resolve it first and remember whether one was present.
    override = None
    if insn.prefix and insn.prefix[1]:
        override = {0x2e: "cs", 0x36: "ss", 0x3e: "ds", 0x26: "es", 0x64: "fs", 0x65: "gs"}.get(insn.prefix[1])
    seg = f"m.regs.{override}" if override else "m.regs.ds"
    # Address size: 32-bit if any base/index register is 32-bit (e.g. eax/edi from a 0x67
    # prefix), else 16-bit. 16-bit addressing wraps the offset at 64K; 32-bit does NOT (the
    # full effective address is added to seg*16). The final off expr is always typed u32.
    def is32(reg):
        return reg.startswith("e") and reg not in ("es",)
    regs = []
    if m.base != 0:
        b = insn.reg_name(m.base)
        if b in ("bp", "ebp") and not override: seg = "m.regs.ss"  # bp/ebp default to SS (unless overridden)
        regs.append(b)
    if m.index != 0:
        regs.append(insn.reg_name(m.index))
    wide = any(is32(r) for r in regs)
    if wide:
        parts = [f"({rd(r)} as u32)" for r in regs]
        if m.disp != 0 or not parts:
            parts.append(f"0x{m.disp & 0xffffffff:x}u32")
        off = parts[0]
        for p in parts[1:]:
            off = f"{off}.wrapping_add({p})"
    else:
        parts = [rd(r) for r in regs]  # 16-bit reg reads -> u16
        if m.disp != 0 or not parts:
            parts.append(f"0x{m.disp & 0xffff:x}u16")
        if len(parts) == 1 and not regs:
            off = f"0x{m.disp & 0xffff:x}u32"  # bare displacement: emit u32 directly
        else:
            o16 = parts[0]
            for p in parts[1:]:
                o16 = f"{o16}.wrapping_add({p})"
            off = f"(({o16}) as u32)"  # wrap at 16 bits, then widen
    size = opnd.size
    return seg, off, size

def emit(insn):
    """Emit Rust lines for one linear instruction. Returns list[str] or raises NotImplementedError."""
    m, op = insn.mnemonic, insn.operands
    # Operand read/write helpers, defined first so every handler below can use them
    # (Python treats these as function-locals; referencing them before assignment
    # would raise UnboundLocalError).
    def opval(o):
        if o.type == capstone.x86.X86_OP_REG:
            return rd(insn.reg_name(o.reg)), o.size
        if o.type == capstone.x86.X86_OP_IMM:
            # mask the (possibly sign-extended) immediate to the operand size so the emitted
            # literal fits the Rust cast target (e.g. `cmp word,-1` -> 0xffff, not 0xffffffff)
            mask = (1 << (o.size * 8)) - 1
            return f"0x{o.imm & mask:x}", o.size
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
    if m in ("ret", "retf", "nop"):
        return [f"// {m}"]
    if m == "clc": return ["m.regs.cf = false;"]
    if m == "stc": return ["m.regs.cf = true;"]
    if m == "cld": return ["m.regs.df = false;"]
    if m == "std": return ["m.regs.df = true;"]
    if m == "cbw": return ["let __s = m.regs.al() as i8 as i16 as u16; m.regs.set_ax(__s);"]
    if m == "cwde":
        # capstone (CS_MODE_16) mislabels the bare single-byte 0x98 as CWDE; without a 0x66
        # operand-size prefix it is actually CBW (AL -> AX, sign-extended). Only 0x66 0x98 is
        # a true CWDE (AX -> EAX). The oracle caught this on func_6023 too.
        if 0x66 in insn.bytes[:-1]:
            return ["let __s = m.regs.ax() as i16 as i32 as u32; m.regs.eax = __s;"]  # CWDE
        return ["let __s = m.regs.al() as i8 as i16 as u16; m.regs.set_ax(__s);"]      # CBW
    if m == "leave":
        # SP = BP; BP = pop16
        return ["m.regs.set_sp(m.regs.bp());",
                "let __v = m.read16(m.regs.ss, m.regs.sp() as u32);",
                "m.regs.set_sp(m.regs.sp().wrapping_add(2));",
                "m.regs.set_bp(__v);"]
    if m in ("cli", "sti"): return [f"// {m} (IF not modelled)"]
    if m in ("shl", "sal", "shr", "sar"):
        a, _ = opval(op[0]); cnt, _ = opval(op[1])
        base = "shl" if m in ("shl", "sal") else ("sar" if m == "sar" else "shr")
        pfx = {1: f"{base}8", 2: f"{base}16", 4: f"{base}32"}
        cast = {1: "u8", 2: "u16", 4: "u32"}[op[0].size]
        h = pfx.get(op[0].size)
        if h:
            return [f"let __r = m.regs.{h}(({a}) as {cast}, ({cnt}) as u8);"] + opset(op[0], "__r")
        raise NotImplementedError(f"{m} size {op[0].size}")
    if m in ("inc", "dec"):
        a, _ = opval(op[0])
        pfx = {1: "inc8", 2: "inc16", 4: "inc32"} if m == "inc" else {1: "dec8", 2: "dec16", 4: "dec32"}
        cast = {1: "u8", 2: "u16", 4: "u32"}[op[0].size]
        h = pfx.get(op[0].size)
        if h:
            return [f"let __r = m.regs.{h}(({a}) as {cast});"] + opset(op[0], "__r")
        raise NotImplementedError(f"{m} size {op[0].size}")
    if m.startswith("rep ") or m in ("lodsb", "lodsw", "lodsd", "stosb", "stosw", "stosd",
                                      "movsb", "movsw", "movsd", "scasb", "scasw"):
        rep = m.startswith("rep ")
        base = m[4:] if rep else m
        sz = {"b": 1, "w": 2, "d": 4}[base[-1]]
        rdfn = {1: "read8", 2: "read16", 4: "read32"}[sz]
        wrfn = {1: "write8", 2: "write16", 4: "write32"}[sz]
        rd_acc = {1: "m.regs.al()", 2: "m.regs.ax()", 4: "m.regs.eax"}[sz]
        def wr_acc(v):
            return {1: f"m.regs.set_al({v});", 2: f"m.regs.set_ax({v});", 4: f"m.regs.eax = {v};"}[sz]
        step = f"({sz} as u16)"
        stepd = f"(if m.regs.df {{ {step}.wrapping_neg() }} else {{ {step} }})"
        # lods/movs SOURCE (DS:si) honors a segment-override prefix (e.g. `lodsw es:[si]`);
        # stos/movs DEST is always ES:di (not overridable).
        src_seg = "m.regs.ds"
        if insn.prefix and insn.prefix[1]:
            ov = {0x2e: "cs", 0x36: "ss", 0x3e: "ds", 0x26: "es", 0x64: "fs", 0x65: "gs"}.get(insn.prefix[1])
            if ov:
                src_seg = f"m.regs.{ov}"
        body = []
        if base.startswith("lods"):
            body += [f"let __v = m.{rdfn}({src_seg}, m.regs.si() as u32);",
                     wr_acc("__v"),
                     f"m.regs.set_si(m.regs.si().wrapping_add({stepd}));"]
        elif base.startswith("stos"):
            body += [f"m.{wrfn}(m.regs.es, m.regs.di() as u32, {rd_acc});",
                     f"m.regs.set_di(m.regs.di().wrapping_add({stepd}));"]
        elif base.startswith("movs"):
            body += [f"let __v = m.{rdfn}({src_seg}, m.regs.si() as u32);",
                     f"m.{wrfn}(m.regs.es, m.regs.di() as u32, __v);",
                     f"m.regs.set_si(m.regs.si().wrapping_add({stepd}));",
                     f"m.regs.set_di(m.regs.di().wrapping_add({stepd}));"]
        else:
            raise NotImplementedError(f"string op {m}")
        if rep:
            return ["let __sd = " + stepd + ";",
                    "while m.regs.cx() != 0 {"] + \
                   ["    " + b.replace(stepd, "__sd") for b in body] + \
                   ["    m.regs.set_cx(m.regs.cx().wrapping_sub(1));", "}"]
        return body
    if m == "neg":
        a, _ = opval(op[0])
        pfx = {1: "neg8", 2: "neg16", 4: "neg32"}
        cast = {1: "u8", 2: "u16", 4: "u32"}[op[0].size]
        h = pfx.get(op[0].size)
        if h:
            return [f"let __r = m.regs.{h}(({a}) as {cast});"] + opset(op[0], "__r")
        raise NotImplementedError(f"neg size {op[0].size}")
    if m == "not":
        a, _ = opval(op[0])
        castsz = {1: "u8", 2: "u16", 4: "u32"}[op[0].size]
        return opset(op[0], f"!(({a}) as {castsz})")
    if m == "mul":
        src, _ = opval(op[0])
        pfx = {1: "mul8", 2: "mul16"}
        cast = {1: "u8", 2: "u16"}.get(op[0].size)
        h = pfx.get(op[0].size)
        if h:
            return [f"m.regs.{h}(({src}) as {cast});"]
        raise NotImplementedError(f"mul size {op[0].size}")
    if m == "adc":
        a, _ = opval(op[0]); b, _ = opval(op[1]); sz = op[0].size
        pfx = {1: "adc8", 2: "adc16"}
        cast = {1: "u8", 2: "u16"}.get(sz)
        h = pfx.get(sz)
        if h:
            return [f"let __r = m.regs.{h}(({a}) as {cast}, ({b}) as {cast});"] + opset(op[0], "__r")
        raise NotImplementedError(f"adc size {sz}")
    if m in ("movsx", "movzx"):
        src, _ = opval(op[1])
        dsz = op[0].size; ssz = op[1].size
        scast = {1: "u8", 2: "u16"}[ssz]
        dcast = {2: "u16", 4: "u32"}[dsz]
        if m == "movsx":
            sicast = {1: "i8", 2: "i16"}[ssz]
            dicast = {2: "i16", 4: "i32"}[dsz]
            expr = f"(({src}) as {scast}) as {sicast} as {dicast} as {dcast}"
        else:
            expr = f"(({src}) as {scast}) as {dcast}"
        return opset(op[0], expr)
    if m.startswith("set"):
        # setcc r/m8 : write 1 if condition holds else 0. Reuse the JCC flag table.
        cond = "j" + m[3:]
        if cond in JCC:
            return opset(op[0], f"(if {JCC[cond]} {{ 1u8 }} else {{ 0u8 }})")
        raise NotImplementedError(f"opcode {m}")
    if m == "sbb":
        a, _ = opval(op[0]); b, _ = opval(op[1]); sz = op[0].size
        pfx = {1: "sbb8", 2: "sbb16"}
        cast = {1: "u8", 2: "u16"}.get(sz)
        h = pfx.get(sz)
        if h:
            return [f"let __r = m.regs.{h}(({a}) as {cast}, ({b}) as {cast});"] + opset(op[0], "__r")
        raise NotImplementedError(f"sbb size {sz}")
    if m in ("rol", "ror", "rcl", "rcr"):
        a, _ = opval(op[0]); cnt, _ = opval(op[1]); sz = op[0].size
        cast = {1: "u8", 2: "u16", 4: "u32"}.get(sz)
        # only rol/ror have 32-bit helpers; rcl/rcr stay 8/16-bit
        avail = {1: f"{m}8", 2: f"{m}16"}
        if m in ("rol", "ror"):
            avail[4] = f"{m}32"
        h = avail.get(sz)
        if h:
            return [f"let __r = m.regs.{h}(({a}) as {cast}, ({cnt}) as u8);"] + opset(op[0], "__r")
        raise NotImplementedError(f"{m} size {sz}")
    if m == "btr":
        a, _ = opval(op[0]); b, _ = opval(op[1])
        if op[0].size == 2:
            return [f"let __r = m.regs.btr16(({a}) as u16, ({b}) as u8);"] + opset(op[0], "__r")
        raise NotImplementedError(f"btr size {op[0].size}")
    if m in ("div", "idiv"):
        src, _ = opval(op[0]); sz = op[0].size
        cast = {1: "u8", 2: "u16"}.get(sz)
        h = {("div", 1): "div8", ("div", 2): "div16", ("idiv", 1): "idiv8", ("idiv", 2): "idiv16"}.get((m, sz))
        if h:
            return [f"m.regs.{h}(({src}) as {cast});"]
        raise NotImplementedError(f"{m} size {sz}")
    if m == "imul":
        sz = op[0].size
        if len(op) == 1:  # one-operand: AX/DX:AX = acc * src
            src, _ = opval(op[0]); cast = {1: "u8", 2: "u16"}.get(sz)
            h = {1: "imul8_1", 2: "imul16_1"}.get(sz)
            if h:
                return [f"m.regs.{h}(({src}) as {cast});"]
            raise NotImplementedError(f"imul1 size {sz}")
        if sz in (2, 4):  # two/three-operand, 16- or 32-bit dst
            if len(op) == 2:
                a, _ = opval(op[0]); b, _ = opval(op[1])
            else:  # three-operand: dst = src * imm
                a, _ = opval(op[1]); b, _ = opval(op[2])
            cast = {2: "u16", 4: "u32"}[sz]
            h = {2: "imul16_2", 4: "imul32_2"}[sz]
            return [f"let __r = m.regs.{h}(({a}) as {cast}, ({b}) as {cast});"] + opset(op[0], "__r")
        raise NotImplementedError(f"imul{len(op)} size {sz}")
    if m == "bsf":
        src, _ = opval(op[1]); cur, _ = opval(op[0])
        return [f"let __r = m.regs.bsf16(({src}) as u16, ({cur}) as u16);"] + opset(op[0], "__r")
    if m == "pushf" or m == "pushfd":
        # push the flags word (only the modelled bits; bit1 always set on 8086)
        return ["let __f: u16 = 0x0002",
                "    | (m.regs.cf as u16)",
                "    | ((m.regs.pf as u16) << 2)",
                "    | ((m.regs.af as u16) << 4)",
                "    | ((m.regs.zf as u16) << 6)",
                "    | ((m.regs.sf as u16) << 7)",
                "    | ((m.regs.df as u16) << 10)",
                "    | ((m.regs.of as u16) << 11);",
                "m.regs.set_sp(m.regs.sp().wrapping_sub(2));",
                "m.write16(m.regs.ss, m.regs.sp() as u32, __f);"]
    if m == "popf" or m == "popfd":
        return ["let __f = m.read16(m.regs.ss, m.regs.sp() as u32);",
                "m.regs.set_sp(m.regs.sp().wrapping_add(2));",
                "m.regs.cf = __f & 1 != 0; m.regs.pf = __f & 4 != 0; m.regs.af = __f & 0x10 != 0;",
                "m.regs.zf = __f & 0x40 != 0; m.regs.sf = __f & 0x80 != 0;",
                "m.regs.df = __f & 0x400 != 0; m.regs.of = __f & 0x800 != 0;"]
    if m == "call":
        o = op[0]
        if o.type == capstone.x86.X86_OP_IMM:
            t = o.imm & 0xffffffff
            if t in AVAILABLE:
                ret_ip = insn.address + insn.size
                # Model the near CALL exactly: push the return offset (the callee sees the
                # correct SP and its transient stack writes land where the oracle records
                # them), invoke the lifted callee, then pop (the callee's near RET, modelled
                # as a Rust `return`, would have popped it).
                return [f"m.regs.set_sp(m.regs.sp().wrapping_sub(2));",
                        f"m.write16(m.regs.ss, m.regs.sp() as u32, 0x{ret_ip:x});",
                        f"func_{t:x}(m);",
                        "m.regs.set_sp(m.regs.sp().wrapping_add(2));"]
            raise NotImplementedError(f"call 0x{t:x} (callee not available)")
        raise NotImplementedError("indirect call")
    if m == "xlatb":
        # AL = [DS:BX + AL]  (table lookup translate; 16-bit addressing wraps at 64K)
        return ["let __a = m.read8(m.regs.ds, m.regs.bx().wrapping_add(m.regs.al() as u16) as u32);",
                "m.regs.set_al(__a);"]
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
    if m == "push":
        o = op[0]
        if o.size == 2:
            v, _ = opval(o)
            return ["m.regs.set_sp(m.regs.sp().wrapping_sub(2));",
                    f"m.write16(m.regs.ss, m.regs.sp() as u32, ({v}) as u16);"]
        if o.size == 4:
            v, _ = opval(o)
            return ["m.regs.set_sp(m.regs.sp().wrapping_sub(4));",
                    f"m.write32(m.regs.ss, m.regs.sp() as u32, ({v}) as u32);"]
        raise NotImplementedError(f"push size {o.size}")
    if m == "pop":
        o = op[0]
        if o.size == 2:
            return ["let __v = m.read16(m.regs.ss, m.regs.sp() as u32);",
                    "m.regs.set_sp(m.regs.sp().wrapping_add(2));"] + opset(o, "(__v) as u16")
        if o.size == 4:
            return ["let __v = m.read32(m.regs.ss, m.regs.sp() as u32);",
                    "m.regs.set_sp(m.regs.sp().wrapping_add(4));"] + opset(o, "(__v) as u32")
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
        if sz == 4:
            h = {"add": "add32", "xor": "xor32", "sub": "sub32", "and": "and32", "or": "or32"}[m]
            return [f"let __r = m.regs.{h}(({a}) as u32, ({b}) as u32);"] + opset(op[0], "__r")
        raise NotImplementedError(f"{m} size {sz}")
    if m in ("cmp", "test"):
        a, _ = opval(op[0]); b, _ = opval(op[1]); sz = op[0].size
        suf = {1: "8", 2: "16", 4: "32"}.get(sz)
        if suf:
            cast = {1: "u8", 2: "u16", 4: "u32"}[sz]
            if m == "test" and sz == 4:
                # no test32 helper; AND-discard via and32 (identical flags)
                return [f"m.regs.and32(({a}) as u32, ({b}) as u32);"]
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
            if (mn in JCC or mn in ("loop", "loope", "loopz", "loopne", "loopnz", "jcxz", "jecxz")) \
                    and i.op_str.startswith("0x"):
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
           "    let mut __guard: u32 = 0;",
           "    loop {",
           "        __guard += 1;",
           f'        if __guard > 5_000_000 {{ panic!("{name}: iteration guard tripped (non-terminating input)"); }}',
           "        match __blk {"]
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
        elif mn in ("loope", "loopz", "loopne", "loopnz") and last.op_str.startswith("0x"):
            t = int(last.op_str, 16); fall = nxt[lead]
            zc = "m.regs.zf" if mn in ("loope", "loopz") else "!m.regs.zf"
            out.append("                let __c = m.regs.cx().wrapping_sub(1); m.regs.set_cx(__c);")
            out.append(f"                if __c != 0 && {zc} {{ __blk = 0x{t:x}; }} else {{ __blk = 0x{fall:x}; }}")
        elif mn in ("jcxz", "jecxz") and last.op_str.startswith("0x"):
            t = int(last.op_str, 16); fall = nxt[lead]
            reg = "m.regs.cx()" if mn == "jcxz" else "m.regs.ecx"
            out.append(f"                if {reg} == 0 {{ __blk = 0x{t:x}; }} else {{ __blk = 0x{fall:x}; }}")
        else:
            # non-terminator last insn (block ends because next addr is a leader): lift + fallthrough
            try:
                for l in emit(last):
                    out.append("                " + l)
                if nxt[lead] is None:
                    out.append("                return; // fell off end (no successor)")
                else:
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