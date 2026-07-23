#!/usr/bin/env python3
"""Verify the fully-decoded 0x270 local matrix (product-to-sum builder) against every
skeleton node in the live dump: stored_i ?= parent_rows . local(angles_i) (Q15), and
T_i ?= parent_rows . L_i + T_parent with L from +0x42/+0x46/+0x4A."""
import struct

import numpy as np

ds = open("accuracy/manu3/manu3_ds.bin", "rb").read()
trig = open("accuracy/manu3/trig_tables.bin", "rb").read()


def tc(a):
    return struct.unpack_from("<h", trig, ((a & 0xFFC) % 4096))[0]


def ts(a):
    return struct.unpack_from("<h", trig, ((a & 0xFFC) % 4096) + 2)[0]


def build(a1, a2, a3):
    """0x270 exact: Q14 table, cells are 2x products => Q15. Decoded formulas:
    m0=c2c3-s1s2s3  m1=c2s3+s1s2c3  m2=c1s2
    m3=-c1s3        m4=c1c3         m5=-s1
    m6=-(s2c3+s1c2s3) m7=s1c2c3-s2s3 m8=c1c2   (x2, Q14 trig)"""
    c1, s1 = tc(a1), ts(a1)
    c2, s2 = tc(a2), ts(a2)
    c3, s3 = tc(a3), ts(a3)
    q = 1 << 14

    def p2(x, y):  # Q14*Q14 -> Q14
        return (x * y) // q

    m = np.zeros((3, 3), dtype=np.int64)
    m[0, 0] = 2 * (p2(c2, c3) - p2(p2(s1, s2), s3))
    m[0, 1] = 2 * (p2(c2, s3) + p2(p2(s1, s2), c3))
    m[0, 2] = 2 * p2(c1, s2)
    m[1, 0] = -2 * p2(c1, s3)
    m[1, 1] = 2 * p2(c1, c3)
    m[1, 2] = -2 * s1
    m[2, 0] = -2 * (p2(s2, c3) + p2(p2(s1, c2), s3))
    m[2, 1] = 2 * (p2(p2(s1, c2), c3) - p2(s2, s3))
    m[2, 2] = 2 * p2(c1, c2)
    return m


def rows(at):
    return np.array(struct.unpack_from("<9i", ds, at + 0x12), dtype=np.int64).reshape(3, 3)


def tvec(at):
    return np.array(struct.unpack_from("<3i", ds, at + 0x36), dtype=np.int64)


def lpos(at):
    return np.array(struct.unpack_from("<3i", ds, at + 0x42), dtype=np.int64)


def angles(at):
    return struct.unpack_from("<3h", ds, at + 0x4E)


def parent(at):
    return struct.unpack_from("<H", ds, at)[0]


def mulq(a, b):
    return (a @ b) >> 15


nodes = [0x2274] + [0x2394 + i * 0x5E for i in range(16)]
print("node    rot-err(plain) rot-err(T)   T-err(pred vs stored)")
for at in nodes[1:]:
    p = parent(at)
    sp = rows(p)
    loc = build(*angles(at))
    e_plain = int(np.abs(mulq(sp, loc) - rows(at)).max())
    e_tr = int(np.abs(mulq(sp, loc.T) - rows(at)).max())
    tpred = sp @ lpos(at) + tvec(p)
    te = int(np.abs(tpred - tvec(at)).max())
    print(f"0x{at:04X}  {e_plain:12} {e_tr:10}   {te}  L={lpos(at)} a={angles(at)}")
