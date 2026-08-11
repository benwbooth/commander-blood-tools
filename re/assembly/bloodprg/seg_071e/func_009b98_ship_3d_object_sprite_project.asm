; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x009b98
; seg_off: 071e:23b8
; group: seg_071e
; provenance: recursive_graph
; label: ship_3d_object_sprite_project
; label_comment: NAV-DESTINATION PROJECTOR - FULL decode. Loop counter gs:[0x2F77] runs 0x0B-1 down to 0 (11 iterations); entity = 0x6212+((i+0x15)<<5) so entities 0x15..0x1F. GATE: test word [si],0x80 = the entity flags bit7 (active) - inactive entities are skipped. POSITION SOURCE: bx starts at DS:0x4F09 and advances by 6 each iteration (add bx,6 @0x9CF5), i.e. an 11-entry table of THREE int16 (x,y,z), stride 6 - CORRECTION: 0x4F09 is the projector INPUT table, NOT per-frame scratch (0x4F01 is the 8-byte working copy it copies into). Camera origin gs:0x2F65/0x2F67/0x2F69 is subtracted. depth=(x*m[0x18]+y*m[0x1C]+z*m[0x20])>>15, skip if 0, +0x10000 if negative; scale=0x100000/depth -> [bp+0x2A]. screen_x=((x*m[0]+y*m[4]+z*m[8])>>7)/depth+0xA0(160); screen_y=((x*m[0x0C]+y*m[0x10]+z*m[0x14])>>7)/depth+0x64(100). Sprite dims: the far ptr at [si+4] gives w/h, each *scale then shrd 10 (>>10) - matches the port dim*depth_scale>>10. Draw at (screen_x-[si+0xC]/2, screen_y-[si+0xE]/2). BAKED table @file 0x12329 = 11 x (10200,12100,900) IDENTICAL, and the literal 0x4F09 is referenced ONLY here, so the per-destination positions must be written through a POINTER at runtime - locating that writer is the remaining task. PORT GAP: engine.rs render_nav_pyramid_sprites fabricates a 7x4=28-point grid (ROW_Z 600/1500/3000/5600, xi*700, CAMERA_DEPTH_BIAS 8804) - wrong in COUNT (should be 11), GATE (should be entity active bit7) and POSITION SOURCE
; byte_count: 369
; boundary: cfg_blocks_9_terminals_2
; terminal: jmp 0x9bba:1, retf:1
; direct_callees: none
; indirect_calls: 2
; cxx_source: re/borland/bloodprg/seg_071e/func_009b98_ship_3d_object_sprite_project.cpp
; routine_bytes_sha256: 4c1e816863fe14d2f7835c89e6d0692195f55abacb012b3c7db8e1338a165051

