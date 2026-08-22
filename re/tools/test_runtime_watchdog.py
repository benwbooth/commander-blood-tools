#!/usr/bin/env python3
"""Deterministic tests for the DOS runtime watchdog's structural checks."""
from __future__ import annotations

import os
import sys

_TOOL_DIRECTORY = os.path.dirname(os.path.abspath(__file__))
if sys.path and os.path.abspath(sys.path[0]) == _TOOL_DIRECTORY:
    del sys.path[0]

import importlib.util
import io
import struct
import tempfile
import unittest
from pathlib import Path


WATCHDOG_PATH = Path(__file__).with_name("runtime_watchdog.py")
SPEC = importlib.util.spec_from_file_location("runtime_watchdog", WATCHDOG_PATH)
assert SPEC is not None and SPEC.loader is not None
watchdog = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = watchdog
SPEC.loader.exec_module(watchdog)


MEMORY_SIZE = 0x100000
CHAIN_START = 0x0100
PROGRAM_MCB = 0x0200
PSP = PROGRAM_MCB + 1
FINAL_MCB = 0x0301


def put_mcb(
    memory: bytearray,
    segment: int,
    kind: str,
    owner: int,
    paragraphs: int,
    name: bytes = b"",
) -> None:
    address = segment * 16
    memory[address] = ord(kind)
    struct.pack_into("<HH", memory, address + 1, owner, paragraphs)
    memory[address + 8 : address + 16] = name.ljust(8, b"\0")


def valid_memory() -> bytearray:
    memory = bytearray(MEMORY_SIZE)
    put_mcb(memory, CHAIN_START, "M", 8, 0x00FF, b"DOS")
    put_mcb(memory, PROGRAM_MCB, "M", PSP, 0x0100, b"BPRG_RE")
    put_mcb(memory, FINAL_MCB, "Z", 0, 0x0010)
    return memory


class McbChainTests(unittest.TestCase):
    def test_discovers_complete_chain_and_game_owned_segment(self) -> None:
        blocks = watchdog.discover_mcb_chain(valid_memory(), PROGRAM_MCB, PSP)
        self.assertEqual([block.segment for block in blocks], [0x0100, 0x0200, 0x0301])
        owner = watchdog.program_owned_block(blocks, 0x0250, PSP)
        self.assertIsNotNone(owner)
        self.assertEqual(owner.segment, PROGRAM_MCB)

    def test_rejects_corrupt_successor_signature(self) -> None:
        memory = valid_memory()
        memory[FINAL_MCB * 16] = 0
        with self.assertRaisesRegex(watchdog.McbError, "invalid MCB type"):
            watchdog.parse_mcb_chain(memory, CHAIN_START, PROGRAM_MCB)

    def test_rejects_wrong_program_owner(self) -> None:
        memory = valid_memory()
        struct.pack_into("<H", memory, PROGRAM_MCB * 16 + 1, 0x9999)
        with self.assertRaisesRegex(watchdog.McbError, "is not owned by PSP"):
            watchdog.discover_mcb_chain(memory, PROGRAM_MCB, PSP)

    def test_rejects_chain_that_omits_program_header(self) -> None:
        with self.assertRaisesRegex(watchdog.McbError, "omits required header"):
            watchdog.parse_mcb_chain(valid_memory(), FINAL_MCB, PROGRAM_MCB)


class LinkLayoutTests(unittest.TestCase):
    def test_reads_zero_based_runtime_segments(self) -> None:
        text = (
            "GAME_DATA FAR_DATA DGROUP 105b:0000 00007c78\n"
            "FS_DATA FAR_DATA AUTO 1823:0000 00001230\n"
        )
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "link.map"
            path.write_text(text, encoding="ascii")
            self.assertEqual(
                watchdog.parse_segment_layout(path),
                watchdog.SegmentLayout(game_data=0x105B, fs_data=0x1823),
            )

    def test_rejects_nonzero_segment_base(self) -> None:
        text = (
            "GAME_DATA FAR_DATA DGROUP 105b:0000 00007c78\n"
            "FS_DATA FAR_DATA AUTO 1823:0008 00001230\n"
        )
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "link.map"
            path.write_text(text, encoding="ascii")
            with self.assertRaisesRegex(watchdog.WatchdogError, "not zero"):
                watchdog.parse_segment_layout(path)


