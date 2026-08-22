#!/usr/bin/env python3
"""Guard rebuilt BLOODPRG invariants in a live DOSBox-X guest.

The watchdog derives DOS address zero from GAME_DATA:0000 and the live GS
value. It then verifies the final-link segment layout, the interrupt vector
table, and the conventional DOS memory-control-block chain while the game is
driven. A report is successful only after calibration and at least one guarded
sample.
"""
from __future__ import annotations

import argparse
import ctypes
import hashlib
import importlib.util
import json
import os
import re
import struct
import subprocess
import threading
import time
from dataclasses import dataclass
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
GAME_DATA_ANCHOR = b"386 minimum !\0Not enough memory (570Ko min) !\0"
CONVENTIONAL_MEMORY_END = 0xA0000
GUEST_SNAPSHOT_SIZE = 0x100000
PTRACE_ATTACH = 16
PTRACE_DETACH = 17

SEGMENT_ROW = re.compile(
    r"^(?P<name>GAME_DATA|FS_DATA)\s+\S+\s+\S+\s+"
    r"(?P<segment>[0-9A-Fa-f]{4}):(?P<offset>[0-9A-Fa-f]{4})\s+"
    r"(?P<size>[0-9A-Fa-f]{8})$"
)


class WatchdogError(RuntimeError):
    pass


class McbError(WatchdogError):
    pass


@dataclass(frozen=True)
class SegmentLayout:
    game_data: int
    fs_data: int


@dataclass(frozen=True)
class HostMapping:
    start: int
    end: int
    readable: bool


@dataclass(frozen=True)
class Mcb:
    segment: int
    kind: str
    owner: int
    paragraphs: int
    name: str

    @property
    def data_start(self) -> int:
        return self.segment + 1

    @property
    def data_end(self) -> int:
        return self.data_start + self.paragraphs

    def owns_segment(self, segment: int) -> bool:
        return self.data_start <= segment < self.data_end


def parse_segment_layout(path: Path) -> SegmentLayout:
    placements: dict[str, tuple[int, int]] = {}
    for line in path.read_text(encoding="ascii", errors="replace").splitlines():
        match = SEGMENT_ROW.match(line.strip())
        if match:
            placements[match["name"]] = (
                int(match["segment"], 16),
                int(match["offset"], 16),
            )
    missing = {"GAME_DATA", "FS_DATA"} - placements.keys()
    if missing:
        raise WatchdogError(
            f"{path}: missing segment(s): {', '.join(sorted(missing))}"
        )
    for name, (_, offset) in placements.items():
        if offset != 0:
            raise WatchdogError(
                f"{path}: {name} begins at offset {offset:#06x}, not zero"
            )
    return SegmentLayout(
        game_data=placements["GAME_DATA"][0],
        fs_data=placements["FS_DATA"][0],
    )


def ptrace_libc():
    libc = ctypes.CDLL("libc.so.6", use_errno=True)
    libc.ptrace.restype = ctypes.c_long
    libc.ptrace.argtypes = [
        ctypes.c_long,
        ctypes.c_long,
        ctypes.c_void_p,
        ctypes.c_void_p,
    ]
    return libc


def locate_cpu_state(pid: int) -> dict[str, int] | None:
    executable = os.path.realpath(f"/proc/{pid}/exe")
    output = subprocess.run(
        ["nm", "-P", executable],
        text=True,
        capture_output=True,
        check=False,
    )
    if output.returncode != 0:
        return None
    symbols: dict[str, int] = {}
    for line in output.stdout.splitlines():
        fields = line.split()
        if len(fields) >= 3 and fields[0] in ("Segs", "cpu_regs"):
            symbols[fields[0]] = int(fields[2], 16)
    if set(symbols) != {"Segs", "cpu_regs"}:
        return None

    image_base = None
    with open(f"/proc/{pid}/maps", encoding="ascii") as stream:
        for line in stream:
            fields = line.split()
            if len(fields) < 6:
                continue
            mapped = fields[-1].removesuffix(" (deleted)")
            if os.path.realpath(mapped) != executable:
                continue
            start = int(fields[0].split("-", 1)[0], 16)
            image_base = start - int(fields[2], 16)
            break
    if image_base is None:
        return None
    return {name: image_base + offset for name, offset in symbols.items()}


