#!/usr/bin/env python3

from __future__ import annotations

import importlib.util
from pathlib import Path
import sys
import tempfile
import unittest


TOOL_PATH = Path(__file__).with_name("audit_xdb_tail_transfers.py")
SPEC = importlib.util.spec_from_file_location("audit_xdb_tail_transfers", TOOL_PATH)
assert SPEC is not None and SPEC.loader is not None
AUDIT = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = AUDIT
SPEC.loader.exec_module(AUDIT)


class TailTransferAuditTests(unittest.TestCase):
    def make_module(self, root: Path, module: str, corrupt: bool = False) -> None:
        module_dir = root / module
        module_dir.mkdir(parents=True)
        image = bytearray(0x80)
        map_lines: list[str] = []
        offset = 0x10
        for symbol, prefix in AUDIT.MODULE_SYMBOLS[module]:
            payload = bytearray(prefix)
            if corrupt:
                payload[0] = 0x55
                corrupt = False
            image[offset : offset + len(payload)] = payload
            map_lines.append(f"0000:{offset:04x}      {symbol}\n")
            offset += 0x20
        (module_dir / f"{module}.xdb").write_bytes(image)
        (module_dir / f"{module}_source_link.map").write_text(
            "".join(map_lines), encoding="ascii"
        )

    def test_exact_prefixes_pass(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            self.make_module(root, "amer")
            results, errors = AUDIT.audit_module(root, "amer")
            self.assertEqual([], errors)
            self.assertEqual(
                ["exact_tail_prefix", "exact_tail_prefix"],
                [result.status for result in results],
            )

    def test_stack_prologue_prefix_fails(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            self.make_module(root, "croolis", corrupt=True)
            results, errors = AUDIT.audit_module(root, "croolis")
            self.assertEqual("prefix_mismatch", results[0].status)
            self.assertIn("changes the callback tail contract", errors[0])

    def test_duplicate_map_symbol_fails(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            self.make_module(root, "scrut")
            map_path = root / "scrut" / "scrut_source_link.map"
            original = map_path.read_text(encoding="ascii")
            map_path.write_text(
                original + original.splitlines(True)[0], encoding="ascii"
            )
            results, errors = AUDIT.audit_module(root, "scrut")
            self.assertEqual("missing_symbol", results[0].status)
            self.assertIn("2 map locations", errors[0])


if __name__ == "__main__":
    unittest.main()
