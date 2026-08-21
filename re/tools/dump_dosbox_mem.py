#!/usr/bin/env python3
"""Dump BLOODPRG's live DOS memory from a running DOSBox-X, via ptrace.

This DOSBox-X build has no savestate/debugger, but DOSBox-X is a Linux process and the
DOS RAM lives in its address space. Under yama ptrace_scope=1 a process can ptrace its
own CHILD, so this script LAUNCHES dosbox-x itself, then PTRACE_ATTACHes and reads
/proc/<pid>/mem — no root needed.

It locates BLOODPRG's DS by finding the adjacent startup error strings at DS:0 and
validating live arena and VGA fields; every DS-relative global is then a fixed offset
from that anchor, independent of the load segment. Reads the requested DS offsets (default: the star-map nav state —
the 11 destination records at DS:0x4F09 and the camera origin at DS:0x2F65).

Usage: nix develop --command re/tools/dump_dosbox_mem.py <cd-dir> [wait_secs] [install-parent] [executable] [cycles]
  <cd-dir>         the CD image dir CONTAINING BLOODPRG.EXE (e.g. output/_tmp_iso),
                   mounted as D: — NOT the installed data dir.
  <install-parent> parent of the `cblood` install dir, mounted as C: so the game's
                   write path C:\\cblood\\ resolves (defaults to accuracy/cblood_install).
It launches exactly what BLOOD.BAT does: `D:` then
`BLOODPRG AMR S162227 EMS WRIC:\\cblood\\`; without those args the game loops the
attract demo and never reaches navigation.
The cycle setting defaults to `max`. Compare behavior at matching guest timer
ticks; wall-clock throughput depends on compiler code generation.
NOTE: the star-map's 0x4F09 records are the *default* (10200,12100,900) until the game
is in ACTIVE navigation — drive it there (see drive_real_game.sh) before dumping.
"""
import ctypes, hashlib, subprocess, time, os, re, struct, sys
from pathlib import Path