class CpuStateTests(unittest.TestCase):
    def test_reads_dosbox_x_interleaved_segment_records(self) -> None:
        memory = io.BytesIO(bytes(0x200))
        registers = (1, 2, 3, 4, 5, 6, 7, 8)
        memory.seek(0x100)
        memory.write(struct.pack("<8II", *registers, 0x1234))
        for index, value in enumerate((0x10, 0x20, 0x30, 0x40, 0x50, 0x60)):
            memory.seek(0x40 + index * 8)
            memory.write(struct.pack("<Q", value))

        state = watchdog.read_cpu_state(
            memory, {"cpu_regs": 0x100, "Segs": 0x40, "Segs_size": 0x88}
        )

        self.assertEqual(
            [state[name] for name in ("es", "cs", "ss", "ds", "fs", "gs")],
            [0x10, 0x20, 0x30, 0x40, 0x50, 0x60],
        )
        self.assertEqual(state["ip"], 0x1234)

    def test_reads_dosbox_staging_segment_arrays(self) -> None:
        memory = io.BytesIO(bytes(0x200))
        memory.seek(0x100)
        memory.write(struct.pack("<8II", *range(1, 9), 0x5678))
        memory.seek(0x40)
        memory.write(
            struct.pack(
                "<8H8I",
                0x10,
                0x20,
                0x30,
                0x40,
                0x50,
                0x60,
                0,
                0,
                *range(8),
            )
        )

        state = watchdog.read_cpu_state(
            memory, {"cpu_regs": 0x100, "Segs": 0x40, "Segs_size": 0x30}
        )

        self.assertEqual(
            [state[name] for name in ("es", "cs", "ss", "ds", "fs", "gs")],
            [0x10, 0x20, 0x30, 0x40, 0x50, 0x60],
        )
        self.assertEqual(state["ip"], 0x5678)


class InterruptVectorTests(unittest.TestCase):
    def test_reports_only_changed_vectors(self) -> None:
        before = bytearray(0x400)
        after = bytearray(before)
        struct.pack_into("<HH", before, 0x08 * 4, 0x1234, 0x5678)
        struct.pack_into("<HH", after, 0x08 * 4, 0x1111, 0x2222)
        self.assertEqual(
            watchdog.changed_interrupt_vectors(before, after),
            [
                {
                    "vector": "0x08",
                    "before": "5678:1234",
                    "after": "2222:1111",
                }
            ],
        )

    def test_marks_loader_interrupt_as_transient(self) -> None:
        self.assertEqual(watchdog.TRANSIENT_INTERRUPT_VECTORS, {0x0F})


class GuestMemoryTests(unittest.TestCase):
    def test_startup_anchor_can_be_reused_after_calibration(self) -> None:
        memory = bytearray(MEMORY_SIZE)
        game_segment = 0x1000
        game = game_segment * 16
        memory[game : game + len(watchdog.GAME_DATA_ANCHOR)] = (
            watchdog.GAME_DATA_ANCHOR
        )
        struct.pack_into("<H", memory, 0x0413, 640)
        struct.pack_into("<HH", memory, 0x21 * 4, 0x1234, 0x5678)

        self.assertTrue(watchdog.guest_memory_is_plausible(memory, game_segment))
        self.assertTrue(watchdog.game_data_anchor_is_present(memory, game_segment))

        memory[game : game + len(watchdog.GAME_DATA_ANCHOR)] = b"x" * len(
            watchdog.GAME_DATA_ANCHOR
        )

        self.assertFalse(watchdog.game_data_anchor_is_present(memory, game_segment))
        self.assertTrue(watchdog.guest_memory_environment_is_plausible(memory))


