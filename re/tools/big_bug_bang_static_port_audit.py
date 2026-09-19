#!/usr/bin/env python3
"""Check reviewed BBB-to-Commander body equivalences, not fuzzy similarity."""

from __future__ import annotations

import argparse
import csv
import hashlib
import json
import struct
from pathlib import Path

from capstone import Cs, CS_ARCH_X86, CS_MODE_16
from capstone.x86 import X86_OP_IMM, X86_OP_MEM

ROOT = Path(__file__).resolve().parents[2]
BBB = ROOT / "output/big-bug-bang/disc/BLOOD2PG.EXE"
COMMANDER = ROOT / "re/bin/BLOODPRG.EXE"
PORTED = ROOT / "re/rust-port/ported.tsv"
OUTPUT = ROOT / "re/big_bug_bang_static_port_audit.json"
HOVER = "crates/commander-blood-game/src/native/bloodprg/presentation_hover.rs"
SERVICES = "crates/commander-blood-game/src/runtime/services.rs"
GROWTH = "crates/commander-blood-game/src/native/bloodprg/sequel_growth.rs"
DISPATCH = "crates/commander-blood-game/src/native/bloodprg/script_dispatch.rs"
BRIDGE_ACTORS = "crates/commander-blood-game/src/runtime/bridge_actors.rs"
SEQUEL_PRESENTATION = (
    "crates/commander-blood-game/src/native/bloodprg/sequel_presentation.rs"
)

