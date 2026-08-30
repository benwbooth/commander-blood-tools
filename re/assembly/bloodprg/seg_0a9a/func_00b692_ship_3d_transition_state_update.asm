; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x00b692
; seg_off: 0a9a:06f2
; group: seg_0a9a
; provenance: manual_binary_boundary
; label: ship_3d_transition_state_update
; label_comment: Dead compiled ship-depth control routine. Bit-zero-clear DS:0x2533 arms opening after mouse idle exceeds 120; an armed zero idle counter or completed opening starts closing, with the latter gated by a zero PRNG draw modulo 20. No direct, relative, pointer, static-table, or unresolved indirect route reaches this entry in the shipped executable. NATURAL C: func_00b692_ship_3d_transition_state_update.c
; incoming: none
; byte_count: 75
; boundary: reviewed_ret_at_0x00b6dc
; terminal: ret:1
; direct_callees: 0x002de2
; indirect_calls: 0
; routine_bytes_sha256: e12b8f381dfe836a526af3f50cd8040af309127fa102175cee3cc1075be6e1fb

00B692:  F6 06 33 25 01               test     byte ptr [0x2533], 1
00B697:  75 18                        jne      0xb6b1
00B699:  83 3E 3B 0B 78               cmp      word ptr [0xb3b], 0x78
00B69E:  76 3C                        jbe      0xb6dc
00B6A0:  C6 06 31 25 04               mov      byte ptr [0x2531], 4
00B6A5:  C6 06 2F 25 01               mov      byte ptr [0x252f], 1
00B6AA:  C6 06 33 25 01               mov      byte ptr [0x2533], 1
00B6AF:  EB 2B                        jmp      0xb6dc
00B6B1:  83 3E 3B 0B 00               cmp      word ptr [0xb3b], 0
00B6B6:  75 11                        jne      0xb6c9
00B6B8:  C6 06 31 25 08               mov      byte ptr [0x2531], 8
00B6BD:  C6 06 30 25 01               mov      byte ptr [0x2530], 1
00B6C2:  C6 06 33 25 00               mov      byte ptr [0x2533], 0
00B6C7:  EB 13                        jmp      0xb6dc
00B6C9:  F6 06 2F 25 01               test     byte ptr [0x252f], 1
00B6CE:  75 0C                        jne      0xb6dc
00B6D0:  B8 14 00                     mov      ax, 0x14
00B6D3:  9A 02 0B CE 01               lcall    0x1ce, 0xb02
00B6D8:  0B C0                        or       ax, ax
00B6DA:  74 DC                        je       0xb6b8
00B6DC:  C3                           ret
