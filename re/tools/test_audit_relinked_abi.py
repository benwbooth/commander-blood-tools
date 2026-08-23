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


class RelinkedAbiAuditTests(unittest.TestCase):
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


if __name__ == "__main__":
    unittest.main()