ANCHOR = b"386 minimum !\0Not enough memory (570Ko min) !\0"
LOCATOR_ANCHOR = b"386 minimum "
DS_ANCHOR = 0x0000
# DS globals of interest -> (name, offset, count_words)
GLOBALS = [
    ("origin_2F65", 0x2F65, 3),   # camera origin x,y,z
    ("angle_2F71", 0x2F71, 1),    # camera angle
    ("angle_2F6D", 0x2F6D, 1),    # compass angle
    ("nav_recs_4F09", 0x4F09, 33),  # 11 records x 3 words
]
STARTUP_GLOBALS = [
    ("startup_dos_pool_0A42", 0x0A42, "<HH"),
    ("resource_free_bytes_0A46", 0x0A46, "<I"),
    ("resource_copy_buffer_0A7C", 0x0A7C, "<HH"),
    ("list_d8c_base_segment_0A7E", 0x0A7E, "<H"),
    ("resource_copy_file_handle_0A84", 0x0A84, "<H"),
    ("resource_archive_offset_0A8A", 0x0A8A, "<I"),
    ("resource_archive_remaining_0A8E", 0x0A8E, "<I"),
    ("snd_source_remaining_0A92", 0x0A92, "<I"),
    ("snd_bank_xms_handle_0A5E", 0x0A5E, "<h"),
    ("snd_bank_ems_handle_0A60", 0x0A60, "<h"),
    ("alien_overlay_slot_0A96", 0x0A96, "<HH"),
    ("video_crtc_base_port_0A9E", 0x0A9E, "<H"),
    ("graphics_work_surface_0ABC", 0x0ABC, "<HH"),
    ("list_d8c_default_entry_segment_0ABE", 0x0ABE, "<H"),
    ("startup_write_directory_active_0AE0", 0x0AE0, "<B"),
    ("game_mode_0ADF", 0x0ADF, "<B"),
    ("resource_force_write_directory_0AE1", 0x0AE1, "<B"),
    ("resource_path_is_embedded_0AE2", 0x0AE2, "<B"),
    ("timer_state_block_offset_0AF0", 0x0AF0, "<H"),
    ("video_retrace_phase_0B12", 0x0B12, "<B"),
    ("ship_3d_nav_choice_sound_gate_0B13", 0x0B13, "<B"),
    ("timer_hook_active_0B21", 0x0B21, "<B"),
    ("timer_divider_0B22", 0x0B22, "<B"),
    ("timer_tick_count_0B29", 0x0B29, "<H"),
    ("main_frame_delay_ticks_0B2D", 0x0B2D, "<H"),
    ("video_calibration_ticks_0B35", 0x0B35, "<H"),
    ("snd_bank_memory_0BB3", 0x0BB3, "<HH"),
    ("snd_stream_storage_0BB7", 0x0BB7, "<HH"),
    ("snd_stream_buffer_0_0B89", 0x0B89, "<HHHBB"),
    ("snd_stream_buffer_1_0B91", 0x0B91, "<HHHBB"),
    ("snd_stream_header_0B99", 0x0B99, "<3H"),
    ("snd_driver_pending_flag_0BA0", 0x0BA0, "<B"),
    ("snd_stream_header_mode_0BA2", 0x0BA2, "<B"),
    ("snd_stream_channel_active_0BA3", 0x0BA3, "<B"),
    ("snd_stream_next_page_0BA5", 0x0BA5, "<H"),
    ("snd_stream_page_count_0BA7", 0x0BA7, "<H"),
    ("snd_stream_final_page_bytes_0BA9", 0x0BA9, "<H"),
    ("list_d8c_audio_phase_0C41", 0x0C41, "<H"),
    ("snd_bank_file_handle_0C49", 0x0C49, "<H"),
    ("audio_position_callback_0CF3", 0x0CF3, "<HH"),
    ("list_d8c_file_handle_0D5B", 0x0D5B, "<H"),
    ("list_d8c_state_byte_0D5F", 0x0D5F, "<B"),
    ("list_d8c_read_wrap_index_0D60", 0x0D60, "<H"),
    ("list_d8c_wrap_count_0D62", 0x0D62, "<H"),
    ("list_d8c_read_wrap_limit_0D64", 0x0D64, "<H"),
    ("list_d8c_secondary_wrap_limit_0D66", 0x0D66, "<H"),
    ("resource_flags_0D76", 0x0D76, "<H"),
    ("list_d8c_tick_threshold_0D77", 0x0D77, "<B"),
    ("resource_range_start_0D6E", 0x0D6E, "<I"),
    ("resource_range_remaining_0D72", 0x0D72, "<I"),
    ("resource_index_start_0D78", 0x0D78, "<I"),
    ("resource_index_remaining_0D7C", 0x0D7C, "<I"),
    ("resource_requested_id_0D80", 0x0D80, "<H"),
    ("resource_active_id_0D82", 0x0D82, "<H"),
    ("resource_source_offset_0D84", 0x0D84, "<I"),
    ("resource_source_remaining_0D88", 0x0D88, "<I"),
    ("list_d8c_head_pointer_0D8C", 0x0D8C, "<HH"),
    ("list_d8c_tail_pointer_0D90", 0x0D90, "<HH"),
    ("list_d8c_active_pointer_0D94", 0x0D94, "<HH"),
    ("list_d8c_wrap_limit_0D98", 0x0D98, "<H"),
    ("list_d8c_byte_count_0D9A", 0x0D9A, "<H"),
    ("list_d8c_palette_offset_0D9E", 0x0D9E, "<H"),
    ("list_d8c_iteration_count_0DA0", 0x0DA0, "<H"),
    ("list_d8c_previous_tick_0DA2", 0x0DA2, "<H"),
    ("list_d8c_active_layout_0DA4", 0x0DA4, "<H"),
    ("list_d8c_active_row_mode_0DA6", 0x0DA6, "<H"),
    ("list_d8c_retired_segment_0DAA", 0x0DAA, "<H"),
    ("list_d8c_rollover_state_0DAC", 0x0DAC, "<B"),
    ("list_d8c_entry_metric_0DAF", 0x0DAF, "<H"),
    ("resource_frame_presented_0DB8", 0x0DB8, "<B"),
    ("resource_draw_via_back_buffer_0DB9", 0x0DB9, "<B"),
    ("resource_decode_rectangular_0DBA", 0x0DBA, "<B"),
    ("resource_skip_back_buffer_present_0DBB", 0x0DBB, "<B"),
    ("resource_unclamped_row_count_0DBD", 0x0DBD, "<B"),
    ("resource_source_is_banked_0DBC", 0x0DBC, "<B"),
    ("resource_decode_mode_0AA0", 0x0AA0, "<H"),
    ("vm_c2_presentation_gate_1FB2", 0x1FB2, "<B"),
    ("vm_resource_handles_6712", 0x6712, "<5H"),
    ("vm_profile_cursor_6730", 0x6730, "<H"),
    ("vm_subtitle_wrap_marker_6732", 0x6732, "<H"),
    ("vm_profile_record_word_6734", 0x6734, "<H"),
    ("vm_resource_profile_index_677E", 0x677E, "<H"),
    ("vm_script_profile_request_6780", 0x6780, "<h"),
    ("vm_execution_enabled_67A8", 0x67A8, "<B"),
    ("vm_presentation_request_flags_67AA", 0x67AA, "<B"),
    ("vm_presentation_active_67AC", 0x67AC, "<B"),
    ("vm_query_mode_67AD", 0x67AD, "<B"),
    ("vm_resume_state_67B1", 0x67B1, "<B"),
    ("vm_yield_flag_67B4", 0x67B4, "<B"),
    ("vm_branch_stack_6820", 0x6820, "<8H"),
    ("vm_branch_stack_top_6884", 0x6884, "<H"),
    ("vm_ui_state_2793", 0x2793, "<H"),
    ("nav_screen_rebuild_pending_27D9", 0x27D9, "<B"),
    ("presentation_mode_flag_27E0", 0x27E0, "<B"),
    ("presentation_mode_flag_27E1", 0x27E1, "<B"),
    ("presentation_box_phase_2B93", 0x2B93, "<h"),
    ("ship_3d_depth_offset_2527", 0x2527, "<H"),
    ("ship_3d_scene_dispatch_blocked_252D", 0x252D, "<B"),
    ("ship_3d_plane_blit_crop_enabled_252E", 0x252E, "<B"),
    ("nav_actor_transition_phase_2792", 0x2792, "<B"),
    ("graphics_draw_framebuffer_5219", 0x5219, "<HH"),
    ("graphics_screen_buffer_521D", 0x521D, "<HH"),
    ("graphics_display_buffer_5221", 0x5221, "<HH"),
    ("graphics_back_buffer_5229", 0x5229, "<HH"),
    ("graphics_viewport_descriptor_522D", 0x522D, "<HH"),
    ("list_d8c_buffer_end_offset_5233", 0x5233, "<H"),
    ("vm_active_line_6788", 0x6788, "<H"),
    ("vm_displayed_line_678A", 0x678A, "<H"),
]
STARTUP_STRINGS = [
    ("startup_write_directory_01BA", 0x01BA, 32),
    ("startup_original_directory_01DA", 0x01DA, 32),
]
RUNTIME_RANGES = [
    ("live_palette_5251", 0x5251, 768),
    ("palette_transition_target_5551", 0x5551, 768),
    ("palette_transition_source_5851", 0x5851, 768),
    ("palette_control_5B51", 0x5B51, 8),
    ("bridge_panorama_palette_5B58", 0x5B58, 768),
]


