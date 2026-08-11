; Commander Blood recovered routine assembly
; module: xdb_manu3
; artifact: output/_tmp_dat/manu3.xdb
; artifact_sha256: d0f64e99a646197906e273edfa0124172307a5cd766c88591c12ebd9ea556d31
; overlay_offset: 0x000700
; group: manu3_labeled
; provenance: label:manu3_face_bucket_sort, manu3 face bucket sorter
; label: manu3_face_bucket_sort
; label_comment: triangle setup: face records 8 bytes {next, v0ptr, v1ptr, v2ptr} from fs:[0x2300] (count fs:[0x2304]) — a per-frame WORK buffer @0xF137 (zeroed in file); clip = AND of vertex +0x12 flags; Y-sort the 3 verts (screen Y at vtx+0x0A); insert into the Y-BUCKET chain table @0x686 (2 bytes/scanline) for scanline rendering. SOURCE mesh + face builder = the stage between the transform loop and 0x700 (next trace)
; byte_count: 117
; boundary: cfg_blocks_14_terminals_1
; terminal: jmp 0x745:1
; direct_callees: none
; indirect_calls: 0
; cxx_source: re/borland/xdb/manu3/manu3_labeled/func_000700_manu3_face_bucket_sort.cpp
; routine_bytes_sha256: 7c332d7f4ed8cddf1dc6289e33919c57f0109bddda0a68301cf67eb912eaaa32

000700:  64 8B 0E 04 23               mov      cx, word ptr fs:[0x2304]
000705:  64 8B 36 00 23               mov      si, word ptr fs:[0x2300]
00070A:  8B 5C 02                     mov      bx, word ptr [si + 2]
00070D:  8B 7C 04                     mov      di, word ptr [si + 4]
000710:  8B 47 12                     mov      ax, word ptr [bx + 0x12]
000713:  8B 6C 06                     mov      bp, word ptr [si + 6]
000716:  23 45 12                     and      ax, word ptr [di + 0x12]
000719:  3E 23 46 12                  and      ax, word ptr ds:[bp + 0x12]
00071D:  75 51                        jne      0x770
00071F:  51                           push     cx
000720:  8B 47 0A                     mov      ax, word ptr [bx + 0xa]
000723:  8B 55 0A                     mov      dx, word ptr [di + 0xa]
000726:  3E 8B 4E 0A                  mov      cx, word ptr ds:[bp + 0xa]
00072A:  3B D1                        cmp      dx, cx
00072C:  7E 0D                        jle      0x73b
00072E:  3B C1                        cmp      ax, cx
000730:  7C 1C                        jl       0x74e
000732:  87 DD                        xchg     bp, bx
000734:  91                           xchg     cx, ax
000735:  87 FD                        xchg     bp, di
000737:  87 CA                        xchg     dx, cx
000739:  EB 0A                        jmp      0x745
00073B:  3B C2                        cmp      ax, dx
00073D:  7E 0F                        jle      0x74e
00073F:  87 DD                        xchg     bp, bx
000741:  91                           xchg     cx, ax
000742:  87 DF                        xchg     di, bx
000744:  92                           xchg     dx, ax
000745:  89 5C 02                     mov      word ptr [si + 2], bx
000748:  89 7C 04                     mov      word ptr [si + 4], di
00074B:  89 6C 06                     mov      word ptr [si + 6], bp
00074E:  2B D0                        sub      dx, ax
000750:  2B C8                        sub      cx, ax
000752:  81 FA 90 01                  cmp      dx, 0x190
000756:  73 17                        jae      0x76f
000758:  81 F9 90 01                  cmp      cx, 0x190
00075C:  73 11                        jae      0x76f
00075E:  03 C0                        add      ax, ax
000760:  BF 86 06                     mov      di, 0x686
000763:  78 02                        js       0x767
000765:  03 F8                        add      di, ax
000767:  26 8B 1D                     mov      bx, word ptr es:[di]
00076A:  26 89 35                     mov      word ptr es:[di], si
00076D:  89 1C                        mov      word ptr [si], bx
00076F:  59                           pop      cx
000770:  83 C6 08                     add      si, 8
000773:  E2 95                        loop     0x70a
