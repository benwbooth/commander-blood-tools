#!/usr/bin/env python3

from __future__ import annotations

import importlib.util
import json
from pathlib import Path
import sys
import unittest


TOOL_PATH = Path(__file__).with_name("big_bug_bang_field_display_audit.py")
REPORT_PATH = TOOL_PATH.parents[1] / "big_bug_bang_field_display_audit.json"
SPEC = importlib.util.spec_from_file_location(
    "big_bug_bang_field_display_audit", TOOL_PATH
)
assert SPEC is not None and SPEC.loader is not None
AUDIT = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = AUDIT
SPEC.loader.exec_module(AUDIT)


class BigBugBangFieldDisplayAuditTests(unittest.TestCase):
    def test_selector_spec_covers_the_complete_sequel_display_table(self):
        table = next(
            table
            for table in AUDIT.ATLAS.TABLES
            if table["name"] == "field_display_handlers"
        )
        self.assertEqual(table["count"], len(AUDIT.FIELD_NAMES))
        self.assertEqual(table["selector_base"], 1)
        self.assertEqual(AUDIT.FIELD_NAMES[0], "POP")
        self.assertEqual(AUDIT.FIELD_NAMES[-1], "ATTAQUE")

    def test_every_dispatch_target_has_one_structural_formatter_role(self):
        table = next(
            table
            for table in AUDIT.ATLAS.TABLES
            if table["name"] == "field_display_handlers"
        )

        class SyntheticMZ:
            header_size = 0x800
            image_total = 0x10000
            data = bytes(0x7C3B) + b"".join(
                (target - 0x5820).to_bytes(2, "little")
                for target in AUDIT.HANDLER_FORMATS
            )

            @staticmethod
            def segoff_to_file(_segment, _offset):
                raise AssertionError("near table must not resolve a far pointer")

        synthetic = SyntheticMZ()
        definition = dict(table, count=len(AUDIT.HANDLER_FORMATS))
        entries, _targets = AUDIT.ATLAS.table_entries(synthetic, definition)
        decoded = {
            int(str(entry["target_file_offset"]), 16) for entry in entries
        }
        self.assertEqual(decoded, set(AUDIT.HANDLER_FORMATS))

    def test_known_object_list_helper_is_exclusively_owned_by_the_inspector(self):
        report = json.loads(REPORT_PATH.read_text())
        self.assertEqual(
            report["display"]["exclusive_helpers"],
            [
                {
                    "entry": "0x00669f",
                    "end": "0x0066ed",
                    "body_sha256": AUDIT.KNOWN_OBJECT_LIST_HELPER_SHA256,
                    "role": "known_object_bit_list",
                    "owner_handler": "0x007d83",
                    "direct_callers": ["0x007d83"],
                    "direct_callees": ["0x006633"],
                    "static_dispatch_target": False,
                }
            ],
        )


if __name__ == "__main__":
    unittest.main()
