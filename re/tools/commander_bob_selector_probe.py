#!/usr/bin/env python3
"""Probe Bob's chapter-four COD/BAS handoff in unmodified original instructions.

This is a bounded diagnostic, not a normal-game route witness. An earned save
supplies VAR and procedure patches; explicit frame/contact inputs are reported.
No original instruction or script byte is replaced with a stub.
"""

import argparse
import hashlib
import json
from pathlib import Path
import struct

from unicorn import Uc, UC_ARCH_X86, UC_MODE_16, UC_HOOK_CODE, UC_HOOK_MEM_WRITE
from unicorn.x86_const import (
    UC_X86_REG_CS, UC_X86_REG_DS, UC_X86_REG_ES, UC_X86_REG_GS,
    UC_X86_REG_SS, UC_X86_REG_SP, UC_X86_REG_SI,
)

EXE_SHA256 = "7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823"
GLOBALS, COD, BAS, VAR, DIC, DEB, HISTORY = range(0x30000, 0xA0000, 0x10000)
DATA_FILE_BASE = 0xD420
BOB, PLAYER = 578, 40
START, END = 0x4DDF, 0x4E90
ROOT, TALK_BODY, KANARY_BODY = 4328, 4332, 5503
TALK, KANARY = 1, 11703


def digest(data):
    return hashlib.sha256(data).hexdigest()


