; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x00bb9d
; seg_off: 0b1b:03ed
; group: seg_0b1b
; provenance: recursive_graph, relocation_proven_far_transfer_target
; label: snd_driver_call
; label_comment: SND driver callback invoke (2 calls): ds=gs; lcall [0xcdf] (the registered SND-driver far fn-ptr, per dead_ends sess 001-002 - statically unresolvable). Dispatches to the loaded .drv sound driver
; incoming: call@0x001252->0b1b:03ed
; incoming: call@0x0012e0->0b1b:03ed
; incoming: call@0x0012e8->0b1b:03ed
; incoming: call@0x005c10->0b1b:03ed
; incoming: call@0x007b3a->0b1b:03ed
; incoming: call@0x007bf3->0b1b:03ed
; incoming: call@0x00b254->0b1b:03ed
; byte_count: 22
; boundary: cfg_blocks_1_terminals_1
; terminal: retf:1
; direct_callees: none
; indirect_calls: 1
; cxx_source: re/borland/bloodprg/seg_0b1b/func_00bb9d_snd_driver_call.cpp
; routine_bytes_sha256: e7fc7b7a0177bf7cbb2bd10dad92f9144b1bb5af1d5fb5d71b53b43f03ac725b

00BB9D:  50                           push     ax
00BB9E:  1E                           push     ds
00BB9F:  06                           push     es
00BBA0:  8C E8                        mov      ax, gs
00BBA2:  8E D8                        mov      ds, ax
00BBA4:  33 C0                        xor      ax, ax
00BBA6:  FF 1E DF 0C                  lcall    [0xcdf]
00BBAA:  C6 06 A0 0B 00               mov      byte ptr [0xba0], 0
00BBAF:  07                           pop      es
00BBB0:  1F                           pop      ds
00BBB1:  58                           pop      ax
00BBB2:  CB                           retf    
