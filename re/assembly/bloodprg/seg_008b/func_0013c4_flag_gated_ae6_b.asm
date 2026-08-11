; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x0013c4
; seg_off: 008b:0514
; group: seg_008b
; provenance: recursive_graph, relocation_proven_far_transfer_target
; label: flag_gated_ae6_b
; label_comment: flag-gated render routine (sibling of 0x1397): same gs:0xae6 gate, skips to 0x1476 when clear
; incoming: call@0x00b5f9->008b:0514
; byte_count: 187
; boundary: cfg_blocks_3_terminals_1
; terminal: retf:1
; direct_callees: none
; indirect_calls: 0
; cxx_source: re/borland/bloodprg/seg_008b/func_0013c4_flag_gated_ae6_b.cpp
; routine_bytes_sha256: b5dc8b1767297a8a0800d4247522ddafeb89e72e56f9186c7c81d473e3f121fb

0013C4:  66 50                        push     eax
0013C6:  06                           push     es
0013C7:  1E                           push     ds
0013C8:  53                           push     bx
0013C9:  51                           push     cx
0013CA:  66 52                        push     edx
0013CC:  65 F6 06 E6 0A 01            test     byte ptr gs:[0xae6], 1
0013D2:  0F 84 A0 00                  je       0x1476
0013D6:  8C E8                        mov      ax, gs
0013D8:  8E C0                        mov      es, ax
0013DA:  8E D8                        mov      ds, ax
0013DC:  BB 72 0B                     mov      bx, 0xb72
0013DF:  C6 07 16                     mov      byte ptr [bx], 0x16
0013E2:  C6 47 02 84                  mov      byte ptr [bx + 2], 0x84
0013E6:  66 A1 6D 0B                  mov      eax, dword ptr [0xb6d]
0013EA:  66 89 47 0E                  mov      dword ptr [bx + 0xe], eax
0013EE:  53                           push     bx
0013EF:  8A C8                        mov      cl, al
0013F1:  66 C1 E8 08                  shr      eax, 8
0013F5:  8A E8                        mov      ch, al
0013F7:  C1 E8 08                     shr      ax, 8
0013FA:  BA 94 11                     mov      dx, 0x1194
0013FD:  F7 E2                        mul      dx
0013FF:  8B D8                        mov      bx, ax
001401:  B0 4B                        mov      al, 0x4b
001403:  F6 E5                        mul      ch
001405:  03 C3                        add      ax, bx
001407:  83 D2 00                     adc      dx, 0
00140A:  02 C1                        add      al, cl
00140C:  80 D4 00                     adc      ah, 0
00140F:  83 D2 00                     adc      dx, 0
001412:  66 C1 C8 10                  ror      eax, 0x10
001416:  8B C2                        mov      ax, dx
001418:  66 C1 C8 10                  ror      eax, 0x10
00141C:  66 2D 96 00 00 00            sub      eax, 0x96
001422:  66 50                        push     eax
001424:  66 A1 5E 0B                  mov      eax, dword ptr [0xb5e]
001428:  8A C8                        mov      cl, al
00142A:  66 C1 E8 08                  shr      eax, 8
00142E:  8A E8                        mov      ch, al
001430:  C1 E8 08                     shr      ax, 8
001433:  BA 94 11                     mov      dx, 0x1194
001436:  F7 E2                        mul      dx
001438:  8B D8                        mov      bx, ax
00143A:  B0 4B                        mov      al, 0x4b
00143C:  F6 E5                        mul      ch
00143E:  03 C3                        add      ax, bx
001440:  83 D2 00                     adc      dx, 0
001443:  02 C1                        add      al, cl
001445:  80 D4 00                     adc      ah, 0
001448:  83 D2 00                     adc      dx, 0
00144B:  66 C1 C8 10                  ror      eax, 0x10
00144F:  8B C2                        mov      ax, dx
001451:  66 C1 C8 10                  ror      eax, 0x10
001455:  66 2D 96 00 00 00            sub      eax, 0x96
00145B:  66 5A                        pop      edx
00145D:  66 2B C2                     sub      eax, edx
001460:  66 33 D2                     xor      edx, edx
001463:  5B                           pop      bx
001464:  66 89 47 12                  mov      dword ptr [bx + 0x12], eax
001468:  66 B8 10 15 00 00            mov      eax, 0x1510
00146E:  33 C9                        xor      cx, cx
001470:  8A 0E B9 01                  mov      cl, byte ptr [0x1b9]
001474:  CD 2F                        int      0x2f
001476:  66 5A                        pop      edx
001478:  59                           pop      cx
001479:  5B                           pop      bx
00147A:  1F                           pop      ds
00147B:  07                           pop      es
00147C:  66 58                        pop      eax
00147E:  CB                           retf    
