; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x002dd3
; seg_off: 01ce:0af3
; group: seg_01ce
; provenance: recursive_graph, relocation_proven_far_transfer_target
; label: cmos_rtc_read
; label_comment: CMOS RTC read: xor ax,ax; out 0x70,al (select CMOS register 0); in al,0x71 (read); cs:[0xaee]=ax. Reads the CMOS real-time-clock seconds register directly
; incoming: call@0x00068d->01ce:0af3
; byte_count: 15
; boundary: cfg_blocks_1_terminals_1
; terminal: retf:1
; direct_callees: none
; indirect_calls: 0
; cxx_source: re/borland/bloodprg/seg_01ce/func_002dd3_cmos_rtc_read.cpp
; routine_bytes_sha256: aa2e7338694624fa1308e00b1a2cf05357396ec02147a6cf1015933d150747cf

002DD3:  50                           push     ax
002DD4:  33 C0                        xor      ax, ax
002DD6:  E6 70                        out      0x70, al
002DD8:  E4 71                        in       al, 0x71
002DDA:  8A E0                        mov      ah, al
002DDC:  2E A3 EE 0A                  mov      word ptr cs:[0xaee], ax
002DE0:  58                           pop      ax
002DE1:  CB                           retf    
