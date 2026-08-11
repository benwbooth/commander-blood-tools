; Commander Blood recovered routine assembly
; module: xdb_manu3
; artifact: output/_tmp_dat/manu3.xdb
; artifact_sha256: d0f64e99a646197906e273edfa0124172307a5cd766c88591c12ebd9ea556d31
; overlay_offset: 0x000150
; group: manu3_labeled
; provenance: manu3 no-cursor per-frame entry
; byte_count: 44
; boundary: cfg_blocks_3_terminals_1
; terminal: retf:1
; direct_callees: 0x00019b, 0x000270, 0x000549, 0x0006f6
; indirect_calls: 0
; cxx_source: re/borland/xdb/manu3/manu3_labeled/func_000150_routine.cpp
; routine_bytes_sha256: 5b722c6d62fdc873ebb82a18e20efcbd82febab4f0e954f6bdfab7b805fc09af

000150:  1E                           push     ds
000151:  2E 8B 0E 6A 13               mov      cx, word ptr cs:[0x136a]
000156:  0B C9                        or       cx, cx
000158:  74 20                        je       0x17a
00015A:  8E E1                        mov      fs, cx
00015C:  8E D9                        mov      ds, cx
00015E:  8E C1                        mov      es, cx
000160:  36 A1 CE 20                  mov      ax, word ptr ss:[0x20ce]
000164:  C1 E8 04                     shr      ax, 4
000167:  80 C4 A0                     add      ah, 0xa0
00016A:  64 A3 18 00                  mov      word ptr fs:[0x18], ax
00016E:  E8 2A 00                     call     0x19b
000171:  E8 FC 00                     call     0x270
000174:  E8 D2 03                     call     0x549
000177:  E8 7C 05                     call     0x6f6
00017A:  1F                           pop      ds
00017B:  CB                           retf    
