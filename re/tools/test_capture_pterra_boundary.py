#!/usr/bin/env python3

from __future__ import annotations

import io
import importlib.util
import struct
import sys
import unittest
from pathlib import Path
from unittest import mock


CAPTURE_PATH = Path(__file__).with_name("capture_pterra_boundary.py")
SPEC = importlib.util.spec_from_file_location(
    "capture_pterra_boundary", CAPTURE_PATH)
assert SPEC is not None and SPEC.loader is not None
capture = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = capture
SPEC.loader.exec_module(capture)


class CaptureHelperTests(unittest.TestCase):
    @staticmethod
    def script2_memory(*, pterra_flags: int = 1,
                       unlock_state: int = 0,
                       init_enabled: int = 1,
                       current_location: int = 0) -> io.BytesIO:
        data = bytearray(0x100000)
        game_segment = 0x1000
        game = game_segment * 16
        cod_segment = 0x2000
        record_segment = 0x3000
        struct.pack_into(
            "<HH", data, game + capture.VM_RESOURCE_IMAGES_OFFSET,
            0, cod_segment)
        struct.pack_into(
            "<HH", data, game + capture.VM_RECORD_BASE_POINTER_OFFSET,
            0, record_segment)
        struct.pack_into(
            "<H", data, game + capture.VM_NAMED_ORXX_OBJECT_OFFSET,
            0x1244)
        struct.pack_into(
            "<H", data, game + capture.VM_NAMED_ARCHETYPE_OBJECT_OFFSET,
            0x0F38)
        struct.pack_into(
            "<H", data, game + capture.VM_NAMED_ARK_OBJECT_OFFSET,
            0x0F5C)
        records = record_segment * 16
        struct.pack_into("<H", data, records + 0x1244, 0x0200)
        struct.pack_into(
            "<H", data, records + capture.SCRIPT2_PTERRA_RECORD, 0x0008)
        struct.pack_into(
            "<H", data,
            records + capture.SCRIPT2_ARCHETYPE_CURRENT_LOCATION_OFFSET,
            current_location)
        struct.pack_into(
            "<H", data, records + capture.SCRIPT2_PTERRA_FLAGS_OFFSET,
            pterra_flags)
        struct.pack_into(
            "<H", data,
            records + capture.SCRIPT2_PTERRA_UNLOCK_STATE_OFFSET,
            unlock_state)
        struct.pack_into(
            "<HH", data,
            records + capture.SCRIPT2_PTERRA_RECORD + 0x18,
            201, 93)
        data[cod_segment * 16 + capture.SCRIPT2_INIT_PROCEDURE_OFFSET] = \
            init_enabled
        return io.BytesIO(data)

    def test_illegal_interrupt_detector_includes_divide_error(self) -> None:
        match = capture.ILLEGAL_INTERRUPT_RE.search(
            b"ERROR CPU:Illegal Unhandled Interrupt Called 0")
        self.assertIsNotNone(match)
        assert match is not None
        self.assertEqual(int(match.group(1), 10), 0)

    def test_reads_dosbox_staging_segment_array(self) -> None:
        data = bytearray(256)
        struct.pack_into("<8II", data, 32, *range(0x10, 0x18), 0x12345)
        struct.pack_into("<6H", data, 128,
                         0x100, 0x200, 0x300, 0x400, 0x500, 0x600)
        state = capture.read_cpu_state(
            io.BytesIO(data),
            {"cpu_regs": 32, "Segs": 128, "Segs_size": 0x30})
        self.assertEqual(
            [state[name] for name in ("es", "cs", "ss", "ds", "fs", "gs")],
            [0x100, 0x200, 0x300, 0x400, 0x500, 0x600])
        self.assertEqual(state["ip"], 0x2345)

    def test_reads_dosbox_x_interleaved_segments(self) -> None:
        data = bytearray(256)
        struct.pack_into("<8II", data, 32, *range(0x10, 0x18), 0x12345)
        for index, value in enumerate(
                (0x100, 0x200, 0x300, 0x400, 0x500, 0x600)):
            struct.pack_into("<Q", data, 128 + index * 8, value)
        state = capture.read_cpu_state(
            io.BytesIO(data),
            {"cpu_regs": 32, "Segs": 128, "Segs_size": 0})
        self.assertEqual(
            [state[name] for name in ("es", "cs", "ss", "ds", "fs", "gs")],
            [0x100, 0x200, 0x300, 0x400, 0x500, 0x600])

    def test_choice_row_point_uses_list_widget_row_pitch(self) -> None:
        self.assertEqual(
            capture.choice_row_point((100, 52, 120, 96), 4),
            (160, 105),
        )

    def test_captured_mouse_accepts_observed_target(self) -> None:
        with mock.patch.object(capture.subprocess, "run") as run:
            self.assertTrue(capture.move_captured_game_mouse(
                ":9", 200, 95, 201, 93))
        run.assert_not_called()

    def test_captured_mouse_moves_in_bounded_relative_steps(self) -> None:
        completed = mock.Mock(returncode=0)
        with mock.patch.object(
                capture.subprocess, "run", return_value=completed) as run:
            self.assertFalse(capture.move_captured_game_mouse(
                ":9", 110, 140, 201, 93))
        self.assertEqual(
            run.call_args.args[0],
            ["xdotool", "mousemove_relative", "--sync", "--", "32", "-32"],
        )

    def test_guest_primary_adapter_injects_native_edge_latches(self) -> None:
        memory = io.BytesIO(bytearray(0x40000))
        evidence = capture.inject_guest_primary_click(
            memory, 0, 0x1000, 202, 104)
        game = 0x1000 * 16
        memory.seek(game + capture.MOUSE_X_OFFSET)
        self.assertEqual(memory.read(4), struct.pack("<hh", 202, 104))
        memory.seek(game + capture.MOUSE_LAST_X_OFFSET)
        self.assertEqual(memory.read(4), struct.pack("<hh", 202, 104))
        memory.seek(game + capture.MOUSE_PRIMARY_PRESSED_OFFSET)
        self.assertEqual(memory.read(1), b"\x01")
        memory.seek(game + capture.MOUSE_PRESS_PENDING_OFFSET)
        self.assertEqual(memory.read(1), b"\x01")
        self.assertEqual(evidence["adapter"], "guest-primary-edge")

    def test_guest_mouse_point_rejects_staging_wrap_coordinate(self) -> None:
        self.assertTrue(capture.guest_mouse_point_is_valid(202, 104))
        self.assertFalse(capture.guest_mouse_point_is_valid(2524, 100))

    def test_recapture_moves_pointer_to_game_window_center(self) -> None:
        search = mock.Mock(returncode=0, stdout="1234\n")
        geometry = mock.Mock(
            returncode=0, stdout="X=80\nY=100\nWIDTH=640\nHEIGHT=400\n")
        completed = mock.Mock(returncode=0, stdout="")
        with mock.patch.object(
                capture.subprocess, "run",
                side_effect=[search, geometry, completed, completed]) as run:
            result = capture.recapture_game_mouse(":9", "BLOODPRG.EXE")
        self.assertEqual(result["window_point"], [320, 200])
        self.assertEqual(
            run.call_args_list[-1].args[0],
            ["xdotool", "mousemove", "--sync", "--window", "1234",
             "320", "200"],
        )

    def test_staging_recapture_repositions_only_while_released(self) -> None:
        search = mock.Mock(returncode=0, stdout="1234\n")
        geometry = mock.Mock(
            returncode=0, stdout="WIDTH=640\nHEIGHT=400\n")
        completed = mock.Mock(returncode=0, stdout="")
        with mock.patch.object(
                capture.subprocess, "run",
                side_effect=[search, geometry, completed, completed,
                             completed, completed]) as run, \
                mock.patch.object(capture.time, "sleep"):
            result = capture.recapture_game_mouse(
                ":9", "BLOODPRG.EXE", toggle_capture=True)
        self.assertTrue(result["capture_toggled"])
        self.assertEqual(
            [call.args[0] for call in run.call_args_list[-3:]],
            [
                ["xdotool", "click", "2"],
                ["xdotool", "mousemove", "--sync", "--window", "1234",
                 "320", "200"],
                ["xdotool", "click", "2"],
            ],
        )

    def test_dosbox_staging_captures_mouse_on_start(self) -> None:
        self.assertEqual(
            capture.dosbox_mouse_settings("/nix/store/hash/bin/dosbox"),
            [
                "-set", "mouse mouse_capture=onstart",
                "-set", "mouse mouse_raw_input=false",
            ],
        )
        self.assertTrue(capture.dosbox_needs_capture_toggle("dosbox"))

    def test_dosbox_x_uses_autolock(self) -> None:
        self.assertEqual(
            capture.dosbox_mouse_settings("dosbox-x"),
            ["-set", "sdl autolock=true"],
        )
        self.assertFalse(capture.dosbox_needs_capture_toggle("dosbox-x"))

    def test_bridge_rotation_progress_extends_open_watchdog(self) -> None:
        self.assertFalse(capture.bridge_navigation_timed_out(
            now=170.0, started_at=0.0, last_progress_at=160.0))

    def test_bridge_rotation_still_has_hard_deadline(self) -> None:
        self.assertTrue(capture.bridge_navigation_timed_out(
            now=capture.PTERRA_BRIDGE_ROTATION_TIMEOUT_SECONDS,
            started_at=0.0,
            last_progress_at=capture.PTERRA_BRIDGE_ROTATION_TIMEOUT_SECONDS
            - 1.0))

    def test_pterra_choices_are_recorded_only_in_verified_order(self) -> None:
        results: list[int] = []
        self.assertFalse(capture.record_expected_pterra_choice(
            results, capture.SCRIPT2_TELEPORT_WORD))
        self.assertTrue(capture.record_expected_pterra_choice(
            results, capture.SCRIPT2_EXXOS_WORD))
        self.assertTrue(capture.record_expected_pterra_choice(
            results, capture.SCRIPT2_TELEPORT_WORD))
        self.assertFalse(capture.record_expected_pterra_choice(
            results, capture.SCRIPT2_TELEPORT_WORD))
        self.assertEqual(results, list(capture.EXPECTED_PTERRA_CHOICES))

    def test_pterra_commit_requires_choices_and_idle_runtime(self) -> None:
        blockers = {
            name: 0 for name, _offset, _mask in capture.TELEPORT_BLOCKERS
        }
        flow = {"active_line": 0xffff, "c2_presentation_gate": 0}
        self.assertTrue(capture.pterra_encounter_idle(
            blockers, flow, list(capture.EXPECTED_PTERRA_CHOICES)))
        blockers["ship"] = 1
        self.assertFalse(capture.pterra_encounter_idle(
            blockers, flow, list(capture.EXPECTED_PTERRA_CHOICES)))

    def test_destination_readiness_accepts_idle_bridge_ui(self) -> None:
        blockers = {
            name: (4 if name == "vm_ui" else 0)
            for name, _offset, _mask in capture.TELEPORT_BLOCKERS
        }
        flow = {
            "active_line": 0xffff,
            "c2_presentation_gate": 0,
            "resource_source_remaining": 0,
            "list_queued_bytes": 0,
            "list_active": "0000:0000",
        }
        self.assertTrue(capture.pterra_destination_ready(blockers, flow))

    def test_destination_readiness_rejects_transition_activity(self) -> None:
        blockers = {
            name: 0 for name, _offset, _mask in capture.TELEPORT_BLOCKERS
        }
        blockers["render"] = 1
        flow = {
            "active_line": 0xffff,
            "c2_presentation_gate": 0,
            "resource_source_remaining": 0,
            "list_queued_bytes": 0,
            "list_active": "0000:0000",
        }
        self.assertFalse(capture.pterra_destination_ready(blockers, flow))

    def test_destination_readiness_accepts_stale_completed_line(self) -> None:
        blockers = {
            name: 0 for name, _offset, _mask in capture.TELEPORT_BLOCKERS
        }
        flow = {
            "active_line": 2,
            "c2_presentation_gate": 0,
            "resource_source_remaining": 0,
            "list_queued_bytes": 0,
            "list_active": "0000:0000",
        }
        self.assertTrue(capture.pterra_destination_ready(blockers, flow))

    def test_destination_readiness_rejects_live_line_gate(self) -> None:
        blockers = {
            name: 0 for name, _offset, _mask in capture.TELEPORT_BLOCKERS
        }
        flow = {
            "active_line": 2,
            "c2_presentation_gate": 1,
            "resource_source_remaining": 0,
            "list_queued_bytes": 0,
            "list_active": "0000:0000",
        }
        self.assertFalse(capture.pterra_destination_ready(blockers, flow))

    def test_destination_readiness_rejects_active_resource_pipeline(self) \
            -> None:
        blockers = {
            name: 0 for name, _offset, _mask in capture.TELEPORT_BLOCKERS
        }
        flow = {
            "active_line": 0xffff,
            "c2_presentation_gate": 0,
            "resource_source_remaining": 12,
            "list_queued_bytes": 0,
            "list_active": "0000:0000",
        }
        self.assertFalse(capture.pterra_destination_ready(blockers, flow))

    def test_authentic_start_stops_after_title_acceptance(self) -> None:
        self.assertEqual(
            capture.authentic_gameplay_start_actions(),
            ["wait_title", "move_relative 0 11", "mouse_button 1"],
        )

    def test_title_transition_uses_durable_downstream_evidence(self) -> None:
        self.assertEqual(
            capture.title_transition_evidence(
                startup_presentation_line_seen=True,
                load_menu_requested=True,
                authentic_save_loaded=True,
            ),
            [
                "startup-presentation-line",
                "native-gameplay-load-boundary",
                "authentic-save-loaded",
            ],
        )

    def test_title_input_without_transition_is_not_confirmation(self) -> None:
        self.assertEqual(
            capture.title_transition_evidence(
                startup_presentation_line_seen=False,
                load_menu_requested=False,
                authentic_save_loaded=False,
            ),
            [],
        )

    def test_ambient_resource_queue_is_not_a_gameplay_blocker(self) -> None:
        audio_flow = {
            "presentation_mode_27e0": 0,
            "presentation_mode_27e1": 0,
        }
        blockers = {
            name: 0 for name, _offset, _mask in capture.TELEPORT_BLOCKERS
        }
        scene_flow = {
            "active_line": 2,
            "c2_presentation_gate": 0,
            "resource_source_remaining": 17_801_480,
            "list_queued_bytes": 44_855,
            "list_active": "0000:0000",
        }
        self.assertTrue(capture.native_gameplay_control_ready(
            audio_flow, blockers, scene_flow))

    def test_pterra_unlock_uses_vm_init_predicate_only(self) -> None:
        memory = self.script2_memory()
        setup = capture.request_script2_pterra_unlock(
            memory, 0, 0x1000)
        self.assertEqual(setup["source"], "recovered-init-predicate")
        memory.seek(
            0x3000 * 16 + capture.SCRIPT2_PTERRA_UNLOCK_STATE_OFFSET)
        self.assertEqual(memory.read(2), struct.pack("<H", 1))
        context = capture.read_script2_pterra_context(memory, 0, 0x1000)
        self.assertEqual(context["pterra_flags"], 1)
        self.assertEqual(context["init_procedure_enabled"], 1)
        self.assertFalse(capture.script2_pterra_unlock_completed(context))

    def test_pterra_unlock_requires_vm_to_set_flag_and_disable_init(self) \
            -> None:
        memory = self.script2_memory(
            pterra_flags=3, unlock_state=1, init_enabled=0)
        context = capture.read_script2_pterra_context(memory, 0, 0x1000)
        self.assertTrue(capture.script2_pterra_unlock_completed(context))

    def test_pterra_unlock_rejects_disabled_init(self) -> None:
        memory = self.script2_memory(init_enabled=0)
        with self.assertRaisesRegex(RuntimeError, "init procedure is disabled"):
            capture.request_script2_pterra_unlock(memory, 0, 0x1000)

    def test_nav_chart_prepare_preserves_destination_commands(self) -> None:
        memory = self.script2_memory(
            pterra_flags=3, unlock_state=1, init_enabled=0)
        setup = capture.prepare_native_nav_chart(memory, 0, 0x1000)
        self.assertEqual(setup["entry"], "native-bridge-station")
        self.assertEqual(setup["pterra_marker"], [201, 93])
        memory.seek(0x1000 * 16 + capture.NAV_CAMERA_VIEW_STATE_OFFSET)
        self.assertEqual(memory.read(1), b"\x00")
        context = capture.read_script2_pterra_context(memory, 0, 0x1000)
        self.assertEqual(context["arche_action"], [0, 0, 0])
        self.assertEqual(context["orxx_action"], [0, 0, 0])

    def test_selectable_rect_center_rejects_hidden_station(self) -> None:
        self.assertIsNone(capture.selectable_rect_center((-1, -1, -1, -1)))
        self.assertEqual(
            capture.selectable_rect_center((138, 140, 48, 42)),
            (162, 161),
        )

    def test_ship_navigation_prepare_does_not_write_ship_state(self) -> None:
        memory = self.script2_memory(
            pterra_flags=3,
            unlock_state=1,
            init_enabled=0,
            current_location=capture.SCRIPT2_PTERRA_RECORD,
        )
        setup = capture.prepare_script2_orxx_descent(
            memory, 0, 0x1000)
        self.assertEqual(setup["entry"], "native-current-location-entity")
        self.assertEqual(
            setup["entity_index"], capture.CURRENT_LOCATION_ENTITY_INDEX)
        memory.seek(0x1000 * 16 + capture.VM_SHIP_ACTIVE_FLAGS_OFFSET)
        self.assertEqual(memory.read(2), b"\x00\x00")

if __name__ == "__main__":
    unittest.main()