class ProfileStateTests(unittest.TestCase):
    def test_ui_release_preserves_every_unrelated_flag(self) -> None:
        self.assertEqual(watchdog.clear_presentation_ui_busy(0xFF), 0xFB)

    def test_recognizes_completed_resource_profile(self) -> None:
        memory = bytearray(MEMORY_SIZE)
        game_segment = 0x1000
        fs_segment = 0x2000
        game = game_segment * 16
        fs = fs_segment * 16
        target = 2
        handles = (0x4C, 0x4D, 0x4E, 0x4F, 0x50)
        struct.pack_into(
            "<5H",
            memory,
            fs
            + watchdog.VM_RESOURCE_PROFILES_OFFSET
            + target * watchdog.VM_RESOURCE_COUNT * 2,
            *handles,
        )
        struct.pack_into(
            "<5H", memory, game + watchdog.VM_RESOURCE_HANDLES_OFFSET, *handles
        )
        struct.pack_into(
            "<H", memory, game + watchdog.VM_RESOURCE_PROFILE_INDEX_OFFSET, target
        )
        struct.pack_into(
            "<h", memory, game + watchdog.VM_SCRIPT_PROFILE_REQUEST_OFFSET, -1
        )
        memory[game + watchdog.VM_EXECUTION_ENABLED_OFFSET] = 1
        for index in range(watchdog.VM_RESOURCE_COUNT):
            struct.pack_into(
                "<HH",
                memory,
                game + watchdog.VM_RESOURCE_IMAGES_OFFSET + index * 4,
                index * 0x10,
                0x3000 + index,
            )

        state = watchdog.read_profile_state(memory, game_segment, fs_segment)
        self.assertTrue(state.initialized)
        self.assertTrue(state.completed(target))
        self.assertFalse(state.completed(target + 1))
        self.assertFalse(state.teleport_releaseable)

        blockers = dict(state.blockers)
        blockers["vm_ui"] = 4
        releaseable = watchdog.ProfileState(
            profile=state.profile,
            request=state.request,
            execution_enabled=state.execution_enabled,
            handles=state.handles,
            expected_handles=state.expected_handles,
            images=state.images,
            blockers=tuple(blockers.items()),
        )
        self.assertTrue(releaseable.teleport_releaseable)

    def test_rejects_handle_mismatch_and_unresolved_image(self) -> None:
        memory = bytearray(MEMORY_SIZE)
        game_segment = 0x1000
        fs_segment = 0x2000
        game = game_segment * 16
        struct.pack_into(
            "<H", memory, game + watchdog.VM_RESOURCE_PROFILE_INDEX_OFFSET, 0
        )
        struct.pack_into(
            "<h", memory, game + watchdog.VM_SCRIPT_PROFILE_REQUEST_OFFSET, -1
        )
        state = watchdog.read_profile_state(memory, game_segment, fs_segment)
        self.assertFalse(state.initialized)

    def test_rejects_teleport_while_non_ui_blocker_is_active(self) -> None:
        state = watchdog.ProfileState(
            profile=0,
            request=-1,
            execution_enabled=1,
            handles=(2, 3, 4, 5, 6),
            expected_handles=(2, 3, 4, 5, 6),
            images=((0, 1),) * 5,
            blockers=(("vm_ui", 4), ("presentation", 1)),
        )
        self.assertFalse(state.teleport_releaseable)


