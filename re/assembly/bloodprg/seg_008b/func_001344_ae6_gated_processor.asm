; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x001344
; seg_off: 008b:0494
; group: seg_008b
; provenance: recursive_graph
; label: ae6_gated_processor
; label_comment: family of routines (0x1344/0x1397/0x13c4) gated on gs:[0xae6]&1 (a mode flag - intro/special state); es=gs then process/copy. Mode-conditional processing
; byte_count: 83
; boundary: cfg_blocks_3_terminals_1
; terminal: ret:1
; direct_callees: none
; indirect_calls: 0
; routine_bytes_sha256: 82a19bceb5835190242fddfcd708a524b73132661667cfe6b3478a3a113906d1

001344:  06                           push     es
001345:  50                           push     ax
001346:  53                           push     bx
001347:  51                           push     cx
001348:  65 F6 06 E6 0A 01            test     byte ptr gs:[0xae6], 1
00134E:  74 42                        je       0x1392
001350:  8C E8                        mov      ax, gs
001352:  8E C0                        mov      es, ax
001354:  BB 41 0B                     mov      bx, 0xb41
001357:  33 C9                        xor      cx, cx
001359:  65 8A 0E B9 01               mov      cl, byte ptr gs:[0x1b9]
00135E:  C6 47 02 03                  mov      byte ptr [bx + 2], 3
001362:  C7 47 0E 5B 0B               mov      word ptr [bx + 0xe], 0xb5b
001367:  8C 6F 10                     mov      word ptr [bx + 0x10], gs
00136A:  B8 10 15                     mov      ax, 0x1510
00136D:  CD 2F                        int      0x2f
00136F:  C7 47 0E 6B 0B               mov      word ptr [bx + 0xe], 0xb6b
001374:  65 C6 06 6C 0B 02            mov      byte ptr gs:[0xb6c], 2
00137A:  B8 10 15                     mov      ax, 0x1510
00137D:  CD 2F                        int      0x2f
00137F:  C6 47 02 0C                  mov      byte ptr [bx + 2], 0xc
001383:  C7 47 0E 62 0B               mov      word ptr [bx + 0xe], 0xb62
001388:  C7 47 12 09 00               mov      word ptr [bx + 0x12], 9
00138D:  B8 10 15                     mov      ax, 0x1510
001390:  CD 2F                        int      0x2f
001392:  59                           pop      cx
001393:  5B                           pop      bx
001394:  58                           pop      ax
001395:  07                           pop      es
001396:  C3                           ret     