def locate_cpu_state(pid):
    executable = os.path.realpath(f"/proc/{pid}/exe")
    symbols = {}
    output = subprocess.check_output(
        ["nm", "-P", executable], text=True, stderr=subprocess.DEVNULL
    )
    for line in output.splitlines():
        fields = line.split()
        if len(fields) >= 3 and fields[0] in ("Segs", "cpu_regs"):
            symbols[fields[0]] = int(fields[2], 16)
    if set(symbols) != {"Segs", "cpu_regs"}:
        return None

    image_base = None
    with open(f"/proc/{pid}/maps", encoding="ascii") as maps:
        for line in maps:
            fields = line.split()
            if len(fields) < 6:
                continue
            mapped_path = fields[-1].removesuffix(" (deleted)")
            if os.path.realpath(mapped_path) != executable:
                continue
            start = int(fields[0].split("-", 1)[0], 16)
            offset = int(fields[2], 16)
            image_base = start - offset
            break
    if image_base is None:
        return None
    return {
        name: image_base + offset for name, offset in symbols.items()
    }


def read_cpu_state(mem, addresses):
    if addresses is None:
        return None
    mem.seek(addresses["cpu_regs"])
    registers = struct.unpack("<8I", mem.read(32))
    ip = struct.unpack("<I", mem.read(4))[0]
    segments = []
    for index in range(6):
        mem.seek(addresses["Segs"] + index * 8)
        segments.append(struct.unpack("<Q", mem.read(8))[0] & 0xffff)
    es, cs, ss, ds, fs, gs = segments
    return (es, cs, ss, ds, fs, gs, ip) + registers


