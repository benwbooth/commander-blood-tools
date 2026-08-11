; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x0098b9
; seg_off: 071e:20d9
; group: seg_071e
; provenance: recursive_graph
; label: ship_3d_projection_matrix_build
; label_comment: builds 3x3 fixed-point projection matrix at DS:0x2F95 from angle table DS:0x4F45 and angle words DS:0x2F71/0x2F6D/0x2F6F
; byte_count: 343
; boundary: cfg_blocks_1_terminals_1
; terminal: retf:1
; direct_callees: none
; indirect_calls: 0
; routine_bytes_sha256: ee9cefae7bb3c3bcc0acfa72dd6f6f3731e166b91b2e15c3d3e62eee82653bb5

0098B9:  66 50                        push     eax
0098BB:  66 53                        push     ebx
0098BD:  66 51                        push     ecx
0098BF:  66 55                        push     ebp
0098C1:  1E                           push     ds
0098C2:  06                           push     es
0098C3:  56                           push     si
0098C4:  57                           push     di
0098C5:  8C E8                        mov      ax, gs
0098C7:  8E D8                        mov      ds, ax
0098C9:  8E C0                        mov      es, ax
0098CB:  BD 45 4F                     mov      bp, 0x4f45
0098CE:  BE 7D 2F                     mov      si, 0x2f7d
0098D1:  8B 3E 71 2F                  mov      di, word ptr [0x2f71]
0098D5:  C1 E7 02                     shl      di, 2
0098D8:  66 0F BF 1B                  movsx    ebx, word ptr [bp + di]
0098DC:  66 0F BF 4B 02               movsx    ecx, word ptr [bp + di + 2]
0098E1:  66 03 DB                     add      ebx, ebx
0098E4:  66 03 C9                     add      ecx, ecx
0098E7:  66 89 5C 10                  mov      dword ptr [si + 0x10], ebx
0098EB:  66 89 4C 14                  mov      dword ptr [si + 0x14], ecx
0098EF:  8B 3E 6D 2F                  mov      di, word ptr [0x2f6d]
0098F3:  C1 E7 02                     shl      di, 2
0098F6:  66 0F BF 1B                  movsx    ebx, word ptr [bp + di]
0098FA:  66 0F BF 4B 02               movsx    ecx, word ptr [bp + di + 2]
0098FF:  66 03 DB                     add      ebx, ebx
009902:  66 03 C9                     add      ecx, ecx
009905:  66 89 1C                     mov      dword ptr [si], ebx
009908:  66 89 4C 04                  mov      dword ptr [si + 4], ecx
00990C:  8B 3E 6F 2F                  mov      di, word ptr [0x2f6f]
009910:  C1 E7 02                     shl      di, 2
009913:  66 0F BF 1B                  movsx    ebx, word ptr [bp + di]
009917:  66 0F BF 4B 02               movsx    ecx, word ptr [bp + di + 2]
00991C:  66 03 DB                     add      ebx, ebx
00991F:  66 03 C9                     add      ecx, ecx
009922:  66 89 5C 08                  mov      dword ptr [si + 8], ebx
009926:  66 89 4C 0C                  mov      dword ptr [si + 0xc], ecx
00992A:  BE 7D 2F                     mov      si, 0x2f7d
00992D:  BF 95 2F                     mov      di, 0x2f95
009930:  66 8B 44 10                  mov      eax, dword ptr [si + 0x10]
009934:  66 0F AF 04                  imul     eax, dword ptr [si]
009938:  66 8B 5C 04                  mov      ebx, dword ptr [si + 4]
00993C:  66 0F AF 5C 0C               imul     ebx, dword ptr [si + 0xc]
009941:  66 C1 FB 0F                  sar      ebx, 0xf
009945:  66 0F AF 5C 14               imul     ebx, dword ptr [si + 0x14]
00994A:  66 03 C3                     add      eax, ebx
00994D:  66 C1 F8 0F                  sar      eax, 0xf
009951:  66 AB                        stosd    dword ptr es:[di], eax
009953:  66 8B 44 08                  mov      eax, dword ptr [si + 8]
009957:  66 0F AF 44 14               imul     eax, dword ptr [si + 0x14]
00995C:  66 F7 D8                     neg      eax
00995F:  66 C1 F8 0F                  sar      eax, 0xf
009963:  66 AB                        stosd    dword ptr es:[di], eax
009965:  66 8B 44 0C                  mov      eax, dword ptr [si + 0xc]
009969:  66 0F AF 04                  imul     eax, dword ptr [si]
00996D:  66 C1 F8 0F                  sar      eax, 0xf
009971:  66 0F AF 44 14               imul     eax, dword ptr [si + 0x14]
009976:  66 8B 5C 10                  mov      ebx, dword ptr [si + 0x10]
00997A:  66 0F AF 5C 04               imul     ebx, dword ptr [si + 4]
00997F:  66 2B C3                     sub      eax, ebx
009982:  66 C1 F8 0F                  sar      eax, 0xf
009986:  66 AB                        stosd    dword ptr es:[di], eax
009988:  66 8B 5C 14                  mov      ebx, dword ptr [si + 0x14]
00998C:  66 0F AF 1C                  imul     ebx, dword ptr [si]
009990:  66 8B 44 04                  mov      eax, dword ptr [si + 4]
009994:  66 0F AF 44 0C               imul     eax, dword ptr [si + 0xc]
009999:  66 C1 F8 0F                  sar      eax, 0xf
00999D:  66 0F AF 44 10               imul     eax, dword ptr [si + 0x10]
0099A2:  66 2B C3                     sub      eax, ebx
0099A5:  66 C1 F8 0F                  sar      eax, 0xf
0099A9:  66 AB                        stosd    dword ptr es:[di], eax
0099AB:  66 8B 44 08                  mov      eax, dword ptr [si + 8]
0099AF:  66 0F AF 44 10               imul     eax, dword ptr [si + 0x10]
0099B4:  66 C1 F8 0F                  sar      eax, 0xf
0099B8:  66 F7 D8                     neg      eax
0099BB:  66 AB                        stosd    dword ptr es:[di], eax
0099BD:  66 8B 44 04                  mov      eax, dword ptr [si + 4]
0099C1:  66 0F AF 44 14               imul     eax, dword ptr [si + 0x14]
0099C6:  66 8B 5C 0C                  mov      ebx, dword ptr [si + 0xc]
0099CA:  66 0F AF 1C                  imul     ebx, dword ptr [si]
0099CE:  66 C1 FB 0F                  sar      ebx, 0xf
0099D2:  66 0F AF 5C 10               imul     ebx, dword ptr [si + 0x10]
0099D7:  66 03 C3                     add      eax, ebx
0099DA:  66 C1 F8 0F                  sar      eax, 0xf
0099DE:  66 AB                        stosd    dword ptr es:[di], eax
0099E0:  66 8B 44 04                  mov      eax, dword ptr [si + 4]
0099E4:  66 0F AF 44 08               imul     eax, dword ptr [si + 8]
0099E9:  66 C1 F8 0F                  sar      eax, 0xf
0099ED:  66 AB                        stosd    dword ptr es:[di], eax
0099EF:  66 8B 44 0C                  mov      eax, dword ptr [si + 0xc]
0099F3:  66 AB                        stosd    dword ptr es:[di], eax
0099F5:  66 8B 44 08                  mov      eax, dword ptr [si + 8]
0099F9:  66 0F AF 04                  imul     eax, dword ptr [si]
0099FD:  66 C1 F8 0F                  sar      eax, 0xf
009A01:  66 AB                        stosd    dword ptr es:[di], eax
009A03:  5F                           pop      di
009A04:  5E                           pop      si
009A05:  07                           pop      es
009A06:  1F                           pop      ds
009A07:  66 5D                        pop      ebp
009A09:  66 59                        pop      ecx
009A0B:  66 5B                        pop      ebx
009A0D:  66 58                        pop      eax
009A0F:  CB                           retf    
