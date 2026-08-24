#!/usr/bin/env python3

from __future__ import annotations

import importlib.util
from pathlib import Path
import tempfile
import unittest


ROOT = Path(__file__).resolve().parents[2]
MODULE_PATH = ROOT / "re/tools/audit_far_pointer_lifetimes.py"
SPEC = importlib.util.spec_from_file_location("audit_far_pointer_lifetimes", MODULE_PATH)
assert SPEC is not None and SPEC.loader is not None
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


class FarPointerLifetimeAuditTests(unittest.TestCase):
    def write(self, root: Path, name: str, text: str) -> Path:
        path = root / name
        path.write_text(text, encoding="ascii")
        return path

    def test_source_rejects_nonvolatile_saved_pointer(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            path = self.write(
                Path(directory),
                "unsafe.c",
                """void f(void) {
    bloodprg_graphics_buffer_ptr saved;
    saved = graphics_back_buffer_ds;
    page_flip();
    graphics_back_buffer_ds = saved;
}
""",
            )
            self.assertEqual(len(MODULE.source_errors(path)), 1)

    def test_source_accepts_top_level_volatile_pointer(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            path = self.write(
                Path(directory),
                "safe.c",
                """void f(void) {
    bloodprg_graphics_buffer_ptr volatile saved;
    saved = graphics_back_buffer_ds;
    page_flip();
    graphics_back_buffer_ds = saved;
}
""",
            )
            self.assertEqual(MODULE.source_errors(path), [])

    def test_listing_rejects_register_restore_across_call(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            path = self.write(
                Path(directory),
                "unsafe.lst",
                """0000    8B 16 00 00               mov dx,word ptr _graphics_back_buffer_ds
0004    8B 0E 02 00               mov cx,word ptr _graphics_back_buffer_ds+0x2
0008    9A 00 00 00 00            call page_flip_
000D    89 16 00 00               mov word ptr _graphics_back_buffer_ds,dx
0011    89 0E 02 00               mov word ptr _graphics_back_buffer_ds+0x2,cx
""",
            )
            self.assertEqual(len(MODULE.listing_errors(path)), 2)

    def test_listing_accepts_stack_backed_restore(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            path = self.write(
                Path(directory),
                "safe.lst",
                """0000    A1 00 00                  mov ax,word ptr _graphics_back_buffer_ds
0003    89 46 FC                  mov word ptr -0x4[bp],ax
0006    A1 02 00                  mov ax,word ptr _graphics_back_buffer_ds+0x2
0009    89 46 FE                  mov word ptr -0x2[bp],ax
000C    9A 00 00 00 00            call page_flip_
0011    8B 46 FC                  mov ax,word ptr -0x4[bp]
0014    A3 00 00                  mov word ptr _graphics_back_buffer_ds,ax
0017    8B 46 FE                  mov ax,word ptr -0x2[bp]
001A    A3 02 00                  mov word ptr _graphics_back_buffer_ds+0x2,ax
""",
            )
            self.assertEqual(MODULE.listing_errors(path), [])

    def test_repository_source_is_clean(self) -> None:
        self.assertEqual(
            MODULE.audit(ROOT / "re/source/bloodprg/candidates", None), []
        )


if __name__ == "__main__":
    unittest.main()