def read_cpu_state(mem, addresses: dict[str, int]) -> dict[str, int]:
    mem.seek(addresses["cpu_regs"])
    registers = struct.unpack("<8I", mem.read(32))
    ip = struct.unpack("<I", mem.read(4))[0]
    segments = []
    for index in range(6):
        mem.seek(addresses["Segs"] + index * 8)
        segments.append(struct.unpack("<Q", mem.read(8))[0] & 0xFFFF)
    return {
        "es": segments[0],
        "cs": segments[1],
        "ss": segments[2],
        "ds": segments[3],
        "fs": segments[4],
        "gs": segments[5],
        "ip": ip & 0xFFFF,
        "ax": registers[0] & 0xFFFF,
        "bx": registers[3] & 0xFFFF,
    }


def host_mappings(pid: int) -> list[HostMapping]:
    mappings = []
    with open(f"/proc/{pid}/maps", encoding="ascii") as stream:
        for line in stream:
            fields = line.split()
            start_text, end_text = fields[0].split("-", 1)
            mappings.append(
                HostMapping(
                    int(start_text, 16),
                    int(end_text, 16),
                    "r" in fields[1],
                )
            )
    return mappings


def exact_read(mem, address: int, size: int) -> bytes:
    mem.seek(address)
    data = mem.read(size)
    if len(data) != size:
        raise WatchdogError(
            f"short host-memory read at {address:#x}: {len(data)} of {size}"
        )
    return data


def find_guest_base(pid: int, mem, game_segment: int) -> int | None:
    overlap = len(GAME_DATA_ANCHOR) - 1
    for mapping in host_mappings(pid):
        size = mapping.end - mapping.start
        if not mapping.readable or size < GUEST_SNAPSHOT_SIZE or size > 300_000_000:
            continue
        cursor = mapping.start
        tail = b""
        while cursor < mapping.end:
            chunk_size = min(2 * 1024 * 1024, mapping.end - cursor)
            try:
                chunk = exact_read(mem, cursor, chunk_size)
            except (OSError, WatchdogError):
                break
            haystack = tail + chunk
            search_from = 0
            while True:
                index = haystack.find(GAME_DATA_ANCHOR, search_from)
                if index < 0:
                    break
                anchor = cursor - len(tail) + index
                guest_base = anchor - game_segment * 16
                if (
                    mapping.start <= guest_base
                    and guest_base + GUEST_SNAPSHOT_SIZE <= mapping.end
                ):
                    snapshot = exact_read(mem, guest_base, GUEST_SNAPSHOT_SIZE)
                    if guest_memory_is_plausible(snapshot, game_segment):
                        return guest_base
                search_from = index + 1
            tail = haystack[-overlap:]
            cursor += chunk_size
    return None


def guest_memory_is_plausible(memory: bytes, game_segment: int) -> bool:
    anchor = game_segment * 16
    if memory[anchor : anchor + len(GAME_DATA_ANCHOR)] != GAME_DATA_ANCHOR:
        return False
    conventional_kib = struct.unpack_from("<H", memory, 0x0413)[0]
    if not 128 <= conventional_kib <= 640:
        return False
    int_21_offset, int_21_segment = struct.unpack_from("<HH", memory, 0x21 * 4)
    return (int_21_offset | int_21_segment) != 0


