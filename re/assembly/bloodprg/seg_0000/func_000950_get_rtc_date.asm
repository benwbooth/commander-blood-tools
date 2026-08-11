; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x000950
; seg_off: 0000:0350
; group: seg_0000
; provenance: relocation_proven_far_transfer_target
; label: get_rtc_date
; label_comment: read the real-time clock date: ah=4; int 1Ah (BIOS get RTC date); al=dl; call bcd_to_binary 0x986. Reads + decodes the CMOS RTC date (used for the PRNG seed + any date logic)
; incoming: call@0x0055bb->0000:0350
; byte_count: 54
; boundary: cfg_blocks_4_terminals_2
; terminal: jmp 0x97e:1, retf:1
; direct_callees: 0x000986
; indirect_calls: 0
; routine_bytes_sha256: f39e5811197c4750660ae551ca7b3fb80f75ea8ef5e99107e2ccc23c504ba763

000950:  50                           push     ax
000951:  51                           push     cx
000952:  52                           push     dx
000953:  B4 04                        mov      ah, 4
000955:  CD 1A                        int      0x1a
000957:  8A C2                        mov      al, dl
000959:  E8 2A 00                     call     0x986
00095C:  98                           cwde    
00095D:  65 A3 A8 0A                  mov      word ptr gs:[0xaa8], ax
000961:  8A C6                        mov      al, dh
000963:  E8 20 00                     call     0x986
000966:  98                           cwde    
000967:  65 A3 AA 0A                  mov      word ptr gs:[0xaaa], ax
00096B:  8A C1                        mov      al, cl
00096D:  E8 16 00                     call     0x986
000970:  98                           cwde    
000971:  80 FD 13                     cmp      ch, 0x13
000974:  75 05                        jne      0x97b
000976:  05 6C 07                     add      ax, 0x76c
000979:  EB 03                        jmp      0x97e
00097B:  05 D0 07                     add      ax, 0x7d0
00097E:  65 A3 AC 0A                  mov      word ptr gs:[0xaac], ax
000982:  5A                           pop      dx
000983:  59                           pop      cx
000984:  58                           pop      ax
000985:  CB                           retf    
