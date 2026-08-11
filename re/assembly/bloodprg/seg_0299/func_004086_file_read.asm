; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x004086
; seg_off: 0299:10f6
; group: seg_0299
; provenance: recursive_graph
; label: file_read
; label_comment: DOS file read: [0x5b55]=1; cx=2 (or size); ax=0x3f00; bx=di (handle); int21h (read from file into buffer). Reads bytes from an open resource file
; byte_count: 74
; boundary: cfg_blocks_4_terminals_2
; terminal: jmp 0x408c:1, ret:1
; direct_callees: none
; indirect_calls: 0
; cxx_source: re/borland/bloodprg/seg_0299/func_004086_file_read.cpp
; routine_bytes_sha256: cb4950819183084423bcb5c17e9ef1881b4096f05f6a81b2764726ff8221b9b7

004086:  66 50                        push     eax
004088:  53                           push     bx
004089:  51                           push     cx
00408A:  52                           push     dx
00408B:  56                           push     si
00408C:  C6 06 55 5B 01               mov      byte ptr [0x5b55], 1
004091:  B9 02 00                     mov      cx, 2
004094:  B8 00 3F                     mov      ax, 0x3f00
004097:  8B DF                        mov      bx, di
004099:  CD 21                        int      0x21
00409B:  8B DA                        mov      bx, dx
00409D:  8B 07                        mov      ax, word ptr [bx]
00409F:  66 4D                        dec      ebp
0040A1:  66 4D                        dec      ebp
0040A3:  83 F8 FF                     cmp      ax, -1
0040A6:  74 21                        je       0x40c9
0040A8:  52                           push     dx
0040A9:  8A DC                        mov      bl, ah
0040AB:  BA 51 52                     mov      dx, 0x5251
0040AE:  B7 03                        mov      bh, 3
0040B0:  F6 E7                        mul      bh
0040B2:  03 D0                        add      dx, ax
0040B4:  8A C7                        mov      al, bh
0040B6:  F6 E3                        mul      bl
0040B8:  66 98                        cwde    
0040BA:  66 2B E8                     sub      ebp, eax
0040BD:  8B C8                        mov      cx, ax
0040BF:  B8 00 3F                     mov      ax, 0x3f00
0040C2:  8B DF                        mov      bx, di
0040C4:  CD 21                        int      0x21
0040C6:  5A                           pop      dx
0040C7:  EB C3                        jmp      0x408c
0040C9:  5E                           pop      si
0040CA:  5A                           pop      dx
0040CB:  59                           pop      cx
0040CC:  5B                           pop      bx
0040CD:  66 58                        pop      eax
0040CF:  C3                           ret     
