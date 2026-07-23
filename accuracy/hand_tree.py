#!/usr/bin/env python3
"""Verify the full manu3 skeleton composition against the live dump:
rows_i = rows_parent(i) . build(angles_i)^T (Q15), parent from record field +0,
root chain at node 0x2274. Also solve the translation rule: T_i = T_parent + rows_parent . L_i
and locate where each node's local offset L_i lives in the record."""
import struct
import sys

import numpy as np

ds = open("accuracy/manu3/manu3_ds.bin", "rb").read()
trig = open("accuracy/manu3/trig_tables.bin", "rb").read()


def tcos(a):
    idx = (a // 4) % 1024
    return struct.unpack_from("<h", trig, idx * 4)[0]


def tsin(a):
    idx = (a // 4) % 1024
    return struct.unpack_from("<h", trig, idx * 4 + 2)[0]


def build(a1, a2, a3):
    """Mirror of the Rust build_matrix (0x270 literal transcription)."""
    c1, s1 = tcos(a1), tsin(a1)
    c2, s2 = tcos(a2), tsin(a2)
    c3, s3 = tcos(a3), tsin(a3)
    # From src/manu3_hand.rs build_matrix — recompute here in the same order.
    m = [0] * 9
    m[0] = (c2 * c3) >> 14
    m[1] = (-c2 * s3) >> 14
    m[2] = (s2 << 14) >> 13
    t1 = (s1 * s2) >> 14
    m[3] = (c1 * s3 + ((t1 * c3) >> 14) * 1) >> 14 if False else ((c1 * s3) + (t1 * c3)) >> 14
    m[4] = ((c1 * c3) - (t1 * s3)) >> 14
    m[5] = (-s1 * c2) >> 14
    t2 = (c1 * s2) >> 14
    m[6] = ((s1 * s3) - (t2 * c3)) >> 14
    m[7] = ((s1 * c3) + (t2 * s3)) >> 14
    m[8] = (c1 * c2) >> 14
    return np.array(m, dtype=np.int64).reshape(3, 3)


def rows(at):
    return np.array(
        struct.unpack_from("<9i", ds, at + 0x12), dtype=np.int64
    ).reshape(3, 3)


def tvec(at):
    return np.array(struct.unpack_from("<3i", ds, at + 0x36), dtype=np.int64)


def angles(at):
    return struct.unpack_from("<3h", ds, at + 0x4E)


def parent(at):
    return struct.unpack_from("<H", ds, at)[0]


def q15(m):
    return (m @ np.eye(3, dtype=np.int64))


def mulq(a, b):
    return (a @ b) >> 15


nodes = [0x2274] + [0x2394 + i * 0x5E for i in range(16)]
print("node   parent   angles                 rows==parent*buildT?  maxerr")
for at in nodes:
    p = parent(at)
    a = angles(at)
    s = rows(at)
    if p in nodes and at != 0x2274:
        sp = rows(p).astype(np.float64) / 32768.0
        sf = s.astype(np.float64) / 32768.0
        loc = sp.T @ sf
        ortho = np.abs(loc @ loc.T - np.eye(3)).max()
        ident = np.abs(loc - np.eye(3)).max()
        print(f"0x{at:04X} 0x{p:04X}  {str(a):22} local ortho-err={ortho:.4f}"
              + (f"  IDENTITY(err={ident:.4f})" if a == (0,0,0) else ""))
    else:
        sf = rows(at).astype(np.float64) / 32768.0
        ortho = np.abs(sf @ sf.T - np.eye(3)).max()
        print(f"0x{at:04X} 0x{p:04X}  {str(a):22} (root; own rows ortho-err={ortho:.4f})")

print("\nTranslation rule: solve L = parent_rows^-1 (T_child - T_parent) (Q15)")
for at in nodes[1:]:
    p = parent(at)
    if p not in nodes:
        print(f"0x{at:04X}: parent 0x{p:04X} outside")
        continue
    dp = tvec(at) - tvec(p)
    sp = rows(p).astype(np.float64) / 32768.0
    L = np.linalg.inv(sp) @ dp.astype(np.float64)
    print(f"0x{at:04X}: T-T_p={dp}  L~{np.round(L).astype(int)}")

print("\nnode 0x2274 record bytes:")
print(" ".join(f"{b:02x}" for b in ds[0x2274 : 0x2274 + 0x5E]))
print("\nrecord tail fields (+0x40..+0x5D) per node (candidate local-L storage):")
for at in nodes:
    tail = struct.unpack_from("<15h", ds, at + 0x40)
    print(f"0x{at:04X}: {tail}")
