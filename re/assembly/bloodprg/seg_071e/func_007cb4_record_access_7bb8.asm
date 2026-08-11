; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x007cb4
; seg_off: 071e:04d4
; group: seg_071e
; provenance: recursive_graph
; label: record_access_7bb8
; label_comment: 32-byte-record accessor (2 calls): si=0x7bb8 + [0x27e3]*32 (al=[0x27e3]; cbw; shl 5). Indexes a 32-byte-record table at 0x7bb8 by the [0x27e3] index
; byte_count: 52
; boundary: cfg_blocks_5_terminals_1
; terminal: ret:1
; direct_callees: none
; indirect_calls: 0
; cxx_source: re/borland/bloodprg/seg_071e/func_007cb4_record_access_7bb8.cpp
; routine_bytes_sha256: 1c9098a89a5306202fce335d2cb43b064b913101451d01deda9115263d87d20d

007CB4:  06                           push     es
007CB5:  57                           push     di
007CB6:  BE B8 7B                     mov      si, 0x7bb8
007CB9:  A0 E3 27                     mov      al, byte ptr [0x27e3]
007CBC:  98                           cwde    
007CBD:  C1 E0 05                     shl      ax, 5
007CC0:  03 F0                        add      si, ax
007CC2:  C4 3E 21 52                  les      di, ptr [0x5221]
007CC6:  BF C5 12                     mov      di, 0x12c5
007CC9:  B9 10 00                     mov      cx, 0x10
007CCC:  B2 FE                        mov      dl, 0xfe
007CCE:  57                           push     di
007CCF:  AD                           lodsw    ax, word ptr [si]
007CD0:  86 C4                        xchg     ah, al
007CD2:  D1 E0                        shl      ax, 1
007CD4:  73 03                        jae      0x7cd9
007CD6:  26 88 15                     mov      byte ptr es:[di], dl
007CD9:  47                           inc      di
007CDA:  0B C0                        or       ax, ax
007CDC:  75 F4                        jne      0x7cd2
007CDE:  5F                           pop      di
007CDF:  81 C7 40 01                  add      di, 0x140
007CE3:  E2 E9                        loop     0x7cce
007CE5:  5F                           pop      di
007CE6:  07                           pop      es
007CE7:  C3                           ret     