def parse_mcb_chain(
    memory: bytes,
    start_segment: int,
    required_segment: int,
) -> list[Mcb]:
    blocks = []
    segment = start_segment
    seen: set[int] = set()
    for _ in range(2048):
        if segment in seen:
            raise McbError(f"MCB cycle at {segment:#06x}")
        seen.add(segment)
        if not 0x0040 <= segment < 0xA000:
            raise McbError(f"MCB header outside conventional memory: {segment:#06x}")
        address = segment * 16
        if address + 16 > min(len(memory), CONVENTIONAL_MEMORY_END):
            raise McbError(f"truncated MCB header at {segment:#06x}")
        kind_byte = memory[address]
        if kind_byte not in (ord("M"), ord("Z")):
            raise McbError(
                f"invalid MCB type {kind_byte:#04x} at {segment:#06x}"
            )
        owner, paragraphs = struct.unpack_from("<HH", memory, address + 1)
        raw_name = memory[address + 8 : address + 16].rstrip(b"\0 ")
        name = "".join(
            chr(value) if 0x20 <= value < 0x7F else "." for value in raw_name
        )
        block = Mcb(segment, chr(kind_byte), owner, paragraphs, name)
        blocks.append(block)
        next_segment = block.data_end
        if not segment < next_segment <= 0xA000:
            raise McbError(
                f"MCB {segment:#06x} extends to invalid segment "
                f"{next_segment:#06x}"
            )
        if block.kind == "Z":
            if not any(entry.segment == required_segment for entry in blocks):
                raise McbError(
                    f"MCB chain omits required header {required_segment:#06x}"
                )
            return blocks
        segment = next_segment
    raise McbError("MCB chain exceeds 2048 blocks")


def discover_mcb_chain(memory: bytes, program_mcb: int, psp: int) -> list[Mcb]:
    address = program_mcb * 16
    if address + 5 > len(memory):
        raise McbError(f"program MCB {program_mcb:#06x} is outside guest memory")
    if memory[address] not in (ord("M"), ord("Z")):
        raise McbError(f"program MCB {program_mcb:#06x} has no M/Z signature")
    if struct.unpack_from("<H", memory, address + 1)[0] != psp:
        raise McbError(f"program MCB {program_mcb:#06x} is not owned by PSP {psp:#06x}")

    candidates = []
    for start in range(0x0040, program_mcb + 1):
        if memory[start * 16] != ord("M") and start != program_mcb:
            continue
        try:
            blocks = parse_mcb_chain(memory, start, program_mcb)
        except McbError:
            continue
        program = next(block for block in blocks if block.segment == program_mcb)
        if program.owner == psp:
            candidates.append(blocks)
    if not candidates:
        raise McbError(
            f"no complete MCB chain contains program header {program_mcb:#06x}"
        )
    return min(candidates, key=lambda blocks: blocks[0].segment)


def program_owned_block(
    blocks: list[Mcb], segment: int, psp: int
) -> Mcb | None:
    for block in blocks:
        if block.owner == psp and block.owns_segment(segment):
            return block
    return None


def game_is_ready(memory: bytes, game_segment: int) -> bool:
    base = game_segment * 16
    free_bytes = struct.unpack_from("<I", memory, base + 0x0A46)[0]
    crtc_port = struct.unpack_from("<H", memory, base + 0x0A9E)[0]
    timer_hook_active = memory[base + 0x0B21]
    return 0 < free_bytes <= 0x000A0000 and crtc_port == 0x03D4 and timer_hook_active == 1


def cpu_for_report(state: dict[str, int]) -> dict[str, str]:
    return {name: f"{value:#06x}" for name, value in state.items()}


def mcb_for_report(block: Mcb) -> dict[str, str | int]:
    return {
        "segment": f"{block.segment:#06x}",
        "kind": block.kind,
        "owner": f"{block.owner:#06x}",
        "paragraphs": block.paragraphs,
        "name": block.name,
    }


