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
    def make_fixture(
        self,
        root: Path,
        *,
        original_target: int = 0x0200,
        emitted_mnemonic: str = "jmp",
        linked_target: int = 0x0020,
        include_target_symbol: bool = True,
    ) -> tuple[Path, Path]:
        assembly_root = root / "assembly"
        module_assembly = assembly_root / "amer" / "callbacks"
        module_assembly.mkdir(parents=True)
        (module_assembly / "func_000100_source.asm").write_text(
            "; overlay_offset: 0x000100\n"
            "; byte_count: 3\n"
            "; routine_entry: 0x000100\n"
            "; raw stop: 0x000103\n"
            f"000100:  E9 FD 00  jmp  0x{original_target:x}\n",
            encoding="ascii",
        )
        (module_assembly / "func_000200_target.asm").write_text(
            "; overlay_offset: 0x000200\n"
            "; byte_count: 1\n"
            "; routine_entry: 0x000200\n"
            "; raw stop: 0x000201\n"
            "000200:  C3  ret\n",
            encoding="ascii",
        )

        source_root = root / "linked"
        module_dir = source_root / "amer"
        listings = module_dir / "segment_contract_listings"
        listings.mkdir(parents=True)
        image = bytearray(0x40)
        opcode = 0xE9 if emitted_mnemonic == "jmp" else 0xE8
        displacement = (linked_target - 0x0013) & 0xFFFF
        image[0x10:0x13] = bytes((opcode, displacement & 0xFF, displacement >> 8))
        image[0x13] = 0xC3
        image[0x20] = 0xC3
        (module_dir / "amer.xdb").write_bytes(image)

        target_map_line = (
            "0000:0020      xdb_amer_target_\n" if include_target_symbol else ""
        )
        (module_dir / "amer_source_link.map").write_text(
            "func_000100_source_TEXT CODE AUTO 0000:0010       00000004\n"
            "func_000200_target_TEXT CODE AUTO 0000:0020       00000001\n"
            "0000:0010      xdb_amer_source_\n"
            + target_map_line,
            encoding="ascii",
        )
        listing_opcode = "E9" if emitted_mnemonic == "jmp" else "E8"
        (listings / "func_000100_source.lst").write_text(
            "Segment: func_000100_source_TEXT BYTE USE16 00000004 bytes\n"
            "0000                          xdb_amer_source_:\n"
            f"0000    {listing_opcode} 00 00                  "
            f"{emitted_mnemonic}        xdb_amer_target_\n"
            "0003    C3                        ret\n",
            encoding="ascii",
        )
        (listings / "func_000200_target.lst").write_text(
            "Segment: func_000200_target_TEXT BYTE USE16 00000001 bytes\n"
            "0000                          xdb_amer_target_:\n"
            "0000    C3                        ret\n",
            encoding="ascii",
        )
        return assembly_root, source_root

    def audit_fixture(self, root: Path, **kwargs: object):
        assembly_root, source_root = self.make_fixture(root, **kwargs)
        return AUDIT.audit_module(
            assembly_root, source_root, "amer", include_dynamic=False
        )

    def test_repository_derives_all_fifteen_direct_sites(self) -> None:
        offsets = {}
        transfers = []
        errors = []
        for module in AUDIT.MODULES:
            module_transfers, module_errors = AUDIT.derive_original_transfers(
                AUDIT.DEFAULT_ASSEMBLY_ROOT, module
            )
            offsets[module] = {
                transfer.original_jump_offset for transfer in module_transfers
            }
            transfers.extend(module_transfers)
            errors.extend(module_errors)
        self.assertEqual([], errors)
        self.assertEqual(
            {
                "amer": {0x0BCD, 0x19BD, 0x1A28},
                "croolis": {0x0C21, 0x1804, 0x1812, 0x19CB, 0x19DA},
                "scrut": {0x0C15, 0x17F1, 0x17FF, 0x19CC, 0x1A00, 0x1A80, 0x1A8F},
            },
            offsets,
        )
        self.assertEqual(15, len(transfers))
        self.assertEqual(15, len({(item.module, item.original_jump_offset) for item in transfers}))

    def test_direct_linked_tail_jump_passes(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            results, errors = self.audit_fixture(Path(directory))
            self.assertEqual([], errors)
            self.assertEqual(1, len(results))
            self.assertEqual("tail_jump", results[0].status)
            self.assertEqual(0x0100, results[0].original_jump_offset)
            self.assertEqual(0x0010, results[0].emitted_offset)

    def test_call_then_return_requires_fingerprinted_review(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            results, errors = self.audit_fixture(
                Path(directory), emitted_mnemonic="call"
            )
            self.assertEqual([], errors)
            self.assertEqual("call_return_equivalent", results[0].status)
            reviewed, review_errors = AUDIT.apply_reviews(results, [])
            self.assertEqual("unreviewed_call_return", reviewed[0].status)
            self.assertTrue(any(
                "unreviewed CALL/RET" in error for error in review_errors
            ))

    def test_matching_call_then_return_review_passes(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            results, errors = self.audit_fixture(
                Path(directory), emitted_mnemonic="call"
            )
            self.assertEqual([], errors)
            result = results[0]
            review = AUDIT.ReviewedTransfer(
                module=result.module,
                source_symbol=result.source_symbol,
                target_symbol=result.target_symbol,
                original_jump_offset=result.original_jump_offset,
                original_target_offset=result.original_target_offset,
                source_sha256=result.source_sha256,
                target_sha256=result.target_sha256,
                epilogue=result.epilogue,
                evidence="fixture",
            )
            reviewed, review_errors = AUDIT.apply_reviews(results, [review])
            self.assertEqual([], review_errors)
            self.assertEqual(
                "reviewed_call_return_equivalent", reviewed[0].status
            )

            invalid = AUDIT.replace(review, source_sha256="0" * 64)
            reviewed, review_errors = AUDIT.apply_reviews(results, [invalid])
            self.assertEqual("invalidated_call_return", reviewed[0].status)
            self.assertTrue(any(
                "invalidated CALL/RET review" in error
                for error in review_errors
            ))

    def test_observable_call_epilogue_is_rejected(self) -> None:
        image = bytes.fromhex("40c3")
        proved, evidence = AUDIT.prove_near_call_epilogue(
            image, 0, AUDIT.CodeSegment("fixture", 0, 0, len(image))
        )
        self.assertFalse(proved)
        self.assertEqual("opcode_40", evidence)

    def test_linked_jump_target_mutation_fails(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            results, errors = self.audit_fixture(Path(directory), linked_target=0x0021)
            self.assertEqual("linked_target_mismatch", results[0].status)
            self.assertTrue(any("do not match linked bytes" in error for error in errors))

    def test_unresolved_original_target_fails_closed(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            assembly_root, _source_root = self.make_fixture(
                Path(directory), original_target=0x0300
            )
            transfers, errors = AUDIT.derive_original_transfers(assembly_root, "amer")
            self.assertEqual([], transfers)
            self.assertTrue(
                any("unresolved original cross-routine jump" in error for error in errors)
            )
            self.assertTrue(any("derived zero" in error for error in errors))

    def test_missing_recovered_target_symbol_fails_closed(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            results, errors = self.audit_fixture(
                Path(directory), include_target_symbol=False
            )
            self.assertEqual("unresolved_emitted_symbol", results[0].status)
            self.assertTrue(any("has 0 map locations" in error for error in errors))

    def test_dynamic_dispatch_prefix_mutation_still_fails(self) -> None:
        symbols = {"xdb_amer_method_slot_2_dispatch_or_init_": [(0, 0x10)]}
        image = bytearray(0x40)
        image[0x10 : 0x10 + len(AUDIT.SLOT2_PREFIX)] = AUDIT.SLOT2_PREFIX
        results, errors = AUDIT.audit_dynamic_dispatches("amer", bytes(image), symbols)
        self.assertEqual("exact_tail_prefix", results[0].status)
        self.assertEqual("unresolved_emitted_symbol", results[1].status)
        self.assertTrue(any("method_slot_13" in error for error in errors))

        image[0x10] = 0x55
        results, errors = AUDIT.audit_dynamic_dispatches("amer", bytes(image), symbols)
        self.assertEqual("prefix_mismatch", results[0].status)
        self.assertTrue(any("changes the dynamic tail contract" in error for error in errors))


if __name__ == "__main__":
    unittest.main()
