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

    def test_linear_surface_summary_hashes_every_row(self) -> None:
        surface = bytes(range(256)) * 250
        summary = capture.linear_surface_summary(surface)
        self.assertEqual(summary["byte_count"], 64000)
        self.assertEqual(summary["unique_byte_count"], 256)
        self.assertEqual(summary["nonzero_row_count"], 200)
        self.assertEqual(len(summary["row_sha256"]), 200)

    def test_linear_surface_summary_rejects_wrong_size(self) -> None:
        with self.assertRaisesRegex(ValueError, "320x200"):
            capture.linear_surface_summary(b"short")

    def test_graphics_pointer_monitor_accepts_known_surface_swaps(self) \
            -> None:
        baseline = {
            "work_surface": {
                "offset": 0, "segment": 0x5000,
                "pointer": "5000:0000", "linear": 0x50000,
            },
            "draw_framebuffer": {
                "offset": 0x4000, "segment": 0xA000,
                "pointer": "a000:4000", "linear": 0xA4000,
            },
            "screen_buffer": {
                "offset": 0, "segment": 0xA000,
                "pointer": "a000:0000", "linear": 0xA0000,
            },
            "display_buffer": {
                "offset": 0, "segment": 0x2000,
                "pointer": "2000:0000", "linear": 0x20000,
            },
            "back_buffer": {
                "offset": 0, "segment": 0x3000,
                "pointer": "3000:0000", "linear": 0x30000,
            },
        }
        current = {name: entry.copy() for name, entry in baseline.items()}
        current["display_buffer"] = baseline["back_buffer"].copy()
        current["back_buffer"] = baseline["work_surface"].copy()
        current["draw_framebuffer"] = {
            "offset": 0xC000, "segment": 0xA000,
            "pointer": "a000:c000", "linear": 0xAC000,
        }
        current["screen_buffer"] = {
            "offset": 0x8000, "segment": 0xA000,
            "pointer": "a000:8000", "linear": 0xA8000,
        }
        self.assertEqual(
            capture.graphics_pointer_errors(current, baseline, 0x1000), [])

    def test_graphics_pointer_monitor_rejects_dgroup_restore(self) -> None:
        baseline = {
            "back_buffer": {
                "offset": 0, "segment": 0x3000,
                "pointer": "3000:0000", "linear": 0x30000,
            },
        }
        current = {
            "back_buffer": {
                "offset": 0, "segment": 0x107C,
                "pointer": "107c:0000", "linear": 0x107C0,
            },
        }
        self.assertEqual(
            capture.graphics_pointer_errors(current, baseline, 0x107C),
            ["back_buffer points into DGROUP at 107c:0000"],
        )

    def test_graphics_pointer_monitor_rejects_unknown_surface(self) -> None:
        baseline = {
            "display_buffer": {
                "offset": 0, "segment": 0x2000,
                "pointer": "2000:0000", "linear": 0x20000,
            },
        }
        current = {
            "display_buffer": {
                "offset": 0, "segment": 0x2800,
                "pointer": "2800:0000", "linear": 0x28000,
            },
        }
        self.assertEqual(
            capture.graphics_pointer_errors(current, baseline, 0x1000),
            ["display_buffer selected unknown surface 2800:0000"],
        )

    def test_pterra_intro_input_stops_before_hud_selector(self) -> None:
        blockers = {"ship": 5}
        flow = {
            "active_line": 5,
            "displayed_line": 5,
            "dialogue_hold_complete": 1,
            "dialogue_hold_countdown": 6,
        }
        input_state = {
            "ship_hud_initialized": 0,
            "ship_target_select_phase": 0,
        }
        self.assertTrue(capture.pterra_ship_intro_waiting_for_input(
            blockers, flow, input_state))
        input_state["ship_hud_initialized"] = 1
        input_state["ship_target_select_phase"] = 1
        self.assertFalse(capture.pterra_ship_intro_waiting_for_input(
            blockers, flow, input_state))

    def test_pterra_intro_edge_waits_for_decisive_countdown(self) -> None:
        flow = {"dialogue_hold_countdown": 8}
        self.assertFalse(capture.pterra_ship_intro_ready_for_edge(flow))
        flow["dialogue_hold_countdown"] = 7
        self.assertTrue(capture.pterra_ship_intro_ready_for_edge(flow))
        flow["dialogue_hold_countdown"] = 0
        self.assertFalse(capture.pterra_ship_intro_ready_for_edge(flow))

    def test_pterra_intro_press_uses_guest_observable_pulse(self) -> None:
        self.assertFalse(capture.pterra_ship_intro_press_should_release(
            10.19, 10.0))
        self.assertTrue(capture.pterra_ship_intro_press_should_release(
            10.21, 10.0))
        self.assertFalse(capture.pterra_ship_intro_press_should_release(
            10.21, None))

    def test_pterra_intro_active_pulse_cannot_start_another_press(self) \
            -> None:
        self.assertEqual(capture.pterra_ship_intro_input_action(
            pressing=True, release_ready=False, latch_active=False,
            can_press=True), "hold")
        self.assertEqual(capture.pterra_ship_intro_input_action(
            pressing=True, release_ready=True, latch_active=False,
            can_press=True), "release")
        self.assertEqual(capture.pterra_ship_intro_input_action(
            pressing=False, release_ready=False, latch_active=True,
            can_press=True), "wait")
        self.assertEqual(capture.pterra_ship_intro_input_action(
            pressing=False, release_ready=False, latch_active=False,
            can_press=True), "press")

    def test_pterra_intro_accepts_guest_latched_hud_before_expiry(self) \
            -> None:
        flow = {
            "dialogue_hold_complete": 1,
            "dialogue_hold_countdown": 2,
        }
        input_state = {
            "ship_hud_initialized": 1,
            "ship_target_select_phase": 1,
        }
        self.assertEqual(capture.pterra_ship_intro_consumed_before_expiry(
            flow, input_state, [4, 5], edge_count=5,
            raw_seen=True, latch_seen=True),
            (True, "guest-latched-hud-handoff"))
        flow["dialogue_hold_countdown"] = 0
        self.assertEqual(capture.pterra_ship_intro_consumed_before_expiry(
            flow, input_state, [4, 5], edge_count=5,
            raw_seen=True, latch_seen=True), (False, None))

    def test_pterra_intro_hud_handoff_requires_guest_input_evidence(self) \
            -> None:
        flow = {
            "dialogue_hold_complete": 1,
            "dialogue_hold_countdown": 2,
        }
        input_state = {
            "ship_hud_initialized": 1,
            "ship_target_select_phase": 1,
        }
        self.assertEqual(capture.pterra_ship_intro_consumed_before_expiry(
            flow, input_state, [4, 5], edge_count=5,
            raw_seen=True, latch_seen=False), (False, None))

    def test_pterra_intro_accepts_complete_no_hold_hud_handoff(self) -> None:
        blockers = {"ship": 5}
        flow = {
            "dialogue_hold_complete": 0,
            "dialogue_hold_countdown": 0,
        }
        input_state = {
            "ship_hud_initialized": 1,
            "ship_target_select_phase": 1,
        }
        self.assertTrue(capture.pterra_ship_intro_is_naturally_complete(
            blockers, flow, input_state, [4, 5], edge_count=0))

    def test_pterra_intro_natural_completion_rejects_ambiguous_edge(self) \
            -> None:
        blockers = {"ship": 5}
        flow = {
            "dialogue_hold_complete": 0,
            "dialogue_hold_countdown": 0,
        }
        input_state = {
            "ship_hud_initialized": 1,
            "ship_target_select_phase": 1,
        }
        self.assertFalse(capture.pterra_ship_intro_is_naturally_complete(
            blockers, flow, input_state, [4, 5], edge_count=1))
        self.assertFalse(capture.pterra_ship_intro_is_naturally_complete(
            blockers, flow, input_state, [5], edge_count=0))

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

    def test_reads_compact_manu3_runtime_state(self) -> None:
        memory = bytearray(0x100000)
        game_segment = 0x1000
        code_segment = 0x2000
        data_segment = 0x2300
        raster_segment = 0x3000
        struct.pack_into(
            "<HH", memory,
            game_segment * 16 + capture.STARTUP_DOS_POOL_POINTER_OFFSET,
            0, code_segment)
        memory[code_segment * 16:code_segment * 16 + 4] = \
            capture.MANU3_RECOVERED_PREFIX
        struct.pack_into(
            "<H", memory,
            code_segment * 16
            + capture.MANU3_RECOVERED_DATA_SEGMENT_DELTA_OFFSET,
            data_segment - code_segment)
        data = data_segment * 16
        struct.pack_into(
            "<10H", memory, data,
            0xAABB, 0x3100, 0x3200, raster_segment,
            0x3400, 0x3500, 0x3600, 0x3700, 0x3800, 0x3900)
        struct.pack_into(
            "<HH", memory, data + capture.MANU3_CURRENT_STATE_OFFSET,
            0x1111, 0x2222)
        struct.pack_into(
            "<H", memory, data + capture.MANU3_FACE_LIST_OFFSET, 0x3333)
        struct.pack_into(
            "<H", memory, data + capture.MANU3_FACE_COUNT_OFFSET, 7)
        raster = raster_segment * 16
        for index, offset in enumerate(capture.MANU3_RASTER_STATE_OFFSETS):
            struct.pack_into("<H", memory, raster + offset, 0x4000 + index)
        for index, offset in enumerate(capture.MANU3_RASTER_RECORD_OFFSETS):
            memory[raster + offset:raster + offset + 16] = bytes(
                [0x50 + index]) * 16
        struct.pack_into(
            "<HHHHHh", memory,
            raster + capture.MANU3_ACTIVE_LIST_HEAD_OFFSET,
            0, 1, 0, capture.MANU3_ACTIVE_LIST_MIDDLE_OFFSET, 0, -1)
        struct.pack_into(
            "<HHHHHh", memory,
            raster + capture.MANU3_ACTIVE_LIST_MIDDLE_OFFSET,
            0, 0x8000, 0, capture.MANU3_ACTIVE_LIST_TAIL_OFFSET, 0, 200)

        state = capture.read_manu3_runtime_state(
            io.BytesIO(memory), 0, game_segment,
            {"cs": code_segment, "ip": 0x1431})

        self.assertTrue(state["loaded"])
        self.assertEqual(state["image_layout"], "recovered")
        self.assertTrue(state["cpu_in_manu3"])
        self.assertEqual(state["local_ip"], 0x1431)
        self.assertEqual(state["data_segment"], data_segment)
        self.assertEqual(state["renderer"]["projection_remaining"], 0x2222)
        self.assertEqual(state["renderer"]["face_list"], 0x3333)
        self.assertEqual(state["renderer"]["face_count"], 7)
        self.assertEqual(state["raster"]["words"]["0684"], 0x4003)
        self.assertEqual(
            state["raster"]["boundary_chain"]["termination"], "terminal")
        self.assertEqual(
            state["raster"]["boundary_chain"]["visited_count"], 2)

    def test_reads_original_manu3_data_segment_delta(self) -> None:
        memory = bytearray(0x100000)
        game_segment = 0x1000
        code_segment = 0x2000
        data_segment = 0x2137
        raster_segment = 0x3000
        struct.pack_into(
            "<HH", memory,
            game_segment * 16 + capture.STARTUP_DOS_POOL_POINTER_OFFSET,
            0, code_segment)
        code = code_segment * 16
        memory[code:code + 4] = capture.MANU3_ORIGINAL_PREFIX
        struct.pack_into(
            "<H", memory,
            code + capture.MANU3_ORIGINAL_DATA_SEGMENT_DELTA_OFFSET,
            data_segment - code_segment)
        struct.pack_into(
            "<10H", memory, data_segment * 16,
            0, 0, 0, raster_segment, 0, 0, 0, 0, 0, 0)
        struct.pack_into(
            "<HHHHHh", memory,
            raster_segment * 16 + capture.MANU3_ACTIVE_LIST_HEAD_OFFSET,
            0, 0x8000, 0, 0, 0, -1)

        state = capture.read_manu3_runtime_state(
            io.BytesIO(memory), 0, game_segment,
            {"cs": code_segment, "ip": 0x0C2A})

        self.assertTrue(state["loaded"])
        self.assertEqual(state["image_layout"], "original")
        self.assertEqual(state["data_segment"], data_segment)
        self.assertEqual(
            state["data_segment_delta_offset"],
            capture.MANU3_ORIGINAL_DATA_SEGMENT_DELTA_OFFSET)

    def test_manu3_runtime_state_rejects_unknown_image_layout(self) -> None:
        memory = bytearray(0x40000)
        game_segment = 0x1000
        code_segment = 0x2000
        struct.pack_into(
            "<HH", memory,
            game_segment * 16 + capture.STARTUP_DOS_POOL_POINTER_OFFSET,
            0, code_segment)
        memory[code_segment * 16:code_segment * 16 + 4] = b"BAD!"

        state = capture.read_manu3_runtime_state(
            io.BytesIO(memory), 0, game_segment,
            {"cs": code_segment, "ip": 0})

        self.assertFalse(state["loaded"])
        self.assertEqual(state["image_layout"], "unknown")
        self.assertEqual(state["code_prefix"], "42414421")

    def test_manu3_boundary_chain_reports_invalid_offset(self) -> None:
        memory = bytearray(0x50000)
        raster_segment = 0x3000
        raster = raster_segment * 16
        struct.pack_into(
            "<HHHHHh", memory,
            raster + capture.MANU3_ACTIVE_LIST_HEAD_OFFSET,
            0, 1, 0, 1, 0, 0)
        chain = capture.read_manu3_boundary_chain(
            io.BytesIO(memory), 0, raster_segment)
        self.assertEqual(chain["termination"], "invalid-offset")
        self.assertEqual(chain["invalid_at"], 1)
        self.assertEqual(chain["visited_count"], 1)

    def test_manu3_boundary_chain_accepts_sentinel_overlay(self) -> None:
        memory = bytearray(0x50000)
        raster_segment = 0x3000
        raster = raster_segment * 16
        overlay = capture.MANU3_ACTIVE_LIST_TAIL_OFFSET + 0x10
        struct.pack_into(
            "<HHHHHh", memory,
            raster + capture.MANU3_ACTIVE_LIST_HEAD_OFFSET,
            0, 1, 0, overlay, 0, -1)
        struct.pack_into(
            "<HHHHHh", memory, raster + overlay,
            0, 0x8000, 0, 0, 0, 200)

        chain = capture.read_manu3_boundary_chain(
            io.BytesIO(memory), 0, raster_segment)

        self.assertEqual(chain["termination"], "terminal")
        self.assertEqual(chain["visited_count"], 2)

    def test_manu3_runtime_state_handles_unloaded_pool(self) -> None:
        memory = bytearray(0x20000)
        state = capture.read_manu3_runtime_state(
            io.BytesIO(memory), 0, 0x1000,
            {"cs": 0x1111, "ip": 0x2222})
        self.assertFalse(state["loaded"])
        self.assertFalse(state["cpu_in_manu3"])
        self.assertIsNone(state["local_ip"])

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
                side_effect=[search, geometry, completed, completed,
                             completed]) as run:
            result = capture.recapture_game_mouse(":9", "BLOODPRG.EXE")
        self.assertEqual(result["window_point"], [320, 200])
        self.assertTrue(result["window_activated"])
        self.assertEqual(
            [call.args[0] for call in run.call_args_list[2:4]],
            [
                ["xdotool", "windowactivate", "--sync", "1234"],
                ["xdotool", "windowfocus", "--sync", "1234"],
            ],
        )
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
                             completed, completed, completed]) as run, \
                mock.patch.object(capture.time, "sleep"):
            result = capture.recapture_game_mouse(
                ":9", "BLOODPRG.EXE", toggle_capture=True)
        self.assertTrue(result["capture_toggled"])
        self.assertTrue(result["window_activated"])
        self.assertEqual(
            [call.args[0] for call in run.call_args_list[-3:]],
            [
                ["xdotool", "click", "2"],
                ["xdotool", "mousemove", "--sync", "--window", "1234",
                 "320", "200"],
                ["xdotool", "click", "2"],
            ],
        )

    def test_recapture_allows_missing_ewmh_window_manager(self) -> None:
        search = mock.Mock(returncode=0, stdout="1234\n")
        geometry = mock.Mock(
            returncode=0, stdout="WIDTH=640\nHEIGHT=400\n")
        unavailable = mock.Mock(returncode=1, stdout="")
        completed = mock.Mock(returncode=0, stdout="")
        with mock.patch.object(
                capture.subprocess, "run",
                side_effect=[search, geometry, unavailable, completed,
                             completed]):
            result = capture.recapture_game_mouse(":9", "BLOODPRG.EXE")
        self.assertFalse(result["window_activated"])
        self.assertEqual(result["window_point"], [320, 200])

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

    def test_bridge_station_click_gets_bounded_activation_window(self) -> None:
        self.assertFalse(capture.bridge_navigation_timed_out(
            now=365.0, started_at=0.0, last_progress_at=365.0,
            first_click_at=360.0))
        self.assertTrue(capture.bridge_navigation_timed_out(
            now=390.0, started_at=0.0, last_progress_at=389.0,
            first_click_at=360.0))

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
