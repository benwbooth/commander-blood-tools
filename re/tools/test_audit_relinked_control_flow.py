#!/usr/bin/env python3
from __future__ import annotations

import importlib.util
import sys
import unittest
from pathlib import Path
from types import SimpleNamespace


ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location(
    "audit_relinked_control_flow",
    ROOT / "re/tools/audit_relinked_control_flow.py",
)
MODULE = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = MODULE
SPEC.loader.exec_module(MODULE)


def listing(branch: str):
    return SimpleNamespace(
        instructions=[
            SimpleNamespace(text="call list_d8c_state_le_one_"),
            SimpleNamespace(text="test ax,ax"),
            SimpleNamespace(text=branch),
        ]
    )


class RelinkedControlFlowAuditTests(unittest.TestCase):
    def test_accepts_normalized_true_branch_around_teardown(self):
        self.assertEqual(MODULE.audit_scene_dispatch(listing("jne L$1")), [])

    def test_rejects_inverted_zero_branch(self):
        errors = MODULE.audit_scene_dispatch(listing("je L$1"))
        self.assertTrue(any("skip teardown" in error for error in errors))


if __name__ == "__main__":
    unittest.main()
