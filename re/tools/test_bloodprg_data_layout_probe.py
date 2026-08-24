#!/usr/bin/env python3
"""Regression tests for BLOODPRG global ownership extraction."""

from __future__ import annotations

import importlib.util
import tempfile
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
MODULE_PATH = ROOT / "re" / "tools" / "bloodprg_data_layout_probe.py"
SPEC = importlib.util.spec_from_file_location("bloodprg_data_layout_probe", MODULE_PATH)
assert SPEC is not None and SPEC.loader is not None
PROBE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(PROBE)


class DeclarationParsingTests(unittest.TestCase):
    def test_standalone_comment_belongs_to_following_declaration(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            header = Path(directory) / "layout.h"
            header.write_text(
                "/* DS:0x1111 */\n"
                "extern volatile unsigned first[];\n"
                "/* DS:0x2222 */\n"
                "extern volatile unsigned second[];\n",
                encoding="ascii",
            )
            declarations = PROBE.declarations(Path(directory))

        self.assertEqual(declarations["_first"].offset, 0x1111)
        self.assertEqual(declarations["_second"].offset, 0x2222)

    def test_same_line_comment_belongs_to_current_declaration(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            header = Path(directory) / "layout.h"
            header.write_text(
                "extern volatile unsigned first; /* GS:0x1234 */\n"
                "extern volatile unsigned second; /* DS:0x5678 */\n",
                encoding="ascii",
            )
            declarations = PROBE.declarations(Path(directory))

        self.assertEqual(declarations["_first"].offset, 0x1234)
        self.assertEqual(declarations["_first"].segment, "GAME_DATA")
        self.assertEqual(declarations["_second"].offset, 0x5678)

    def test_ship_scratch_arrays_have_original_offsets(self) -> None:
        declarations = PROBE.declarations(PROBE.DEFAULT_HEADERS)
        expected = {
            "_vm_resource_profiles": 0x11F4,
            "_vm_arche_position_match_offsets": 0x24FB,
            "_ship_3d_presentable_name_offsets": 0x250B,
            "_nav_kind2_target_offsets": 0x2B13,
            "_ship_3d_nav_source_offsets": 0x6886,
            "_ship_3d_navigation_candidate_offsets": 0x2B53,
            "_ship_3d_hud_layout": 0x2BC7,
            "_selected_mask_rows": 0x7BB8,
        }
        self.assertEqual(
            {symbol: declarations[symbol].offset for symbol in expected},
            expected,
        )


if __name__ == "__main__":
    unittest.main()
