#!/usr/bin/env python3
"""Decode the path-template asset table in BLOODPRG.EXE.

The game does NOT enumerate its media directories. It holds a table of
fixed-stride records, each carrying a 16-byte relative-path buffer whose
filename portion is a 12-'x' placeholder patched at load time, e.g.

    'pe\\xxxxxxxxxxxx\\0'   ->  'pe\\aabob.hnm\\0'

The directory prefix ('pe\\', 'sq\\', 'ob\\', 'pl\\', 'sn\\', 'mu\\', 'fd\\')
is therefore a property of the SLOT, not of a filesystem search. Records whose
asset never varies store the literal name instead ('sq\\cryorad.hnm').

Records are VARIABLE length -- NUL-terminated path then METALEN metadata
bytes -- not a fixed stride. ('sq\\the_star.HNM' 15+1+10 = 26 bytes, but
'sq\\cryogel.hnm' 14+1+10 = 25.) Assuming a uniform 26 desynchronises the
table at the first short name and silently corrupts every later record.

Usage:  python3 re/tools/dump_asset_table.py [anchor_hex]
"""
import sys

EXE = 're/bin/BLOODPRG.EXE'
METALEN = 10
ANCHOR = 0x0F557  # first 'pe\' slot


def read_rec(data, off):
    """Parse one record at off -> (path, meta, next_off), or None."""
    end = data.find(b'\x00', off)
    if end < 0 or end - off < 4 or end - off > 24:
        return None
    name = data[off:end]
    # the prefix must be a full 2-char directory: a truncated tail like
    # '\xxxxxxxxxxxx' still contains a backslash and would let a backward
    # walk settle one or two bytes INSIDE a record, shifting the whole table.
    if len(name) < 4 or name[2:3] != b'\\' or not name[:2].isalnum():
        return None
    if not all(0x20 <= c < 0x7F for c in name):
        return None
    return name.decode('latin1'), data[end + 1:end + 1 + METALEN], end + 1 + METALEN


def rec_start_before(data, off):
    """The record whose parse lands exactly on off, if any."""
    for back in range(6, 30):
        r = read_rec(data, off - back)
        if r and r[2] == off:
            return off - back
    return None


def main():
    data = open(EXE, 'rb').read()
    anchor = int(sys.argv[1], 16) if len(sys.argv) > 1 else ANCHOR

    lo = anchor
    while True:
        prev = rec_start_before(data, lo)
        if prev is None:
            break
        lo = prev

    offs, off = [], lo
    while True:
        r = read_rec(data, off)
        if r is None:
            break
        offs.append((off, r[0], r[1]))
        off = r[2]

    print(f'table {lo:#07x}..{off:#07x}  {len(offs)} variable-length records\n')
    dirs = {}
    for i, (o, p, meta) in enumerate(offs):
        tmpl = 'TEMPLATE' if 'xxxx' in p else 'literal '
        dirs.setdefault(p.split('\\')[0], []).append(i)
        print(f'  [{i:2}] {o:#07x} {tmpl} {p:<20} meta {meta.hex(" ")}')

    print('\nslots per directory prefix:')
    for d, idxs in sorted(dirs.items()):
        print(f'  {d + chr(92):<5} {len(idxs):>3} slot(s)  indices {idxs[0]}..{idxs[-1]}')


if __name__ == '__main__':
    main()
