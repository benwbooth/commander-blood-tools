; Commander Blood recovered routine assembly
; module: xdb_manu3
; artifact: output/_tmp_dat/manu3.xdb
; artifact_sha256: d0f64e99a646197906e273edfa0124172307a5cd766c88591c12ebd9ea556d31
; overlay_offset: 0x000849
; group: manu3_labeled
; provenance: label:manu3_span_insert, manu3 span insertion routine
; label: manu3_span_insert
; label_comment: depth-sorted span insert: spans linked by {next[0], prev[0x10]}, ordered by depth dword +8 (sentinels 0x964/0xA18); column cursor [0x680] 0..0x140 walks columns; [0x684] = per-column face-bucket cursor; faces pop at 0x8B1 -> triangle->edge/gradient conversion 0x8B1..0xBE6 (the u/v gradient math, next slices)
; byte_count: 1153
; boundary: cfg_blocks_55_terminals_11
; terminal: jmp 0x884:1, jmp 0x950:1, jmp 0x96a:5, jmp 0xa30:1, jmp 0xa83:1, jmp word ptr [0x67e]:1, jmp word ptr [si + 0x2c]:1
; direct_callees: 0x000d7d
; indirect_calls: 0
; cxx_source: re/borland/xdb/manu3/manu3_labeled/func_000849_manu3_span_insert.cpp
; routine_bytes_sha256: 3d75eed4836942d2d76a5388132ce1bcf995856babb604bcac2c9d5ff58c5f3b

