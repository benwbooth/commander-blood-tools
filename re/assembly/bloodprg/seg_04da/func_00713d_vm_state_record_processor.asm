; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x00713d
; seg_off: 04da:1d9d
; group: seg_04da
; provenance: recursive_graph, relocation_proven_far_transfer_target
; label: vm_state_record_processor
; label_comment: builds the zero-terminated SS:0x24FB list of active kind-mask 0x0098 records whose selector-0x0B position matches the arche record; kind-0x80 candidates are validated through their selector-0x11 parent link
; natural_c: re/source/bloodprg/candidates/seg_04da/func_00713d_vm_state_record_processor.c
; incoming: call@0x00b0c7->04da:1d9d
; byte_count: 146
; boundary: cfg_blocks_12_terminals_2
; terminal: jmp 0x7165:1, retf:1
; direct_callees: 0x006023
; indirect_calls: 0
; routine_bytes_sha256: 6851fef835189af38010ac811d0e61891725683e320cf63c20512756c0f11857

00713D:  1E                           push     ds
00713E:  56                           push     si
00713F:  06                           push     es
007140:  57                           push     di
007141:  55                           push     bp
007142:  66 52                        push     edx
007144:  53                           push     bx
007145:  50                           push     ax
007146:  65 C5 36 24 67               lds      si, ptr gs:[0x6724]
00714B:  65 C4 3E 2C 67               les      di, ptr gs:[0x672c]
007150:  BD FB 24                     mov      bp, 0x24fb
007153:  65 8B 36 52 67               mov      si, word ptr gs:[0x6752]
007158:  8B 1C                        mov      bx, word ptr [si]
00715A:  B8 0B 00                     mov      ax, 0xb
00715D:  E8 C3 EE                     call     0x6023
007160:  03 F0                        add      si, ax
007162:  66 8B 14                     mov      edx, dword ptr [si]
007165:  26 8A 45 12                  mov      al, byte ptr es:[di + 0x12]
007169:  3C 01                        cmp      al, 1
00716B:  75 53                        jne      0x71c0
00716D:  26 8B 75 10                  mov      si, word ptr es:[di + 0x10]
007171:  8A 44 02                     mov      al, byte ptr [si + 2]
007174:  24 01                        and      al, 1
007176:  74 43                        je       0x71bb
007178:  8B 04                        mov      ax, word ptr [si]
00717A:  A9 98 00                     test     ax, 0x98
00717D:  74 3C                        je       0x71bb
00717F:  65 3B 36 52 67               cmp      si, word ptr gs:[0x6752]
007184:  74 35                        je       0x71bb
007186:  89 76 00                     mov      word ptr [bp], si
007189:  A9 80 00                     test     ax, 0x80
00718C:  74 1B                        je       0x71a9
00718E:  BB 80 00                     mov      bx, 0x80
007191:  B8 11 00                     mov      ax, 0x11
007194:  E8 8C EE                     call     0x6023
007197:  03 F0                        add      si, ax
007199:  8B 34                        mov      si, word ptr [si]
00719B:  8A 44 02                     mov      al, byte ptr [si + 2]
00719E:  A8 01                        test     al, 1
0071A0:  74 19                        je       0x71bb
0071A2:  8B 04                        mov      ax, word ptr [si]
0071A4:  A9 18 00                     test     ax, 0x18
0071A7:  74 12                        je       0x71bb
0071A9:  8B D8                        mov      bx, ax
0071AB:  B8 0B 00                     mov      ax, 0xb
0071AE:  E8 72 EE                     call     0x6023
0071B1:  03 F0                        add      si, ax
0071B3:  66 3B 14                     cmp      edx, dword ptr [si]
0071B6:  75 03                        jne      0x71bb
0071B8:  83 C5 02                     add      bp, 2
0071BB:  83 C7 14                     add      di, 0x14
0071BE:  EB A5                        jmp      0x7165
0071C0:  C7 46 00 00 00               mov      word ptr [bp], 0
0071C5:  58                           pop      ax
0071C6:  5B                           pop      bx
0071C7:  66 5A                        pop      edx
0071C9:  5D                           pop      bp
0071CA:  5F                           pop      di
0071CB:  07                           pop      es
0071CC:  5E                           pop      si
0071CD:  1F                           pop      ds
0071CE:  CB                           retf    
