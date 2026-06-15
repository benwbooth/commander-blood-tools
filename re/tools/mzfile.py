"""Shared MZ (DOS executable) loader + labels.csv loader for the RE tools.

BLOODPRG.EXE is a plain DOS MZ image (16-bit segmented, 386 instructions via
0x66/0x67 prefixes, EMS/XMS for large memory -- NOT a flat 32-bit DOS-extender).
See re/REVERSE.md for the identification evidence.

Addressing model used by all tools
-----------------------------------
* file offset      : byte offset into BLOODPRG.EXE on disk
* image offset     : byte offset into the *load module* (file minus the header)
                     i.e. image_off = file_off - header_size
* relative segment : a paragraph index into the load module. DOS adds the load
                     segment at runtime; for static analysis we use base 0, so
                     relative segment N starts at image offset N*16.
* SEG:OFF          : relative segment + offset; file_off = header_size + SEG*16 + OFF

The data segment the program runs with (DS) is loaded by the startup code with
an immediate `mov ax, <seg>` that the relocation table patches. We expose
`ds_to_file(ds_seg, off)` so DS-relative data references can be resolved.
"""

import csv
import os
import struct

HERE = os.path.dirname(os.path.abspath(__file__))
RE_ROOT = os.path.dirname(HERE)
DEFAULT_BIN = os.path.join(RE_ROOT, "bin", "BLOODPRG.EXE")
LABELS_CSV = os.path.join(RE_ROOT, "labels.csv")


class MZ:
    def __init__(self, path=DEFAULT_BIN):
        self.path = path
        with open(path, "rb") as fh:
            self.data = fh.read()
        d = self.data
        if d[:2] not in (b"MZ", b"ZM"):
            raise ValueError(f"{path}: not an MZ executable (magic {d[:2]!r})")
        (self.e_magic, self.e_cblp, self.e_cp, self.e_crlc, self.e_cparhdr,
         self.e_minalloc, self.e_maxalloc, self.e_ss, self.e_sp, self.e_csum,
         self.e_ip, self.e_cs, self.e_lfarlc, self.e_ovno) = struct.unpack_from(
            "<HHHHHHHHHHHHHH", d, 0)

        self.header_size = self.e_cparhdr * 16
        # Total bytes occupied by the load image (pages * 512, last page partial)
        if self.e_cblp == 0:
            self.image_total = self.e_cp * 512
        else:
            self.image_total = (self.e_cp - 1) * 512 + self.e_cblp
        self.load_size = self.image_total - self.header_size
        # The load module bytes (what gets mapped to segment:offset 0:0)
        self.image = d[self.header_size:self.image_total]

        # Relocation table: e_crlc entries of (offset:u16, segment:u16)
        self.relocs = []
        off = self.e_lfarlc
        for _ in range(self.e_crlc):
            ro, rs = struct.unpack_from("<HH", d, off)
            self.relocs.append((rs, ro))  # (rel_seg, off_in_seg)
            off += 4
        # Set of image offsets that hold a relocated segment word.
        self.reloc_image_offsets = {
            rs * 16 + ro for (rs, ro) in self.relocs
        }

        self.entry_file = self.header_size + self.e_cs * 16 + self.e_ip

    # -- conversions -----------------------------------------------------
    def segoff_to_file(self, seg, off):
        return self.header_size + seg * 16 + off

    def segoff_to_image(self, seg, off):
        return seg * 16 + off

    def file_to_image(self, file_off):
        return file_off - self.header_size

    def image_to_file(self, image_off):
        return image_off + self.header_size

    def ds_to_file(self, ds_seg, off):
        """Resolve a DS-relative data reference to a file offset."""
        return self.segoff_to_file(ds_seg, off)

    def summary(self):
        return {
            "file": os.path.basename(self.path),
            "file_size": len(self.data),
            "e_cblp": self.e_cblp,
            "e_cp": self.e_cp,
            "e_crlc": self.e_crlc,
            "e_cparhdr": self.e_cparhdr,
            "header_size": self.header_size,
            "image_total": self.image_total,
            "load_size": self.load_size,
            "e_ss:e_sp": f"{self.e_ss:#06x}:{self.e_sp:#06x}",
            "e_cs:e_ip": f"{self.e_cs:#06x}:{self.e_ip:#06x}",
            "e_lfarlc": self.e_lfarlc,
            "entry_file_off": self.entry_file,
            "trailing_bytes": len(self.data) - self.image_total,
        }


def load_labels(path=LABELS_CSV):
    """Return dict mapping an address key -> (name, comment).

    labels.csv rows: addr,name,comment
      addr forms:
        0xXXXXX        -> file offset
        IMG:0xXXXX     -> image offset
        SEG:OFF        -> relative segment:offset (both hex)
        DS:0xXXXX      -> DS-relative offset (resolved by caller if DS known)
    Keys are normalised to a canonical 'file:0xNNN' / 'ds:0xNNN' / 'img:0xNNN'
    string by callers as needed; here we just return the raw addr string keyed
    map plus a file-offset map for convenience when the form is a bare 0x offset.
    """
    raw = {}
    file_off = {}
    if not os.path.exists(path):
        return raw, file_off
    with open(path, newline="") as fh:
        for row in csv.reader(fh):
            if not row or row[0].strip().startswith("#"):
                continue
            addr = row[0].strip()
            name = row[1].strip() if len(row) > 1 else ""
            comment = row[2].strip() if len(row) > 2 else ""
            raw[addr] = (name, comment)
            a = addr.lower()
            if a.startswith("0x"):
                try:
                    file_off[int(a, 16)] = (name, comment)
                except ValueError:
                    pass
    return raw, file_off


if __name__ == "__main__":
    import json
    import sys
    path = sys.argv[1] if len(sys.argv) > 1 else DEFAULT_BIN
    mz = MZ(path)
    print(json.dumps(mz.summary(), indent=2))
    print(f"relocations: {len(mz.relocs)}")
    if mz.relocs:
        print("first relocs (rel_seg:off):",
              ", ".join(f"{s:#06x}:{o:#06x}" for s, o in mz.relocs[:8]))
