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
    def test_rejects_sound_access_through_caller_ds(self):
        subject = listing(
            "snd_play_clip_",
            [
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


if __name__ == "__main__":
    unittest.main()
