; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x009a10
; seg_off: 071e:2230
; group: seg_071e
; provenance: recursive_graph
; label: ship_3d_point_cloud_project
; label_comment: projects 1000 DS:0x2FC1 point-cloud records through DS:0x2F95 matrix and plots clipped depth shades
; byte_count: 244
; boundary: cfg_blocks_6_terminals_1
; terminal: retf:1
; direct_callees: 0x009b04
; indirect_calls: 0
; cxx_source: re/borland/bloodprg/seg_071e/func_009a10_ship_3d_point_cloud_project.cpp
; routine_bytes_sha256: 10a3734ef018c6766adeb2def5cb606e4af026d9ae3f26463eb2baf9c5cd45da

009A10:  66 50                        push     eax
009A12:  66 53                        push     ebx
009A14:  66 51                        push     ecx
009A16:  66 52                        push     edx
009A18:  55                           push     bp
009A19:  1E                           push     ds
009A1A:  57                           push     di
009A1B:  06                           push     es
009A1C:  56                           push     si
009A1D:  C7 06 77 2F E8 03            mov      word ptr [0x2f77], 0x3e8
009A23:  BE C1 2F                     mov      si, 0x2fc1
009A26:  BF 01 4F                     mov      di, 0x4f01
009A29:  8C E8                        mov      ax, gs
009A2B:  8E D8                        mov      ds, ax
009A2D:  8E 06 23 52                  mov      es, word ptr [0x5223]
009A31:  BD 95 2F                     mov      bp, 0x2f95
009A34:  66 AD                        lodsd    eax, dword ptr [si]
009A36:  66 89 05                     mov      dword ptr [di], eax
009A39:  66 AD                        lodsd    eax, dword ptr [si]
009A3B:  66 89 45 04                  mov      dword ptr [di + 4], eax
009A3F:  A1 65 2F                     mov      ax, word ptr [0x2f65]
009A42:  29 05                        sub      word ptr [di], ax
009A44:  A1 67 2F                     mov      ax, word ptr [0x2f67]
009A47:  29 45 02                     sub      word ptr [di + 2], ax
009A4A:  A1 69 2F                     mov      ax, word ptr [0x2f69]
009A4D:  29 45 04                     sub      word ptr [di + 4], ax
009A50:  66 0F BF 05                  movsx    eax, word ptr [di]
009A54:  66 0F AF 46 18               imul     eax, dword ptr [bp + 0x18]
009A59:  66 8B C8                     mov      ecx, eax
009A5C:  66 0F BF 45 02               movsx    eax, word ptr [di + 2]
009A61:  66 0F AF 46 1C               imul     eax, dword ptr [bp + 0x1c]
009A66:  66 03 C8                     add      ecx, eax
009A69:  66 0F BF 45 04               movsx    eax, word ptr [di + 4]
009A6E:  66 0F AF 46 20               imul     eax, dword ptr [bp + 0x20]
009A73:  66 03 C8                     add      ecx, eax
009A76:  66 C1 F9 0F                  sar      ecx, 0xf
009A7A:  74 72                        je       0x9aee
009A7C:  78 70                        js       0x9aee
009A7E:  66 0F BF 05                  movsx    eax, word ptr [di]
009A82:  66 0F AF 46 00               imul     eax, dword ptr [bp]
009A87:  66 8B D8                     mov      ebx, eax
009A8A:  66 0F BF 45 02               movsx    eax, word ptr [di + 2]
009A8F:  66 0F AF 46 04               imul     eax, dword ptr [bp + 4]
009A94:  66 03 D8                     add      ebx, eax
009A97:  66 0F BF 45 04               movsx    eax, word ptr [di + 4]
009A9C:  66 0F AF 46 08               imul     eax, dword ptr [bp + 8]
009AA1:  66 03 C3                     add      eax, ebx
009AA4:  66 C1 F8 07                  sar      eax, 7
009AA8:  66 99                        cdq     
009AAA:  66 F7 F9                     idiv     ecx
009AAD:  05 A0 00                     add      ax, 0xa0
009AB0:  89 46 24                     mov      word ptr [bp + 0x24], ax
009AB3:  66 0F BF 05                  movsx    eax, word ptr [di]
009AB7:  66 0F AF 46 0C               imul     eax, dword ptr [bp + 0xc]
009ABC:  66 8B D8                     mov      ebx, eax
009ABF:  66 0F BF 45 02               movsx    eax, word ptr [di + 2]
009AC4:  66 0F AF 46 10               imul     eax, dword ptr [bp + 0x10]
009AC9:  66 03 D8                     add      ebx, eax
009ACC:  66 0F BF 45 04               movsx    eax, word ptr [di + 4]
009AD1:  66 0F AF 46 14               imul     eax, dword ptr [bp + 0x14]
009AD6:  66 03 C3                     add      eax, ebx
009AD9:  66 C1 F8 07                  sar      eax, 7
009ADD:  66 99                        cdq     
009ADF:  66 F7 F9                     idiv     ecx
009AE2:  83 C0 64                     add      ax, 0x64
009AE5:  89 46 26                     mov      word ptr [bp + 0x26], ax
009AE8:  89 4E 28                     mov      word ptr [bp + 0x28], cx
009AEB:  E8 16 00                     call     0x9b04
009AEE:  FF 0E 77 2F                  dec      word ptr [0x2f77]
009AF2:  0F 85 3E FF                  jne      0x9a34
009AF6:  5E                           pop      si
009AF7:  07                           pop      es
009AF8:  5F                           pop      di
009AF9:  1F                           pop      ds
009AFA:  5D                           pop      bp
009AFB:  66 5A                        pop      edx
009AFD:  66 59                        pop      ecx
009AFF:  66 5B                        pop      ebx
009B01:  66 58                        pop      eax
009B03:  CB                           retf    
