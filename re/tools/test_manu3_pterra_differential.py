#!/usr/bin/env python3
"""Tests for the deterministic MANU3 Pterra renderer oracle."""

from __future__ import annotations

import importlib.util
from pathlib import Path
import struct
import sys
import unittest


TOOL = Path(__file__).with_name("manu3_pterra_differential.py")
SPEC = importlib.util.spec_from_file_location(
    "manu3_pterra_differential", TOOL
)
assert SPEC is not None and SPEC.loader is not None
oracle = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = oracle
SPEC.loader.exec_module(oracle)


class Manu3PterraDifferentialTests(unittest.TestCase):
    def test_recovers_renderer_entry_from_link_map(self) -> None:
        link_map = """
0000:0010      unrelated_symbol_
0000:0b3e      xdb_manu3_face_bucket_sort_
"""
        self.assertEqual(oracle.recovered_renderer_entry(link_map), 0x0B3E)

    def test_rejects_missing_renderer_symbol(self) -> None:
        with self.assertRaisesRegex(ValueError, "found 0"):
            oracle.recovered_renderer_entry("0000:0010 other_symbol_\n")

    def test_rejects_renderer_in_nonzero_segment(self) -> None:
        link_map = "0001:0b3e      xdb_manu3_face_bucket_sort_\n"
        with self.assertRaisesRegex(ValueError, "unexpected segment"):
            oracle.recovered_renderer_entry(link_map)

    def test_relocates_renderer_segments_and_framebuffer(self) -> None:
        globals_image = bytearray(0x10000)
        raster_image = bytearray(0x10000)
        renderer_input = oracle.PterraRendererInput(
            globals_image=bytes(globals_image),
            geometry_image=bytes(0x10000),
            texture_image=bytes(0x10000),
            raster_image=bytes(raster_image),
            source_segments=(1, 2, 3, 4),
        )

        relocated_globals, relocated_raster = oracle.relocated_input(
            renderer_input
        )

        self.assertEqual(
            struct.unpack_from("<HHH", relocated_globals, 2),
            (
                oracle.GEOMETRY_SEGMENT,
                oracle.TEXTURE_SEGMENT,
                oracle.RASTER_SEGMENT,
            ),
        )
        self.assertEqual(
            struct.unpack_from("<H", relocated_globals, 0x14)[0],
            oracle.FRAMEBUFFER_SEGMENT,
        )
        self.assertEqual(
            struct.unpack_from(
                "<H", relocated_raster, oracle.RENDER_CONTINUATION_OFFSET
            )[0],
            oracle.ORIGINAL_RENDER_LINEAR_OFFSET,
        )

    def test_reports_first_sixteen_region_differences(self) -> None:
        expected = bytes(range(32))
        actual = bytes(value ^ 1 for value in expected)
        differences = oracle.region_differences(expected, actual)
        self.assertEqual(len(differences), 16)
        self.assertEqual(
            differences[0], {"offset": 0, "original": 0, "recovered": 1}
        )

    def test_face_list_and_count_offsets_are_not_adjacent(self) -> None:
        self.assertEqual(oracle.FACE_LIST_OFFSET, 0x2300)
        self.assertEqual(oracle.FACE_COUNT_OFFSET, 0x2304)

    def test_normalizes_only_documented_ephemeral_raster_cells(self) -> None:
        image = bytearray(0x10000)
        image[0x061B:0x0635] = bytes(range(0x1A))
        image[0x066F:0x067F] = bytes(range(0x10))
        image[0x0681:0x0685] = b"ABCD"
        first_sort_next = (
            oracle.RASTER_POOL_OFFSET + oracle.RASTER_SORT_NEXT_OFFSET
        )
        second_sort_next = first_sort_next + oracle.RASTER_RECORD_SIZE
        image[first_sort_next:first_sort_next + 2] = b"EF"
        image[second_sort_next:second_sort_next + 2] = b"GH"

        normalized = oracle.normalize_raster_ephemeral(bytes(image))

        self.assertEqual(normalized[0x061B], image[0x061B])
        self.assertEqual(normalized[0x061C:0x0634], bytes(0x18))
        self.assertEqual(normalized[0x0634], image[0x0634])
        self.assertEqual(normalized[0x066F], image[0x066F])
        self.assertEqual(normalized[0x0670:0x067E], bytes(0x0E))
        self.assertEqual(normalized[0x067E], image[0x067E])
        self.assertEqual(normalized[0x0681], ord("A"))
        self.assertEqual(normalized[0x0682:0x0684], b"\0\0")
        self.assertEqual(normalized[0x0684], ord("D"))
        self.assertEqual(normalized[first_sort_next:first_sort_next + 2], b"\0\0")
        self.assertEqual(normalized[second_sort_next:second_sort_next + 2], b"\0\0")
        self.assertEqual(normalized[first_sort_next - 1], image[first_sort_next - 1])


if __name__ == "__main__":
    unittest.main()