# These are individual reviewed identities, never a blanket displacement delta.
# CS flip bytes are written by BBB 0x4944/0x494C; the other CS slots are local
# stride/run scratch. GS pointers become owned framebuffers and remap slices.
SPRITE_MEMORY = {
    ("cs", 0x14DF): (0x15DC, "horizontal_flip"),
    ("cs", 0x14E0): (0x15DD, "vertical_flip"),
    ("cs", 0x15A4): (0x16A1, "raw_source_row_stride"),
    ("cs", 0x1726): (0x1823, "rle_source_row_width"),
    ("cs", 0x1728): (0x1825, "rle_left_clip"),
    ("cs", 0x172A): (0x1827, "rle_right_clip"),
    ("gs", 0x5221): (0x55F1, "drawing_framebuffer"),
    ("gs", 0x524B): (0x561B, "selected_remap_table"),
}
SPRITE_IMMEDIATES = {
    ("mov", "bx", 0x5F11): (0x62E1, "first_remap_table"),
    ("mov", "bx", 0x6011): (0x63E1, "second_remap_table"),
}
HOVER_MEMORY = {
    ("", 0x2793): (0x2A33, "presentation_mode_bits"),
    ("", 0x0A2A): (0x0C22, "pointer_x"),
    ("", 0x0A2C): (0x0C24, "pointer_y"),
    ("", 0x27EA): (0x2A8B, "hover_latch"),
    ("", 0x0A32): (0x0C2A, "actor_state"),
    ("", 0x0A36): (0x0C2E, "previous_actor_state"),
}
HOVER_IMMEDIATES = {
    ("mov", "bp", 0x2A27): (0x2CC7, "primary_actor_rectangle"),
}
CD_MEMORY = {
    ("gs", 0x0AE6): (0x0CEF, "cd_audio_available"),
    ("gs", 0x01B9): (0x0205, "cd_drive_number"),
    ("", 0x01B9): (0x0205, "cd_drive_number"),
    ("gs", 0x0B6C): (0x0D76, "requested_physical_track"),
    ("", 0x0B6D): (0x0D77, "packed_track_start"),
    ("", 0x0B5E): (0x0D68, "packed_disc_leadout"),
}
CD_IMMEDIATES = {
    ("mov", "bx", 0x0B41): (0x0D4B, "cd_ioctl_request"),
    ("mov", "bx", 0x0B72): (0x0D7C, "cd_playback_request"),
}
COPY_MEMORY = {
    ("gs", 0x5229): (0x55F9, "secondary_drawing_framebuffer"),
    ("gs", 0x5219): (0x55E9, "back_framebuffer"),
    ("gs", 0x252E): (0x2780, "cropped_scene_gate"),
    ("gs", 0x2527): (0x2779, "ship_depth_crop"),
}
ARCHIVE_MEMORY = {
    ("gs", 0x0A86): (0x0C7E, "archive_handle"),
    ("gs", 0x0A64): (0x0C5C, "directory_ems_handle"),
    ("gs", 0x0A66): (0x0C5E, "directory_ems_window"),
    ("gs", 0x0ABC): (0x0CB4, "directory_work_buffer"),
    ("gs", 0x0A62): (0x0C5A, "directory_xms_handle"),
    ("", 0x0A4A): (0x0C42, "xms_driver_entry"),
    ("gs", 0x0A88): (0x0C80, "directory_file_handle"),
    ("gs", 0x0AE2): (0x0CEB, "archive_member_selected"),
    ("gs", 0x0A8E): (0x0C86, "member_byte_count"),
    ("gs", 0x0A92): (0x0C8A, "member_bytes_remaining"),
    ("gs", 0x0A8A): (0x0C82, "member_position_low"),
    ("gs", 0x0A8C): (0x0C84, "member_position_high"),
}
SUBTITLE_MEMORY = {
    ("", 0x0F18): (0x1166, "sequence_subtitle_cursor"),
    ("", 0x131C): (0x156A, "visible_video_frame"),
}
NOISE_MEMORY = {
    ("", 0x5235): (0x5605, "clip_left"),
    ("", 0x5237): (0x5607, "clip_right"),
    ("", 0x5239): (0x5609, "clip_top"),
    ("", 0x5221): (0x55F1, "drawing_framebuffer"),
}
BRIDGE_FIELDS = {
    0x6724: (0x6AEC, "var_pointer"),
    0x6726: (0x6AEE, "var_segment"),
    0x672C: (0x6AF0, "object_directory_pointer"),
    0x6752: (0x6B22, "arche_object"),
    0x0A7C: (0x0C74, "loaded_artwork_pointer"),
    0x0A80: (0x0C78, "presentation_resource_pointer"),
    0x2793: (0x2A33, "presentation_mode_bits"),
    0x27E4: (0x2A85, "presentation_reverse"),
    0x2A93: (0x2D33, "black_hole_actor_latch"),
    0x0A32: (0x0C2A, "actor_state"),
    0x0A34: (0x0C2C, "actor_animation_tick"),
    0x6768: (0x6B3A, "requested_script_action"),
    0x676A: (0x6B3C, "requested_script_object"),
    0x27D5: (0x2A75, "black_hole_destination"),
    0x2792: (0x2A2D, "ship_scene_dispatch_pending"),
    0x278C: (0x2A27, "ship_active"),
    0x278E: (0x2A29, "black_hole_enabled"),
    0x278A: (0x2A25, "chart_view_active"),
    0x675A: (0x6B2C, "requested_radio_object"),
    0x6756: (0x6B26, "radio_object"),
    0x67AC: (0x6B82, "presentation_active"),
    0x1FB2: (0x2200, "scene_presentation_queued"),
    0x2565: (0x27B7, "choice_active"),
    0x2736: (0x29C4, "save_active"),
    0x2737: (0x29C5, "load_active"),
    0x2A19: (0x2CB9, "console_item_selected"),
    0x27E7: (0x2A88, "target_selection_active"),
    0x27DA: (0x2A7A, "transition_pending"),
    0x0B13: (0x0D1D, "input_stop_gate"),
    0x0A3E: (0x0C36, "primary_pointer_pressed"),
    0x0A40: (0x0C38, "pointer_press_pending"),
    0x2795: (0x2A35, "bridge_view_frame"),
    0x279B: (0x2A3B, "bridge_seek_target"),
    0x0A2A: (0x0C22, "pointer_x"),
    0x0A2C: (0x0C24, "pointer_y"),
    0x27E2: (0x2A82, "status_hover_state"),
    0x6758: (0x6B28, "ark_object"),
    0x5E58: (0x6228, "text_reveal_cursor"),
    0x27DF: (0x2A7F, "camera_approach_phase"),
    0x2F65: (0x3335, "ship_camera_x"),
    0x2F69: (0x3339, "ship_camera_z"),
    0x2F6B: (0x333B, "ship_camera_acceleration"),
    0x2F71: (0x3341, "ship_projection_angle"),
    0x1F20: (0x216E, "hyperspace_sequence_index"),
    0x6788: (0x6B5A, "active_presentation_line"),
    0x5221: (0x55F1, "drawing_framebuffer"),
    0x27E8: (0x2A89, "name_effect_active"),
    0x27E9: (0x2A8A, "name_effect_restart"),
    0x27F1: (0x2A91, "name_effect_sequence_table"),
    0x27ED: (0x2A8D, "name_effect_frame_cursor"),
    0x27EF: (0x2A8F, "name_effect_operation_and_count"),
    0x27F0: (0x2A90, "name_effect_frames_remaining"),
    0x0B15: (0x0D1F, "keyboard_character"),
    0x272E: (0x29BC, "selected_save_row"),
    0x2734: (0x29C2, "selected_save_name_pointer"),
    0x2732: (0x29C0, "save_edit_length"),
    0x2AAB: (0x2D4B, "choice_rectangle_x"),
    0x2AAF: (0x2D4F, "choice_rectangle_width"),
    0x0ADB: (0x0CE4, "text_selector"),
    0x27E6: (0x2A87, "choice_layout_guard"),
    0x2751: (0x29DF, "contact_presentation_pending"),
    0x24F3: (0x2745, "ship_flags"),
    0x2527: (0x2779, "ship_depth_offset"),
}
BRIDGE_MEMORY = {
    (segment, offset): identity
    for segment in ("", "gs")
    for offset, identity in BRIDGE_FIELDS.items()
}
BRIDGE_IMMEDIATES = {
    ("mov", "bp", 0x2BC7): (0x2F97, "world_artwork_rows"),
    ("mov", "si", 0x2BC7): (0x2F97, "world_artwork_rows"),
    ("mov", "bp", 0x6886): (0x6C2E, "navigation_scratch_list"),
    ("mov", "bp", 0x24FB): (0x274D, "aboard_position_list"),
    ("mov", "si", 0x0D16): (0x0F64, "menu_sound_bank_name"),
    ("mov", "bp", 0x2A1B): (0x2CBB, "actor_slot_records"),
    ("mov", "si", 0x65F2): (0x69C2, "status_hover_entity"),
    ("mov", "di", 0x0E18): (0x1066, "status_text_buffer"),
    ("mov", "si", 0x012E): (0x012D, "planet_label"),
    ("mov", "si", 0x013E): (0x0142, "ship_label"),
    ("mov", "si", 0x014B): (0x014E, "black_hole_label"),
    ("mov", "si", 0x016C): (0x0170, "life_support_label"),
    ("mov", "si", 0x1F22): (0x2170, "hyperspace_resource_names"),
    ("mov", "di", 0x2106): (0x2358, "hyperspace_name_buffer"),
    ("mov", "si", 0x27F1): (0x2A91, "name_effect_sequence_table"),
    ("mov", "si", 0x273B): (0x29C9, "save_edit_buffer"),
    ("mov", "si", 0x2B13): (0x2DC3, "record_choice_list"),
    ("mov", "di", 0x2B13): (0x2DC3, "record_choice_list"),
    ("mov", "si", 0x6D3E): (0x70E6, "contact_slots"),
    ("mov", "si", 0x2AAB): (0x2D4B, "choice_rectangle"),
    ("mov", "di", 0x253D): (0x278F, "choice_transition_target"),
    ("mov", "si", 0x5251): (0x5621, "live_palette"),
    ("mov", "di", 0x5B58): (0x5F28, "ship_palette_snapshot"),
}
INDEXED_MEMORY = {("cs", "bx", "", 0x6D4): (0x758, "actor_handler_table")}
MEMORY_MAPS = {
    "archive": ARCHIVE_MEMORY,
    "sprite": SPRITE_MEMORY,
    "hover": HOVER_MEMORY,
    "cd": CD_MEMORY,
    "copy": COPY_MEMORY,
    "subtitle": SUBTITLE_MEMORY,
    "noise": NOISE_MEMORY,
    "bridge": BRIDGE_MEMORY,
}
IMMEDIATE_MAPS = {
    "archive": {("mov", "di", 0x0A6C): (0x0C64, "xms_directory_transfer_request")},
    "sprite": SPRITE_IMMEDIATES,
    "hover": HOVER_IMMEDIATES,
    "cd": CD_IMMEDIATES,
    "copy": {},
    "noise": {},
    "subtitle": {("mov", "bp", 0x0AF2): (0x0CFC, "subtitle_line_buffer")},
    "bridge": BRIDGE_IMMEDIATES,
}
# Memory-destination immediates are approved at their exact instruction sites.
SITE_IMMEDIATES = {
    0x1362: (0x0B5B, 0x0D65, "disc_information_transfer"),
    0x136F: (0x0B6B, 0x0D75, "track_information_transfer"),
    0x1383: (0x0B62, 0x0D6C, "channel_mix_transfer"),
}
FAR_CALLS = {
    (0x299, 0x00D6): (0x2B1, 0x00D6, "draw_bios_font_line", 0x33E6),
    (0x1CE, 0x0B02): (0x1E6, 0x0B03, "random_word", 0x3163),
    (0x1CE, 0x02C4): (0x1E6, 0x02C4, "load_resource", 0x2924),
    (0x299, 0x1241): (0x2B1, 0x133E, "transition_entity", 0x464E),
    (0x299, 0x1037): (0x2B1, 0x1134, "load_palette_resource", 0x4444),
    (0x299, 0x11BE): (0x2B1, 0x12BB, "set_entity_record", 0x45CB),
    (0x1CE, 0x07DB): (0x1E6, 0x07E0, "load_named_resource", 0x2E40),
    (0xB1B, 0x011D): (0xC74, 0x011D, "update_audio_playback", 0xD05D),
    (0xB1B, 0x0855): (0xC74, 0x08AC, "load_sound_bank", 0xD7EC),
    (0x4DA, 0x0EAB): (0x502, 0x103D, "collect_navigation_source", 0x685D),
    (0x299, 0x12B0): (0x2B1, 0x13AD, "mark_entity_range_dirty", 0x46BD),
    (0x971, 0): (0xACB, 0, "dispatch_presentation_scene", 0xB4B0),
    (0x299, 0x0DEB): (0x2B1, 0x0EE8, "clear_display_band", 0x41F8),
    (0x299, 0x0E2F): (0x2B1, 0x0F2C, "clear_back_buffer_band", 0x423C),
    (0x299, 0x0F3E): (0x2B1, 0x103B, "present_chunky_frame", 0x434B),
    (0x299, 0x0CDC): (0x2B1, 0x0DD9, "fill_solid_rectangle", 0x40E9),
    (0x299, 0x0176): (0x2B1, 0x0176, "draw_game_font", 0x3486),
    (0x4DA, 0x1E2F): (0x502, 0x29C6, "rebuild_target_list", 0x81E6),
    (0x8B, 0x0FAD): (0x77, 0x115E, "transition_choice_rectangle", 0x20CE),
}
NEAR_CALLS = {
    0x32AC: (0x3D49, "draw_horizontal_span"),
    0x3321: (0x3E41, "draw_vertical_span"),
    0x6023: (0x6633, "object_field_resolve"),
    0x7E1C: (0x8EF0, "update_presentation_line"),
    0x8269: (0x93CB, "actor_pointer_hit_test"),
    0x959D: (0xAD37, "initialize_bridge_screen"),
    0x98B9: (0xB058, "build_ship_projection_matrix"),
    0x9A10: (0xB1AF, "project_ship_point_cloud"),
    0x9B98: (0xB337, "project_ship_object_sprites"),
    0x8C96: (0x9EA6, "snapshot_hud_palette_and_reset_camera"),
    0x210E: (0x23A2, "dispatch_input"),
    0x17AF: (0x1971, "select_display_page"),
    0x178B: (0x194D, "upload_palette"),
    0x8428: (0x958A, "update_choice_list"),
}
ROUTINES = (
    (0x1502, 0x1555, 0x1344, "cd"),
    (0x1582, 0x163D, 0x13C4, "cd"),
    (0x2049, 0x20CE, 0x1DD8, "bridge"),
    (0x2142, 0x2191, 0x1EC1, "bridge"),
    (0x2A4F, 0x2B43, 0x26CF, "archive"),
    (0x3EF3, 0x3F13, 0x3B45, "bridge"),
    (0x4002, 0x40E9, 0x3B85, "noise"),
    (0x42D8, 0x42ED, 0x3E5B, "copy"),
    (0x434B, 0x43E4, 0x3ECE, "copy"),
    (0x49B3, 0x4B33, 0x4536, "sprite"),
    (0x4B39, 0x5025, 0x46BC, "sprite"),
    (0x5025, 0x5153, 0x4BA8, "sprite"),
    (0x5153, 0x53DF, 0x4CD6, "sprite"),
    (0x53DF, 0x5517, 0x4F62, "sprite"),
    (0x8008, 0x8103, 0x6FF3, "bridge"),
    (0x8154, 0x81E6, 0x713D, "bridge"),
    (0x8923, 0x8987, 0x78D0, "hover"),
    (0x8DBC, 0x8E4F, 0x7CE8, "subtitle"),
    (0x8E4F, 0x8EF0, 0x7D7B, "bridge"),
    (0x8EF0, 0x8F88, 0x7E1C, "bridge"),
    (0x8F94, 0x9070, 0x7EC0, "bridge"),
    (0x935D, 0x93CB, 0x81FB, "bridge"),
    (0x944A, 0x958A, 0x82E8, "bridge"),
    (0x98D1, 0x9962, 0x872C, "bridge"),
    (0x9962, 0x99ED, 0x87BD, "bridge"),
    (0x99ED, 0x9A11, 0x8848, "bridge"),
    (0x9C5E, 0x9DBB, 0x8A4E, "bridge"),
    (0x9DBB, 0x9EA6, 0x8BAB, "bridge"),
)

