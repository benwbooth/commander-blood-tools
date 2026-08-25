#!/usr/bin/env python3

from __future__ import annotations

import importlib.util
from pathlib import Path
import sys
import tempfile
import unittest


ROOT = Path(__file__).resolve().parents[2]
SCRIPT = ROOT / "re/tools/audit_inline_assembly.py"
SPEC = importlib.util.spec_from_file_location("audit_inline_assembly", SCRIPT)
assert SPEC is not None and SPEC.loader is not None
AUDIT = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = AUDIT
SPEC.loader.exec_module(AUDIT)


class InlineAssemblyAuditTests(unittest.TestCase):
    def scan(self, source: str):
        with tempfile.NamedTemporaryFile(
            mode="w", encoding="ascii", suffix=".c", delete=False
        ) as handle:
            handle.write(source)
            path = Path(handle.name)
        try:
            return AUDIT.scan_file(path)
        finally:
            path.unlink()

    def test_abi_only_pragma_is_not_code(self) -> None:
        self.assertEqual([], self.scan(
            "#pragma aux subject parm [ax] value [ax] modify exact [ax]\n"
        ))

    def test_multiline_code_pragma_is_counted(self) -> None:
        blocks = self.scan(
            '#pragma aux subject = \\\n                    "mov ax,bx" \\\n                    "done:" \\\n                    "ret" modify exact [ax]\n'
        )
        self.assertEqual(1, len(blocks))
        self.assertEqual(("mov ax,bx", "done:", "ret"), blocks[0].instructions)
        self.assertEqual(2, blocks[0].instruction_count)

    def test_direct_asm_is_counted(self) -> None:
        blocks = self.scan("void f(void) { __asm { nop } }\n")
        self.assertEqual("<direct-asm>", blocks[0].function)

    def test_unreviewed_code_fails(self) -> None:
        block = AUDIT.AssemblyBlock(
            ROOT / "re/source/xdb/candidates/unreviewed.c",
            7,
            "game_rule",
            ("inc ax",),
        )
        self.assertIn("unreviewed code-emitting assembly", AUDIT.audit_blocks([block])[0])

    def test_modified_reviewed_block_fails(self) -> None:
        relative, function = next(iter(AUDIT.ALLOWED_BLOCKS))
        block = AUDIT.AssemblyBlock(
            ROOT / relative,
            1,
            function,
            ("nop",),
        )
        self.assertIn("changed", AUDIT.audit_blocks([block])[0])

    def test_production_sources_have_only_reviewed_platform_assembly(self) -> None:
        blocks = [
            block
            for path in AUDIT.source_files()
            for block in AUDIT.scan_file(path)
        ]
        self.assertEqual([], AUDIT.audit_blocks(blocks))
        self.assertEqual(10, len(blocks))
        self.assertEqual(42, sum(block.instruction_count for block in blocks))


if __name__ == "__main__":
    unittest.main()
