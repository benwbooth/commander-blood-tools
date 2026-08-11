; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x00647b
; seg_off: 04da:10db
; group: seg_04da
; provenance: direct_call_target, label_csv_target, manual_binary_boundary
; label: scan_zero_word
; label_comment: 0x0000-word-terminated operand scanner; preserves SI and stores scanned word count in gs:[0x27cf]
; incoming: call@0x00675e->0x00647b
; byte_count: 25
; boundary: cfg_blocks_5_terminals_1
; terminal: ret:1
; direct_callees: none
; indirect_calls: 0
; routine_bytes_sha256: ed072c1a1a0bea5ef23461b0e01057f2dbe04b954d7a9e738542c498f50015df

00647B:  51                           push     cx
00647C:  56                           push     si
00647D:  B9 FF FF                     mov      cx, 0xffff
006480:  AD                           lodsw    ax, word ptr [si]
006481:  0B C0                        or       ax, ax
006483:  74 04                        je       0x6489
006485:  78 02                        js       0x6489
006487:  E2 F7                        loop     0x6480
006489:  F7 D9                        neg      cx
00648B:  49                           dec      cx
00648C:  65 89 0E CF 27               mov      word ptr gs:[0x27cf], cx
006491:  5E                           pop      si
006492:  59                           pop      cx
006493:  C3                           ret
