#!/usr/bin/env python3
from __future__ import annotations

import importlib.util
import sys
import tempfile
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location(
    "audit_code_data_ownership",
    ROOT / "re/tools/audit_code_data_ownership.py",
)
MODULE = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = MODULE
SPEC.loader.exec_module(MODULE)


class CodeDataOwnershipAuditTests(unittest.TestCase):
    def write_original(self, directory: Path, name: str, body: str) -> None:
        (directory / name).write_text(
            "; file_offset: 0x001000\n"
            "; seg_off: 008b:0100\n"
            "; group: seg_008b\n"
            + body,
            encoding="ascii",
        )

    def write_listing(self, directory: Path, name: str, body: str) -> None:
        (directory / name).write_text(body, encoding="ascii")

    def audit(self, original: Path, listings: Path) -> list[str]:
        return MODULE.audit(listings, original, original_image=None)

    def input_original(self) -> str:
        return """001000:  BB 3E 11                     mov      bx, 0x113e
001003:  2E D7                        xlatb
001005:  98                           cwde
001006:  03 C0                        add      ax, ax
001008:  8B D8                        mov      bx, ax
00100A:  2E FF 97 3E 12               call     word ptr cs:[bx + 0x123e]
00100F:  CB                           retf
"""

    def input_listing(
        self,
        translation_segment: str = "input_TEXT",
        translation_prefix: str = "cs:",
    ) -> str:
        return f"""Segment: input_TEXT BYTE USE16 00000020 bytes
0000                          L$1:
0000    10 00 12 00               DW offset L$2, offset L$3
0004                          input_action_dispatch_:
0004    0F B6 DB                  movzx bx,bl
0007    2E 8A 87 00 00            mov al,byte ptr {translation_prefix}_input_xlat[bx]
000C    01 DB                     add bx,bx
000E    2E FF A7 00 00            jmp word ptr cs:L$1[bx]
0013                          L$2:
0013    CB                        retf
0014                          L$3:
0014    CB                        retf
Segment: {translation_segment} WORD USE16 00000100 bytes
0000                          _input_xlat:
0000    FF FF 00 01               ....
Segment: _DATA WORD USE16 00000000 bytes
"""

    def test_discovers_hidden_cs_xlat_and_dispatch_table(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            original = root / "original"
            listings = root / "listings"
            original.mkdir()
            listings.mkdir()
            self.write_original(
                original,
                "func_001000_input_action_dispatch.asm",
                self.input_original(),
            )
            self.write_listing(
                listings,
                "func_001000_input_action_dispatch.lst",
                self.input_listing(),
            )
            objects, errors = MODULE.derive_original_inventory(original)
            audit_errors = self.audit(original, listings)

        self.assertEqual(errors, [])
        self.assertEqual(len(objects), 2)
        self.assertEqual(sum(item.is_table for item in objects.values()), 2)
        self.assertEqual(audit_errors, [])

    def test_mutation_rejects_xlat_table_moved_to_const2(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            original = root / "original"
            listings = root / "listings"
            original.mkdir()
            listings.mkdir()
            self.write_original(
                original,
                "func_001000_input_action_dispatch.asm",
                self.input_original(),
            )
            self.write_listing(
                listings,
                "func_001000_input_action_dispatch.lst",
                self.input_listing(translation_segment="CONST2"),
            )
            errors = self.audit(original, listings)

        self.assertTrue(any("_input_xlat is in CONST2" in error for error in errors))

    def test_mutation_rejects_unqualified_code_table_access(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            original = root / "original"
            listings = root / "listings"
            original.mkdir()
            listings.mkdir()
            self.write_original(
                original,
                "func_001000_input_action_dispatch.asm",
                self.input_original(),
            )
            self.write_listing(
                listings,
                "func_001000_input_action_dispatch.lst",
                self.input_listing(translation_prefix=""),
            )
            errors = self.audit(original, listings)

        self.assertTrue(any(
            "non-CS access to code-owned _input_xlat" in error for error in errors
        ))

    def test_mutation_rejects_missing_emitted_table_counterpart(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            original = root / "original"
            listings = root / "listings"
            original.mkdir()
            listings.mkdir()
            self.write_original(
                original,
                "func_001000_input_action_dispatch.asm",
                self.input_original(),
            )
            mutated = self.input_listing().replace(
                "0007    2E 8A 87 00 00            "
                "mov al,byte ptr cs:_input_xlat[bx]\n",
                "0007    88 D8                     mov al,bl\n",
            )
            self.write_listing(
                listings,
                "func_001000_input_action_dispatch.lst",
                mutated,
            )
            errors = self.audit(original, listings)

        self.assertTrue(any(
            "1 emitted code-table counterparts for 2" in error for error in errors
        ))

    def test_fails_closed_when_xlat_base_is_unresolved(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            original = root / "original"
            listings = root / "listings"
            original.mkdir()
            listings.mkdir()
            self.write_original(
                original,
                "func_001000_input_action_dispatch.asm",
                "001000:  2E D7                        xlatb\n"
                "001002:  CB                           retf\n",
            )
            self.write_listing(
                listings,
                "func_001000_input_action_dispatch.lst",
                self.input_listing(),
            )
            errors = self.audit(original, listings)

        self.assertTrue(any(
            "unresolved CS-relative target" in error for error in errors
        ))

    def test_derives_cs_aliased_sequential_table(self):
        original_body = """001000:  8C C8                        mov ax,cs
001002:  8E D8                        mov ds,ax
001004:  BE 97 03                     mov si,0x397
001007:  F3 A6                        repe cmpsb byte ptr [si],byte ptr es:[di]
001009:  1F                           pop ds
00100A:  CB                           retf
"""
        listing_body = """Segment: ems_TEXT BYTE USE16 00000010 bytes
0000                          _ems_signature:
0000    45 4D 4D 58 58 58 58 30   EMMXXXX0
0008                          ems_probe_:
0008    2E 3A 87 00 00            cmp al,byte ptr cs:_ems_signature[bx]
000D    CB                        retf
Segment: _DATA WORD USE16 00000000 bytes
"""
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            original = root / "original"
            listings = root / "listings"
            original.mkdir()
            listings.mkdir()
            self.write_original(
                original, "func_001000_ems_probe.asm", original_body
            )
            self.write_listing(
                listings, "func_001000_ems_probe.lst", listing_body
            )
            objects, inventory_errors = MODULE.derive_original_inventory(original)
            errors = self.audit(original, listings)

        self.assertEqual(inventory_errors, [])
        self.assertEqual(len(objects), 1)
        self.assertEqual(next(iter(objects.values())).target, 0x397)
        self.assertEqual(errors, [])

    def test_derives_direct_indexed_read_as_a_table(self):
        original_body = """001000:  BF DA 02                     mov di,0x2da
001003:  2E 8B 01                     mov ax,word ptr cs:[bx+di]
001006:  CB                           retf
"""
        listing_body = """Segment: digits_TEXT BYTE USE16 00000010 bytes
0000                          _digits:
0000    00 00 01 00               ....
0004                          digit_lookup_:
0004    2E 8B 84 00 00            mov ax,word ptr cs:_digits[si]
0009    CB                        retf
Segment: _DATA WORD USE16 00000000 bytes
"""
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            original = root / "original"
            listings = root / "listings"
            original.mkdir()
            listings.mkdir()
            self.write_original(
                original, "func_001000_digit_lookup.asm", original_body
            )
            self.write_listing(
                listings, "func_001000_digit_lookup.lst", listing_body
            )
            objects, inventory_errors = MODULE.derive_original_inventory(original)
            errors = self.audit(original, listings)

        self.assertEqual(inventory_errors, [])
        self.assertEqual(next(iter(objects.values())).target, 0x2DA)
        self.assertTrue(next(iter(objects.values())).is_table)
        self.assertEqual(errors, [])

    def test_mutation_rejects_derived_mutable_code_cell_owner(self):
        original_body = """001000:  2E A1 EE 0A                  mov ax,word ptr cs:[0xaee]
001004:  2E A3 EE 0A                  mov word ptr cs:[0xaee],ax
001008:  CB                           retf
"""
        listing_body = """Segment: cell_TEXT BYTE USE16 00000008 bytes
0000                          cell_update_:
0000    2E A1 00 00               mov ax,word ptr cs:_seed
0004    2E A3 00 00               mov word ptr cs:_seed,ax
0008    CB                        retf
Segment: _DATA WORD USE16 00000002 bytes
0000                          _seed:
0000    00 00                     ..
"""
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            original = root / "original"
            listings = root / "listings"
            original.mkdir()
            listings.mkdir()
            self.write_original(
                original, "func_001000_cell_update.asm", original_body
            )
            self.write_listing(
                listings, "func_001000_cell_update.lst", listing_body
            )
            errors = self.audit(original, listings)

        self.assertTrue(any("_seed is in _DATA" in error for error in errors))

    def test_missing_original_corpus_fails_closed(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            listings = root / "listings"
            listings.mkdir()
            errors = MODULE.audit(
                listings, root / "missing", original_image=None
            )

        self.assertTrue(any(
            "missing original BLOODPRG assembly" in error for error in errors
        ))


if __name__ == "__main__":
    unittest.main()