class DialogueAudioTests(unittest.TestCase):
    def test_reads_audio_state_and_rejects_zero_clip_selection(self) -> None:
        memory = bytearray(MEMORY_SIZE)
        game_segment = 0x1000
        game = game_segment * 16
        memory[game + 0x0ADE] = 1
        memory[game + 0x0CFA] = 1
        struct.pack_into("<H", memory, game + 0x0C4D, 0)
        struct.pack_into("<H", memory, game + 0x0C53, 0)

        state = watchdog.read_dialogue_audio_state(memory, game_segment)

        self.assertEqual(state["streamed_clip_count"], 0)
        self.assertEqual(
            watchdog.dialogue_audio_stall_reason(state),
            "dialogue-clip-selection-no-candidates=count:0,last:0",
        )

    def test_allows_unarmed_or_selectable_dialogue_audio(self) -> None:
        state = {name: 0 for name in watchdog.DIALOGUE_AUDIO_OFFSETS}
        self.assertIsNone(watchdog.dialogue_audio_stall_reason(state))

        state["voc_playback_enabled"] = 1
        state["text_mode_play"] = 1
        state["streamed_clip_count"] = 2
        self.assertIsNone(watchdog.dialogue_audio_stall_reason(state))

        state["streamed_clip_count"] = 1
        state["last_clip"] = 0
        self.assertIsNotNone(watchdog.dialogue_audio_stall_reason(state))

    def test_reads_presentation_handoff_state(self) -> None:
        memory = bytearray(MEMORY_SIZE)
        game_segment = 0x1000
        game = game_segment * 16
        struct.pack_into("<H", memory, game + 0x6788, 8)
        struct.pack_into("<H", memory, game + 0x0DAF, 23)
        struct.pack_into("<H", memory, game + 0x0D64, 19)
        struct.pack_into("<I", memory, game + 0x0D88, 0x12345678)
        struct.pack_into("<h", memory, game + 0x0A2A, 230)
        struct.pack_into("<h", memory, game + 0x0A2C, 103)
        struct.pack_into("<h", memory, game + 0x2795, 45)
        struct.pack_into("<H", memory, game + 0x675A, 0x06C2)
        struct.pack_into("<H", memory, game + 0x6768, 0x00C4)
        struct.pack_into("<H", memory, game + 0x676A, 0x06C2)

        state = watchdog.read_presentation_flow_state(memory, game_segment)

        self.assertEqual(state["active_line"], 8)
        self.assertEqual(state["list_entry_metric"], 23)
        self.assertEqual(state["list_read_wrap_limit"], 19)
        self.assertEqual(state["resource_source_remaining"], 0x12345678)
        self.assertEqual(state["mouse_x"], 230)
        self.assertEqual(state["mouse_y"], 103)
        self.assertEqual(state["bridge_view_frame"], 45)
        self.assertEqual(state["nav_pending_record_link"], 0x06C2)
        self.assertEqual(state["deferred_record_type"], 0x00C4)
        self.assertEqual(state["deferred_record_related"], 0x06C2)