def changed_interrupt_vectors(before: bytes, after: bytes) -> list[dict[str, str]]:
    changes = []
    for vector in range(256):
        offset = vector * 4
        if before[offset : offset + 4] == after[offset : offset + 4]:
            continue
        before_offset, before_segment = struct.unpack_from("<HH", before, offset)
        after_offset, after_segment = struct.unpack_from("<HH", after, offset)
        changes.append(
            {
                "vector": f"{vector:#04x}",
                "before": f"{before_segment:04x}:{before_offset:04x}",
                "after": f"{after_segment:04x}:{after_offset:04x}",
            }
        )
    return changes


def load_action_driver():
    path = ROOT / "re" / "tools" / "capture_pterra_boundary.py"
    spec = importlib.util.spec_from_file_location("dosbox_action_driver", path)
    if spec is None or spec.loader is None:
        raise WatchdogError(f"cannot load action driver from {path}")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module.run_driver


def default_link_map(cd_dir: Path) -> Path:
    return cd_dir.parent / "validation/bloodprg_runtime/final/link.map"


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--cd-dir", type=Path, required=True)
    parser.add_argument("--install-parent", type=Path, required=True)
    parser.add_argument("--executable", default="BPRG_RE.EXE")
    parser.add_argument("--link-map", type=Path)
    parser.add_argument("--dosbox", default="dosbox-x")
    parser.add_argument("--display", default=":83")
    parser.add_argument("--seconds", type=float, default=600.0)
    parser.add_argument("--calibration-timeout", type=float, default=30.0)
    parser.add_argument("--stable-samples", type=int, default=3)
    parser.add_argument("--poll-seconds", type=float, default=0.25)
    parser.add_argument(
        "--driver-delay",
        type=float,
        default=0.0,
        help="seconds to wait before the action driver begins window discovery",
    )
    parser.add_argument(
        "--actions",
        type=Path,
        help="input script in drive_real_game.sh vocabulary",
    )
    parser.add_argument("--report", type=Path)
    parser.add_argument("--xvfb", action="store_true")
    args = parser.parse_args()

    cd_dir = args.cd_dir.resolve()
    install_parent = args.install_parent.resolve()
    link_map = (args.link_map or default_link_map(cd_dir)).resolve()
    layout = parse_segment_layout(link_map)
    if args.stable_samples < 1:
        raise WatchdogError("--stable-samples must be positive")

    report: dict[str, object] = {
        "verdict": "INCOMPLETE",
        "samples": 0,
        "guarded_samples": 0,
        "anomalies": [],
    }
    env = dict(os.environ, DISPLAY=args.display, SDL_VIDEODRIVER="x11")
    xvfb = None
    dosbox = None
    attached = False
    driver_errors: list[str] = []
    libc = ptrace_libc()

    try:
        if args.xvfb:
            xvfb = subprocess.Popen(
                ["Xvfb", args.display, "-screen", "0", "800x600x24"],
                stdout=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL,
            )
            time.sleep(2.0)

        dosbox_args = [
            args.dosbox,
            "--noprimaryconf",
            "--nolocalconf",
            "-set",
            "sdl output=surface",
            "-set",
            "cpu cycles=max",
            "-set",
            "cpu core=dynamic",
            "-set",
            "render frameskip=10",
            "-c",
            f"mount c {install_parent}",
            "-c",
            f"mount d {cd_dir} -t cdrom",
            "-c",
            "d:",
            "-c",
            f"{args.executable} AMR S162227 EMS WRIC:\\cblood\\",
            "-c",
            "exit",
        ]
        dosbox = subprocess.Popen(
            dosbox_args,
            env=env,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        )

        if args.actions:
            actions = args.actions.read_text(encoding="utf-8").splitlines()
            run_driver = load_action_driver()

            def drive() -> None:
                if args.driver_delay > 0:
                    time.sleep(args.driver_delay)
                try:
                    run_driver(actions, args.display, args.executable)
                except Exception as error:  # propagated through driver_errors
                    driver_errors.append(f"{type(error).__name__}: {error}")

            threading.Thread(target=drive, daemon=True).start()

        started = time.monotonic()
        deadline = started + args.seconds
        calibration_deadline = started + args.calibration_timeout
        cpu_addresses = None
        guest_base = None
        calibration_key = None
        stable_samples = 0
        expected = None
        last_context = None

        while time.monotonic() < deadline:
            time.sleep(args.poll_seconds)
            if driver_errors:
                report["verdict"] = "DRIVER-ERROR"
                report["error"] = driver_errors[0]
                break
            if dosbox.poll() is not None:
                report["exit_code"] = dosbox.returncode
                report["verdict"] = (
                    "CLEAN-EXIT" if expected is not None and dosbox.returncode == 0
                    else "EXIT-BEFORE-CALIBRATION"
                )
                break
            if expected is None and time.monotonic() >= calibration_deadline:
                report["verdict"] = "CALIBRATION-TIMEOUT"
                break

            ctypes.set_errno(0)
            if libc.ptrace(PTRACE_ATTACH, dosbox.pid, None, None) != 0:
                continue
            os.waitpid(dosbox.pid, 0)
            attached = True
            try:
                with open(f"/proc/{dosbox.pid}/mem", "rb") as mem:
                    if cpu_addresses is None:
                        cpu_addresses = locate_cpu_state(dosbox.pid)
                    if cpu_addresses is None:
                        continue
                    state = read_cpu_state(mem, cpu_addresses)
                    report["samples"] = int(report["samples"]) + 1

                    if expected is None:
                        if state["gs"] < layout.game_data:
                            continue
                        load_segment = state["gs"] - layout.game_data
                        expected_fs = load_segment + layout.fs_data
                        if expected_fs > 0xFFFF or state["fs"] != expected_fs:
                            continue
                        psp = load_segment - 0x10
                        if psp < 0x0050:
                            continue
                        if guest_base is None:
                            guest_base = find_guest_base(
                                dosbox.pid, mem, state["gs"]
                            )
                        if guest_base is None:
                            continue
                        memory = exact_read(mem, guest_base, GUEST_SNAPSHOT_SIZE)
                        if not game_is_ready(memory, state["gs"]):
                            continue
                        blocks = discover_mcb_chain(memory, psp - 1, psp)
                        ivt_hash = hashlib.sha256(memory[:0x400]).hexdigest()
                        key = (
                            guest_base,
                            state["gs"],
                            expected_fs,
                            psp,
                            blocks[0].segment,
                            ivt_hash,
                        )
                        if key == calibration_key:
                            stable_samples += 1
                        else:
                            calibration_key = key
                            stable_samples = 1
                        if stable_samples < args.stable_samples:
                            continue
                        expected = {
                            "guest_base": guest_base,
                            "load_segment": load_segment,
                            "game_segment": state["gs"],
                            "fs_segment": expected_fs,
                            "psp": psp,
                            "program_mcb": psp - 1,
                            "mcb_start": blocks[0].segment,
                            "ivt_sha256": ivt_hash,
                            "ivt_bytes": memory[:0x400],
                        }
                        report["calibrated"] = {
                            "load_segment": f"{load_segment:#06x}",
                            "game_segment": f"{state['gs']:#06x}",
                            "fs_segment": f"{expected_fs:#06x}",
                            "psp": f"{psp:#06x}",
                            "mcb_start": f"{blocks[0].segment:#06x}",
                            "mcb_count": len(blocks),
                            "mcb_chain": [mcb_for_report(block) for block in blocks],
                            "ivt_sha256": ivt_hash,
                            "program_mcb": mcb_for_report(
                                next(
                                    block
                                    for block in blocks
                                    if block.segment == psp - 1
                                )
                            ),
                        }
                        continue

                    memory = exact_read(
                        mem, int(expected["guest_base"]), GUEST_SNAPSHOT_SIZE
                    )
                    issues = []
                    diagnostics: dict[str, object] = {}
                    try:
                        blocks = parse_mcb_chain(
                            memory,
                            int(expected["mcb_start"]),
                            int(expected["program_mcb"]),
                        )
                        program = next(
                            block
                            for block in blocks
                            if block.segment == int(expected["program_mcb"])
                        )
                        if program.owner != int(expected["psp"]):
                            issues.append(
                                f"program-mcb-owner={program.owner:#06x} expected "
                                f"{int(expected['psp']):#06x}"
                            )
                    except McbError as error:
                        blocks = []
                        issues.append(f"mcb-chain: {error}")

                    if state["gs"] != int(expected["game_segment"]):
                        issues.append(
                            f"gs={state['gs']:#06x} expected "
                            f"{int(expected['game_segment']):#06x}"
                        )
                    fs_policy = "startup-table"
                    if state["fs"] != int(expected["fs_segment"]):
                        owner = program_owned_block(
                            blocks, state["fs"], int(expected["psp"])
                        )
                        if owner is None:
                            issues.append(
                                f"fs={state['fs']:#06x} is neither the startup "
                                "table nor game-owned overlay memory"
                            )
                            fs_policy = "invalid"
                        else:
                            fs_policy = f"game-owned-mcb-{owner.segment:#06x}"

                    ivt_hash = hashlib.sha256(memory[:0x400]).hexdigest()
                    if ivt_hash != expected["ivt_sha256"]:
                        changes = changed_interrupt_vectors(
                            expected["ivt_bytes"], memory[:0x400]
                        )
                        diagnostics["ivt_changes"] = changes
                        issues.append(
                            "ivt-vectors-changed="
                            + ",".join(change["vector"] for change in changes)
                        )
                    if not guest_memory_is_plausible(
                        memory, int(expected["game_segment"])
                    ):
                        issues.append("guest-memory-anchor-invalid")

                    report["guarded_samples"] = int(report["guarded_samples"]) + 1
                    context = (
                        state["cs"],
                        state["ds"],
                        state["fs"],
                        state["gs"],
                        fs_policy,
                    )
                    if context != last_context:
                        last_context = context
                        transitions = report.setdefault("contexts", [])
                        if isinstance(transitions, list) and len(transitions) < 100:
                            transitions.append(
                                {
                                    "sample": report["guarded_samples"],
                                    "cpu": cpu_for_report(state),
                                    "fs_policy": fs_policy,
                                }
                            )
                    if issues:
                        report["verdict"] = "ANOMALY"
                        anomalies = report["anomalies"]
                        assert isinstance(anomalies, list)
                        anomaly = {
                            "sample": report["guarded_samples"],
                            "cpu": cpu_for_report(state),
                            "issues": issues,
                        }
                        anomaly.update(diagnostics)
                        anomalies.append(anomaly)
                        break
            finally:
                if attached:
                    libc.ptrace(PTRACE_DETACH, dosbox.pid, None, None)
                    attached = False
        else:
            report["verdict"] = (
                "TIMEOUT-NO-ANOMALY"
                if expected is not None and int(report["guarded_samples"]) > 0
                else "CALIBRATION-TIMEOUT"
            )
    except Exception as error:
        report["verdict"] = "WATCHDOG-ERROR"
        report["error"] = f"{type(error).__name__}: {error}"
    finally:
        if dosbox is not None and dosbox.poll() is None:
            if attached:
                libc.ptrace(PTRACE_DETACH, dosbox.pid, None, None)
            dosbox.kill()
            dosbox.wait()
        if xvfb is not None:
            xvfb.terminate()
            xvfb.wait()

    if args.report:
        args.report.parent.mkdir(parents=True, exist_ok=True)
        args.report.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    print(
        json.dumps(
            {
                "verdict": report["verdict"],
                "samples": report["samples"],
                "guarded_samples": report["guarded_samples"],
                "anomalies": report["anomalies"],
            }
        )
    )
    return 0 if report["verdict"] in ("TIMEOUT-NO-ANOMALY", "CLEAN-EXIT") else 1


if __name__ == "__main__":
    raise SystemExit(main())
