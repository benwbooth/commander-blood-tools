#!/usr/bin/env python3

from __future__ import annotations

import os
import importlib.util
from pathlib import Path
import sys
import tempfile


_HERE = Path(__file__).resolve().parent
sys.path[:] = [
    path
    for path in sys.path
    if Path(os.path.abspath(path or os.curdir)) != _HERE
]

import unittest


TOOL_PATH = Path(__file__).with_name("audit_relinked_register_contracts.py")
SPEC = importlib.util.spec_from_file_location(
    "audit_relinked_register_contracts", TOOL_PATH
)
assert SPEC is not None and SPEC.loader is not None
AUDIT = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = AUDIT
SPEC.loader.exec_module(AUDIT)


def listing(rows, labels=None):
    instructions = []
    offset = 0
    for encoded, text in rows:
        data = bytes.fromhex(encoded)
        instructions.append(AUDIT.CORE.ListingInstruction(offset, data, text))
        offset += len(data)
    return AUDIT.CORE.Listing(
        Path("synthetic.lst"),
        tuple(instructions),
        labels or {"subject_": 0},
        {},
        (0,),
        ((0, offset),),
    )


def routine(
    key,
    rows,
    resolver=lambda _item, _kind: None,
    *,
    effect_resolver=None,
    public_effect=None,
):
    return AUDIT.build_routine(
        key,
        key,
        listing(rows),
        resolver,
        effect_resolver=effect_resolver,
        public_effect=public_effect,
    )


