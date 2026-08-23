#!/usr/bin/env python3
from __future__ import annotations

import os
from pathlib import Path
import sys


ROOT = Path(__file__).resolve().parents[2]
_HERE = Path(__file__).resolve().parent
sys.path[:] = [
    path for path in sys.path if Path(os.path.abspath(path or os.curdir)) != _HERE
]

import csv
from dataclasses import dataclass
import hashlib
import importlib.util
import io
import tempfile
import unittest


SPEC = importlib.util.spec_from_file_location(
    "audit_xdb_callback_edges",
    ROOT / "re" / "tools" / "audit_xdb_callback_edges.py",
)
MODULE = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = MODULE
SPEC.loader.exec_module(MODULE)


@dataclass(frozen=True)
class Routine:
    entry: int
    function: str
    body: bytes
    source_target: str | None = None
    source_field: str = "state+0x0e"


def state_store(target: int) -> bytes:
    return bytes((0xC7, 0x44, 0x0E, target & 0xFF, target >> 8, 0xC3))


def context_store(value: int) -> bytes:
    return bytes((0xC7, 0x45, 0x36, value & 0xFF, value >> 8, 0xC3))


def source_text(routine: Routine) -> str:
    assignment = ""
    if routine.source_target is not None:
        if routine.source_field == "state+0x0e":
            assignment = f"    state->callback = {routine.source_target};\n"
        else:
            assignment = f"    context->control.resume = {routine.source_target};\n"
    return (
        f"void {routine.function}(void)\n"
        "{\n"
        f"{assignment}"
        "}\n"
    )


def sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


class Fixture:
    def __init__(self, directory: Path, module: str, routines: list[Routine]):
        self.root = directory
        self.module = module
        self.short = module.removeprefix("xdb_")
        self.filename, self.code_end = MODULE.MODULE_SPECS[module]
        self.routines = routines
        self.raw_dir = self.root / "raw"
        self.index = self.root / "re" / "assembly" / "routine_index.tsv"
        self.manifest = (
            self.root / "re" / "source" / "xdb" / "candidates" / "manifest.tsv"
        )

    def write(self, owned_entries: set[int] | None = None) -> MODULE.AuditConfig:
        owned_entries = (
            {routine.entry for routine in self.routines}
            if owned_entries is None
            else owned_entries
        )
        image = bytearray(self.code_end)
        for routine in self.routines:
            image[routine.entry : routine.entry + len(routine.body)] = routine.body
        self.raw_dir.mkdir(parents=True)
        raw_path = self.raw_dir / self.filename
        raw_path.write_bytes(image)
        artifact_hash = sha256(image)

        index_rows = []
        manifest_rows = []
        for routine in self.routines:
            if routine.entry not in owned_entries:
                continue
            asm_relative = Path(
                f"re/assembly/xdb/{self.short}/callback_state_machine/"
                f"func_{routine.entry:06x}_fixture.asm"
            )
            asm_path = self.root / asm_relative
            asm_path.parent.mkdir(parents=True, exist_ok=True)
            asm_path.write_text(
                "; fixture assembly\n"
                f"; module: {self.module}\n"
                f"; artifact: raw/{self.filename}\n"
                f"; artifact_sha256: {artifact_hash}\n"
                f"; overlay_offset: 0x{routine.entry:06X}\n"
                f"; byte_count: {len(routine.body)}\n"
                f"; routine_bytes_sha256: {sha256(routine.body)}\n"
                f"; routine_entry: 0x{routine.entry:06X}\n\n"
                f"{routine.entry:08X}:  {routine.body.hex(' ')}  fixture\n",
                encoding="ascii",
            )
            source_relative = Path(
                f"{self.short}/func_{routine.entry:06x}_fixture.c"
            )
            source_path = self.manifest.parent / source_relative
            source_path.parent.mkdir(parents=True, exist_ok=True)
            source_path.write_text(source_text(routine), encoding="ascii")
            index_rows.append(
                {
                    "module": self.module,
                    "entry": f"0x{routine.entry:06x}",
                    "group": "callback_state_machine",
                    "provenance": "fixture",
                    "labels": routine.function,
                    "asm_path": str(asm_relative),
                    "boundary": "fixture_owner",
                }
            )
            manifest_rows.append(
                {
                    "entry": f"{self.module}:0x{routine.entry:06x}",
                    "source": str(source_relative),
                    "asm_path": str(asm_relative),
                    "function": routine.function,
                    "status": "fixture",
                    "notes": "fixture",
                }
            )

        self.index.parent.mkdir(parents=True, exist_ok=True)
        with self.index.open("w", newline="", encoding="ascii") as handle:
            writer = csv.DictWriter(
                handle,
                fieldnames=(
                    "module",
                    "entry",
                    "group",
                    "provenance",
                    "labels",
                    "asm_path",
                    "boundary",
                ),
                delimiter="\t",
                lineterminator="\n",
            )
            writer.writeheader()
            writer.writerows(index_rows)
        self.manifest.parent.mkdir(parents=True, exist_ok=True)
        with self.manifest.open("w", newline="", encoding="ascii") as handle:
            writer = csv.DictWriter(
                handle,
                fieldnames=(
                    "entry",
                    "source",
                    "asm_path",
                    "function",
                    "status",
                    "notes",
                ),
                delimiter="\t",
                lineterminator="\n",
            )
            writer.writeheader()
            writer.writerows(manifest_rows)
        include = self.manifest.parent / "include"
        include.mkdir()
        pragmas = [
            f"#pragma aux {routine.function} parm [si] [di]\n"
            for routine in self.routines
            if routine.entry in owned_entries
        ]
        (include / "xdb_alien.h").write_text("".join(pragmas), encoding="ascii")
        return MODULE.AuditConfig(
            self.root,
            self.index,
            self.manifest,
            self.raw_dir,
        )


