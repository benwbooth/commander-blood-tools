#!/usr/bin/env python3
from __future__ import annotations

import importlib.util
import sys
import tempfile
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location(
    "audit_code_data_ownership",
    ROOT / "re/tools/audit_code_data_ownership.py",
)
MODULE = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = MODULE
SPEC.loader.exec_module(MODULE)


class CodeDataOwnershipAuditTests(unittest.TestCase):
    def test_rejects_data_segment_owner(self):
        with tempfile.TemporaryDirectory() as directory:
            listing_dir = Path(directory)
            for symbol, owner in MODULE.CODE_DATA_OWNERS.items():
                segment = "_DATA" if symbol == "_ems_device_signature" else "owner_TEXT"
                with (listing_dir / owner).open("a", encoding="ascii") as handle:
                    handle.write(f"Segment: {segment} BYTE USE16 00000002 bytes\n")
                    handle.write(f"0000                          {symbol}:\n")
            errors = MODULE.audit(listing_dir)
        self.assertTrue(any("_ems_device_signature is in _DATA" in error for error in errors))

    def test_rejects_unqualified_direct_reference(self):
        with tempfile.TemporaryDirectory() as directory:
            listing_dir = Path(directory)
            for symbol, owner in MODULE.CODE_DATA_OWNERS.items():
                with (listing_dir / owner).open("a", encoding="ascii") as handle:
                    handle.write("Segment: owner_TEXT BYTE USE16 00000002 bytes\n")
                    handle.write(f"0000                          {symbol}:\n")
            owner = listing_dir / MODULE.CODE_DATA_OWNERS["_ems_device_signature"]
            with owner.open("a", encoding="ascii") as handle:
                handle.write(
                    "0002    3A 87 00 00               "
                    "cmp al,byte ptr _ems_device_signature[bx]\n"
                )
            errors = MODULE.audit(listing_dir)
        self.assertTrue(any("non-CS access to _ems_device_signature" in error for error in errors))


if __name__ == "__main__":
    unittest.main()