class Probe:
    def __init__(self, executable, scripts, saved):
        assert digest(executable) == EXE_SHA256
        self.header = struct.unpack_from("<H", executable, 8)[0] * 16
        self.module = executable[self.header:]
        self.scripts = scripts
        assert struct.unpack_from("<H", saved)[0] == 3
        allocation_size = (len(scripts["VAR"]) + 15) // 16 * 16
        state = saved[610:610 + allocation_size]
        assert len(scripts["VAR"]) == 5428 and len(state) == 5440
        code = bytearray(scripts["COD"])
        for offset, enabled in struct.iter_unpack("<HB", saved[610 + len(state):]):
            assert code[offset - 1] == 0xA9
            code[offset] = enabled
        assert scripts["BAS"][KANARY_BODY] == 0xA3
        assert struct.unpack_from("<HH", scripts["DEB"], 10 * 20 + 16) == (BOB, 1)

        globals_ = bytearray(0x10000)
        globals_[:len(executable) - DATA_FILE_BASE] = executable[DATA_FILE_BASE:]
        for offset, address in ((0x671C, COD), (0x6720, BAS), (0x6724, VAR),
                                (0x6728, DIC), (0x672C, DEB + 10 * 20),
                                (0x6746, HISTORY)):
            struct.pack_into("<HH", globals_, offset, address % 16, address // 16)
        # The presentation scan starts at Bob's actual directory entry. Other
        # records and the action-maintenance tail are outside this probe.
        struct.pack_into("<HHH", globals_, 0x6772, TALK_BODY, 0, ROOT)
        struct.pack_into("<HH", globals_, 0x6782, TALK, 0)
        struct.pack_into("<HH", globals_, 0x674E, PLAYER, BOB)
        self.player_action = PLAYER + globals_[0x6D60 + 19 * 16]
        self.bob_action = BOB + globals_[0x6D60 + 19 * 16 + 1]
        struct.pack_into("<H", globals_, 0x675E, self.player_action)
        globals_[0x67A8] = 1
        globals_[0x67AC] = 1
        globals_[0x274F] = 1
        self.cpu = Uc(UC_ARCH_X86, UC_MODE_16)
        self.cpu.mem_map(0, 0x100000)
        for address, data in ((0, self.module), (GLOBALS, globals_), (COD, code),
                              (BAS, scripts["BAS"]), (VAR, state),
                              (DIC, scripts["DIC"]), (DEB, scripts["DEB"])):
            self.cpu.mem_write(address, bytes(data))
        for register, value in ((UC_X86_REG_CS, 0x4DA), (UC_X86_REG_GS, GLOBALS // 16),
                                (UC_X86_REG_SS, GLOBALS // 16)):
            self.cpu.reg_write(register, value)
        self.writes, self.entries, self.calls = [], [], []
        self.source = None
        self.stop = None
        self.terminal = None
        self.cpu.hook_add(UC_HOOK_CODE, self.instruction)
        self.cpu.hook_add(UC_HOOK_MEM_WRITE, self.write)
        self.prepare_frame()

    def word(self, address):
        return struct.unpack("<H", self.cpu.mem_read(address, 2))[0]

    def set_word(self, address, value):
        self.cpu.mem_write(address, struct.pack("<H", value))

    def prepare_frame(self):
        for actor in (PLAYER, BOB):
            self.set_word(VAR + actor + 2, (self.word(VAR + actor + 2) & 0x7FFF) | 1)
        self.cpu.mem_write(VAR + self.player_action, struct.pack("<HHH", 0xC4, BOB, 0))
        self.cpu.mem_write(VAR + self.bob_action, struct.pack("<HHH", 0xC4, PLAYER, 0))
        for offset in (0x5E64, 0x67B0, 0x67B7, 0x27D7):
            self.cpu.mem_write(GLOBALS + offset, b"\0")

    def instruction(self, cpu, address, size, _context):
        file_offset = address + self.header
        self.last_instruction = file_offset
        assert (0x5500 <= file_offset < file_offset + size <= 0x7409
                or 0x2DE2 <= file_offset < file_offset + size <= 0x2E33), hex(file_offset)
        if file_offset == self.stop:
            self.terminal = file_offset
            cpu.emu_stop()
        elif file_offset in (0x5613, 0x56A9):
            domain = "cod" if cpu.reg_read(UC_X86_REG_DS) == COD // 16 else "bas"
            self.source = dict(domain=domain, offset=cpu.reg_read(UC_X86_REG_SI))
            self.entries.append(self.source)
        elif file_offset in (0x56FE, 0x5791, 0x58A5):
            self.calls.append(dict(file_offset=file_offset, source=self.source))

    def write(self, _cpu, _access, address, size, value, _context):
        if (GLOBALS + 0x6782 <= address < GLOBALS + 0x6786
                or VAR + BOB + 70 <= address < VAR + BOB + 72):
            self.writes.append(dict(source=self.source, address=address, size=size, value=value))

    def run(self, entry, stop):
        self.stop, self.terminal = stop, None
        self.cpu.reg_write(UC_X86_REG_SP, 0xFF00)
        self.cpu.reg_write(UC_X86_REG_DS, GLOBALS // 16)
        self.cpu.reg_write(UC_X86_REG_ES, GLOBALS // 16)
        self.cpu.emu_start(entry - self.header, 0, count=1000000)
        assert self.terminal == stop, (hex(entry), self.terminal,
                                      hex(self.last_instruction), self.entries[-4:])
        # The original PRNG stores its five mutable data bytes in its code segment.
        after = bytearray(self.cpu.mem_read(0, len(self.module)))
        start, end = 0x2DCE - self.header, 0x2DD3 - self.header
        after[start:end] = self.module[start:end]
        assert after == self.module
        assert bytes(self.cpu.mem_read(DIC, len(self.scripts["DIC"]))) == self.scripts["DIC"]
        assert bytes(self.cpu.mem_read(DEB, len(self.scripts["DEB"]))) == self.scripts["DEB"]
        if stop in (0x568A, 0x568D):
            assert self.cpu.reg_read(UC_X86_REG_SP) == 0xFF00
        else:
            assert stop == 0x58A8 and self.cpu.reg_read(UC_X86_REG_SP) == 0xFEFC
        return self.snapshot()

    def cod_pass(self):
        self.prepare_frame()
        self.cpu.mem_write(GLOBALS + 0x67B1, b"\2")
        self.set_word(GLOBALS + 0x6778, END)
        self.set_word(GLOBALS + 0x677A, START)
        result = self.run(0x55F8, 0x568A)
        self.cpu.mem_write(GLOBALS + 0x67B1, b"\0")
        return result

    def selection(self, concept, gate):
        self.set_word(GLOBALS + 0x6762, concept)
        # Commit is an actual near call in the wrapper, stopping at its return.
        committed = self.run(0x568A, 0x568D)
        self.prepare_frame()
        self.cpu.mem_write(GLOBALS + 0x1FB2, bytes([gate]))
        scanned = self.run(0x568D, 0x58A8)
        return dict(concept=concept, scene_gate=gate, committed=committed, scanned=scanned)

    def snapshot(self):
        pending = []
        for offset in range(0x67F8, 0x6878, 2):
            value = self.word(GLOBALS + offset)
            if value == 0:
                break
            pending.append(value)
        labels = [self.scripts["DIC"][value:].split(b"\0", 1)[0].decode("ascii")
                  for value in pending]
        return dict(current_control=self.word(GLOBALS + 0x6782),
                    parent_control=self.word(GLOBALS + 0x6784),
                    current_body=self.word(GLOBALS + 0x6772),
                    parent_body=self.word(GLOBALS + 0x6774),
                    actor_control=self.word(VAR + BOB + 70),
                    pending_words=pending, pending_labels=labels)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("directory", type=Path)
    parser.add_argument("save", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    assert not args.output.exists(), "refusing to replace an existing diagnostic"
    executable = (Path(__file__).resolve().parents[2] / "re/bin/BLOODPRG.EXE").read_bytes()
    scripts = {suffix: (args.directory / f"SCRIPT4.{suffix}").read_bytes()
               for suffix in ("COD", "BAS", "VAR", "DIC", "DEB")}
    saved = args.save.read_bytes()
    result = dict(scope=__doc__, executable_sha256=digest(executable),
                  save_sha256=digest(saved), script_sha256={k: digest(v) for k, v in scripts.items()},
                  frame_inputs=dict(player=PLAYER, actor=BOB, active=True,
                                    contact=True, clear_text_and_actor_block=True,
                                    directory_start_record=10, cod_slice=[START, END]), cases=[])
    employees = scripts["DIC"].index(b"employees\0")
    assert employees == 12244
    for gate in (0, 1):
        probe = Probe(executable, scripts, saved)
        preparation = [probe.cod_pass() for _ in range(3)]
        kanary = probe.selection(KANARY, gate)
        next_pass = probe.cod_pass()
        employee = probe.selection(employees, 0)
        assert preparation[-1]["current_control"] == TALK
        assert kanary["committed"]["current_control"] == KANARY
        assert kanary["scanned"]["current_body"] == KANARY_BODY
        assert kanary["scanned"]["pending_labels"] == (
            [] if gate else ["talk", "bye_bye", "cottage", "clones", "employees", "reports"])
        assert next_pass["current_control"] == TALK
        assert employee["scanned"]["current_body"] == TALK_BODY
        assert "employees" not in employee["scanned"]["pending_labels"]
        assert any(write["source"] == dict(domain="cod", offset=19966)
                   and write["address"] == GLOBALS + 0x6782
                   and write["value"] == TALK for write in probe.writes)
        result["cases"].append(dict(scene_gate=gate, preparation=preparation,
                                    kanary=kanary, next_cod_pass=next_pass, employees=employee,
                                    writes=probe.writes, calls=probe.calls, entries=probe.entries))
    args.output.write_text(json.dumps(result, indent=2) + "\n")
    print(json.dumps({"output": str(args.output), "cases": [
        {key: case[key] for key in ("scene_gate", "kanary", "next_cod_pass", "employees")}
        for case in result["cases"]]}))


if __name__ == "__main__":
    main()