class CallbackEdgeAuditTests(unittest.TestCase):
    def fixture_result(
        self,
        module: str,
        routines: list[Routine],
        owned_entries: set[int] | None = None,
    ):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        fixture = Fixture(Path(temporary.name), module, routines)
        config = fixture.write(owned_entries)
        return MODULE.audit_module(config, module)

    def test_rejects_former_slot3_wave_target_substitutions(self):
        cases = (
            (
                "xdb_amer",
                0x1414,
                0x0C81,
                "xdb_amer_slot3_update",
                "xdb_amer_slot1_motion_update",
                "xdb_amer_slot1_wave_update",
            ),
            (
                "xdb_croolis",
                0x146C,
                0x0CD9,
                "xdb_croolis_slot3_update",
                "xdb_croolis_slot1_motion_update",
                "xdb_croolis_slot1_wave_update",
            ),
            (
                "xdb_scrut",
                0x145A,
                0x0CC7,
                "xdb_scrut_slot3_update",
                "xdb_scrut_slot1_motion_update",
                "xdb_scrut_slot1_wave_update",
            ),
        )
        for module, writer, target, writer_name, motion_name, wrong_name in cases:
            with self.subTest(module=module):
                result = self.fixture_result(
                    module,
                    [
                        Routine(
                            writer,
                            writer_name,
                            state_store(target),
                            source_target=wrong_name,
                        ),
                        Routine(target, motion_name, b"\xC3"),
                    ],
                )
                self.assertTrue(
                    any("target mismatch" in error for error in result.errors),
                    result.errors,
                )
                self.assertEqual(result.stores[0].target_function, motion_name)
                self.assertEqual(result.stores[0].status, "source_target_mismatch")

    def test_closes_formerly_missing_croolis_and_scrut_chains(self):
        cases = (
            (
                "xdb_croolis",
                (
                    (0x146C, 0x0CD9),
                    (0x0CD9, 0x0CF9),
                    (0x0CF9, 0x0C3E),
                    (0x0C3E, 0x0B78),
                    (0x0B78, 0x0C24),
                ),
            ),
            (
                "xdb_scrut",
                (
                    (0x145A, 0x0CC7),
                    (0x0CC7, 0x0CE7),
                    (0x0CE7, 0x0C32),
                    (0x0C32, 0x0B78),
                    (0x0B78, 0x0C18),
                ),
            ),
        )
        for module, chain in cases:
            with self.subTest(module=module):
                target_entries = {target for _, target in chain}
                function = lambda entry: f"{module}_callback_{entry:04x}"
                routines = [
                    Routine(
                        writer,
                        function(writer),
                        state_store(target),
                        source_target=function(target),
                    )
                    for writer, target in chain
                ]
                terminal = chain[-1][1]
                routines.append(Routine(terminal, function(terminal), b"\xC3"))
                result = self.fixture_result(module, routines)
                self.assertEqual(result.errors, ())
                self.assertEqual(
                    {store.value for store in result.stores}, target_entries
                )
                self.assertTrue(
                    all(store.status == "owned_pointer" for store in result.stores)
                )

    def test_missing_chain_ownership_fails_but_discovery_continues(self):
        chain = (
            (0x146C, 0x0CD9),
            (0x0CD9, 0x0CF9),
            (0x0CF9, 0x0C3E),
            (0x0C3E, 0x0B78),
            (0x0B78, 0x0C24),
        )
        function = lambda entry: f"xdb_croolis_callback_{entry:04x}"
        routines = [
            Routine(
                writer,
                function(writer),
                state_store(target),
                source_target=function(target),
            )
            for writer, target in chain
        ]
        routines.append(Routine(0x0C24, function(0x0C24), b"\xC3"))
        result = self.fixture_result(
            "xdb_croolis", routines, owned_entries={0x146C}
        )
        self.assertEqual(
            {store.value for store in result.stores},
            {target for _, target in chain},
        )
        for target in {target for _, target in chain}:
            self.assertTrue(
                any(f"0x{target:06x}" in error for error in result.errors),
                (target, result.errors),
            )

    def test_context_control_scalars_are_not_callback_targets(self):
        body = b"".join(
            context_store(value)[:-1]
            for value in (0x0000, 0x0001, 0x8001, 0xFFFF)
        ) + b"\xC3"
        result = self.fixture_result(
            "xdb_amer", [Routine(0x1000, "xdb_amer_context_states", body)]
        )
        self.assertEqual(result.errors, ())
        self.assertEqual(len(result.stores), 4)
        self.assertTrue(
            all(store.classification == "context_scalar" for store in result.stores)
        )

    def test_unresolved_state_pointer_fails_closed(self):
        result = self.fixture_result(
            "xdb_amer",
            [
                Routine(
                    0x1414,
                    "xdb_amer_slot3_update",
                    state_store(0x4000),
                    source_target="xdb_amer_outside_code",
                )
            ],
        )
        self.assertTrue(any("outside code" in error for error in result.errors))
        self.assertEqual(result.stores[0].status, "unresolved_pointer")

    def test_missing_callback_register_abi_fails(self):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        fixture = Fixture(
            Path(temporary.name),
            "xdb_amer",
            [
                Routine(
                    0x1414,
                    "xdb_amer_slot3_update",
                    state_store(0x0C81),
                    source_target="xdb_amer_slot1_motion_update",
                ),
                Routine(0x0C81, "xdb_amer_slot1_motion_update", b"\xC3"),
            ],
        )
        config = fixture.write()
        header = fixture.manifest.parent / "include" / "xdb_alien.h"
        header.write_text(
            "#pragma aux xdb_amer_slot3_update parm [si] [di]\n",
            encoding="ascii",
        )
        result = MODULE.audit_module(config, "xdb_amer")
        self.assertTrue(
            any("missing explicit" in error for error in result.errors),
            result.errors,
        )
        self.assertEqual(result.stores[0].status, "callback_abi_missing")

    def test_tsv_is_deterministic_and_sorted(self):
        stores = [
            MODULE.CallbackStore(
                "xdb_scrut", 0x20, "state+0x0e", "pointer", 0x30
            ),
            MODULE.CallbackStore(
                "xdb_amer", 0x10, "context+0x36", "context_scalar", 1
            ),
        ]
        first = MODULE.render_tsv(stores)
        second = MODULE.render_tsv(reversed(stores))
        self.assertEqual(first, second)
        rows = list(csv.DictReader(io.StringIO(first), delimiter="\t"))
        self.assertEqual([row["module"] for row in rows], ["xdb_amer", "xdb_scrut"])


if __name__ == "__main__":
    unittest.main()
