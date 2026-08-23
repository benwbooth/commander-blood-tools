#!/usr/bin/env python3
from __future__ import annotations

import importlib.util
import sys
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location(
    "audit_segment_contracts", ROOT / "re/tools/audit_segment_contracts.py"
)
MODULE = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = MODULE
SPEC.loader.exec_module(MODULE)


class SegmentContractAuditTests(unittest.TestCase):
    def listing(self, rows: str):
        return MODULE.parse_listing(Path("routine.obj"), rows)

    def test_game_data_access_after_segment_load_is_proven(self):
        listing = self.listing("""
0000                          routine_:
0000    B8 00 00                  mov ax,seg _game_word
0003    8E C0                     mov es,ax
0005    26 A1 00 00               mov ax,word ptr es:_game_word
0009    CB                        retf
""")
        findings, reached = MODULE.analyze_listing(
            listing, {"_game_word": "GAME_DATA"}
        )
        self.assertEqual(reached, 4)
        self.assertEqual([finding.status for finding in findings], ["ok"])

    def test_wrong_owner_is_a_definite_mismatch(self):
        listing = self.listing("""
0000                          routine_:
0000    B8 00 00                  mov ax,seg _fs_table
0003    8E C0                     mov es,ax
0005    26 A1 00 00               mov ax,word ptr es:_game_word
0009    CB                        retf
""")
        findings, _ = MODULE.analyze_listing(
            listing,
            {"_fs_table": "FS_DATA", "_game_word": "GAME_DATA"},
        )
        self.assertEqual([finding.status for finding in findings], ["mismatch"])
        self.assertEqual(findings[0].proven_owner, "FS_DATA")

    def test_branch_merge_becomes_unproven(self):
        listing = self.listing("""
0000                          routine_:
0000    74 05                     je L$1
0002    B8 00 00                  mov ax,seg _game_word
0005    8E C0                     mov es,ax
0007                          L$1:
0007    26 A1 00 00               mov ax,word ptr es:_game_word
000B    CB                        retf
""")
        findings, _ = MODULE.analyze_listing(
            listing, {"_game_word": "GAME_DATA"}
        )
        self.assertEqual([finding.status for finding in findings], ["unproven"])

    def test_bp_local_preserves_segment_provenance(self):
        listing = self.listing("""
0000                          routine_:
0000    B8 00 00                  mov ax,seg _game_word
0003    89 46 FE                  mov word ptr -0x2[bp],ax
0006    8E 46 FE                  mov es,word ptr -0x2[bp]
0009    26 A1 00 00               mov ax,word ptr es:_game_word
000D    CB                        retf
""")
        findings, _ = MODULE.analyze_listing(
            listing, {"_game_word": "GAME_DATA"}
        )
        self.assertEqual([finding.status for finding in findings], ["ok"])

    def test_jump_table_targets_are_reached(self):
        listing = self.listing("""
0000                          L$1:
0000    08 00                     DW offset L$2
0002    0C 00                     DW offset L$3
0004                          routine_:
0004    2E FF A5 00 00            jmp word ptr cs:L$1[di]
0008                          L$2:
0008    A1 00 00                  mov ax,word ptr _game_word
000B    CB                        retf
000C                          L$3:
000C    A1 00 00                  mov ax,word ptr _game_word
000F    CB                        retf
""")
        findings, reached = MODULE.analyze_listing(
            listing, {"_game_word": "GAME_DATA"}
        )
        self.assertEqual(reached, 5)
        self.assertEqual([finding.status for finding in findings], ["ok", "ok"])

    def test_segment_save_survives_callee_cleaned_arguments(self):
        listing = self.listing("""
0000                          routine_:
0000    1E                        push ds
0001    B8 00 00                  mov ax,seg _fs_table
0004    8E D8                     mov ds,ax
0006    68 00 00                  push offset _game_word
0009    9A 00 00 00 00            call helper_
000E    1F                        pop ds
000F    A1 00 00                  mov ax,word ptr _game_word
0012    CB                        retf
""")
        findings, _ = MODULE.analyze_listing(
            listing,
            {"_fs_table": "FS_DATA", "_game_word": "GAME_DATA"},
        )
        self.assertEqual([finding.status for finding in findings], ["ok"])

    def test_watcom_loadds_dgroup_spelling_is_game_data(self):
        listing = self.listing("""
0000                          routine_:
0000    B8 00 00                  mov ax,DGROUP:CONST
0003    8E D8                     mov ds,ax
0005    A1 00 00                  mov ax,word ptr _game_word
0008    CB                        retf
""")
        findings, _ = MODULE.analyze_listing(
            listing, {"_game_word": "GAME_DATA"}
        )
        self.assertEqual([finding.status for finding in findings], ["ok"])

    def test_code_segment_data_before_routine_is_not_disassembled(self):
        listing = self.listing("""
Segment: func_example_TEXT BYTE USE16 0000000E bytes
0000                          _signature:
0000    45 4D 4D 58 58 58 58 30   EMMXXXX0
0008                          routine_:
0008    2E A0 00 00               mov al,byte ptr cs:_signature
000C    CB                        retf
Segment: _DATA WORD USE16 00000000 bytes
""")
        self.assertEqual(
            [instruction.offset for instruction in listing.instructions],
            [0x0008, 0x000C],
        )


if __name__ == "__main__":
    unittest.main()