000849:  56                           push     si
00084A:  8B 1D                        mov      bx, word ptr [di]
00084C:  89 1C                        mov      word ptr [si], bx
00084E:  89 77 10                     mov      word ptr [bx + 0x10], si
000851:  66 8B 45 08                  mov      eax, dword ptr [di + 8]
000855:  8B 74 10                     mov      si, word ptr [si + 0x10]
000858:  81 FE 64 09                  cmp      si, 0x964
00085C:  74 06                        je       0x864
00085E:  66 3B 44 08                  cmp      eax, dword ptr [si + 8]
000862:  7C F1                        jl       0x855
000864:  8B 1C                        mov      bx, word ptr [si]
000866:  89 3C                        mov      word ptr [si], di
000868:  89 1D                        mov      word ptr [di], bx
00086A:  89 75 10                     mov      word ptr [di + 0x10], si
00086D:  89 7F 10                     mov      word ptr [bx + 0x10], di
000870:  5E                           pop      si
000871:  EB 11                        jmp      0x884
; -- non-contiguous block: next 0x000884 --
000884:  8B 3C                        mov      di, word ptr [si]
000886:  81 FF 18 0A                  cmp      di, 0xa18
00088A:  74 18                        je       0x8a4
00088C:  66 8B 44 08                  mov      eax, dword ptr [si + 8]
000890:  66 8B 4C 18                  mov      ecx, dword ptr [si + 0x18]
000894:  66 3B 45 08                  cmp      eax, dword ptr [di + 8]
000898:  7F AF                        jg       0x849
00089A:  8B F7                        mov      si, di
00089C:  8B 3D                        mov      di, word ptr [di]
00089E:  81 FF 18 0A                  cmp      di, 0xa18
0008A2:  75 E8                        jne      0x88c
0008A4:  8B 3E 84 06                  mov      di, word ptr [0x684]
0008A8:  83 C7 02                     add      di, 2
0008AB:  89 3E 84 06                  mov      word ptr [0x684], di
0008AF:  8B 35                        mov      si, word ptr [di]
0008B1:  0B F6                        or       si, si
0008B3:  74 1C                        je       0x8d1
0008B5:  C7 05 00 00                  mov      word ptr [di], 0
0008B9:  F7 06 08 09 FF FF            test     word ptr [0x908], 0xffff
0008BF:  74 10                        je       0x8d1
0008C1:  64 8E 06 02 00               mov      es, word ptr fs:[2]
0008C6:  26 FF 34                     push     word ptr es:[si]
0008C9:  E8 B1 04                     call     0xd7d
0008CC:  5E                           pop      si
0008CD:  0B F6                        or       si, si
0008CF:  75 F5                        jne      0x8c6
0008D1:  BE 64 09                     mov      si, 0x964
0008D4:  8B 04                        mov      ax, word ptr [si]
0008D6:  3D BE 09                     cmp      ax, 0x9be
0008D9:  0F 84 B4 03                  je       0xc91
0008DD:  BA 74 09                     mov      dx, 0x974
0008E0:  C7 44 02 01 00               mov      word ptr [si + 2], 1
0008E5:  89 54 06                     mov      word ptr [si + 6], dx
0008E8:  8B FE                        mov      di, si
0008EA:  8B EE                        mov      bp, si
0008EC:  33 DB                        xor      bx, bx
0008EE:  8B 3D                        mov      di, word ptr [di]
0008F0:  F7 45 1A 00 80               test     word ptr [di + 0x1a], 0x8000
0008F5:  75 F7                        jne      0x8ee
0008F7:  89 5C 58                     mov      word ptr [si + 0x58], bx
0008FA:  89 5D 58                     mov      word ptr [di + 0x58], bx
0008FD:  3B 5D 0A                     cmp      bx, word ptr [di + 0xa]
000900:  0F 8E A1 00                  jle      0x9a5
000904:  8B F7                        mov      si, di
000906:  8B EA                        mov      bp, dx
000908:  89 3E 62 09                  mov      word ptr [0x962], di
00090C:  66 0F B7 45 0A               movzx    eax, word ptr [di + 0xa]
000911:  F7 D8                        neg      ax
000913:  66 F7 6D 28                  imul     dword ptr [di + 0x28]
000917:  66 03 45 20                  add      eax, dword ptr [di + 0x20]
00091B:  66 89 45 04                  mov      dword ptr [di + 4], eax
00091F:  EB 2F                        jmp      0x950
000921:  BB 0A 09                     mov      bx, 0x90a
000924:  66 0F B7 45 0A               movzx    eax, word ptr [di + 0xa]
000929:  F7 D8                        neg      ax
00092B:  66 F7 6D 28                  imul     dword ptr [di + 0x28]
00092F:  66 03 45 20                  add      eax, dword ptr [di + 0x20]
000933:  66 89 45 04                  mov      dword ptr [di + 4], eax
000937:  66 3B 44 04                  cmp      eax, dword ptr [si + 4]
00093B:  7E 09                        jle      0x946
00093D:  8B DE                        mov      bx, si
00093F:  8B 74 58                     mov      si, word ptr [si + 0x58]
000942:  0B F6                        or       si, si
000944:  75 F1                        jne      0x937
000946:  89 7F 58                     mov      word ptr [bx + 0x58], di
000949:  89 75 58                     mov      word ptr [di + 0x58], si
00094C:  8B 36 62 09                  mov      si, word ptr [0x962]
000950:  8B 3D                        mov      di, word ptr [di]
000952:  F7 45 1A 00 80               test     word ptr [di + 0x1a], 0x8000
000957:  75 F7                        jne      0x950
000959:  F7 45 0A 00 80               test     word ptr [di + 0xa], 0x8000
00095E:  75 C1                        jne      0x921
000960:  3E C7 46 02 00 00            mov      word ptr ds:[bp + 2], 0
000966:  3E 89 76 04                  mov      word ptr ds:[bp + 4], si
00096A:  3E 8B 4E 0A                  mov      cx, word ptr ds:[bp + 0xa]
00096E:  8B 44 1A                     mov      ax, word ptr [si + 0x1a]
000971:  8B 55 0A                     mov      dx, word ptr [di + 0xa]
000974:  3B 55 1A                     cmp      dx, word ptr [di + 0x1a]
000977:  7D 55                        jge      0x9ce
000979:  3B C2                        cmp      ax, dx
00097B:  0F 8F B1 00                  jg       0xa30
00097F:  74 72                        je       0x9f3
000981:  8D 54 10                     lea      dx, [si + 0x10]
000984:  8B 4C 1A                     mov      cx, word ptr [si + 0x1a]
000987:  3E 89 56 06                  mov      word ptr ds:[bp + 6], dx
00098B:  8B EA                        mov      bp, dx
00098D:  8B 74 58                     mov      si, word ptr [si + 0x58]
000990:  0B F6                        or       si, si
000992:  74 11                        je       0x9a5
000994:  3B 4C 1A                     cmp      cx, word ptr [si + 0x1a]
000997:  7D F4                        jge      0x98d
000999:  3E C7 46 02 00 00            mov      word ptr ds:[bp + 2], 0
00099F:  3E 89 76 04                  mov      word ptr ds:[bp + 4], si
0009A3:  EB C5                        jmp      0x96a
0009A5:  81 FF BE 09                  cmp      di, 0x9be
0009A9:  0F 84 1C 02                  je       0xbc9
0009AD:  3E C7 46 02 01 00            mov      word ptr ds:[bp + 2], 1
0009B3:  3E 89 7E 06                  mov      word ptr ds:[bp + 6], di
0009B7:  8B EF                        mov      bp, di
0009B9:  C7 45 58 00 00               mov      word ptr [di + 0x58], 0
0009BE:  8B F7                        mov      si, di
0009C0:  3E C7 46 02 00 00            mov      word ptr ds:[bp + 2], 0
0009C6:  3E 89 7E 04                  mov      word ptr ds:[bp + 4], di
0009CA:  8B 3D                        mov      di, word ptr [di]
0009CC:  EB 9C                        jmp      0x96a
0009CE:  8B 3D                        mov      di, word ptr [di]
0009D0:  EB 98                        jmp      0x96a
; -- non-contiguous block: next 0x0009f3 --
0009F3:  3E 89 7E 06                  mov      word ptr ds:[bp + 6], di
0009F7:  8B 4D 0A                     mov      cx, word ptr [di + 0xa]
0009FA:  8B EF                        mov      bp, di
0009FC:  81 FF BE 09                  cmp      di, 0x9be
000A00:  0F 84 C5 01                  je       0xbc9
000A04:  8B 74 58                     mov      si, word ptr [si + 0x58]
000A07:  0B F6                        or       si, si
000A09:  74 11                        je       0xa1c
000A0B:  3B 4C 1A                     cmp      cx, word ptr [si + 0x1a]
000A0E:  7D F4                        jge      0xa04
000A10:  3E C7 46 02 00 00            mov      word ptr ds:[bp + 2], 0
000A16:  3E 89 76 04                  mov      word ptr ds:[bp + 4], si
000A1A:  EB 14                        jmp      0xa30
000A1C:  89 75 58                     mov      word ptr [di + 0x58], si
000A1F:  8B F7                        mov      si, di
000A21:  3E C7 46 02 00 00            mov      word ptr ds:[bp + 2], 0
000A27:  3E 89 7E 04                  mov      word ptr ds:[bp + 4], di
000A2B:  8B 3D                        mov      di, word ptr [di]
000A2D:  E9 3A FF                     jmp      0x96a
000A30:  81 FF BE 09                  cmp      di, 0x9be
000A34:  0F 84 8B 01                  je       0xbc3
000A38:  8B 55 0A                     mov      dx, word ptr [di + 0xa]
000A3B:  3B 55 1A                     cmp      dx, word ptr [di + 0x1a]
000A3E:  7D 49                        jge      0xa89
000A40:  3E 8B 4E 0A                  mov      cx, word ptr ds:[bp + 0xa]
000A44:  BB 0A 09                     mov      bx, 0x90a
000A47:  89 77 58                     mov      word ptr [bx + 0x58], si
000A4A:  66 8B 4D 08                  mov      ecx, dword ptr [di + 8]
000A4E:  66 3B 4C 18                  cmp      ecx, dword ptr [si + 0x18]
000A52:  7C 0C                        jl       0xa60
000A54:  8B 74 58                     mov      si, word ptr [si + 0x58]
000A57:  0B F6                        or       si, si
000A59:  89 77 58                     mov      word ptr [bx + 0x58], si
000A5C:  75 F0                        jne      0xa4e
000A5E:  EB 23                        jmp      0xa83
000A60:  66 8B C1                     mov      eax, ecx
000A63:  66 2B 44 08                  sub      eax, dword ptr [si + 8]
000A67:  66 F7 6C 28                  imul     dword ptr [si + 0x28]
000A6B:  66 0F AC D0 10               shrd     eax, edx, 0x10
000A70:  66 03 44 20                  add      eax, dword ptr [si + 0x20]
000A74:  66 3B 45 20                  cmp      eax, dword ptr [di + 0x20]
000A78:  7D 09                        jge      0xa83
000A7A:  8B DE                        mov      bx, si
000A7C:  8B 74 58                     mov      si, word ptr [si + 0x58]
000A7F:  0B F6                        or       si, si
000A81:  75 CB                        jne      0xa4e
000A83:  89 7F 58                     mov      word ptr [bx + 0x58], di
000A86:  89 75 58                     mov      word ptr [di + 0x58], si
000A89:  8B 36 62 09                  mov      si, word ptr [0x962]
000A8D:  3B F7                        cmp      si, di
000A8F:  75 0E                        jne      0xa9f
000A91:  3E 89 7E 06                  mov      word ptr ds:[bp + 6], di
000A95:  8B EF                        mov      bp, di
000A97:  C7 45 02 00 00               mov      word ptr [di + 2], 0
000A9C:  89 7D 04                     mov      word ptr [di + 4], di
000A9F:  8B 3D                        mov      di, word ptr [di]
000AA1:  E9 C6 FE                     jmp      0x96a
; -- non-contiguous block: next 0x000bc3 --
000BC3:  3E 89 7E 06                  mov      word ptr ds:[bp + 6], di
000BC7:  8B EF                        mov      bp, di
000BC9:  3E C7 46 02 00 80            mov      word ptr ds:[bp + 2], 0x8000
000BCF:  BB 64 09                     mov      bx, 0x964
000BD2:  FF 26 7E 06                  jmp      word ptr [0x67e]
; -- non-contiguous block: next 0x000c91 --
000C91:  8B 36 64 09                  mov      si, word ptr [0x964]
000C95:  FF 4C 2E                     dec      word ptr [si + 0x2e]
000C98:  78 2D                        js       0xcc7
000C9A:  8B 44 4A                     mov      ax, word ptr [si + 0x4a]
000C9D:  8B 5C 4C                     mov      bx, word ptr [si + 0x4c]
000CA0:  66 8B 4C 0C                  mov      ecx, dword ptr [si + 0xc]
000CA4:  66 8B 54 24                  mov      edx, dword ptr [si + 0x24]
000CA8:  01 44 42                     add      word ptr [si + 0x42], ax
000CAB:  01 5C 44                     add      word ptr [si + 0x44], bx
000CAE:  66 01 4C 08                  add      dword ptr [si + 8], ecx
000CB2:  66 01 54 20                  add      dword ptr [si + 0x20], edx
000CB6:  8B DE                        mov      bx, si
000CB8:  66 8B 4C 1C                  mov      ecx, dword ptr [si + 0x1c]
000CBC:  8B 37                        mov      si, word ptr [bx]
000CBE:  66 01 4F 18                  add      dword ptr [bx + 0x18], ecx
000CC2:  FF 4C 2E                     dec      word ptr [si + 0x2e]
000CC5:  79 D3                        jns      0xc9a
000CC7:  FF 64 2C                     jmp      word ptr [si + 0x2c]