class RelinkedRegisterContractTests(unittest.TestCase):
    def compare_single(self, original_rows, emitted_rows):
        original = {"subject": routine("subject", original_rows)}
        emitted = {"subject": routine("subject", emitted_rows)}
        return AUDIT.compare_programs(original, emitted)

    def test_saved_register_and_flags_are_proved(self):
        subject = routine("subject", [
            ("9c", "pushf"),
            ("53", "push bx"),
            ("43", "inc bx"),
            ("5b", "pop bx"),
            ("f8", "clc"),
            ("9d", "popf"),
            ("c3", "ret"),
        ])
        summary = AUDIT.summarize_program({"subject": subject})["subject"]
        self.assertIn("BX", summary.preserved)
        self.assertIn("SP", summary.preserved)
        self.assertTrue(summary.flags_preserved)
        self.assertEqual(frozenset(), summary.blockers)

    def test_identity_arithmetic_preserves_register_but_clobbers_flags(self):
        subject = routine("subject", [
            ("09c0", "or ax,ax"),
            ("83c300", "add bx,0"),
            ("c3", "ret"),
        ])
        summary = AUDIT.summarize_program({"subject": subject})["subject"]
        self.assertIn("AX", summary.preserved)
        self.assertIn("BX", summary.preserved)
        self.assertFalse(summary.flags_preserved)

    def test_mutation_clobbering_one_of_two_emitted_exits_fails(self):
        original = [
            ("85c0", "test ax,ax"),
            ("7402", "je 0x6"),
            ("90", "nop"),
            ("c3", "ret"),
            ("c3", "ret"),
        ]
        mutated = [
            ("85c0", "test ax,ax"),
            ("7402", "je 0x6"),
            ("43", "inc bx"),
            ("c3", "ret"),
            ("c3", "ret"),
        ]
        rows = self.compare_single(original, mutated)
        self.assertEqual(2, len(rows))
        self.assertEqual(
            ["register_mismatch", "pass"],
            [row.status for row in rows],
        )
        self.assertEqual("BX", rows[0].emitted_clobbers)

    def test_mutation_clobbering_flags_after_restore_fails(self):
        original = [
            ("9c", "pushf"),
            ("85c0", "test ax,ax"),
            ("9d", "popf"),
            ("c3", "ret"),
        ]
        mutated = [
            ("9c", "pushf"),
            ("85c0", "test ax,ax"),
            ("9d", "popf"),
            ("f8", "clc"),
            ("c3", "ret"),
        ]
        rows = self.compare_single(original, mutated)
        self.assertEqual("flags_mismatch", rows[0].status)
        self.assertEqual("preserved", rows[0].original_flags)
        self.assertEqual("clobbered", rows[0].emitted_flags)

    def test_mutated_callee_contract_propagates_to_caller(self):
        def resolver(item, _kind):
            return ("callee",) if "callee" in item.text else None

        caller_rows = [("e80000", "call callee"), ("c3", "ret")]
        original = {
            "caller": routine("caller", caller_rows, resolver),
            "callee": routine("callee", [("c3", "ret")]),
        }
        emitted = {
            "caller": routine("caller", caller_rows, resolver),
            "callee": routine(
                "callee", [("89c6", "mov si,ax"), ("c3", "ret")]
            ),
        }
        rows = AUDIT.compare_programs(original, emitted)
        caller = next(row for row in rows if row.routine == "caller")
        callee = next(row for row in rows if row.routine == "callee")
        self.assertEqual("register_mismatch", caller.status)
        self.assertEqual("SI", caller.emitted_clobbers)
        self.assertEqual("register_mismatch", callee.status)

    def test_unresolved_indirect_call_is_a_hard_failure(self):
        subject = routine("subject", [("ffd3", "call bx"), ("c3", "ret")])
        summary = AUDIT.summarize_program({"subject": subject})["subject"]
        self.assertTrue(summary.blockers)
        self.assertTrue(any("unresolved indirect" in item for item in summary.blockers))

    def test_typed_indirect_call_uses_only_its_proven_effect(self):
        effect = AUDIT.register_effect("AX", "DX")
        subject = routine(
            "subject",
            [("ffd3", "call bx"), ("c3", "ret")],
            effect_resolver=lambda _item: effect,
        )
        summary = AUDIT.summarize_program({"subject": subject})["subject"]
        self.assertEqual(frozenset(), summary.blockers)
        self.assertNotIn("AX", summary.preserved)
        self.assertNotIn("DX", summary.preserved)
        self.assertIn("BX", summary.preserved)

    def test_public_effect_hides_internal_interrupt_details_only(self):
        subject = routine(
            "subject",
            [("cd21", "int 0x21"), ("c3", "ret")],
            public_effect=AUDIT.register_effect("AX"),
        )
        summary = AUDIT.summarize_program({"subject": subject})["subject"]
        self.assertEqual(frozenset(), summary.blockers)
        self.assertNotIn("AX", summary.preserved)
        self.assertIn("DX", summary.preserved)

    def test_recursive_preservation_contract_converges(self):
        def resolver(item, _kind):
            return ("subject",) if "subject" in item.text else None

        subject = routine(
            "subject",
            [
                ("85c0", "test ax,ax"),
                ("7403", "je 0x7"),
                ("e80000", "call subject"),
                ("c3", "ret"),
            ],
            resolver,
        )
        summary = AUDIT.summarize_program({"subject": subject})["subject"]
        self.assertEqual(frozenset(), summary.blockers)
        self.assertIn("BX", summary.preserved)

    def test_recursive_mutation_is_not_optimistically_preserved(self):
        def resolver(item, _kind):
            return ("subject",) if "subject" in item.text else None

        subject = routine(
            "subject",
            [
                ("85c0", "test ax,ax"),
                ("7404", "je 0x8"),
                ("43", "inc bx"),
                ("e80000", "call subject"),
                ("c3", "ret"),
            ],
            resolver,
        )
        summary = AUDIT.summarize_program({"subject": subject})["subject"]
        self.assertEqual(frozenset(), summary.blockers)
        self.assertNotIn("BX", summary.preserved)

    def test_unresolved_indirect_jump_is_not_treated_as_an_exit(self):
        subject = routine("subject", [("ffe3", "jmp bx")])
        summary = AUDIT.summarize_program({"subject": subject})["subject"]
        self.assertFalse(summary.exits)
        self.assertTrue(any("unresolved indirect" in item for item in summary.blockers))

    def test_both_sides_unresolved_are_reported(self):
        original = {"subject": routine(
            "subject", [("ffd3", "call bx"), ("c3", "ret")]
        )}
        emitted = {"subject": routine(
            "subject", [("ffd0", "call ax"), ("c3", "ret")]
        )}
        rows = AUDIT.compare_programs(original, emitted)
        self.assertEqual("unresolved_both", rows[0].status)
        self.assertIn("original:", rows[0].blockers)
        self.assertIn("emitted:", rows[0].blockers)

    def test_disconnected_executable_bytes_fail_closed(self):
        subject = routine("subject", [("c3", "ret"), ("90", "nop")])
        summary = AUDIT.summarize_program({"subject": subject})["subject"]
        self.assertTrue(any("disconnected" in item for item in summary.blockers))

    def test_unbalanced_return_stack_is_a_hard_failure(self):
        subject = routine("subject", [("50", "push ax"), ("c3", "ret")])
        summary = AUDIT.summarize_program({"subject": subject})["subject"]
        self.assertTrue(any(
            "return stack delta" in item for item in summary.blockers
        ))

    def test_original_far_target_uses_mz_header_and_segment(self):
        item = AUDIT.CORE.ListingInstruction(
            0x068D,
            bytes.fromhex("9af30ace01"),
            "lcall 0x1ce, 0xaf3",
        )
        resolver = AUDIT.original_resolver(
            {0x2DD3: "cmos"}, 0x600, {}
        )
        self.assertEqual(("cmos",), resolver(item, "call"))

    def test_push_cs_near_call_retf_thunk_consumes_segment_word(self):
        def resolver(item, _kind):
            return ("callee",) if "callee" in item.text else None

        caller = routine(
            "caller",
            [("0e", "push cs"), ("e80000", "call callee"), ("c3", "ret")],
            resolver,
        )
        callee = routine("callee", [("cb", "retf")])
        summaries = AUDIT.summarize_program({"caller": caller, "callee": callee})
        self.assertIn("SP", summaries["caller"].preserved)
        self.assertFalse(any(
            "stack" in blocker for blocker in summaries["caller"].blockers
        ))

    def test_callee_return_cleanup_consumes_caller_argument(self):
        def resolver(item, _kind):
            return ("callee",) if "callee" in item.text else None

        caller = routine(
            "caller",
            [("6a01", "push 1"), ("e80000", "call callee"), ("c3", "ret")],
            resolver,
        )
        callee = routine("callee", [("c20200", "ret 2")])
        summaries = AUDIT.summarize_program({"caller": caller, "callee": callee})
        self.assertEqual(2, summaries["callee"].cleanup)
        self.assertEqual(0, summaries["caller"].cleanup)
        self.assertIn("SP", summaries["caller"].preserved)

    def test_call_targets_with_different_cleanup_fail_closed(self):
        def resolver(item, _kind):
            return ("plain", "callee_pop") if "dispatch" in item.text else None

        caller = routine(
            "caller", [("e80000", "call dispatch"), ("c3", "ret")], resolver
        )
        plain = routine("plain", [("c3", "ret")])
        callee_pop = routine("callee_pop", [("c20200", "ret 2")])
        summaries = AUDIT.summarize_program({
            "caller": caller,
            "plain": plain,
            "callee_pop": callee_pop,
        })
        self.assertTrue(any(
            "incompatible stack cleanup" in blocker
            for blocker in summaries["caller"].blockers
        ))

    def test_iret_restores_entry_flags_from_interrupt_frame(self):
        subject = routine(
            "subject", [("f8", "clc"), ("cf", "iret")]
        )
        summary = AUDIT.summarize_program({"subject": subject})["subject"]
        self.assertTrue(summary.flags_preserved)

    def test_symbolic_relocated_jump_is_a_tail_not_a_local_zero_jump(self):
        resolver = AUDIT.emitted_resolver({"callee": "callee"})
        caller = routine("caller", [("e90000", "jmp callee_")], resolver)
        callee = routine("callee", [("c3", "ret")])
        self.assertEqual(((0, ("callee",)),), caller.tails)
        self.assertEqual((), caller.edges[0][1])
        summary = AUDIT.summarize_program({"caller": caller, "callee": callee})
        self.assertEqual(frozenset(), summary["caller"].blockers)

    def test_emitted_vm_table_uses_original_static_dispatch_targets(self):
        resolver = AUDIT.emitted_resolver(
            {},
            {
                0x5627: ("op_a0", "op_a1"),
                0x56C4: ("op_a0", "op_a1"),
            },
        )
        item = listing([
            ("26ff97c00c", "call word ptr es:_vm_opcode_handlers[bx]")
        ]).instructions[0]
        self.assertEqual(("op_a0", "op_a1"), resolver(item, "call"))

    def test_linked_direct_dependency_is_derived_from_map_and_image(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            image = bytearray(0x20 + 0x20)
            image[:2] = b"MZ"
            image[8:10] = (2).to_bytes(2, "little")
            image[0x20 + 0x10:0x20 + 0x12] = bytes.fromhex("43c3")
            image_path = root / "subject.exe"
            image_path.write_bytes(image)
            map_path = root / "subject.map"
            map_path.write_text(
                "ext_TEXT CODE AUTO 0000:0010 00000002\n"
                "0000:0010      ext_\n",
                encoding="ascii",
            )
            routines, symbols = AUDIT.discover_linked_dependencies(
                ("ext_",), {}, {}, map_path, image_path
            )
        self.assertEqual("linked_00010", symbols["ext_"])
        summary = AUDIT.summarize_program(routines)["linked_00010"]
        self.assertNotIn("BX", summary.preserved)
        self.assertEqual(frozenset(), summary.blockers)

    def test_near_linked_target_wraps_within_code_segment(self):
        item = AUDIT.CORE.ListingInstruction(
            0xFB0A, bytes.fromhex("e97038"), "jmp 0x1337d"
        )
        self.assertEqual(0x337D, AUDIT.direct_binary_target(item, 0))

    def test_unknown_external_symbol_does_not_inherit_a_safe_abi(self):
        resolver = AUDIT.emitted_resolver({"known": "known_stem"})
        subject = routine(
            "subject",
            [("e80000", "call unknown_external_"), ("c3", "ret")],
            resolver,
        )
        summary = AUDIT.summarize_program({"subject": subject})["subject"]
        self.assertTrue(any("external call effect" in item for item in summary.blockers))


if __name__ == "__main__":
    unittest.main()
