; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x000986
; seg_off: 0038:0006
; group: seg_0000
; provenance: direct_call_target, label_csv_target, manual_binary_boundary
; label: bcd_to_binary
; label_comment: packed-BCD-to-binary byte helper: AL = ((AL >> 4) * 10) + (AL & 0x0f)
; incoming: call@0x000944->0x000986
; incoming: call@0x000959->0x000986
; incoming: call@0x000963->0x000986
; incoming: call@0x00096d->0x000986
; byte_count: 17
; boundary: cfg_blocks_1_terminals_1
; terminal: ret:1
; direct_callees: none
; indirect_calls: 0
; routine_bytes_sha256: 65bd730970b38f982176699cde398c15640791e77c3fb13d0764ec6a3e7db9d5

000986:  53                           push     bx
000987:  8A D8                        mov      bl, al
000989:  80 E3 0F                     and      bl, 0xf
00098C:  C0 E8 04                     shr      al, 4
00098F:  B7 0A                        mov      bh, 0xa
000991:  F6 E7                        mul      bh
000993:  02 C3                        add      al, bl
000995:  5B                           pop      bx
000996:  C3                           ret