def main():
    # <cd-dir> is the CD image dir that CONTAINS BLOODPRG.EXE (e.g. output/_tmp_iso).
    # The installed data dir (C:\cblood, e.g. accuracy/cblood_install/cblood) is a
    # SEPARATE tree: the shipped BLOOD.BAT does `D:` then
    # `BLOODPRG AMR S162227 EMS WRIC:\cblood\`, so the EXE lives on the CD and the
    # write path points at the hard-disk install. Mounting only one of them (or
    # launching BLOODPRG with no args) leaves the game looping the ATTRACT DEMO,
    # which never reaches navigation — and the 0x4F09 records then stay at their
    # baked default (10200,12100,900), which is what made this dump look inert.
    cd_dir = os.path.realpath(sys.argv[1])
    wait = float(sys.argv[2]) if len(sys.argv) > 2 else 40.0
    # Optional 3rd arg: the PARENT of the `cblood` install dir, mounted as C:.
    install_parent = os.path.realpath(sys.argv[3]) if len(sys.argv) > 3 else None
    executable = sys.argv[4] if len(sys.argv) > 4 else "BLOODPRG.EXE"
    cycles = sys.argv[5] if len(sys.argv) > 5 else "max"
    emulator = os.environ.get("BLOODPRG_DOSBOX_BINARY", "dosbox-x")
    cpu_core = os.environ.get("BLOODPRG_DOSBOX_CORE", "normal")
    frame_skip = os.environ.get("BLOODPRG_DOSBOX_FRAMESKIP", "10")
    dump_dir = os.environ.get("BLOODPRG_DUMP_DIR")
    if dump_dir:
        Path(dump_dir).mkdir(parents=True, exist_ok=True)
    if install_parent is None:
        guess = os.path.join(os.path.dirname(os.path.dirname(os.path.dirname(
            os.path.realpath(__file__)))), "accuracy", "cblood_install")
        if os.path.isdir(os.path.join(guess, "cblood")):
            install_parent = guess
    game = cd_dir
    libc = ctypes.CDLL("libc.so.6", use_errno=True)
    libc.ptrace.restype = ctypes.c_long
    libc.ptrace.argtypes = [ctypes.c_long, ctypes.c_long, ctypes.c_void_p, ctypes.c_void_p]
    PTRACE_ATTACH, PTRACE_DETACH = 16, 17
    env = dict(os.environ); env["DISPLAY"] = env.get("DISPLAY", ":53"); env["SDL_VIDEODRIVER"] = "x11"
    xvfb = subprocess.Popen(["Xvfb", env["DISPLAY"], "-screen", "0", "800x600x24"],
                            stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    time.sleep(3)
    # Reproduce BLOOD.BAT: C: = the install parent (so C:\cblood exists as the
    # write path), D: = the CD dir holding BLOODPRG.EXE, then run from D: with the
    # shipped argument list. Without `AMR S162227 EMS WRIC:\cblood\` the game loops
    # the attract demo instead of entering the playable state.
    cmds = []
    if install_parent:
        cmds += ["-c", f"mount c {install_parent}"]
    cmds += ["-c", f"mount d {cd_dir} -t cdrom", "-c", "d:",
             "-c", executable + r" AMR S162227 EMS WRIC:\cblood" + "\\"]
    if "staging" in os.path.basename(emulator):
        dosbox_args = [
            emulator,
            "--noprimaryconf",
            "--nolocalconf",
            "--set",
            "output=surface",
            "--set",
            f"cycles={cycles}",
            "--set",
            f"core={cpu_core}",
            "--set",
            f"frameskip={frame_skip}",
        ]
    else:
        dosbox_args = [
            emulator,
            "-set",
            "sdl output=surface",
            "-set",
            f"cpu cycles={cycles}",
            "-set",
            f"cpu core={cpu_core}",
            "-set",
            f"render frameskip={frame_skip}",
        ]
    db = subprocess.Popen(dosbox_args + cmds,
                          stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, env=env)
    time.sleep(wait)
    if dump_dir:
        subprocess.run(
            [
                "import", "-window", "root",
                str(Path(dump_dir) / f"{executable}.screen.png"),
            ],
            env=env,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
            check=False,
        )
    pid = db.pid
    attached = False
    try:
        if db.poll() is not None:
            print(f"DOSBox-X exited before capture with status {db.returncode}")
            return
        if libc.ptrace(PTRACE_ATTACH, pid, None, None) != 0:
            print("ptrace attach failed errno", ctypes.get_errno()); return
        attached = True
        os.waitpid(pid, 0)
        try:
            mem = open(f"/proc/{pid}/mem", "rb")
        except FileNotFoundError:
            print("DOSBox-X exited before its memory could be opened")
            return
        cpu_state_addresses = locate_cpu_state(pid)
        cpu_state = read_cpu_state(mem, cpu_state_addresses)
        if cpu_state is not None:
            es, cs, ss, ds, fs, gs, ip, *registers = cpu_state
            print(
                "guest_cpu: "
                f"cs:ip={cs:04x}:{ip & 0xffff:04x} "
                f"ds={ds:04x} es={es:04x} ss={ss:04x} "
                f"fs={fs:04x} gs={gs:04x} "
                f"ax={registers[0] & 0xffff:04x} "
                f"cx={registers[1] & 0xffff:04x} "
                f"si={registers[6] & 0xffff:04x} "
                f"di={registers[7] & 0xffff:04x}"
            )
        best = None
        guest_memory_base = None
        candidates = []
        for line in open(f"/proc/{pid}/maps"):
            pr = line.split()
            if 'r' not in pr[1] or '-' not in pr[0]:
                continue
            a, b = [int(x, 16) for x in pr[0].split('-')]
            if b - a > 300_000_000:
                continue
            try:
                mem.seek(a); buf = mem.read(b - a)
            except Exception:
                continue
            for m in re.finditer(re.escape(LOCATOR_ANCHOR), buf):
                A = a + m.start()
                mem.seek(A + 0x0A46)
                free_bytes = struct.unpack("<I", mem.read(4))[0]
                mem.seek(A + 0x0A9E)
                crtc_port = struct.unpack("<H", mem.read(2))[0]
                candidates.append((A, free_bytes, crtc_port))
                if 0 < free_bytes <= 0x000A0000 and crtc_port in (0, 0x03D4):
                    best = A
                    guest_memory_base = a
                    break
            if best:
                break
        if not best and len(candidates) == 1 and candidates[0][2] in (0, 0x03D4):
            best = candidates[0][0]
            print("using unique DS anchor despite invalid arena state")
        if not best:
            print(f"DS anchor not found; candidates={candidates}")
            return
        if guest_memory_base is not None:
            delta = best - guest_memory_base
            print(
                f"guest_memory_candidate: base=0x{guest_memory_base:x} "
                f"ds_delta=0x{delta:x} segment=0x{delta // 16:04x}"
            )
        for name, off, n in GLOBALS:
            mem.seek(best - (DS_ANCHOR - off))
            vals = struct.unpack(f'<{n}h', mem.read(n * 2))
            print(f"{name}: {vals if n > 1 else vals[0]}")
        for name, off, fmt in STARTUP_GLOBALS:
            mem.seek(best - (DS_ANCHOR - off))
            vals = struct.unpack(fmt, mem.read(struct.calcsize(fmt)))
            print(f"{name}: {vals if len(vals) > 1 else vals[0]}")
        for name, off, capacity in STARTUP_STRINGS:
            mem.seek(best - (DS_ANCHOR - off))
            value = mem.read(capacity).split(b"\0", 1)[0]
            print(f"{name}: {value.decode('ascii', errors='replace')!r}")
        for name, off, size in RUNTIME_RANGES:
            mem.seek(best + off)
            data = mem.read(size)
            if dump_dir:
                output = Path(dump_dir)
                output.mkdir(parents=True, exist_ok=True)
                (output / f"{executable}.{name}.bin").write_bytes(data)
            print(
                f"{name}: sha256={hashlib.sha256(data).hexdigest()} "
                f"sample={data[:32].hex()} tail={data[-32:].hex()}"
            )
        if dump_dir:
            output = Path(dump_dir)
            mem.seek(best)
            (output / f"{executable}.game_data_segment.bin").write_bytes(
                mem.read(0x10000)
            )
        if guest_memory_base is not None:
            guest_linear_bias = 0
            descriptor_signature = bytes.fromhex(
                "00000100040000004001c80000000000"
            )
            mem.seek(best + 0x522D)
            descriptor_offset, descriptor_segment = struct.unpack(
                "<HH", mem.read(4)
            )
            descriptor_address = (
                guest_memory_base
                + descriptor_segment * 16
                + descriptor_offset
            )
            for candidate_bias in range(-64, 65):
                mem.seek(descriptor_address + candidate_bias)
                if mem.read(len(descriptor_signature)) == descriptor_signature:
                    guest_linear_bias = candidate_bias
                    break
            print(f"guest_linear_bias: {guest_linear_bias:+d}")
            for index in range(5):
                mem.seek(best + 0x671C + index * 4)
                offset, segment = struct.unpack("<HH", mem.read(4))
                resource_address = (
                    guest_memory_base
                    + guest_linear_bias
                    + segment * 16
                    + offset
                )
                mem.seek(resource_address)
                data = mem.read(64)
                if dump_dir:
                    output = Path(dump_dir)
                    (output / (
                        f"{executable}.vm_resource_image_{index}.head.bin"
                    )).write_bytes(data)
                    mem.seek(resource_address)
                    (output / (
                        f"{executable}.vm_resource_image_{index}.segment.bin"
                    )).write_bytes(mem.read(0x10000))
                print(
                    f"vm_resource_image_{index}: pointer={segment:04x}:{offset:04x} "
                    f"head={data.hex()}"
                )
            mem.seek(best + 0x0CD3)
            driver_offset, driver_segment = struct.unpack("<HH", mem.read(4))
            mem.seek(
                guest_memory_base
                + guest_linear_bias
                + driver_segment * 16
            )
            driver_data = mem.read(0x10000)
            if dump_dir:
                (output / f"{executable}.sound_driver_segment.bin").write_bytes(
                    driver_data
                )
            print(
                f"sound_driver: pointer={driver_segment:04x}:{driver_offset:04x} "
                f"sha256={hashlib.sha256(driver_data).hexdigest()} "
                f"head={driver_data[:64].hex()}"
            )
            for name, pointer_offset in (
                ("graphics_display_buffer", 0x5221),
                ("graphics_back_buffer", 0x5229),
                ("graphics_work_surface", 0x0ABC),
            ):
                mem.seek(best + pointer_offset)
                offset, segment = struct.unpack("<HH", mem.read(4))
                mem.seek(
                    guest_memory_base
                    + guest_linear_bias
                    + segment * 16
                    + offset
                )
                data = mem.read(0xFA00)
                checksum = sum(data) & 0xFFFFFFFF
                if dump_dir:
                    output = Path(dump_dir)
                    output.mkdir(parents=True, exist_ok=True)
                    (output / f"{executable}.{name}.bin").write_bytes(data)
                print(
                    f"{name}_pixels: pointer={segment:04x}:{offset:04x} "
                    f"sum32=0x{checksum:08x} sample={data[:32].hex()} "
                    f"tail={data[-32:].hex()}"
                )
            if dump_dir:
                mem.seek(best + 0x0D8C)
                queue_offset, queue_segment = struct.unpack("<HH", mem.read(4))
                mem.seek(
                    guest_memory_base
                    + guest_linear_bias
                    + queue_segment * 16
                )
                (output / f"{executable}.list_d8c_segment.bin").write_bytes(
                    mem.read(0x10000)
                )
        trace_seconds = float(os.environ.get("BLOODPRG_TRACE_SECONDS", "0"))
        if trace_seconds > 0:
            trace_interval = float(
                os.environ.get("BLOODPRG_TRACE_INTERVAL", "0.05")
            )
            trace_heartbeat = float(
                os.environ.get("BLOODPRG_TRACE_HEARTBEAT", "1.0")
            )
            mem.close()
            libc.ptrace(PTRACE_DETACH, pid, None, None)
            attached = False
            deadline = time.monotonic() + trace_seconds
            print(
                "trace: event,elapsed,tick,delay,mode,vm_enabled,vm_ui,"
                "active_line,timer_base,phase,cursor_before,cursor_after,"
                "profile,request,query,skip,resume,yield,branch_top,"
                "presentation_flags,presentation_active,start_lock,owner,"
                "primary_c4,wildcard,primary_kind,primary_related,"
                "primary_value,mode_a,mode_b,box_phase,text_active"
                ",resource_requested,resource_active,source_offset,source_remaining"
                ",list_state,read_wrap,wrap_count,wrap_limit,secondary_wrap_limit"
                ",head_offset,tail_offset,byte_count,iteration_count,entry_metric"
                ",active_layout,active_row_mode,decode_mode,decode_rectangular"
                ",draw_via_back_buffer,skip_back_buffer_present"
                ",display_offset,display_segment,secondary_offset,secondary_segment"
                ",guest_cs,guest_ip,guest_ds,guest_ss"
                ",guest_ax,guest_cx,guest_si,guest_di"
            )
            previous_timer_base = None
            previous_phase = None
            previous_vm_enabled = None
            previous_active_line = None
            previous_vm_ui = None
            previous_presentation_active = None
            previous_mode_b = None
            previous_box_phase = None
            previous_resource_requested = None
            previous_resource_active = None
            previous_list_state = None
            previous_entry_metric = None
            previous_active_layout = None
            previous_active_row_mode = None
            previous_decode_mode = None
            anchor_corrupted = False
            next_heartbeat = time.monotonic()
            while time.monotonic() < deadline and db.poll() is None:
                time.sleep(trace_interval)
                if libc.ptrace(PTRACE_ATTACH, pid, None, None) != 0:
                    print(f"trace: attach_failed errno={ctypes.get_errno()}")
                    break
                os.waitpid(pid, 0)
                try:
                    with open(f"/proc/{pid}/mem", "rb") as trace_mem:
                        trace_mem.seek(best)
                        anchor_bytes = trace_mem.read(len(ANCHOR))
                        if anchor_bytes != ANCHOR and not anchor_corrupted:
                            trace_mem.seek(best)
                            replacement = trace_mem.read(128)
                            print(
                                "trace: anchor_corrupted "
                                f"process_status={db.poll()} "
                                f"replacement={replacement.hex()}"
                            )
                            anchor_corrupted = True
                        def read_trace(offset, fmt):
                            trace_mem.seek(best + offset)
                            return struct.unpack(
                                fmt, trace_mem.read(struct.calcsize(fmt))
                            )[0]
                        def read_record_triple(record_offset):
                            base_offset = read_trace(0x6724, "<H")
                            base_segment = read_trace(0x6726, "<H")
                            trace_mem.seek(
                                guest_memory_base
                                + guest_linear_bias
                                + base_segment * 16
                                + base_offset
                                + record_offset
                            )
                            return struct.unpack("<HHH", trace_mem.read(6))
                        timer_base = read_trace(0x0AF0, "<H")
                        vm_enabled = read_trace(0x67A8, "<B")
                        vm_ui = read_trace(0x2793, "<H")
                        active_line = read_trace(0x6788, "<H")
                        presentation_active = read_trace(0x67AC, "<B")
                        mode_b = read_trace(0x27E1, "<B")
                        box_phase = read_trace(0x2B93, "<h")
                        resource_requested = read_trace(0x0D80, "<H")
                        resource_active = read_trace(0x0D82, "<H")
                        list_state = read_trace(0x0D5F, "<B")
                        entry_metric = read_trace(0x0DAF, "<H")
                        active_layout = read_trace(0x0DA4, "<H")
                        active_row_mode = read_trace(0x0DA6, "<H")
                        decode_mode = read_trace(0x0AA0, "<H")
                        phase = read_trace(0x6730, "<H")
                        primary_c4 = read_trace(0x675E, "<H")
                        primary_kind, primary_related, primary_value = (
                            read_record_triple(primary_c4)
                        )
                        now = time.monotonic()
                        event = None
                        if previous_timer_base is None:
                            event = "initial"
                        elif timer_base != previous_timer_base:
                            event = "timer_base_changed"
                        elif phase != previous_phase:
                            event = "profile_cursor_changed"
                        elif vm_enabled != previous_vm_enabled:
                            event = "vm_enabled_changed"
                        elif active_line != previous_active_line:
                            event = "active_line_changed"
                        elif vm_ui != previous_vm_ui:
                            event = "vm_ui_changed"
                        elif presentation_active != previous_presentation_active:
                            event = "presentation_active_changed"
                        elif mode_b != previous_mode_b:
                            event = "presentation_mode_changed"
                        elif box_phase != previous_box_phase:
                            event = "box_phase_changed"
                        elif resource_requested != previous_resource_requested:
                            event = "resource_requested_changed"
                        elif resource_active != previous_resource_active:
                            event = "resource_active_changed"
                        elif list_state != previous_list_state:
                            event = "list_state_changed"
                        elif entry_metric != previous_entry_metric:
                            event = "entry_metric_changed"
                        elif active_layout != previous_active_layout:
                            event = "active_layout_changed"
                        elif active_row_mode != previous_active_row_mode:
                            event = "active_row_mode_changed"
                        elif decode_mode != previous_decode_mode:
                            event = "decode_mode_changed"
                        elif trace_heartbeat > 0 and now >= next_heartbeat:
                            event = "heartbeat"
                        if event is not None:
                            next_heartbeat = now + trace_heartbeat
                            cpu_state = read_cpu_state(
                                trace_mem, cpu_state_addresses
                            )
                            if cpu_state is None:
                                guest_es = guest_cs = guest_ss = guest_ds = 0
                                guest_fs = guest_gs = guest_ip = 0
                                guest_registers = (0,) * 8
                            else:
                                (guest_es, guest_cs, guest_ss, guest_ds,
                                 guest_fs, guest_gs, guest_ip,
                                 *guest_registers) = cpu_state
                            print(
                                "trace: "
                                f"{event},"
                                f"{trace_seconds - (deadline - time.monotonic()):.3f},"
                                f"{read_trace(0x0B29, '<H')},"
                                f"{read_trace(0x0B2D, '<H')},"
                                f"{read_trace(0x0ADF, '<B')},"
                                f"{vm_enabled},"
                                f"{vm_ui},"
                                f"{active_line},"
                                f"{timer_base},"
                                f"{phase},"
                                f"{read_trace(0x6732, '<H')},"
                                f"{read_trace(0x6734, '<H')},"
                                f"{read_trace(0x677E, '<H')},"
                                f"{read_trace(0x6780, '<h')},"
                                f"{read_trace(0x67AD, '<B')},"
                                f"{read_trace(0x67AB, '<B')},"
                                f"{read_trace(0x67B1, '<B')},"
                                f"{read_trace(0x67B4, '<B')},"
                                f"{read_trace(0x6884, '<H')},"
                                f"{read_trace(0x67AA, '<B')},"
                                f"{presentation_active},"
                                f"{read_trace(0x67B7, '<B')},"
                                f"{read_trace(0x679A, '<H')},"
                                f"{primary_c4},"
                                f"{read_trace(0x674E, '<H')},"
                                f"{primary_kind},"
                                f"{primary_related},"
                                f"{primary_value},"
                                f"{read_trace(0x27E0, '<B')},"
                                f"{mode_b},"
                                f"{box_phase},"
                                f"{read_trace(0x5E64, '<B')},"
                                f"{resource_requested},"
                                f"{resource_active},"
                                f"{read_trace(0x0D84, '<I')},"
                                f"{read_trace(0x0D88, '<I')},"
                                f"{list_state},"
                                f"{read_trace(0x0D60, '<H')},"
                                f"{read_trace(0x0D62, '<H')},"
                                f"{read_trace(0x0D64, '<H')},"
                                f"{read_trace(0x0D66, '<H')},"
                                f"{read_trace(0x0D8C, '<H')},"
                                f"{read_trace(0x0D90, '<H')},"
                                f"{read_trace(0x0D9A, '<H')},"
                                f"{read_trace(0x0DA0, '<H')},"
                                f"{entry_metric},"
                                f"{active_layout},"
                                f"{active_row_mode},"
                                f"{decode_mode},"
                                f"{read_trace(0x0DBA, '<B')},"
                                f"{read_trace(0x0DB9, '<B')},"
                                f"{read_trace(0x0DBB, '<B')},"
                                f"{read_trace(0x5221, '<H')},"
                                f"{read_trace(0x5223, '<H')},"
                                f"{read_trace(0x5229, '<H')},"
                                f"{read_trace(0x522B, '<H')},"
                                f"{guest_cs:04x},"
                                f"{guest_ip & 0xffff:04x},"
                                f"{guest_ds:04x},"
                                f"{guest_ss:04x},"
                                f"{guest_registers[0] & 0xffff:04x},"
                                f"{guest_registers[1] & 0xffff:04x},"
                                f"{guest_registers[6] & 0xffff:04x},"
                                f"{guest_registers[7] & 0xffff:04x}"
                            )
                        previous_timer_base = timer_base
                        previous_phase = phase
                        previous_vm_enabled = vm_enabled
                        previous_active_line = active_line
                        previous_vm_ui = vm_ui
                        previous_presentation_active = presentation_active
                        previous_mode_b = mode_b
                        previous_box_phase = box_phase
                        previous_resource_requested = resource_requested
                        previous_resource_active = resource_active
                        previous_list_state = list_state
                        previous_entry_metric = entry_metric
                        previous_active_layout = active_layout
                        previous_active_row_mode = active_row_mode
                        previous_decode_mode = decode_mode
                except FileNotFoundError:
                    print(
                        "trace: emulator_exited "
                        f"status={db.poll()}"
                    )
                    break
                finally:
                    libc.ptrace(PTRACE_DETACH, pid, None, None)
    finally:
        if attached:
            libc.ptrace(PTRACE_DETACH, pid, None, None)
        if db.poll() is None:
            db.kill()
        xvfb.kill()


if __name__ == "__main__":
    main()