# These changed bodies are reviewed block-by-block, not claimed byte-equivalent.
ACTOR_REVIEWS = (
    (
        0x9070,
        0x91AD,
        0x7F9C,
        "2d66c40262b9fdb8f996a986ec6784edd663717167538cc59e837770b8750da1",
        "big_bug_bang_travel_options.jsonl",
        (
            (
                0x9070,
                0x9082,
                "Require outer mode and zero black-hole slot flags; no writes on either rejection.",
            ),
            (
                0x9082,
                0x90AF,
                "Preserve original AL flags; present+ready restarts selector 10, transitions entities 0/4, advances line.",
            ),
            (
                0x90AF,
                0x90D2,
                "Frame 1: enabled travel publishes C1 before comparing deferred target to Arche; same target enters bridge reset, otherwise camera countdown=8.",
            ),
            (
                0x90D2,
                0x90EF,
                "Other frames: nonzero countdown skips completion; zero writes line flags=7, C1, transition=1; travel option chooses bridge reset.",
            ),
            (
                0x90EF,
                0x9125,
                "Reset clears action, transition, camera view, UI word, dialogue hold, ship block/depth/opening and HUD byte; ship flags=5. Runtime BridgeReset adapter publishes these owned fields.",
            ),
            (
                0x9125,
                0x9139,
                "Transition entity 4, close location panel, request redraw, return; retains deferred target on bridge reset.",
            ),
            (
                0x9139,
                0x9153,
                "Active location panel returns; otherwise resource=20, reverse=1, line flags=0, redraw.",
            ),
            (
                0x9153,
                0x917E,
                "Missing deferred record or no reverse/panel skips. Original AL bit4 controls second-pass preparation, resource20 and audio5.",
            ),
            (
                0x917E,
                0x91AD,
                "Advance line; incomplete returns. Completion without panel clears deferred target and line; with panel selects idle resource18, present flag and redraw.",
            ),
        ),
    ),
    (
        0x91AD,
        0x927A,
        0x8082,
        "ba8a714ed146cc15deeb0117ea5699bf0f016f0dbc2fde0a94a480a23727bc21",
        "big_bug_bang_panel_activation.jsonl",
        (
            (
                0x91AD,
                0x91BF,
                "Overview mask1 blocks before any writes; otherwise require outer mode. Runtime run_camera supplies overview gate.",
            ),
            (
                0x91BF,
                0x91E8,
                "Inactive actor marks line present and checks ready. Black-hole/hyperjump blockers activate camera actor, close location panel and return.",
            ),
            (
                0x91E8,
                0x9207,
                "Transition entity0; clear selected location. Pending CC skips selector10 and primary-input clear; ordinary path performs both.",
            ),
            (
                0x9207,
                0x9230,
                "Advance line preserving completion carry across frame7 camera-page callback, audio3, countdown8 and redraw.",
            ),
            (
                0x9230,
                0x9241,
                "Completion clears camera actor and sets line flags7. AL transition bit controls view toggle.",
            ),
            (
                0x9241,
                0x9267,
                "Toggle view and clear redraw; entering synthesizes secondary press and redraw; leaving restores ship palette/camera and requests rebuild.",
            ),
            (
                0x9267,
                0x927A,
                "Close location panel, line flags1, transition entity4, return.",
            ),
        ),
    ),
    (
        0x927A,
        0x92D0,
        0x813A,
        "82695527e2641df112397594aaae7fab22465525a13cd10f730199b47fe84390",
        None,
        (
            (
                0x927A,
                0x928D,
                "Bind Arche; navigation-link word+22 ==0 returns untouched. High-bit and all other nonzero words pass. Runtime reads live typed VAR field.",
            ),
            (
                0x928D,
                0x929F,
                "Require UI mask90, mark line present, return if not ready.",
            ),
            (
                0x929F,
                0x92B2,
                "Selector16 then advance line; incomplete returns, completed plays audio5.",
            ),
            (
                0x92B2,
                0x92D0,
                "Ship flags1, copy576 palette bytes, depth0, line flags7. Complete shared suffix checked separately by exact relocation comparison.",
            ),
        ),
    ),
    (
        0x92D0,
        0x935D,
        0x817E,
        "3e5a6856b0ca5bf79bcf56315302f4843a59e5cad217d495e95adb7c3f61e750",
        "big_bug_bang_presentation.jsonl",
        (
            (
                0x92D0,
                0x92E4,
                "Require secondary mode, mark line present, skip playback when not ready.",
            ),
            (
                0x92E4,
                0x92F8,
                "Pending CC bypasses hand and ending gate. Otherwise selector13; ending bit1 returns immediately.",
            ),
            (
                0x92F8,
                0x9318,
                "Active panel and signed phase<100 sets phase106; queued scene then finalized.",
            ),
            (
                0x9318,
                0x932F,
                "Clear primary and pending input, advance line; incomplete skips to final latch; complete transitions entity4.",
            ),
            (
                0x932F,
                0x934A,
                "Set line flags1; inactive panel becomes active and requests redraw, already active panel does not add redraw.",
            ),
            (
                0x934A,
                0x935D,
                "Active panel plus loaded line latches completion; return.",
            ),
        ),
    ),
)

