; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x003fc7
; seg_off: 0299:1037
; group: seg_0299
; provenance: recursive_graph, relocation_proven_far_transfer_target
; label: resource_name_table_access
; label_comment: resource name-table access: ds=fs; dx=0xc04 -> FS:0x0c04, the 16-byte-record resource NAME TABLE (worlds are IDs 22-36). Looks up a resource descriptor by id
; incoming: call@0x000fb9->0299:1037
; incoming: call@0x00597f->0299:1037
; incoming: call@0x0070cd->0299:1037
; incoming: call@0x0090c3->0299:1037
; byte_count: 191
; boundary: cfg_blocks_14_terminals_3
; terminal: jmp 0x4041:1, jmp 0x407c:1, retf:1
; direct_callees: 0x004086
; indirect_calls: 2
; routine_bytes_sha256: 2438dcc40d27a1d4699e0b945a5ee3f3e48e98f6965a36956a6efb76e4e0869c

003FC7:  53                           push     bx
003FC8:  51                           push     cx
003FC9:  52                           push     dx
003FCA:  1E                           push     ds
003FCB:  56                           push     si
003FCC:  57                           push     di
003FCD:  06                           push     es
003FCE:  66 55                        push     ebp
003FD0:  8C E3                        mov      bx, fs
003FD2:  8E DB                        mov      ds, bx
003FD4:  BA 04 0C                     mov      dx, 0xc04
003FD7:  8B F0                        mov      si, ax
003FD9:  C1 E0 04                     shl      ax, 4
003FDC:  03 D0                        add      dx, ax
003FDE:  06                           push     es
003FDF:  9A B3 03 CE 01               lcall    0x1ce, 0x3b3
003FE4:  B8 00 2F                     mov      ax, 0x2f00
003FE7:  CD 21                        int      0x21
003FE9:  33 C9                        xor      cx, cx
003FEB:  B8 00 4E                     mov      ax, 0x4e00
003FEE:  CD 21                        int      0x21
003FF0:  0F 82 82 00                  jb       0x4076
003FF4:  66 26 8B 6F 1A               mov      ebp, dword ptr es:[bx + 0x1a]
003FF9:  B8 00 3D                     mov      ax, 0x3d00
003FFC:  CD 21                        int      0x21
003FFE:  72 76                        jb       0x4076
004000:  8E C3                        mov      es, bx
004002:  57                           push     di
004003:  8B F8                        mov      di, ax
004005:  8C E8                        mov      ax, gs
004007:  8E D8                        mov      ds, ax
004009:  BA F2 0A                     mov      dx, 0xaf2
00400C:  B8 00 3F                     mov      ax, 0x3f00
00400F:  8B DF                        mov      bx, di
004011:  B9 02 00                     mov      cx, 2
004014:  CD 21                        int      0x21
004016:  8B DA                        mov      bx, dx
004018:  8B 1F                        mov      bx, word ptr [bx]
00401A:  F7 C3 02 00                  test     bx, 2
00401E:  74 03                        je       0x4023
004020:  E8 63 00                     call     0x4086
004023:  8B C6                        mov      ax, si
004025:  5E                           pop      si
004026:  1F                           pop      ds
004027:  0B C0                        or       ax, ax
004029:  78 0B                        js       0x4036
00402B:  9A 00 00 B9 04               lcall    0x4b9, 0
004030:  0B C0                        or       ax, ax
004032:  78 3B                        js       0x406f
004034:  75 2E                        jne      0x4064
004036:  66 4D                        dec      ebp
004038:  66 4D                        dec      ebp
00403A:  8B D6                        mov      dx, si
00403C:  89 1C                        mov      word ptr [si], bx
00403E:  83 C2 02                     add      dx, 2
004041:  B9 00 7D                     mov      cx, 0x7d00
004044:  8B DF                        mov      bx, di
004046:  B8 00 3F                     mov      ax, 0x3f00
004049:  CD 21                        int      0x21
00404B:  66 98                        cwde    
00404D:  66 2B E8                     sub      ebp, eax
004050:  74 12                        je       0x4064
004052:  8B D8                        mov      bx, ax
004054:  C1 EB 04                     shr      bx, 4
004057:  83 E0 0F                     and      ax, 0xf
00405A:  8C D9                        mov      cx, ds
00405C:  03 CB                        add      cx, bx
00405E:  8E D9                        mov      ds, cx
004060:  03 D0                        add      dx, ax
004062:  EB DD                        jmp      0x4041
004064:  B8 00 3E                     mov      ax, 0x3e00
004067:  8B DF                        mov      bx, di
004069:  CD 21                        int      0x21
00406B:  33 C0                        xor      ax, ax
00406D:  EB 0D                        jmp      0x407c
00406F:  B8 00 3E                     mov      ax, 0x3e00
004072:  8B DF                        mov      bx, di
004074:  CD 21                        int      0x21
004076:  83 C4 02                     add      sp, 2
004079:  B8 FF FF                     mov      ax, 0xffff
00407C:  66 5D                        pop      ebp
00407E:  07                           pop      es
00407F:  5F                           pop      di
004080:  5E                           pop      si
004081:  1F                           pop      ds
004082:  5A                           pop      dx
004083:  59                           pop      cx
004084:  5B                           pop      bx
004085:  CB                           retf    
