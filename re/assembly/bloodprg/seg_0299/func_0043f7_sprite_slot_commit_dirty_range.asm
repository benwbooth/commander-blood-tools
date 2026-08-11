; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x0043f7
; seg_off: 0299:1467
; group: seg_0299
; provenance: recursive_graph, relocation_proven_far_transfer_target
; label: sprite_slot_commit_dirty_range
; label_comment: commits dirty active sprite-slot current geometry into previous-geometry fields across AX..BX
; incoming: call@0x007849->0299:1467
; incoming: call@0x009575->0299:1467
; incoming: call@0x00b1d0->0299:1467
; byte_count: 120
; boundary: cfg_blocks_7_terminals_2
; terminal: jmp 0x4464:1, retf:1
; direct_callees: none
; indirect_calls: 0
; cxx_source: re/borland/bloodprg/seg_0299/func_0043f7_sprite_slot_commit_dirty_range.cpp
; routine_bytes_sha256: 689ca5c23e7a1b41b1c27b312bc0e4549302f37dc56346cb9fab5f2b2907578d

0043F7:  66 55                        push     ebp
0043F9:  06                           push     es
0043FA:  1E                           push     ds
0043FB:  56                           push     si
0043FC:  57                           push     di
0043FD:  66 50                        push     eax
0043FF:  51                           push     cx
004400:  53                           push     bx
004401:  8B E8                        mov      bp, ax
004403:  66 C1 E5 10                  shl      ebp, 0x10
004407:  8B EB                        mov      bp, bx
004409:  8C E8                        mov      ax, gs
00440B:  8E D8                        mov      ds, ax
00440D:  8E C0                        mov      es, ax
00440F:  BE 12 62                     mov      si, 0x6212
004412:  F7 06 49 52 01 00            test     word ptr [0x5249], 1
004418:  74 1B                        je       0x4435
00441A:  BF 12 66                     mov      di, 0x6612
00441D:  66 A1 35 52                  mov      eax, dword ptr [0x5235]
004421:  66 AB                        stosd    dword ptr es:[di], eax
004423:  66 A1 39 52                  mov      eax, dword ptr [0x5239]
004427:  66 AB                        stosd    dword ptr es:[di], eax
004429:  C7 05 FF FF                  mov      word ptr [di], 0xffff
00442D:  C7 06 49 52 00 00            mov      word ptr [0x5249], 0
004433:  EB 2F                        jmp      0x4464
004435:  8B CD                        mov      cx, bp
004437:  66 C1 ED 10                  shr      ebp, 0x10
00443B:  8B C5                        mov      ax, bp
00443D:  41                           inc      cx
00443E:  2B C8                        sub      cx, ax
004440:  C1 E0 05                     shl      ax, 5
004443:  03 F0                        add      si, ax
004445:  F6 04 02                     test     byte ptr [si], 2
004448:  74 15                        je       0x445f
00444A:  F6 04 01                     test     byte ptr [si], 1
00444D:  74 10                        je       0x445f
00444F:  66 8B 44 08                  mov      eax, dword ptr [si + 8]
004453:  66 89 44 10                  mov      dword ptr [si + 0x10], eax
004457:  66 8B 44 0C                  mov      eax, dword ptr [si + 0xc]
00445B:  66 89 44 14                  mov      dword ptr [si + 0x14], eax
00445F:  83 C6 20                     add      si, 0x20
004462:  E2 E1                        loop     0x4445
004464:  5B                           pop      bx
004465:  59                           pop      cx
004466:  66 58                        pop      eax
004468:  5F                           pop      di
004469:  5E                           pop      si
00446A:  1F                           pop      ds
00446B:  07                           pop      es
00446C:  66 5D                        pop      ebp
00446E:  CB                           retf    
