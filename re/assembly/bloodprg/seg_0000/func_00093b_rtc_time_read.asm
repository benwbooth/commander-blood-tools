; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x00093b
; seg_off: 0000:033b
; group: seg_0000
; provenance: relocation_proven_far_transfer_target
; label: rtc_time_read
; label_comment: RTC time read: ah=2; int 1Ah (BIOS get real-time-clock time); al=ch; call 0x986. Reads the hardware clock (used to seed timing/PRNG)
; incoming: call@0x0055b6->0000:033b
; byte_count: 21
; boundary: cfg_blocks_1_terminals_1
; terminal: retf:1
; direct_callees: 0x000986
; indirect_calls: 0
; cxx_source: re/borland/bloodprg/seg_0000/func_00093b_rtc_time_read.cpp
; routine_bytes_sha256: 35de745a4bb6ad6236d826f979558ddcfa4915bdfc4afc63657f21dd24ffa235

00093B:  50                           push     ax
00093C:  51                           push     cx
00093D:  52                           push     dx
00093E:  B4 02                        mov      ah, 2
000940:  CD 1A                        int      0x1a
000942:  8A C5                        mov      al, ch
000944:  E8 3F 00                     call     0x986
000947:  98                           cwde    
000948:  65 A3 A6 0A                  mov      word ptr gs:[0xaa6], ax
00094C:  5A                           pop      dx
00094D:  59                           pop      cx
00094E:  58                           pop      ax
00094F:  CB                           retf    
