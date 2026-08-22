#!/usr/bin/env python3
from __future__ import annotations

import importlib.util
import sys
import unittest
from collections import Counter
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location(
    "compare_segment_roles", ROOT / "re/tools/compare_segment_roles.py"
)
MODULE = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = MODULE
SPEC.loader.exec_module(MODULE)


class SegmentRoleComparisonTests(unittest.TestCase):
    def listing(self, rows: str):
        return MODULE.audit.parse_listing(Path("routine.obj"), rows)

    def test_missing_role_is_reported(self):
        access = MODULE.Access("memseg:GAME_DATA:6726", "r", 2, "based", 4)
        rows = MODULE.compare("routine", Counter({access: 2}), Counter())
        self.assertEqual(len(rows), 1)
        self.assertEqual(rows[0].status, "missing_role")
        self.assertEqual(rows[0].original_count, 2)

    def test_register_allocation_does_not_enter_shape(self):
        left = MODULE.Access("argument", "r", 1, "based", 2)
        right = MODULE.Access("argument", "r", 1, "based", 2)
        rows = MODULE.compare("routine", Counter({left: 1}), Counter({right: 1}))
        self.assertEqual(rows[0].status, "exact")

    def test_shape_difference_is_advisory(self):
        before = MODULE.Access("constant:a000", "w", 1, "based", 0)
        after = MODULE.Access("constant:a000", "w", 2, "based", 0)
        rows = MODULE.compare(
            "routine", Counter({before: 1}), Counter({after: 1})
        )
        self.assertEqual(rows[0].status, "shape_difference")
        self.assertIn("w1", rows[0].missing_shapes)
        self.assertIn("w2", rows[0].extra_shapes)

    def test_dword_and_two_word_accesses_have_equal_byte_footprints(self):
        dword = MODULE.Access("dynamic", "r", 4, "based", 0)
        low = MODULE.Access("dynamic", "r", 2, "based", 0)
        high = MODULE.Access("dynamic", "r", 2, "based", 2)
        rows = MODULE.compare(
            "routine", Counter({dword: 1}), Counter({low: 1, high: 1})
        )
        self.assertEqual(rows[0].status, "footprint_equivalent")

    def test_read_write_footprint_preserves_access_direction(self):
        read = MODULE.Access("dynamic", "r", 2, "based", 0)
        write = MODULE.Access("dynamic", "w", 2, "based", 0)
        self.assertNotEqual(
            MODULE.byte_footprint(Counter({read: 1})),
            MODULE.byte_footprint(Counter({write: 1})),
        )

    def test_xchg_preserves_segment_value_provenance(self):
        listing = self.listing("""
0000                          routine_:
0000    8B 0E 00 00               mov cx,word ptr _page_segment
0004    91                        xchg ax,cx
0005    8E D8                     mov ds,ax
0007    8B 04                     mov ax,word ptr [si]
0009    C3                        ret
""")
        layout = {
            "_page_segment": MODULE.LayoutEntry("GAME_DATA", 0x0A66)
        }
        accesses, _calls = MODULE.analyze(
            listing, layout, original=False
        )
        self.assertEqual(
            {access.role for access in accesses},
            {"memseg:GAME_DATA:0a66"},
        )

    def test_les_uses_high_word_of_local_far_pointer(self):
        listing = self.listing("""
0000                          routine_:
0000    A1 00 00                  mov ax,word ptr _page_segment
0003    89 46 FC                  mov word ptr -0x4[bp],ax
0006    A1 00 00                  mov ax,word ptr _work_segment
0009    89 46 FE                  mov word ptr -0x2[bp],ax
000C    C4 7E FC                  les di,dword ptr -0x4[bp]
000F    8E C7                     mov es,di
0011    26 8B 05                  mov ax,word ptr es:[di]
0014    C3                        ret
""")
        layout = {
            "_page_segment": MODULE.LayoutEntry("GAME_DATA", 0x0A66),
            "_work_segment": MODULE.LayoutEntry("GAME_DATA", 0x0ABE),
        }
        accesses, _calls = MODULE.analyze(
            listing, layout, original=False
        )
        self.assertEqual(
            {access.role for access in accesses},
            {"memseg:GAME_DATA:0a66"},
        )

    def test_stack_argument_provenance_reaches_direct_callee(self):
        caller = self.listing("""
0000                          caller_:
0000    A1 00 00                  mov ax,word ptr _page_segment
0003    50                        push ax
0004    E8 00 00                  call callee_
0007    C3                        ret
""")
        callee = self.listing("""
0000                          callee_:
0000    56                        push si
0001    57                        push di
0002    55                        push bp
0003    89 E5                     mov bp,sp
0005    8B 46 08                  mov ax,word ptr 0x8[bp]
0008    8E C0                     mov es,ax
000A    26 8B 05                  mov ax,word ptr es:[di]
000D    5D                        pop bp
000E    5F                        pop di
000F    5E                        pop si
0010    C3                        ret
""")
        layout = {
            "_page_segment": MODULE.LayoutEntry("GAME_DATA", 0x0A66),
        }
        resolver = lambda item: (
            "callee" if MODULE.audit.mnemonic(item.text) == "call" else None
        )
        accesses = MODULE.interprocedural_accesses(
            {"caller": caller, "callee": callee},
            layout,
            original=False,
            call_resolver=resolver,
        )
        self.assertEqual(
            {access.role for access in accesses["callee"]},
            {"memseg:GAME_DATA:0a66"},
        )

    def test_original_fs_reassignment_overrides_initial_owner(self):
        listing = self.listing("""
0000                          routine_:
0000    8C D8                     mov ax,ds
0002    8E E0                     mov fs,ax
0004    64 8A 07                  mov al,byte ptr fs:[bx]
0007    C3                        ret
""")
        entry = MODULE.initial_state(listing, original=True).with_register(
            "ds", MODULE.ARGUMENT
        )
        accesses, _calls = MODULE.analyze(
            listing, {}, original=True, entry_state=entry
        )
        self.assertEqual(
            {access.role for access in accesses}, {MODULE.ARGUMENT}
        )

    def test_static_effect_uses_canonical_symbol_offset(self):
        listing = self.listing("""
0000                          routine_:
0000    A1 00 00                  mov ax,word ptr _value
0003    C3                        ret
""")
        layout = {"_value": MODULE.LayoutEntry("GAME_DATA", 0x1234)}
        accesses, _calls = MODULE.analyze(
            listing, layout, original=False, include_static=True
        )
        self.assertEqual(
            accesses,
            Counter({MODULE.Access("GAME_DATA", "r", 2, "direct", 0x1234): 1}),
        )

    def test_review_is_bound_to_exact_comparison(self):
        row = MODULE.Comparison(
            "routine", "missing_role", "dynamic", 1, 0,
            "w2:based:+0x0x1", "",
        )
        replacement = MODULE.Comparison(
            "routine", "extra_role", "memseg:GAME_DATA:674a", 0, 1,
            "", "w2:direct:+0x0x1",
        )
        review = MODULE.Review(
            MODULE.routine_fingerprints([row, replacement])["routine"],
            "equivalent", "typed return",
        )
        reviewed = MODULE.apply_reviews(
            [row, replacement], {("routine", "dynamic"): review}
        )
        self.assertEqual(reviewed[0].status, "reviewed_equivalent")
        with self.assertRaisesRegex(ValueError, "stale segment-role review"):
            MODULE.apply_reviews(
                [row, MODULE.replace(replacement, rebuilt_count=2)],
                {("routine", "dynamic"): review},
            )

    def test_exact_caller_context_downgrades_local_difference(self):
        local = MODULE.Comparison(
            "routine", "extra_role", "dynamic", 0, 1, "", "r1:based:+0x0x1"
        )
        context = MODULE.replace(
            local, status="exact", original_count=1, missing_shapes="",
            extra_shapes="",
        )
        rows = MODULE.add_context_evidence([local], [context])
        self.assertEqual([row.status for row in rows], [
            "interprocedural_equivalent"
        ])

    def test_context_mismatch_remains_a_separate_finding(self):
        local = MODULE.Comparison(
            "routine", "extra_role", "dynamic", 0, 1, "", "r1:based:+0x0x1"
        )
        rows = MODULE.add_context_evidence([local], [local])
        self.assertEqual(
            [(row.status, row.role) for row in rows],
            [("extra_role", "dynamic")],
        )

    def test_empty_context_does_not_hide_a_local_difference(self):
        local = MODULE.Comparison(
            "routine", "extra_role", "dynamic", 0, 1, "", "r1:based:+0x0x1"
        )
        self.assertEqual(
            MODULE.add_context_evidence([local], [])[0].status,
            "extra_role",
        )


if __name__ == "__main__":
    unittest.main()