CONTROL_REVIEWS = (
    (
        0x8830,
        0x8923,
        0x77E0,
        "27728a2656dbbad55d02981f72efb03f6b5773dfb36b68246ccc0b0fe68072db",
        "big_bug_bang_panel_activation.jsonl",
        (
            (
                0x8830,
                0x8852,
                "Inactive UI returns; pending scene dispatches and returns without later bridge work.",
            ),
            (
                0x8852,
                0x8886,
                "Rebuild sets current/previous hand1; steering input chooses hand2/3 at unsigned x160 boundary and flips page.",
            ),
            (
                0x8886,
                0x88AD,
                "Advance pending approach, update mode, then BBB-only CC activation helper before sprite commit, hover and actor pass. Runtime backend supplies helper here.",
            ),
            (
                0x88AD,
                0x88D7,
                "Without queued presentation, pending approach draws sprites20..31; otherwise zero animation countdown copies dirty regions.",
            ),
            (
                0x88D7,
                0x8901,
                "Chart/camera-state reconciliation, destination navigation, then panel update. Frame-ready gate precedes actor sprites1..19, name effect, status and console.",
            ),
            (
                0x8901,
                0x8923,
                "Actor-completion latch remaps rectangle137,139,50,44 using second remap; restore and far return.",
            ),
        ),
    ),
    (
        0x8987,
        0x8A48,
        0x792D,
        "9d848cac9f22c5876eda70216b0ea55f7e75f0f7a0a7fe07eaed7abeacc07e5e",
        "big_bug_bang_travel_options.jsonl",
        (
            (
                0x8987,
                0x89BE,
                "Camera view or nonzero approach returns. Arche linked object's kind mask18 and first successful region31 poll are required.",
            ),
            (
                0x89BE,
                0x89E5,
                "Hand12; BBB enabled-travel bypasses zero destination access count. Otherwise set UI bit4, arm unlocked palette slot and return; runtime must not request screen rebuild here.",
            ),
            (
                0x89E5,
                0x8A17,
                "Zero768 fade-target bytes, copy768 live bytes to source, percent0, step20, palette range0..255.",
            ),
            (
                0x8A17,
                0x8A48,
                "Clear complete UI word, ship flags5, request HUD init; clear dialogue hold, scene block, depth, depth opening and HUD byte; restore and return.",
            ),
        ),
    ),
    (
        0x8A7D,
        0x8D88,
        0x79E5,
        "d94892180420584551d760f530de3073af6ff2fbfed00bcfb10976df7300b420",
        "big_bug_bang_presentation.jsonl",
        (
            (
                0x8A7D,
                0x8AD9,
                "Require active panel. Queued scene takes separate path. Initial phase sets text origin1, choice0 overridden/consumed by pending CC, previous hand15, phase1, entity31 transition and audio1.",
            ),
            (
                0x8AD9,
                0x8B43,
                "Six authored expansion rectangles followed by three remap/noise frames. Signed phase>=100 enters close path.",
            ),
            (
                0x8B43,
                0x8BD3,
                "Active panel remaps front/back, clears back140-row band. Empty selected record draws noise and choice; primary press takes common input action.",
            ),
            (
                0x8BD3,
                0x8C14,
                "Load selected DESCRIPT, optionally reload music, start stream; initialize scene-list cursor/count and visible frame, then pump scenes.",
            ),
            (
                0x8C14,
                0x8C3F,
                "Pending CC replaces/consumes choice then accepts input. Otherwise ending suppresses primary cancellation; dispatch queued scene and test continued ownership.",
            ),
            (
                0x8C3F,
                0x8C64,
                "Each remaining scene advances16-byte list entry, publishes line2 and loops into queued dispatch.",
            ),
            (
                0x8C64,
                0x8C8D,
                "Whole list completion resets subtitle cursor, requests rebuild and phase7. Reverse retains transition; ending requests shutdown; ordinary sequel completion closes at phase106 and clears back content.",
            ),
            (
                0x8C8D,
                0x8C9D,
                "Queued frame-ready draws subtitle then choice, otherwise waits.",
            ),
            (
                0x8C9D,
                0x8CEC,
                "Reverse input closes and finalizes only if queued; ordinary input selects hand14, prepares/plays audio1, finalizes, cycles six choices and sets phase7.",
            ),
            (
                0x8CEC,
                0x8D14,
                "Clear back content rectangle0,10,320,130 and restore front target.",
            ),
            (
                0x8D14,
                0x8D58,
                "Final phase100 hides resource, clears panel/UI modal, phase0, completion audio, text origin8, rebuild; reverse restores variant12, otherwise reset ship camera.",
            ),
            (
                0x8D58,
                0x8D88,
                "Closing decrements phase and draws matching reverse authored rectangle; restore and return.",
            ),
        ),
    ),
    (
        0x9778,
        0x98A3,
        0x85E2,
        "32e3b73083bc927d2cf687eb3bd0a7fa56264e44cbd13d11e1add4e4c9b06b65",
        None,
        (
            (
                0x9778,
                0x97AE,
                "Queued scene, save/load, text/BBB simulation submenu, confirmation or active presentation rejects. Existing selection bypasses hit testing.",
            ),
            (
                0x97AE,
                0x97DC,
                "Signed bridge frame40..60 required; input sample then reset five console palette rows to16,12,0.",
            ),
            (
                0x97DC,
                0x982B,
                "Frame-relative signed horizontal bounds; wrapped y difference and native row height select one of five rows.",
            ),
            (
                0x982B,
                0x9845,
                "Highlight selected row red63; primary press required for activation.",
            ),
            (
                0x9845,
                0x988B,
                "Hand5, selection=row+1, UI bits4/8, seek90, layout1, target y80+row18; initialize list input gates, width100, delay10 and audio4.",
            ),
            (
                0x988B,
                0x98A3,
                "UI bit8 defers; otherwise call five-entry handler table by selection-1; restore and return.",
            ),
        ),
    ),
    (
        0x9A11,
        0x9B45,
        0x886C,
        "6de2438c22cd75c5dae406929d6d843855a8aa68f53fcf5d7bab4584f5a968ae",
        "big_bug_bang_options.json",
        (
            (
                0x9A11,
                0x9A46,
                "On layout phase disable VM, reset selector, guarded list layout, increment phase and save measured rectangle.",
            ),
            (
                0x9A46,
                0x9A6D,
                "Transition must complete before interactive list; negative result keeps menu open.",
            ),
            (
                0x9A6D,
                0x9AB6,
                "Rows0/1 open simulation/text speed with phase1; row2 toggles travel independent of audio and replaces label.",
            ),
            (
                0x9AB6,
                0x9B05,
                "Row3 unsupported music does nothing; otherwise toggle, reset stream latches, replace label and load/start only when enabling.",
            ),
            (
                0x9B05,
                0x9B38,
                "Rows4/5 request save/load and panel. Row6 requests confirmation2 and clears primary0C36 and pending0C38, never secondary0C37.",
            ),
            (
                0x9B38,
                0x9B45,
                "Selected action or cancel clears console selection and UI modal bit, then returns.",
            ),
        ),
    ),
)
CONTROL_OWNERS = {0x9A11: "update_sequel_option_menu"}
CONTROL_RUNTIME = (
    "crates/commander-blood-game/src/runtime/bridge_frame.rs",
    "crates/commander-blood-game/src/runtime/camera_navigation.rs",
    "crates/commander-blood-game/src/runtime/bridge_console.rs",
    "crates/commander-blood-game/src/runtime/presentation_screen.rs",
)


def sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def decode(body: bytes, address: int):
    decoder = Cs(CS_ARCH_X86, CS_MODE_16)
    decoder.detail = True
    instructions = list(decoder.disasm(body, address))
    if sum(item.size for item in instructions) != len(body):
        raise ValueError(f"incomplete instruction decode at {address:#x}")
    return instructions


def compare_body(original: bytes, sequel: bytes, old: int, new: int, kind: str):
    """Require byte identity after only specifically approved address patches."""
    memory = MEMORY_MAPS[kind]
    immediates = IMMEDIATE_MAPS[kind]
    instructions = decode(original, old)
    patches = []
    transformed = bytearray(original)
    for ins in instructions:
        if ins.mnemonic == "call" and ins.operands[0].type == X86_OP_IMM:
            target = ins.operands[0].imm
            if old <= target < old + len(original):
                continue
            if target not in NEAR_CALLS or ins.bytes[0] != 0xE8 or ins.size != 3:
                raise ValueError(f"unreviewed near call at {ins.address:#x}")
            entry, identity = NEAR_CALLS[target]
            start = ins.address - old
            relative = (entry - (new + start + ins.size)) & 0xFFFF
            transformed[start + 1 : start + 3] = relative.to_bytes(2, "little")
            patches.append(
                {
                    "commander_site": f"0x{ins.address:04x}",
                    "bbb_site": f"0x{new + start:04x}",
                    "identity": identity,
                    "callee": f"0x{entry:04x}",
                }
            )
            continue
        if ins.mnemonic == "lcall" and ins.operands[0].type == X86_OP_IMM:
            target = tuple(operand.imm for operand in ins.operands)
            if target not in FAR_CALLS:
                raise ValueError(f"unreviewed far call at {ins.address:#x}")
            segment, offset, identity, entry = FAR_CALLS[target]
            start = ins.address - old
            if ins.size != 5 or ins.bytes[0] != 0x9A:
                raise ValueError("unexpected far-call encoding")
            transformed[start + 1 : start + 5] = offset.to_bytes(
                2, "little"
            ) + segment.to_bytes(2, "little")
            patches.append(
                {
                    "commander_site": f"0x{ins.address:04x}",
                    "bbb_site": f"0x{new + ins.address - old:04x}",
                    "identity": identity,
                    "callee": f"0x{entry:04x}",
                }
            )
            continue
        for operand in ins.operands:
            replacement = None
            if operand.type == X86_OP_MEM:
                mem = operand.mem
                if not mem.base and not mem.index:
                    replacement = memory.get(
                        (ins.reg_name(mem.segment) or "", mem.disp)
                    )
                else:
                    replacement = INDEXED_MEMORY.get(
                        (
                            ins.reg_name(mem.segment) or "",
                            ins.reg_name(mem.base) or "",
                            ins.reg_name(mem.index) or "",
                            mem.disp,
                        )
                    )
                offset, size = ins.disp_offset, ins.disp_size
            elif operand.type == X86_OP_IMM and len(ins.operands) == 2:
                destination = ins.reg_name(ins.operands[0].reg)
                replacement = immediates.get((ins.mnemonic, destination, operand.imm))
                if ins.address in SITE_IMMEDIATES:
                    expected, value, identity = SITE_IMMEDIATES[ins.address]
                    if operand.imm != expected:
                        raise ValueError("unexpected transfer-pointer immediate")
                    replacement = (value, identity)
                offset, size = ins.imm_offset, ins.imm_size
            else:
                continue
            if replacement is None:
                continue
            value, identity = replacement
            if size != 2:
                raise ValueError(f"unexpected relocation width at {ins.address:#x}")
            start = ins.address - old + offset
            transformed[start : start + size] = value.to_bytes(size, "little")
            patches.append(
                {
                    "commander_site": f"0x{ins.address:04x}",
                    "bbb_site": f"0x{new + ins.address - old:04x}",
                    "identity": identity,
                }
            )
    if bytes(transformed) != sequel:
        raise ValueError(f"unreviewed instruction change at BBB {new:#x}")
    return instructions, patches


