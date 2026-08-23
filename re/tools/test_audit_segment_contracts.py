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

    def test_wrapped_main_instruction_keeps_tail_reachable(self):
        listing = self.listing("""
Segment: func_main_TEXT BYTE USE16 0000000F bytes
0000                          routine_:
0000    B8 00 00                  mov ax,seg _game_word
0003    66 C7 46 E6 D0 02 96 00
                                  mov dword ptr -0x1a[bp],0x009602d0
000B    A1 00 00                  mov ax,word ptr _game_word
000E    CB                        retf
Routine Size: 15 bytes,    Routine Base: func_main_TEXT + 0000
Segment: _DATA WORD USE16 00000000 bytes
""")
        findings, reached = MODULE.analyze_listing(
            listing, {"_game_word": "GAME_DATA"}
        )
        self.assertEqual(reached, 4)
        self.assertEqual([finding.status for finding in findings], ["ok"])
        self.assertEqual(listing.instructions[1].text,
                         "mov dword ptr -0x1a[bp],0x009602d0")

    def test_wrapped_sprite_instruction_keeps_tail_reachable(self):
        listing = self.listing("""
Segment: func_sprite_TEXT BYTE USE16 0000000C bytes
0000                          routine_:
0000    66 C7 46 D8 00 00 00 00
                                  mov dword ptr -0x28[bp],0x00000000
0008    A1 00 00                  mov ax,word ptr _game_word
000B    CB                        retf
Routine Size: 12 bytes,    Routine Base: func_sprite_TEXT + 0000
Segment: _DATA WORD USE16 00000000 bytes
""")
        findings, reached = MODULE.analyze_listing(
            listing, {"_game_word": "GAME_DATA"}
        )
        self.assertEqual(reached, 3)
        self.assertEqual([finding.status for finding in findings], ["ok"])

    def test_zero_code_data_does_not_replace_prng_function_entry(self):
        listing = self.listing("""
Segment: func_prng_TEXT BYTE USE16 00000007 bytes
0000                          _seed:
0000    00 00                                           ..
0002                          _mix:
0002    00 00 00                                        ...
Routine Size: 5 bytes,    Routine Base: func_prng_TEXT + 0000
0005                          routine_:
0005    31 C0                     xor ax,ax
0007    CB                        retf
Routine Size: 3 bytes,    Routine Base: func_prng_TEXT + 0005
Segment: _DATA WORD USE16 00000000 bytes
""")
        self.assertEqual(listing.entrypoints, (0x0005,))
        self.assertEqual(
            [instruction.offset for instruction in listing.instructions],
            [0x0005, 0x0007],
        )
        _findings, reached = MODULE.analyze_listing(listing, {})
        self.assertEqual(reached, 2)

    def test_public_helper_inside_declared_routine_span_is_an_entrypoint(self):
        listing = self.listing("""
Segment: func_adapter_TEXT BYTE USE16 00000004 bytes
0000                          adapter_:
0000    90                        nop
0001                          helper_:
0001    90                        nop
0002    EB FD                     jmp helper_
Routine Size: 4 bytes,    Routine Base: func_adapter_TEXT + 0000
Segment: _DATA WORD USE16 00000000 bytes
""")
        self.assertEqual(listing.entrypoints, (0x0000, 0x0001))

    def test_public_entry_outside_declared_routine_span_fails_closed(self):
        with self.assertRaisesRegex(ValueError, "no routine span"):
            self.listing("""
Segment: func_adapter_TEXT BYTE USE16 00000004 bytes
0000                          adapter_:
0000    C3                        ret
Routine Size: 1 bytes,    Routine Base: func_adapter_TEXT + 0000
0001                          orphan_:
0001    C3                        ret
Segment: _DATA WORD USE16 00000000 bytes
""")

    def test_consecutive_wrapped_resource_instructions_are_reconstructed(self):
        listing = self.listing("""
Segment: func_resource_TEXT BYTE USE16 00000019 bytes
0000                          routine_:
0000    26 66 C7 06 00 00 00 7D 00 00
                                  mov dword ptr es:_move_request,0x00007d00
000A    26 66 C7 06 06 00 00 00 00 00
                                  mov dword ptr es:_move_request+0x6,0x00000000
0014    26 A1 00 00               mov ax,word ptr es:_move_request
0018    CB                        retf
Routine Size: 25 bytes,    Routine Base: func_resource_TEXT + 0000
Segment: _DATA WORD USE16 00000000 bytes
""")
        findings, reached = MODULE.analyze_listing(
            listing, {"_move_request": "GAME_DATA"}
        )
        self.assertEqual(reached, 4)
        self.assertEqual(
            [finding.status for finding in findings],
            ["unproven", "unproven", "unproven"],
        )

    def test_disconnected_executable_instruction_fails_closed(self):
        listing = self.listing("""
Segment: func_disconnected_TEXT BYTE USE16 00000004 bytes
0000                          routine_:
0000    EB 01                     jmp L$1
0002    90                        nop
0003                          L$1:
0003    CB                        retf
Routine Size: 4 bytes,    Routine Base: func_disconnected_TEXT + 0000
Segment: _DATA WORD USE16 00000000 bytes
""")
        with self.assertRaisesRegex(ValueError, "disconnected executable"):
            MODULE.analyze_listing(listing, {})

    def test_unparsed_executable_instruction_fails_closed(self):
        with self.assertRaisesRegex(ValueError, "unparsed executable row"):
            self.listing("""
Segment: func_unparsed_TEXT BYTE USE16 00000002 bytes
0000                          routine_:
0000    FF                        invalid
0001    CB                        retf
Routine Size: 2 bytes,    Routine Base: func_unparsed_TEXT + 0000
Segment: _DATA WORD USE16 00000000 bytes
""")


if __name__ == "__main__":
    unittest.main()
