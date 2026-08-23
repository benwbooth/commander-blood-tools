#!/usr/bin/env python3
from __future__ import annotations

import importlib.util
import sys
import tempfile
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location(
    "audit_xdb_emitted_abi", ROOT / "re/tools/audit_xdb_emitted_abi.py"
)
MODULE = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = MODULE
SPEC.loader.exec_module(MODULE)


class EmittedAbiTests(unittest.TestCase):
    def listing(self, body: str):
        temporary = tempfile.NamedTemporaryFile(suffix=".obj", delete=False)
        temporary.close()
        self.addCleanup(Path(temporary.name).unlink, missing_ok=True)
        return MODULE.CORE.parse_listing(Path(temporary.name), body)

    def test_accepts_generated_alien_entry_contract(self):
        image = bytes.fromhex(
            "66 50 66 53 66 51 66 52 66 56 66 57 1e 06 0f a0 0f a8 "
            "66 55 fc 89 e8 8c d2 8c cb 0e e8 00 00 66 5d 0f a9 0f a1 "
            "07 1f 66 5f 66 5e 66 5a 66 59 66 5b 66 58 cb"
        )

        self.assertEqual(MODULE.validate_entry_image(image), [])

    def test_rejects_entry_that_does_not_restore_fs(self):
        image = bytes.fromhex(
            "66 50 66 53 66 51 66 52 66 56 66 57 1e 06 0f a0 0f a8 "
            "66 55 fc 89 e8 8c d2 8c cb 0e e8 00 00 66 5d 0f a9 90 07 "
            "1f 66 5f 66 5e 66 5a 66 59 66 5b 66 58 cb"
        )

        self.assertTrue(MODULE.validate_entry_image(image))

    def test_accepts_balanced_near_callback(self):
        listing = self.listing("""
0000                          callback_:
0000    55                        push bp
0001    89 E5                     mov bp,sp
0003    83 EC 02                  sub sp,0x2
0006    C9                        leave
0007    C3                        ret
""")

        self.assertEqual(MODULE.near_return_errors(listing), [])
        self.assertEqual(MODULE.stack_balance_errors(listing), [])

    def test_rejects_far_or_unbalanced_callback(self):
        far_listing = self.listing("""
0000                          callback_:
0000    CB                        retf
""")
        unbalanced = self.listing("""
0000                          callback_:
0000    50                        push ax
0001    C3                        ret
""")

        self.assertTrue(MODULE.near_return_errors(far_listing))
        self.assertTrue(MODULE.stack_balance_errors(unbalanced))

    def test_accepts_stack_neutral_near_tail_callback(self):
        listing = self.listing("""
0000                          callback_:
0000    E9 00 00                  jmp target_
""")

        self.assertEqual(MODULE.near_return_errors(listing), [])
        self.assertEqual(MODULE.stack_balance_errors(listing), [])

    def test_rejects_callback_segment_clobber(self):
        listing = self.listing("""
0000                          callback_:
0000    8E D8                     mov ds,ax
0002    C3                        ret
""")

        self.assertTrue(MODULE.segment_writes(listing))

    def test_far_segment_use_requires_reaching_definition(self):
        missing = self.listing("""
0000                          routine_:
0000    26 8B 07                  mov ax,word ptr es:[bx]
0003    C3                        ret
""")
        defined = self.listing("""
0000                          routine_:
0000    C4 1E 00 00               les bx,dword ptr _pointer
0004    26 8B 07                  mov ax,word ptr es:[bx]
0007    C3                        ret
""")

        self.assertTrue(
            MODULE.far_segment_definition_errors(
                missing, frozenset(("ds", "ss", "cs"))
            )[1]
        )
        self.assertEqual(
            MODULE.far_segment_definition_errors(
                defined, frozenset(("ds", "ss", "cs"))
            )[1],
            [],
        )

    def test_call_invalidates_transient_es_definition(self):
        listing = self.listing("""
0000                          routine_:
0000    8E C0                     mov es,ax
0002    E8 00 00                  call helper_
0005    26 8B 07                  mov ax,word ptr es:[bx]
0008    C3                        ret
""")

        self.assertTrue(
            MODULE.far_segment_definition_errors(
                listing, frozenset(("ds", "ss", "cs"))
            )[1]
        )


if __name__ == "__main__":
    unittest.main()