def build_report():
    bbb, commander = BBB.read_bytes(), COMMANDER.read_bytes()
    if (
        sha256(bbb)
        != "4b65ffca3e113a1826371e3436177861640a1b7aae24caafebb4c2f7aa467834"
    ):
        raise ValueError("unsupported BBB executable")
    if (
        sha256(commander)
        != "7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823"
    ):
        raise ValueError("unsupported Commander executable")
    with PORTED.open(newline="") as stream:
        ported = {
            int(row["entry"], 16): row
            for row in csv.DictReader(stream, delimiter="\t")
            if row["component"] == "bloodprg"
        }
    inputs = {
        str(path.relative_to(ROOT)): sha256(path.read_bytes())
        for path in (BBB, COMMANDER, PORTED, Path(__file__).resolve())
    }
    initializers = []
    for old_offset, new_offset, length, identity in (
        (0x0B41, 0x0D4B, 26, "cd_ioctl_request"),
        (0x0B5B, 0x0D65, 7, "disc_information_transfer"),
        (0x0B62, 0x0D6C, 9, "channel_mix_transfer"),
        (0x0B6B, 0x0D75, 7, "track_information_transfer"),
        (0x0B72, 0x0D7C, 22, "cd_playback_request"),
        (0x2B97, 0x2F67, 48, "presentation_panel_rectangles"),
    ):
        # MZ entry loads DS=GS=0xCE2 / 0xEFF, after 0x600 / 0x800 headers.
        original = commander[0xD420 + old_offset : 0xD420 + old_offset + length]
        native = bbb[0xF7F0 + new_offset : 0xF7F0 + new_offset + length]
        if original != native:
            raise ValueError(f"changed initializer for {identity}")
        initializers.append({"identity": identity, "bytes": native.hex()})
    rows = []
    commander_handlers = (0x7F9C, 0x7EC0, 0x813A, 0x817E, 0x81FB, 0x8082)
    sequel_handlers = (0x9070, 0x8F94, 0x927A, 0x92D0, 0x935D, 0x91AD)
    if (
        tuple(0x77E0 + item for item in struct.unpack_from("<6H", commander, 0x7EB4))
        != commander_handlers
    ):
        raise ValueError("Commander actor handler order changed")
    if (
        tuple(0x8830 + item for item in struct.unpack_from("<6H", bbb, 0x8F88))
        != sequel_handlers
    ):
        raise ValueError("BBB actor handler order changed")
    for entry, end, old, kind in ROUTINES:
        native = bbb[entry:end]
        original = commander[old : old + len(native) - (7 if kind == "hover" else 0)]
        source = ported[old]
        owner = {"path": source["rust_path"], "symbol": source["rust_symbol"]}
        notes = [source["documentation"]]
        if kind == "hover":
            # The only inserted instructions test overview bit 0 and jump to
            # the original pop-bp/ret. The complete remaining suffix is checked.
            if native[:8].hex() != "55f606302a01755a" or native[-2:] != b"\x5d\xc3":
                raise ValueError("BBB overview guard changed")
            if original[:1] != b"\x55":
                raise ValueError("Commander hover prologue changed")
            instructions, patches = compare_body(
                original[1:], native[8:], old + 1, entry + 8, kind
            )
            owner = {"path": HOVER, "symbol": "update_sequel_presentation_hover"}
            inputs[SERVICES] = sha256((ROOT / SERVICES).read_bytes())
            notes.append(
                "BBB overview guard returns without touching hover or actor state; "
                "runtime supplies sequel_overview.active(). Signed inclusive bounds, "
                "selection priority, actor 9, and previous-state restore are unchanged."
            )
        else:
            instructions, patches = compare_body(original, native, old, entry, kind)
            notes.append(
                "Complete body is byte-identical after only the named state/table "
                "and callee relocations. Relative branches, non-address constants, "
                "register widths, memory addressing, traversal, and return are unchanged."
            )
        if (
            f"fn {owner['symbol'].split('::')[-1]}"
            not in (ROOT / owner["path"]).read_text()
        ):
            raise ValueError(f"missing Rust owner {owner}")
        fixture = source["evidence"].split(":", 1)[0]
        inputs[owner["path"]] = sha256((ROOT / owner["path"]).read_bytes())
        inputs[fixture] = sha256((ROOT / fixture).read_bytes())
        rows.append(
            {
                "entry": f"0x{entry:04x}",
                "end": f"0x{end:04x}",
                "commander_entry": f"0x{old:04x}",
                "kind": kind,
                "body_sha256": sha256(native),
                "rust_owner": owner,
                "inherited_fixture": fixture,
                "instruction_count": len(instructions),
                "reviewed_relocations": patches,
                "notes": notes,
            }
        )
    for entry, end, old, digest, delta_fixture, blocks in (
        *ACTOR_REVIEWS,
        *CONTROL_REVIEWS,
    ):
        body = bbb[entry:end]
        if sha256(body) != digest:
            raise ValueError(f"changed reviewed body at {entry:#x}")
        if (
            blocks[0][0] != entry
            or blocks[-1][1] != end
            or any(left[1] != right[0] for left, right in zip(blocks, blocks[1:]))
        ):
            raise ValueError("review must cover the complete body")
        source = ported[old]
        fixture = source["evidence"].split(":", 1)[0]
        paths = [
            source["rust_path"],
            fixture,
            BRIDGE_ACTORS,
            SERVICES,
            SEQUEL_PRESENTATION,
        ]
        paths.extend(CONTROL_RUNTIME)
        if delta_fixture:
            paths.append(f"re/tools/oracle_vectors/{delta_fixture}")
        for path in paths:
            inputs[path] = sha256((ROOT / path).read_bytes())
        patches = []
        if entry == 0x927A:
            if body[:19].hex() != "5106668e06ee6a8b3e226b26837d1600077441":
                raise ValueError("Arche navigation-link gate changed")
            _, patches = compare_body(
                commander[0x813B:0x817E], bbb[0x928D:0x92D0], 0x813B, 0x928D, "bridge"
            )
        rows.append(
            {
                "entry": f"0x{entry:04x}",
                "end": f"0x{end:04x}",
                "commander_entry": f"0x{old:04x}",
                "kind": "reviewed_transcription",
                "body_sha256": digest,
                "rust_owner": {
                    "path": source["rust_path"],
                    "symbol": CONTROL_OWNERS.get(entry, source["rust_symbol"]),
                },
                "inherited_fixture": fixture,
                "instruction_count": len(decode(body, entry)),
                "reviewed_relocations": patches,
                "reviewed_blocks": [
                    [f"0x{start:04x}", f"0x{stop:04x}", note]
                    for start, stop, note in blocks
                ],
                "notes": [
                    "Complete BBB entry compared block-by-block with its Commander "
                    "baseline, typed handler, and production runtime adapter. Sequel "
                    "guards and changed paths are explicit; this is not an exact-body "
                    "equivalence or a new whole-entry native execution claim.",
                    *[note for _, _, note in blocks],
                ],
            }
        )
    bounds_body = bbb[0x5D5D:0x5DD7]
    if (
        sha256(bounds_body)
        != "b6caab984be4acc640532da88331d7d66404adc7b33034f3b56d91897fe90329"
    ):
        raise ValueError("sequel post-scan bounds body changed")
    if bbb[0x5B3D:0x5B46].hex() != "e80101e89402e81702":
        raise ValueError("sequel post-scan call ordering changed")
    for path in (GROWTH, DISPATCH):
        inputs[path] = sha256((ROOT / path).read_bytes())
    rows.append(
        {
            "entry": "0x5d5d",
            "end": "0x5dd7",
            "commander_entry": None,
            "kind": "reviewed_transcription",
            "body_sha256": sha256(bounds_body),
            "rust_owner": {"path": GROWTH, "symbol": "bound_sequel_simulation_fields"},
            "inherited_fixture": None,
            "instruction_count": len(decode(bounds_body, 0x5D5D)),
            "reviewed_relocations": [],
            "reviewed_blocks": [
                ["0x5d5d", "0x5d70", "Bind VAR/directory; DX=1000, CX=0."],
                [
                    "0x5d70",
                    "0x5d80",
                    "Directory-order selection: actor kind bit 1, participation bit 2 only.",
                ],
                [
                    "0x5d80",
                    "0x5da2",
                    "Signed clamps of aggressiveness +50 and relief +56 to [0,1000].",
                ],
                ["0x5da2", "0x5db3", "Signed clamp of quantity +22 to [0,1000]."],
                [
                    "0x5db3",
                    "0x5dc8",
                    "Signed clamp of growth balance +52; both NEG CX operations retain zero.",
                ],
                [
                    "0x5dc8",
                    "0x5dd7",
                    "Advance directory by 20 while next kind is object; restore registers and return.",
                ],
            ],
            "notes": [
                "Reviewed BBB-only transcription, not inherited Commander equivalence. "
                "The four writes preserve native order and signed 16-bit comparisons; "
                "nonparticipating actors and nonactors are untouched. All 256 flag bytes "
                "and signed boundary cases are checked against complete typed state.",
                "Caller 0x5B3D commits concepts, 0x5B40 scans presentation, 0x5B43 applies "
                "bounds, then 0x5B46 reloads the simulation clock. Dispatcher runs bounds "
                "after successful host scan; disabled/error paths bypass it. The focused "
                "dispatch test verifies post-scan writes are bounded and paused state remains untouched.",
            ],
        }
    )
    return {
        "format": "big_bug_bang_static_port_audit_v1",
        "scope": "Reviewed native equivalence plus Rust ownership; not a new native "
        "execution claim or a proof of whole-game runtime wiring.",
        "inputs": inputs,
        "identical_initializers": initializers,
        "actor_handler_order": [
            {"commander": f"0x{old:04x}", "bbb": f"0x{new:04x}", "handler": handler}
            for old, new, handler in zip(
                commander_handlers,
                sequel_handlers,
                (
                    "hyperjump",
                    "black_hole",
                    "ship_palette",
                    "panel_close",
                    "radio",
                    "camera",
                ),
                strict=True,
            )
        ],
        "routines": rows,
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("output", type=Path, nargs="?", default=OUTPUT)
    args = parser.parse_args()
    report = build_report()
    args.output.write_text(json.dumps(report, indent=2) + "\n")
    print(
        f"audited {len(report['routines'])} BBB native bodies against typed Rust owners"
    )


if __name__ == "__main__":
    main()