class Script2RadioTests(unittest.TestCase):
    def test_primary_press_writes_both_input_latches(self) -> None:
        memory = io.BytesIO(bytes(0x20000))
        game_address = 0x10000

        watchdog.write_primary_press(memory, game_address, True)
        self.assertEqual(
            memory.getvalue()[
                game_address + watchdog.MOUSE_PRIMARY_PRESSED_OFFSET
            ],
            1,
        )
        self.assertEqual(
            memory.getvalue()[game_address + watchdog.MOUSE_PRESS_PENDING_OFFSET],
            1,
        )

        watchdog.write_primary_press(memory, game_address, False)
        self.assertEqual(
            memory.getvalue()[
                game_address + watchdog.MOUSE_PRIMARY_PRESSED_OFFSET
            ],
            0,
        )
        self.assertEqual(
            memory.getvalue()[game_address + watchdog.MOUSE_PRESS_PENDING_OFFSET],
            0,
        )

    def test_reads_script_procedure_and_radio_state(self) -> None:
        memory = bytearray(MEMORY_SIZE)
        game_segment = 0x1800
        cod_segment = 0x3000
        record_segment = 0x3800
        profile = watchdog.ProfileState(
            profile=watchdog.SCRIPT2_PROFILE,
            request=-1,
            execution_enabled=1,
            handles=(1, 2, 3, 4, 5),
            expected_handles=(1, 2, 3, 4, 5),
            images=(
                (0, cod_segment),
                (0, 0x3200),
                (0, record_segment),
                (0, 0x3A00),
                (0, 0x3C00),
            ),
            blockers=(),
        )
        cod = cod_segment * 16
        for index, offset in enumerate(
            watchdog.SCRIPT2_RADIO_PROCEDURE_FLAGS.values(), start=1
        ):
            memory[cod + offset] = index
        game = game_segment * 16
        struct.pack_into(
            "<H", memory, game + watchdog.VM_STATE_ARRAY_OFFSET + 3 * 2, 7
        )
        records = record_segment * 16
        struct.pack_into("<H", memory, records + 0x12C0, 2)
        action = record_segment * 16 + watchdog.SCRIPT2_SCRUTER_K_ACTION_OFFSET
        memory[action : action + 6] = bytes.fromhex("c40028000000")
        text = b"MESSAGE RADIO:\0xxx"
        start = game + watchdog.VM_TEXT_BUFFER_OFFSET
        memory[start : start + len(text)] = text

        state = watchdog.read_script2_radio_state(
            bytes(memory), game_segment, profile
        )

        self.assertIsNotNone(state)
        assert state is not None
        self.assertEqual(
            state["procedures"],
            {"time": 1, "radioscr": 2, "sort": 3, "radio1": 4},
        )
        self.assertEqual(state["timer_3"], 7)
        self.assertEqual(state["radio_variant"], 2)
        self.assertEqual(state["scruter_k_action"], "c40028000000")
        self.assertEqual(state["subtitle"], "MESSAGE RADIO:")

    def test_decodes_active_raw_dictionary_subtitle(self) -> None:
        memory = bytearray(MEMORY_SIZE)
        game_segment = 0x1800
        cod_segment = 0x3000
        dictionary_segment = 0x3200
        record_segment = 0x3800
        profile = watchdog.ProfileState(
            profile=watchdog.SCRIPT2_PROFILE,
            request=-1,
            execution_enabled=1,
            handles=(1, 2, 3, 4, 5),
            expected_handles=(1, 2, 3, 4, 5),
            images=(
                (0, cod_segment),
                (0, dictionary_segment),
                (0, record_segment),
                (0, 0x3A00),
                (0, 0x3C00),
            ),
            blockers=(),
        )
        game = game_segment * 16
        cod = cod_segment * 16
        dictionary = dictionary_segment * 16
        struct.pack_into("<HH", memory, game + 0x674A, 0x0200, cod_segment)
        struct.pack_into("<HH", memory, game + 0x6728, 0, dictionary_segment)
        struct.pack_into("<4H", memory, cod + 0x0200, 0x10, 0x20, 0x30, 0)
        memory[dictionary + 0x10:dictionary + 0x15] = b"OKAY\0"
        memory[dictionary + 0x20:dictionary + 0x26] = b"OKAY,\0"
        memory[dictionary + 0x30:dictionary + 0x3A] = b"WISE GUY!\0"
        memory[game + 0x67B0] = 1

        state = watchdog.read_script2_radio_state(
            bytes(memory), game_segment, profile
        )

        self.assertIsNotNone(state)
        assert state is not None
        self.assertEqual(state["subtitle"], "OKAY OKAY, WISE GUY!")
        self.assertEqual(state["menu_words"], "3000:0200")

    def test_requires_script2_profile(self) -> None:
        profile = watchdog.ProfileState(
            profile=0,
            request=-1,
            execution_enabled=1,
            handles=(),
            expected_handles=(),
            images=(),
            blockers=(),
        )
        self.assertIsNone(
            watchdog.read_script2_radio_state(
                bytes(MEMORY_SIZE), 0x1800, profile
            )
        )


if __name__ == "__main__":
    unittest.main()
