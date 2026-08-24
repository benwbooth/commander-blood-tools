#!/usr/bin/env python3

from __future__ import annotations

import importlib.util
from pathlib import Path
from types import SimpleNamespace
import sys
import tempfile
import unittest


TOOL_PATH = Path(__file__).with_name("audit_watcom_modify_contracts.py")
SPEC = importlib.util.spec_from_file_location(
    "audit_watcom_modify_contracts", TOOL_PATH
)
assert SPEC is not None and SPEC.loader is not None
AUDIT = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = AUDIT
SPEC.loader.exec_module(AUDIT)


def row(function: str) -> dict[str, str]:
    return {"function": function, "source": f"seg/func_000000_{function}.c"}


def summary(*, clobbers=(), blockers=()):
    preserved = set(AUDIT.REGISTERS.CONTRACT_REGISTERS) - set(clobbers)
    return SimpleNamespace(
        preserved=frozenset(preserved), blockers=frozenset(blockers)
    )


class WatcomModifyContractTests(unittest.TestCase):
    def test_preprocessor_and_continuation_select_runtime_contract(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "contracts.h").write_text(
                "#if defined(__WATCOMC__)\n"
                "#if !defined(BLOODPRG_RELINKED_RUNTIME)\n"
                "#pragma aux subject modify exact []\n"
                "#else\n"
                "#pragma aux subject \\\n"
                "    parm [ax] modify exact [ax es]\n"
                "#endif\n"
                "#endif\n",
                encoding="ascii",
            )
            contracts = AUDIT.parse_contracts(root)
        self.assertEqual(frozenset(("AX", "ES")), contracts["subject"].modifies)
        self.assertEqual(5, contracts["subject"].line)

    def test_underdeclared_clobber_fails(self):
        contract = AUDIT.ModifyContract(
            "subject", frozenset(("AX",)), Path("subject.h"), 7
        )
        results = AUDIT.audit_contracts(
            {"subject": contract}, [row("subject")],
            {"func_000000_subject": summary(clobbers=("AX", "ES"))},
        )
        self.assertEqual("underdeclared", results[0].status)
        self.assertEqual("ES", results[0].underdeclared)

    def test_overdeclared_clobber_is_conservative(self):
        contract = AUDIT.ModifyContract(
            "subject", frozenset(("AX", "ES")), Path("subject.h"), 7
        )
        results = AUDIT.audit_contracts(
            {"subject": contract}, [row("subject")],
            {"func_000000_subject": summary(clobbers=("AX",))},
        )
        self.assertEqual("pass", results[0].status)
        self.assertEqual("ES", results[0].overdeclared)

    def test_unresolved_transitive_effect_fails_closed(self):
        contract = AUDIT.ModifyContract(
            "subject", frozenset(("AX",)), Path("subject.h"), 7
        )
        results = AUDIT.audit_contracts(
            {"subject": contract}, [row("subject")],
            {"func_000000_subject": summary(blockers=("unknown callback",))},
        )
        self.assertEqual("unresolved", results[0].status)
        self.assertIn("unknown callback", results[0].blockers)

    def test_ship_band_contract_covers_natural_c_clobbers(self):
        header_dir = TOOL_PATH.parents[1] / "source/bloodprg/candidates/include"
        contract = AUDIT.parse_contracts(header_dir)["ship_3d_plane_band_copy"]
        self.assertEqual(frozenset(("AX", "ES")), contract.modifies)


if __name__ == "__main__":
    unittest.main()