009B98:  66 50                        push     eax
009B9A:  66 53                        push     ebx
009B9C:  66 51                        push     ecx
009B9E:  66 52                        push     edx
009BA0:  55                           push     bp
009BA1:  1E                           push     ds
009BA2:  57                           push     di
009BA3:  06                           push     es
009BA4:  56                           push     si
009BA5:  BB 09 4F                     mov      bx, 0x4f09
009BA8:  BF 01 4F                     mov      di, 0x4f01
009BAB:  8C E8                        mov      ax, gs
009BAD:  8E D8                        mov      ds, ax
009BAF:  8E C0                        mov      es, ax
009BB1:  BD 95 2F                     mov      bp, 0x2f95
009BB4:  C7 06 77 2F 0B 00            mov      word ptr [0x2f77], 0xb
009BBA:  FF 0E 77 2F                  dec      word ptr [0x2f77]
009BBE:  0F 88 39 01                  js       0x9cfb
009BC2:  66 8B 07                     mov      eax, dword ptr [bx]
009BC5:  66 89 05                     mov      dword ptr [di], eax
009BC8:  66 8B 47 04                  mov      eax, dword ptr [bx + 4]
009BCC:  66 89 45 04                  mov      dword ptr [di + 4], eax
009BD0:  53                           push     bx
009BD1:  A1 77 2F                     mov      ax, word ptr [0x2f77]
009BD4:  83 C0 15                     add      ax, 0x15
009BD7:  C1 E0 05                     shl      ax, 5
009BDA:  05 12 62                     add      ax, 0x6212
009BDD:  8B F0                        mov      si, ax
009BDF:  8B 04                        mov      ax, word ptr [si]
009BE1:  A9 80 00                     test     ax, 0x80
009BE4:  0F 84 0C 01                  je       0x9cf4
009BE8:  A1 65 2F                     mov      ax, word ptr [0x2f65]
009BEB:  29 05                        sub      word ptr [di], ax
009BED:  A1 67 2F                     mov      ax, word ptr [0x2f67]
009BF0:  29 45 02                     sub      word ptr [di + 2], ax
009BF3:  A1 69 2F                     mov      ax, word ptr [0x2f69]
009BF6:  29 45 04                     sub      word ptr [di + 4], ax
009BF9:  66 0F BF 05                  movsx    eax, word ptr [di]
009BFD:  66 0F AF 46 18               imul     eax, dword ptr [bp + 0x18]
009C02:  66 8B C8                     mov      ecx, eax
009C05:  66 0F BF 45 02               movsx    eax, word ptr [di + 2]
009C0A:  66 0F AF 46 1C               imul     eax, dword ptr [bp + 0x1c]
009C0F:  66 03 C8                     add      ecx, eax
009C12:  66 0F BF 45 04               movsx    eax, word ptr [di + 4]
009C17:  66 0F AF 46 20               imul     eax, dword ptr [bp + 0x20]
009C1C:  66 03 C8                     add      ecx, eax
009C1F:  66 C1 F9 0F                  sar      ecx, 0xf
009C23:  0F 84 CD 00                  je       0x9cf4
009C27:  79 07                        jns      0x9c30
009C29:  66 81 C1 00 00 01 00         add      ecx, 0x10000
009C30:  66 B8 00 00 00 08            mov      eax, 0x8000000
009C36:  66 C1 E8 07                  shr      eax, 7
009C3A:  66 33 D2                     xor      edx, edx
009C3D:  66 F7 F1                     div      ecx
009C40:  89 46 2A                     mov      word ptr [bp + 0x2a], ax
009C43:  66 0F BF 05                  movsx    eax, word ptr [di]
009C47:  66 0F AF 46 00               imul     eax, dword ptr [bp]
009C4C:  66 8B D8                     mov      ebx, eax
009C4F:  66 0F BF 45 02               movsx    eax, word ptr [di + 2]
009C54:  66 0F AF 46 04               imul     eax, dword ptr [bp + 4]
009C59:  66 03 D8                     add      ebx, eax
009C5C:  66 0F BF 45 04               movsx    eax, word ptr [di + 4]
009C61:  66 0F AF 46 08               imul     eax, dword ptr [bp + 8]
009C66:  66 03 C3                     add      eax, ebx
009C69:  66 C1 F8 07                  sar      eax, 7
009C6D:  66 99                        cdq     
009C6F:  66 F7 F9                     idiv     ecx
009C72:  05 A0 00                     add      ax, 0xa0
009C75:  89 46 24                     mov      word ptr [bp + 0x24], ax
009C78:  66 0F BF 05                  movsx    eax, word ptr [di]
009C7C:  66 0F AF 46 0C               imul     eax, dword ptr [bp + 0xc]
009C81:  66 8B D8                     mov      ebx, eax
009C84:  66 0F BF 45 02               movsx    eax, word ptr [di + 2]
009C89:  66 0F AF 46 10               imul     eax, dword ptr [bp + 0x10]
009C8E:  66 03 D8                     add      ebx, eax
009C91:  66 0F BF 45 04               movsx    eax, word ptr [di + 4]
009C96:  66 0F AF 46 14               imul     eax, dword ptr [bp + 0x14]
009C9B:  66 03 C3                     add      eax, ebx
009C9E:  66 C1 F8 07                  sar      eax, 7
009CA2:  66 99                        cdq     
009CA4:  66 F7 F9                     idiv     ecx
009CA7:  83 C0 64                     add      ax, 0x64
009CAA:  89 46 26                     mov      word ptr [bp + 0x26], ax
009CAD:  89 4E 28                     mov      word ptr [bp + 0x28], cx
009CB0:  06                           push     es
009CB1:  57                           push     di
009CB2:  C4 7C 04                     les      di, ptr [si + 4]
009CB5:  26 8B 05                     mov      ax, word ptr es:[di]
009CB8:  F7 66 2A                     mul      word ptr [bp + 0x2a]
009CBB:  0F AC D0 0A                  shrd     ax, dx, 0xa
009CBF:  8B C8                        mov      cx, ax
009CC1:  26 8B 45 02                  mov      ax, word ptr es:[di + 2]
009CC5:  F7 66 2A                     mul      word ptr [bp + 0x2a]
009CC8:  0F AC D0 0A                  shrd     ax, dx, 0xa
009CCC:  8B D0                        mov      dx, ax
009CCE:  5F                           pop      di
009CCF:  07                           pop      es
009CD0:  A1 77 2F                     mov      ax, word ptr [0x2f77]
009CD3:  83 C0 15                     add      ax, 0x15
009CD6:  9A 3D 13 99 02               lcall    0x299, 0x133d
009CDB:  8B 5E 24                     mov      bx, word ptr [bp + 0x24]
009CDE:  8B 54 0C                     mov      dx, word ptr [si + 0xc]
009CE1:  D1 EA                        shr      dx, 1
009CE3:  2B DA                        sub      bx, dx
009CE5:  8B 4E 26                     mov      cx, word ptr [bp + 0x26]
009CE8:  8B 54 0E                     mov      dx, word ptr [si + 0xe]
009CEB:  D1 EA                        shr      dx, 1
009CED:  2B CA                        sub      cx, dx
009CEF:  9A 7D 12 99 02               lcall    0x299, 0x127d
009CF4:  5B                           pop      bx
009CF5:  83 C3 06                     add      bx, 6
009CF8:  E9 BF FE                     jmp      0x9bba
009CFB:  5E                           pop      si
009CFC:  07                           pop      es
009CFD:  5F                           pop      di
009CFE:  1F                           pop      ds
009CFF:  5D                           pop      bp
009D00:  66 5A                        pop      edx
009D02:  66 59                        pop      ecx
009D04:  66 5B                        pop      ebx
009D06:  66 58                        pop      eax
009D08:  CB                           retf    
