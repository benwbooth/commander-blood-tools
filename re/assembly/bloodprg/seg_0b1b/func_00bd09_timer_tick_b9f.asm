; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x00bd09
; seg_off: 0b1b:0559
; group: seg_0b1b
; provenance: recursive_graph
; label: timer_tick_b9f
; label_comment: timer/counter tick (2 calls): bl=gs:[0xb9f]; dec; if it underflows call 0xbd26 (reload/fire) else store back. A countdown timer (sound-tempo/frame pacing) that fires 0xbd26 on expiry
; byte_count: 29
; boundary: cfg_blocks_6_terminals_3
; terminal: jmp 0xbd24:2, ret:1
; direct_callees: 0x00bd26, 0x00bd4e, 0x00bd8d
; indirect_calls: 0
; routine_bytes_sha256: c4f057a1f1e81d9fb22d0a27a472bcb804659cf240fed7127de452fa2d5dc071

00BD09:  53                           push     bx
00BD0A:  65 8A 1E 9F 0B               mov      bl, byte ptr gs:[0xb9f]
00BD0F:  FE CB                        dec      bl
00BD11:  79 05                        jns      0xbd18
00BD13:  E8 10 00                     call     0xbd26
00BD16:  EB 0C                        jmp      0xbd24
00BD18:  FE CB                        dec      bl
00BD1A:  79 05                        jns      0xbd21
00BD1C:  E8 2F 00                     call     0xbd4e
00BD1F:  EB 03                        jmp      0xbd24
00BD21:  E8 69 00                     call     0xbd8d
00BD24:  5B                           pop      bx
00BD25:  C3                           ret     
