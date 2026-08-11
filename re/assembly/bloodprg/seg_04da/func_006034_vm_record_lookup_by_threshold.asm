; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x006034
; seg_off: 05a3:0004
; group: seg_04da
; provenance: direct_call_target, label_csv_target, manual_binary_boundary
; label: vm_record_lookup_by_threshold
; label_comment: scan the gs:0x672c 20-byte directory and return the previous entry threshold whose +0x10 is below/equal AX
; incoming: call@0x00699f->0x006034
; incoming: call@0x0069b7->0x006034
; incoming: call@0x006a15->0x006034
; incoming: call@0x006b36->0x006034
; incoming: call@0x006b60->0x006034
; incoming: call@0x006c92->0x006034
; incoming: call@0x006e48->0x006034
; incoming: call@0x006f02->0x006034
; byte_count: 26
; boundary: cfg_blocks_4_terminals_1
; terminal: ret:1
; direct_callees: none
; indirect_calls: 0
; routine_bytes_sha256: 10978a866c956e4f8e47c15b3f91ad2b0bf30d6acf7d1f6286e89f1d5f71a6e6

006034:  1E                           push     ds
006035:  56                           push     si
006036:  65 C5 36 2C 67               lds      si, ptr gs:[0x672c]
00603B:  3B 44 10                     cmp      ax, word ptr [si + 0x10]
00603E:  76 05                        jbe      0x6045
006040:  83 C6 14                     add      si, 0x14
006043:  EB F6                        jmp      0x603b
006045:  83 EE 14                     sub      si, 0x14
006048:  8B 44 10                     mov      ax, word ptr [si + 0x10]
00604B:  5E                           pop      si
00604C:  1F                           pop      ds
00604D:  C3                           ret
