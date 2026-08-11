; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x00981b
; seg_off: 071e:203b
; group: seg_071e
; provenance: recursive_graph
; label: bridge_panorama_frame_load
; label_comment: TB.BIG frame loader (3 callers: 0x8ee4 console one-shot, 0x9599 page_flip rotate path, 0x95cf station-entry opaque redraw). AX=frame index -> seek [0xac4] (tb.big handle) to idx*8, read {offset:u32,size:u32} dir entry -> [0xad2]/[0xad6], seek+read chunk into buffer [0x5221]. Resets 4x0x18-stride station table gs:0x2a1b bboxes to -1, copies chunk's 8-byte bbox into entry picked by chunk word@+8, then lcall 0x1ce:0xa70 (unpack, ds:si=data+10). If gs:[0x5b53]&1 also refreshes palette 0x5b58->0x5251. Ported: src/tbbig.rs
; byte_count: 158
; boundary: cfg_blocks_3_terminals_1
; terminal: ret:1
; direct_callees: none
; indirect_calls: 1
; cxx_source: re/borland/bloodprg/seg_071e/func_00981b_bridge_panorama_frame_load.cpp
; routine_bytes_sha256: 15f5ca552b2f2c16e3836a20d958494931366820d788599c0d068a5b44c09910

00981B:  66 50                        push     eax
00981D:  56                           push     si
00981E:  57                           push     di
00981F:  06                           push     es
009820:  1E                           push     ds
009821:  51                           push     cx
009822:  52                           push     dx
009823:  53                           push     bx
009824:  66 55                        push     ebp
009826:  C1 E0 03                     shl      ax, 3
009829:  8B D0                        mov      dx, ax
00982B:  33 C9                        xor      cx, cx
00982D:  8B 1E C4 0A                  mov      bx, word ptr [0xac4]
009831:  B8 00 42                     mov      ax, 0x4200
009834:  CD 21                        int      0x21
009836:  B4 3F                        mov      ah, 0x3f
009838:  B9 08 00                     mov      cx, 8
00983B:  BA D2 0A                     mov      dx, 0xad2
00983E:  CD 21                        int      0x21
009840:  8B 16 D2 0A                  mov      dx, word ptr [0xad2]
009844:  8B 0E D4 0A                  mov      cx, word ptr [0xad4]
009848:  B8 00 42                     mov      ax, 0x4200
00984B:  CD 21                        int      0x21
00984D:  8B 0E D6 0A                  mov      cx, word ptr [0xad6]
009851:  B4 3F                        mov      ah, 0x3f
009853:  C5 16 21 52                  lds      dx, ptr [0x5221]
009857:  CD 21                        int      0x21
009859:  8B F2                        mov      si, dx
00985B:  8C E8                        mov      ax, gs
00985D:  8E C0                        mov      es, ax
00985F:  BF 1B 2A                     mov      di, 0x2a1b
009862:  B9 04 00                     mov      cx, 4
009865:  83 C7 0C                     add      di, 0xc
009868:  66 B8 FF FF FF FF            mov      eax, 0xffffffff
00986E:  66 AB                        stosd    dword ptr es:[di], eax
009870:  66 AB                        stosd    dword ptr es:[di], eax
009872:  83 C7 04                     add      di, 4
009875:  E2 EE                        loop     0x9865
009877:  BF 1B 2A                     mov      di, 0x2a1b
00987A:  8B 44 08                     mov      ax, word ptr [si + 8]
00987D:  BA 18 00                     mov      dx, 0x18
009880:  F7 E2                        mul      dx
009882:  83 C0 0C                     add      ax, 0xc
009885:  03 F8                        add      di, ax
009887:  66 A5                        movsd    dword ptr es:[di], dword ptr [si]
009889:  66 A5                        movsd    dword ptr es:[di], dword ptr [si]
00988B:  83 C6 02                     add      si, 2
00988E:  9A 70 0A CE 01               lcall    0x1ce, 0xa70
009893:  65 F6 06 53 5B 01            test     byte ptr gs:[0x5b53], 1
009899:  74 12                        je       0x98ad
00989B:  8C E8                        mov      ax, gs
00989D:  8E C0                        mov      es, ax
00989F:  8E D8                        mov      ds, ax
0098A1:  BE 58 5B                     mov      si, 0x5b58
0098A4:  BF 51 52                     mov      di, 0x5251
0098A7:  B9 C0 00                     mov      cx, 0xc0
0098AA:  F3 66 A5                     rep movsd dword ptr es:[di], dword ptr [si]
0098AD:  66 5D                        pop      ebp
0098AF:  5B                           pop      bx
0098B0:  5A                           pop      dx
0098B1:  59                           pop      cx
0098B2:  1F                           pop      ds
0098B3:  07                           pop      es
0098B4:  5F                           pop      di
0098B5:  5E                           pop      si
0098B6:  66 58                        pop      eax
0098B8:  C3                           ret     
