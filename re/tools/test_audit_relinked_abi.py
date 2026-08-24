#!/usr/bin/env python3
from __future__ import annotations

import importlib.util
import sys
import unittest
from pathlib import Path
from types import SimpleNamespace


ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location(
    "audit_relinked_abi", ROOT / "re/tools/audit_relinked_abi.py"
)
MODULE = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = MODULE
SPEC.loader.exec_module(MODULE)


def listing(label: str, texts: list[str], extra_labels=None):
    instructions = [
        SimpleNamespace(offset=index, text=text)
        for index, text in enumerate(texts)
    ]
    labels = {label: 0, **(extra_labels or {})}
    return SimpleNamespace(
        object_path=Path("fixture.obj"),
        instructions=instructions,
        labels=labels,
    )


def machine_listing(label: str, rows: list[tuple[str, str]]):
    offset = 0
    instructions = []
    for encoded, text in rows:
        data = bytes.fromhex(encoded)
        instructions.append(
            SimpleNamespace(offset=offset, data=data, text=text)
        )
        offset += len(data)
    return SimpleNamespace(
        object_path=Path("fixture.obj"),
        instructions=tuple(instructions),
        labels={label: 0},
    )


class RelinkedAbiAuditTests(unittest.TestCase):
    def test_derives_return_kind_and_cleanup_operand(self):
        subject = machine_listing("sample_", [("ca0400", "retf 0x0004")])
        self.assertEqual(
            MODULE.routine_return_sites(subject.instructions),
            (MODULE.ReturnSite("far", 4),),
        )

    def test_cleanup_operand_mutation_is_rejected(self):
        original = MODULE.RoutineAbi(
            (MODULE.ReturnSite("far", 0),), (), "callee exits"
        )
        recovered = MODULE.RoutineAbi(
            (MODULE.ReturnSite("far", 2),), (), "callee exits"
        )
        errors = MODULE.compare_routine_abi("mutated_cleanup", original, recovered)
        self.assertTrue(any("return convention mismatch" in error for error in errors))

    def test_near_to_far_return_mutation_is_rejected(self):
        original = MODULE.RoutineAbi(
            (MODULE.ReturnSite("near", 0),), (), "callee exits"
        )
        recovered = MODULE.RoutineAbi(
            (MODULE.ReturnSite("far", 0),), (), "callee exits"
        )
        errors = MODULE.compare_routine_abi("mutated_width", original, recovered)
        self.assertTrue(any("return convention mismatch" in error for error in errors))

    def test_unresolved_mixed_returns_fail_closed(self):
        original = MODULE.RoutineAbi(
            (
                MODULE.ReturnSite("near", 0),
                MODULE.ReturnSite("near", 2),
            ),
            (),
            "callee exits",
        )
        errors = MODULE.compare_routine_abi("ambiguous", original, original)
        self.assertTrue(any("unresolved original" in error for error in errors))
        self.assertTrue(any("unresolved recovered" in error for error in errors))

    def test_return_register_mutation_is_rejected(self):
        original = MODULE.RoutineAbi(
            (MODULE.ReturnSite("near", 0),),
            (MODULE.ReturnCarrier("ax", 16),),
            "direct callers",
        )
        recovered = MODULE.RoutineAbi(
            (MODULE.ReturnSite("near", 0),),
            (MODULE.ReturnCarrier("dx", 16),),
            "direct callers",
        )
        errors = MODULE.compare_routine_abi("mutated_register", original, recovered)
        self.assertTrue(any("return carrier mismatch" in error for error in errors))

    def test_derives_resource_resolve_ax_and_far_pointer_carriers(self):
        subject = machine_listing(
            "resource_handle_resolve_",
            [
                ("b80100", "mov ax,0x0001"),
                ("8ed8", "mov ds,ax"),
                ("33f6", "xor si,si"),
                ("cb", "retf"),
            ],
        )
        self.assertEqual(
            MODULE.locally_modified_carriers(subject),
            (
                MODULE.ReturnCarrier("ax", 16),
                MODULE.ReturnCarrier("ds", 16),
                MODULE.ReturnCarrier("si", 16),
            ),
        )

    def test_hidden_far_struct_result_is_rejected(self):
        original = MODULE.RoutineAbi(
            (MODULE.ReturnSite("far", 0),),
            (
                MODULE.ReturnCarrier("ax", 16),
                MODULE.ReturnCarrier("ds", 16),
                MODULE.ReturnCarrier("si", 16),
            ),
            "direct callers",
        )
        recovered = MODULE.RoutineAbi(
            (MODULE.ReturnSite("far", 0),),
            (),
            "direct callers",
            hidden_result_width=6,
        )
        errors = MODULE.compare_routine_abi(
            "resource_handle_resolve", original, recovered
        )
        self.assertTrue(any("hidden-ds:si-memory:48" in error for error in errors))

    def test_derives_hidden_struct_copy_width(self):
        subject = machine_listing(
            "resource_handle_resolve_",
            [
                ("a5", "movsw"),
                ("a5", "movsw"),
                ("a5", "movsw"),
                ("cb", "retf"),
            ],
        )
        self.assertEqual(MODULE.copied_result_width(subject), 6)

    def test_dic_ax_carry_to_dx_ax_mutation_is_rejected(self):
        original_listing = machine_listing(
            "dic_word_lookup_",
            [
                ("b80100", "mov ax,0x0001"),
                ("f9", "stc"),
                ("c3", "ret"),
            ],
        )
        recovered_listing = machine_listing(
            "dic_word_lookup_",
            [
                ("b80100", "mov ax,0x0001"),
                ("ba0100", "mov dx,0x0001"),
                ("c3", "ret"),
            ],
        )
        original = MODULE.RoutineAbi(
            MODULE.routine_return_sites(original_listing.instructions),
            MODULE.locally_modified_carriers(original_listing),
            "callee exits",
        )
        recovered = MODULE.RoutineAbi(
            MODULE.routine_return_sites(recovered_listing.instructions),
            MODULE.locally_modified_carriers(recovered_listing),
            "callee exits",
        )
        errors = MODULE.compare_routine_abi("dic_word_lookup", original, recovered)
        self.assertTrue(any("flags:1" in error and "dx:16" in error for error in errors))

    def test_accepts_sound_entry_that_restores_linked_dgroup(self):
        subject = listing(
            "snd_play_clip_",
            [
                "push bx",
                "push ds",
                "mov ax,DGROUP:CONST",
                "mov ds,ax",
                "test byte ptr gs:_snd_driver_pending_flag_gs,0x02",
                "call dword ptr gs:_audio_position_callback_gs",
            ],
        )
        self.assertEqual(MODULE.audit_sound(subject), [])

    def test_rejects_sound_entry_that_inherits_foreign_ds(self):
        subject = listing(
            "snd_play_clip_",
            [
                "push bx",
                "test byte ptr gs:_snd_driver_pending_flag_gs,0x02",
                "call dword ptr gs:_audio_position_callback_gs",
            ],
        )
        errors = MODULE.audit_sound(subject)
        self.assertTrue(any("restore DS" in error for error in errors))

    def test_rejects_sound_access_through_caller_ds(self):
        subject = listing(
            "snd_play_clip_",
            [
                "push ds",
                "mov ax,DGROUP:CONST",
                "mov ds,ax",
                "test byte ptr _snd_driver_pending_flag_gs,0x02",
                "call dword ptr es:_audio_position_callback_gs",
            ],
        )
        errors = MODULE.audit_sound(subject)
        self.assertTrue(any("inherit caller DS" in error for error in errors))

    def test_rejects_early_sti(self):
        subject = listing(
            "bloodprg_critical_error_handler_", ["sti", "popa", "iret"]
        )
        self.assertTrue(MODULE.audit_critical_error(subject))

    def test_accepts_xms_status_and_handle_mapping(self):
        subject = listing(
            "cb_xms_allocate_kb_",
            [
                "mov ah,0x09",
                "call dword ptr _xms_driver_entry",
                "mov cx,dx",
                "xor dx,dx",
                "or ax,ax",
                "je L$1",
                "inc dx",
                "mov ax,cx",
                "mov word ptr [si],ax",
                "test dx,dx",
                "setne al",
            ],
            {"L$1": 7},
        )
        self.assertEqual(MODULE.audit_xms_allocate(subject), [])

    def test_rejects_xms_failure_as_success(self):
        subject = listing(
            "cb_xms_allocate_kb_",
            [
                "mov ah,0x09",
                "call dword ptr _xms_driver_entry",
                "mov cx,dx",
                "xor dx,dx",
                "or ax,ax",
                "je L$1",
                "inc dx",
                "mov ax,cx",
                "mov word ptr [si],ax",
                "test dx,dx",
                "sete al",
            ],
            {"L$1": 7},
        )
        self.assertTrue(MODULE.audit_xms_allocate(subject))

    def test_accepts_startup_segment_alias(self):
        text = [
            "jmp 0x8b",
            "mov cx, 0x1069",
            "mov es, cx",
            "mov ss, cx",
            "mov sp, bx",
            "mov dx, 0x1069",
            "mov ds, dx",
        ]
        self.assertEqual(MODULE.audit_startup_sequence(text, 0x1069, 0x1069), [])

    def test_rejects_separate_game_data_segment(self):
        self.assertTrue(MODULE.audit_startup_sequence([], 0x1069, 0x106A))

    def test_rejects_delayed_stack_pointer_load(self):
        text = [
            "mov cx, 0x1069",
            "mov es, cx",
            "mov ss, cx",
            "nop",
            "mov sp, bx",
            "mov dx, 0x1069",
            "mov ds, dx",
        ]
        errors = MODULE.audit_startup_sequence(text, 0x1069, 0x1069)
        self.assertTrue(any("immediately" in error for error in errors))

    def test_accepts_main_segment_install(self):
        subject = listing(
            "main_",
            ["mov dx,ds", "mov gs,dx", "mov fs,ax", "call bloodprg_entry_"],
        )
        self.assertEqual(MODULE.audit_segment_install(subject), [])

    def test_accepts_overlay_inherited_bp_contract(self):
        subject = listing(
            "cb_overlay_call_inherited_bp_",
            ["mov bp,si", "call dword ptr ss:[bx]", "ret"],
        )
        self.assertEqual(MODULE.audit_overlay_request_segment(subject), [])

    def test_accepts_vm_record_far_pointer_call(self):
        caller = listing(
            "vm_op_c1_record_state_",
            [
                "mov ax,word ptr es:_vm_record_base_gs+2",
                "mov es,dx",
                "mov word ptr -2[bp],ax",
                "mov ax,word ptr -4[bp]",
                "push cx",
                "mov cx,word ptr -2[bp]",
                "mov bx,si",
                "mov dx,cx",
                "call near ptr ship_3d_position_distance_",
                "ret",
            ],
        )
        callee = listing(
            "ship_3d_position_distance_",
            [
                "mov si,ax",
                "mov word ptr -2[bp],dx",
                "mov di,bx",
                "mov word ptr -4[bp],cx",
                "mov es,dx",
                "mov es,word ptr -4[bp]",
                "ret 2",
            ],
        )
        self.assertEqual(
            MODULE.audit_vm_record_distance_call(caller, callee), []
        )

    def test_rejects_vm_record_near_pointer_call(self):
        caller = listing(
            "vm_op_c1_record_state_",
            [
                "mov ax,word ptr es:_vm_record_base_gs+2",
                "mov word ptr -2[bp],ax",
                "mov si,word ptr -4[bp]",
                "mov di,bx",
                "call near ptr ship_3d_position_distance_",
                "ret",
            ],
        )
        callee = listing(
            "ship_3d_position_distance_",
            ["mov ax,word ptr [si]", "ret"],
        )
        errors = MODULE.audit_vm_record_distance_call(caller, callee)
        self.assertTrue(any("far-pointer pairs" in error for error in errors))
        self.assertTrue(any("retain both" in error for error in errors))

    def test_accepts_preserved_ship_transition_completion(self):
        subject = listing(
            "ship_3d_target_record_select_",
            [
                "sete al",
                "movzx bx,al",
                "call framebuffer_rect_interpolate_and_remap_step_",
                "test bx,bx",
                "je L$1",
            ],
        )
        self.assertEqual(
            MODULE.audit_ship_target_transition_liveness(subject), []
        )

    def test_rejects_ship_transition_completion_left_in_ax(self):
        subject = listing(
            "ship_3d_target_record_select_",
            [
                "sete al",
                "xor ah,ah",
                "call framebuffer_rect_interpolate_and_remap_step_",
                "test ax,ax",
                "je L$1",
            ],
        )
        errors = MODULE.audit_ship_target_transition_liveness(subject)
        self.assertTrue(any("AX-clobbering" in error for error in errors))


if __name__ == "__main__":
    unittest.main()
