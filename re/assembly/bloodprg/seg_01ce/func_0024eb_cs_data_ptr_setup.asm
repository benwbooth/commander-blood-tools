; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x0024eb
; seg_off: 01ce:020b
; group: seg_01ce
; provenance: relocation_proven_far_transfer_target
; label: cs_data_ptr_setup
; label_comment: cs-relative data pointer setup: ds=cs; si=0x1c6; si+=0xb. Points si at a cs-segment constant table (offset 0x1c6+0xb) for read
; incoming: call@0x000e45->01ce:020b
; byte_count: 71
; boundary: cfg_blocks_5_terminals_1
; terminal: retf:1
; direct_callees: none
; indirect_calls: 0
; cxx_source: re/borland/bloodprg/seg_01ce/func_0024eb_cs_data_ptr_setup.cpp
; routine_bytes_sha256: a790e7b89b4440bcaecf68020324903f72b9da52639a6713389f9c829287e79b

0024EB:  66 50                        push     eax
0024ED:  66 51                        push     ecx
0024EF:  66 52                        push     edx
0024F1:  57                           push     di
0024F2:  1E                           push     ds
0024F3:  56                           push     si
0024F4:  8C CA                        mov      dx, cs
0024F6:  8E DA                        mov      ds, dx
0024F8:  BE C6 01                     mov      si, 0x1c6
0024FB:  83 C6 0B                     add      si, 0xb
0024FE:  66 B9 0A 00 00 00            mov      ecx, 0xa
002504:  66 0B C0                     or       eax, eax
002507:  79 08                        jns      0x2511
002509:  26 C6 05 2D                  mov      byte ptr es:[di], 0x2d
00250D:  47                           inc      di
00250E:  66 F7 D8                     neg      eax
002511:  4E                           dec      si
002512:  66 33 D2                     xor      edx, edx
002515:  66 F7 F1                     div      ecx
002518:  83 C2 30                     add      dx, 0x30
00251B:  88 14                        mov      byte ptr [si], dl
00251D:  66 0B C0                     or       eax, eax
002520:  75 EF                        jne      0x2511
002522:  AC                           lodsb    al, byte ptr [si]
002523:  AA                           stosb    byte ptr es:[di], al
002524:  0A C0                        or       al, al
002526:  75 FA                        jne      0x2522
002528:  5E                           pop      si
002529:  1F                           pop      ds
00252A:  5F                           pop      di
00252B:  66 5A                        pop      edx
00252D:  66 59                        pop      ecx
00252F:  66 58                        pop      eax
002531:  CB                           retf    
