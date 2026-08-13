; Commander Blood recovered routine assembly
; module: xdb_manu3
; artifact: output/_tmp_dat/manu3.xdb
; artifact_sha256: d0f64e99a646197906e273edfa0124172307a5cd766c88591c12ebd9ea556d31
; overlay_offset: 0x000700
; group: manu3_labeled
; provenance: label:manu3_face_bucket_sort, internal_label:manu3_span_renderer_init, internal_label:manu3_span_setup_next, internal_label:manu3_span_insert, internal_label:manu3_affine_fill, manu3 face bucket sort and column renderer
; label: manu3_face_bucket_sort
; label_comment: complete column-oriented textured triangle renderer: bucket faces by minimum screen X, allocate and activate raster records, construct depth-ordered vertical span boundaries, draw Mode-X or linear framebuffer columns, advance secondary edges, free expired records, and reorder crossings through column 319
; byte_count: 1661
; boundary: cfg_blocks_117_terminals_1
; terminal: ret:1
; direct_callees: 0x000d7d
; indirect_calls: 0
; routine_bytes_sha256: a687e5cd5b80445d293f096ee73c2952ad61052dc1613636d24d53fdc1484161

000700:  64 8B 0E 04 23                mov      cx, word ptr fs:[0x2304]
000705:  64 8B 36 00 23                mov      si, word ptr fs:[0x2300]
00070A:  8B 5C 02                      mov      bx, word ptr [si + 2]
00070D:  8B 7C 04                      mov      di, word ptr [si + 4]
000710:  8B 47 12                      mov      ax, word ptr [bx + 0x12]
000713:  8B 6C 06                      mov      bp, word ptr [si + 6]
000716:  23 45 12                      and      ax, word ptr [di + 0x12]
000719:  3E 23 46 12                   and      ax, word ptr ds:[bp + 0x12]
00071D:  75 51                         jne      0x770
00071F:  51                            push     cx
000720:  8B 47 0A                      mov      ax, word ptr [bx + 0xa]
000723:  8B 55 0A                      mov      dx, word ptr [di + 0xa]
000726:  3E 8B 4E 0A                   mov      cx, word ptr ds:[bp + 0xa]
00072A:  3B D1                         cmp      dx, cx
00072C:  7E 0D                         jle      0x73b
00072E:  3B C1                         cmp      ax, cx
000730:  7C 1C                         jl       0x74e
000732:  87 DD                         xchg     bp, bx
000734:  91                            xchg     cx, ax
000735:  87 FD                         xchg     bp, di
000737:  87 CA                         xchg     dx, cx
000739:  EB 0A                         jmp      0x745
00073B:  3B C2                         cmp      ax, dx
00073D:  7E 0F                         jle      0x74e
00073F:  87 DD                         xchg     bp, bx
000741:  91                            xchg     cx, ax
000742:  87 DF                         xchg     di, bx
000744:  92                            xchg     dx, ax
000745:  89 5C 02                      mov      word ptr [si + 2], bx
000748:  89 7C 04                      mov      word ptr [si + 4], di
00074B:  89 6C 06                      mov      word ptr [si + 6], bp
00074E:  2B D0                         sub      dx, ax
000750:  2B C8                         sub      cx, ax
000752:  81 FA 90 01                   cmp      dx, 0x190
000756:  73 17                         jae      0x76f
000758:  81 F9 90 01                   cmp      cx, 0x190
00075C:  73 11                         jae      0x76f
00075E:  03 C0                         add      ax, ax
000760:  BF 86 06                      mov      di, 0x686
000763:  78 02                         js       0x767
000765:  03 F8                         add      di, ax
000767:  26 8B 1D                      mov      bx, word ptr es:[di]
00076A:  26 89 35                      mov      word ptr es:[di], si
00076D:  89 1C                         mov      word ptr [si], bx
00076F:  59                            pop      cx
000770:  83 C6 08                      add      si, 8
000773:  E2 95                         loop     0x70a
000775:  64 8E 1E 06 00                mov      ds, word ptr fs:[6]
00077A:  FC                            cld
00077B:  64 8E 06 06 00                mov      es, word ptr fs:[6]
000780:  BB 72 0A                      mov      bx, 0xa72
000783:  89 1E 08 09                   mov      word ptr [0x908], bx
000787:  B9 C8 00                      mov      cx, 0xc8
00078A:  8B F3                         mov      si, bx
00078C:  83 C3 5A                      add      bx, 0x5a
00078F:  89 1C                         mov      word ptr [si], bx
000791:  E2 F7                         loop     0x78a
000793:  C7 04 00 00                   mov      word ptr [si], 0
000797:  C7 06 84 06 86 06             mov      word ptr [0x684], 0x686
00079D:  C7 06 80 06 00 00             mov      word ptr [0x680], 0
0007A3:  B9 40 01                      mov      cx, 0x140
0007A6:  8B 3E 84 06                   mov      di, word ptr [0x684]
0007AA:  2B 0E 80 06                   sub      cx, word ptr [0x680]
0007AE:  33 C0                         xor      ax, ax
0007B0:  F3 AF                         repe scasw ax, word ptr es:[di]
0007B2:  0F 84 92 00                   je       0x848
0007B6:  BE 64 09                      mov      si, 0x964
0007B9:  BB BE 09                      mov      bx, 0x9be
0007BC:  89 1C                         mov      word ptr [si], bx
0007BE:  C7 44 2E 4A 01                mov      word ptr [si + 0x2e], 0x14a
0007C3:  66 C7 44 08 00 00 00 80       mov      dword ptr [si + 8], 0x80000000
0007CB:  66 C7 44 18 00 00 00 00       mov      dword ptr [si + 0x18], 0
0007D3:  66 C7 44 0C 00 00 00 00       mov      dword ptr [si + 0xc], 0
0007DB:  66 C7 44 1C 00 00 00 00       mov      dword ptr [si + 0x1c], 0
0007E3:  89 77 10                      mov      word ptr [bx + 0x10], si
0007E6:  C7 07 18 0A                   mov      word ptr [bx], 0xa18
0007EA:  C7 47 2E 40 01                mov      word ptr [bx + 0x2e], 0x140
0007EF:  66 C7 47 08 00 00 00 00       mov      dword ptr [bx + 8], 0
0007F7:  66 C7 47 18 00 00 00 00       mov      dword ptr [bx + 0x18], 0
0007FF:  C7 47 0A C8 00                mov      word ptr [bx + 0xa], 0xc8
000804:  66 C7 47 18 00 00 FF 7F       mov      dword ptr [bx + 0x18], 0x7fff0000
00080C:  C7 47 02 00 80                mov      word ptr [bx + 2], 0x8000
000811:  BE 18 0A                      mov      si, 0xa18
000814:  66 C7 44 08 FF FF FF 7F       mov      dword ptr [si + 8], 0x7fffffff
00081C:  66 C7 44 18 FF FF FF 7F       mov      dword ptr [si + 0x18], 0x7fffffff
000824:  89 1E 28 0A                   mov      word ptr [0xa28], bx
000828:  C7 06 44 0A 73 08             mov      word ptr [0xa44], 0x873
00082E:  C7 06 46 0A FF FF             mov      word ptr [0xa46], 0xffff
000834:  BB 3F 01                      mov      bx, 0x13f
000837:  83 EF 02                      sub      di, 2
00083A:  2B D9                         sub      bx, cx
00083C:  89 3E 84 06                   mov      word ptr [0x684], di
000840:  89 1E 80 06                   mov      word ptr [0x680], bx
000844:  8B 35                         mov      si, word ptr [di]
000846:  EB 6D                         jmp      0x8b5
000848:  C3                            ret
000849:  56                            push     si
00084A:  8B 1D                         mov      bx, word ptr [di]
00084C:  89 1C                         mov      word ptr [si], bx
00084E:  89 77 10                      mov      word ptr [bx + 0x10], si
000851:  66 8B 45 08                   mov      eax, dword ptr [di + 8]
000855:  8B 74 10                      mov      si, word ptr [si + 0x10]
000858:  81 FE 64 09                   cmp      si, 0x964
00085C:  74 06                         je       0x864
00085E:  66 3B 44 08                   cmp      eax, dword ptr [si + 8]
000862:  7C F1                         jl       0x855
000864:  8B 1C                         mov      bx, word ptr [si]
000866:  89 3C                         mov      word ptr [si], di
000868:  89 1D                         mov      word ptr [di], bx
00086A:  89 75 10                      mov      word ptr [di + 0x10], si
00086D:  89 7F 10                      mov      word ptr [bx + 0x10], di
000870:  5E                            pop      si
000871:  EB 11                         jmp      0x884
000873:  A1 80 06                      mov      ax, word ptr [0x680]
000876:  40                            inc      ax
000877:  3D 40 01                      cmp      ax, 0x140
00087A:  73 CC                         jae      0x848
00087C:  A3 80 06                      mov      word ptr [0x680], ax
00087F:  BB 64 09                      mov      bx, 0x964
000882:  8B 37                         mov      si, word ptr [bx]
000884:  8B 3C                         mov      di, word ptr [si]
000886:  81 FF 18 0A                   cmp      di, 0xa18
00088A:  74 18                         je       0x8a4
00088C:  66 8B 44 08                   mov      eax, dword ptr [si + 8]
000890:  66 8B 4C 18                   mov      ecx, dword ptr [si + 0x18]
000894:  66 3B 45 08                   cmp      eax, dword ptr [di + 8]
000898:  7F AF                         jg       0x849
00089A:  8B F7                         mov      si, di
00089C:  8B 3D                         mov      di, word ptr [di]
00089E:  81 FF 18 0A                   cmp      di, 0xa18
0008A2:  75 E8                         jne      0x88c
0008A4:  8B 3E 84 06                   mov      di, word ptr [0x684]
0008A8:  83 C7 02                      add      di, 2
0008AB:  89 3E 84 06                   mov      word ptr [0x684], di
0008AF:  8B 35                         mov      si, word ptr [di]
0008B1:  0B F6                         or       si, si
0008B3:  74 1C                         je       0x8d1
0008B5:  C7 05 00 00                   mov      word ptr [di], 0
0008B9:  F7 06 08 09 FF FF             test     word ptr [0x908], 0xffff
0008BF:  74 10                         je       0x8d1
0008C1:  64 8E 06 02 00                mov      es, word ptr fs:[2]
0008C6:  26 FF 34                      push     word ptr es:[si]
0008C9:  E8 B1 04                      call     0xd7d
0008CC:  5E                            pop      si
0008CD:  0B F6                         or       si, si
0008CF:  75 F5                         jne      0x8c6
0008D1:  BE 64 09                      mov      si, 0x964
0008D4:  8B 04                         mov      ax, word ptr [si]
0008D6:  3D BE 09                      cmp      ax, 0x9be
0008D9:  0F 84 B4 03                   je       0xc91
0008DD:  BA 74 09                      mov      dx, 0x974
0008E0:  C7 44 02 01 00                mov      word ptr [si + 2], 1
0008E5:  89 54 06                      mov      word ptr [si + 6], dx
0008E8:  8B FE                         mov      di, si
0008EA:  8B EE                         mov      bp, si
0008EC:  33 DB                         xor      bx, bx
0008EE:  8B 3D                         mov      di, word ptr [di]
0008F0:  F7 45 1A 00 80                test     word ptr [di + 0x1a], 0x8000
0008F5:  75 F7                         jne      0x8ee
0008F7:  89 5C 58                      mov      word ptr [si + 0x58], bx
0008FA:  89 5D 58                      mov      word ptr [di + 0x58], bx
0008FD:  3B 5D 0A                      cmp      bx, word ptr [di + 0xa]
000900:  0F 8E A1 00                   jle      0x9a5
000904:  8B F7                         mov      si, di
000906:  8B EA                         mov      bp, dx
000908:  89 3E 62 09                   mov      word ptr [0x962], di
00090C:  66 0F B7 45 0A                movzx    eax, word ptr [di + 0xa]
000911:  F7 D8                         neg      ax
000913:  66 F7 6D 28                   imul     dword ptr [di + 0x28]
000917:  66 03 45 20                   add      eax, dword ptr [di + 0x20]
00091B:  66 89 45 04                   mov      dword ptr [di + 4], eax
00091F:  EB 2F                         jmp      0x950
000921:  BB 0A 09                      mov      bx, 0x90a
000924:  66 0F B7 45 0A                movzx    eax, word ptr [di + 0xa]
000929:  F7 D8                         neg      ax
00092B:  66 F7 6D 28                   imul     dword ptr [di + 0x28]
00092F:  66 03 45 20                   add      eax, dword ptr [di + 0x20]
000933:  66 89 45 04                   mov      dword ptr [di + 4], eax
000937:  66 3B 44 04                   cmp      eax, dword ptr [si + 4]
00093B:  7E 09                         jle      0x946
00093D:  8B DE                         mov      bx, si
00093F:  8B 74 58                      mov      si, word ptr [si + 0x58]
000942:  0B F6                         or       si, si
000944:  75 F1                         jne      0x937
000946:  89 7F 58                      mov      word ptr [bx + 0x58], di
000949:  89 75 58                      mov      word ptr [di + 0x58], si
00094C:  8B 36 62 09                   mov      si, word ptr [0x962]
000950:  8B 3D                         mov      di, word ptr [di]
000952:  F7 45 1A 00 80                test     word ptr [di + 0x1a], 0x8000
000957:  75 F7                         jne      0x950
000959:  F7 45 0A 00 80                test     word ptr [di + 0xa], 0x8000
00095E:  75 C1                         jne      0x921
000960:  3E C7 46 02 00 00             mov      word ptr ds:[bp + 2], 0
000966:  3E 89 76 04                   mov      word ptr ds:[bp + 4], si
00096A:  3E 8B 4E 0A                   mov      cx, word ptr ds:[bp + 0xa]
00096E:  8B 44 1A                      mov      ax, word ptr [si + 0x1a]
000971:  8B 55 0A                      mov      dx, word ptr [di + 0xa]
000974:  3B 55 1A                      cmp      dx, word ptr [di + 0x1a]
000977:  7D 55                         jge      0x9ce
000979:  3B C2                         cmp      ax, dx
00097B:  0F 8F B1 00                   jg       0xa30
00097F:  74 72                         je       0x9f3
000981:  8D 54 10                      lea      dx, [si + 0x10]
000984:  8B 4C 1A                      mov      cx, word ptr [si + 0x1a]
000987:  3E 89 56 06                   mov      word ptr ds:[bp + 6], dx
00098B:  8B EA                         mov      bp, dx
00098D:  8B 74 58                      mov      si, word ptr [si + 0x58]
000990:  0B F6                         or       si, si
000992:  74 11                         je       0x9a5
000994:  3B 4C 1A                      cmp      cx, word ptr [si + 0x1a]
000997:  7D F4                         jge      0x98d
000999:  3E C7 46 02 00 00             mov      word ptr ds:[bp + 2], 0
00099F:  3E 89 76 04                   mov      word ptr ds:[bp + 4], si
0009A3:  EB C5                         jmp      0x96a
0009A5:  81 FF BE 09                   cmp      di, 0x9be
0009A9:  0F 84 1C 02                   je       0xbc9
0009AD:  3E C7 46 02 01 00             mov      word ptr ds:[bp + 2], 1
0009B3:  3E 89 7E 06                   mov      word ptr ds:[bp + 6], di
0009B7:  8B EF                         mov      bp, di
0009B9:  C7 45 58 00 00                mov      word ptr [di + 0x58], 0
0009BE:  8B F7                         mov      si, di
0009C0:  3E C7 46 02 00 00             mov      word ptr ds:[bp + 2], 0
0009C6:  3E 89 7E 04                   mov      word ptr ds:[bp + 4], di
0009CA:  8B 3D                         mov      di, word ptr [di]
0009CC:  EB 9C                         jmp      0x96a
0009CE:  8B 3D                         mov      di, word ptr [di]
0009D0:  EB 98                         jmp      0x96a
0009D2:  8B 5C 58                      mov      bx, word ptr [si + 0x58]
0009D5:  0B DB                         or       bx, bx
0009D7:  75 1A                         jne      0x9f3
0009D9:  3E 89 7E 06                   mov      word ptr ds:[bp + 6], di
0009DD:  8B EF                         mov      bp, di
0009DF:  8B F7                         mov      si, di
0009E1:  3E C7 46 02 00 00             mov      word ptr ds:[bp + 2], 0
0009E7:  3E 89 7E 04                   mov      word ptr ds:[bp + 4], di
0009EB:  89 5C 58                      mov      word ptr [si + 0x58], bx
0009EE:  8B 3C                         mov      di, word ptr [si]
0009F0:  E9 77 FF                      jmp      0x96a
0009F3:  3E 89 7E 06                   mov      word ptr ds:[bp + 6], di
0009F7:  8B 4D 0A                      mov      cx, word ptr [di + 0xa]
0009FA:  8B EF                         mov      bp, di
0009FC:  81 FF BE 09                   cmp      di, 0x9be
000A00:  0F 84 C5 01                   je       0xbc9
000A04:  8B 74 58                      mov      si, word ptr [si + 0x58]
000A07:  0B F6                         or       si, si
000A09:  74 11                         je       0xa1c
000A0B:  3B 4C 1A                      cmp      cx, word ptr [si + 0x1a]
000A0E:  7D F4                         jge      0xa04
000A10:  3E C7 46 02 00 00             mov      word ptr ds:[bp + 2], 0
000A16:  3E 89 76 04                   mov      word ptr ds:[bp + 4], si
000A1A:  EB 14                         jmp      0xa30
000A1C:  89 75 58                      mov      word ptr [di + 0x58], si
000A1F:  8B F7                         mov      si, di
000A21:  3E C7 46 02 00 00             mov      word ptr ds:[bp + 2], 0
000A27:  3E 89 7E 04                   mov      word ptr ds:[bp + 4], di
000A2B:  8B 3D                         mov      di, word ptr [di]
000A2D:  E9 3A FF                      jmp      0x96a
000A30:  81 FF BE 09                   cmp      di, 0x9be
000A34:  0F 84 8B 01                   je       0xbc3
000A38:  8B 55 0A                      mov      dx, word ptr [di + 0xa]
000A3B:  3B 55 1A                      cmp      dx, word ptr [di + 0x1a]
000A3E:  7D 49                         jge      0xa89
000A40:  3E 8B 4E 0A                   mov      cx, word ptr ds:[bp + 0xa]
000A44:  BB 0A 09                      mov      bx, 0x90a
000A47:  89 77 58                      mov      word ptr [bx + 0x58], si
000A4A:  66 8B 4D 08                   mov      ecx, dword ptr [di + 8]
000A4E:  66 3B 4C 18                   cmp      ecx, dword ptr [si + 0x18]
000A52:  7C 0C                         jl       0xa60
000A54:  8B 74 58                      mov      si, word ptr [si + 0x58]
000A57:  0B F6                         or       si, si
000A59:  89 77 58                      mov      word ptr [bx + 0x58], si
000A5C:  75 F0                         jne      0xa4e
000A5E:  EB 23                         jmp      0xa83
000A60:  66 8B C1                      mov      eax, ecx
000A63:  66 2B 44 08                   sub      eax, dword ptr [si + 8]
000A67:  66 F7 6C 28                   imul     dword ptr [si + 0x28]
000A6B:  66 0F AC D0 10                shrd     eax, edx, 0x10
000A70:  66 03 44 20                   add      eax, dword ptr [si + 0x20]
000A74:  66 3B 45 20                   cmp      eax, dword ptr [di + 0x20]
000A78:  7D 09                         jge      0xa83
000A7A:  8B DE                         mov      bx, si
000A7C:  8B 74 58                      mov      si, word ptr [si + 0x58]
000A7F:  0B F6                         or       si, si
000A81:  75 CB                         jne      0xa4e
000A83:  89 7F 58                      mov      word ptr [bx + 0x58], di
000A86:  89 75 58                      mov      word ptr [di + 0x58], si
000A89:  8B 36 62 09                   mov      si, word ptr [0x962]
000A8D:  3B F7                         cmp      si, di
000A8F:  75 0E                         jne      0xa9f
000A91:  3E 89 7E 06                   mov      word ptr ds:[bp + 6], di
000A95:  8B EF                         mov      bp, di
000A97:  C7 45 02 00 00                mov      word ptr [di + 2], 0
000A9C:  89 7D 04                      mov      word ptr [di + 4], di
000A9F:  8B 3D                         mov      di, word ptr [di]
000AA1:  E9 C6 FE                      jmp      0x96a
000AA4:  8B 3E 80 06                   mov      di, word ptr [0x680]
000AA8:  8B CF                         mov      cx, di
000AAA:  83 E1 03                      and      cx, 3
000AAD:  0F 85 E0 01                   jne      0xc91
000AB1:  64 8E 06 18 00                mov      es, word ptr fs:[0x18]
000AB6:  BA C4 03                      mov      dx, 0x3c4
000AB9:  B8 02 0F                      mov      ax, 0xf02
000ABC:  C1 EF 02                      shr      di, 2
000ABF:  89 3E 82 06                   mov      word ptr [0x682], di
000AC3:  F7 47 02 01 80                test     word ptr [bx + 2], 0x8001
000AC8:  EF                            out      dx, ax
000AC9:  0F 88 C4 01                   js       0xc91
000ACD:  74 5E                         je       0xb2d
000ACF:  EB 3D                         jmp      0xb0e
000AD1:  87 DB                         xchg     bx, bx
000AD3:  87 DB                         xchg     bx, bx
000AD5:  87 DB                         xchg     bx, bx
000AD7:  87 DB                         xchg     bx, bx
000AD9:  87 DB                         xchg     bx, bx
000ADB:  87 DB                         xchg     bx, bx
000ADD:  87 DB                         xchg     bx, bx
000ADF:  90                            nop
000AE0:  F7 47 02 01 80                test     word ptr [bx + 2], 0x8001
000AE5:  0F 88 A8 01                   js       0xc91
000AE9:  64 8E 06 18 00                mov      es, word ptr fs:[0x18]
000AEE:  BA C4 03                      mov      dx, 0x3c4
000AF1:  8B 3E 80 06                   mov      di, word ptr [0x680]
000AF5:  B8 02 01                      mov      ax, 0x102
000AF8:  8B CF                         mov      cx, di
000AFA:  C1 EF 02                      shr      di, 2
000AFD:  83 E1 03                      and      cx, 3
000B00:  89 3E 82 06                   mov      word ptr [0x682], di
000B04:  D2 E4                         shl      ah, cl
000B06:  F7 47 02 01 80                test     word ptr [bx + 2], 0x8001
000B0B:  EF                            out      dx, ax
000B0C:  74 1F                         je       0xb2d
000B0E:  8B 3E 82 06                   mov      di, word ptr [0x682]
000B12:  8B 5F 06                      mov      bx, word ptr [bx + 6]
000B15:  8B 47 0A                      mov      ax, word ptr [bx + 0xa]
000B18:  C1 E0 04                      shl      ax, 4
000B1B:  03 F8                         add      di, ax
000B1D:  C1 E0 02                      shl      ax, 2
000B20:  03 F8                         add      di, ax
000B22:  F7 47 02 01 80                test     word ptr [bx + 2], 0x8001
000B27:  0F 88 66 01                   js       0xc91
000B2B:  75 E1                         jne      0xb0e
000B2D:  8B 77 06                      mov      si, word ptr [bx + 6]
000B30:  8B 47 0A                      mov      ax, word ptr [bx + 0xa]
000B33:  8B 4C 0A                      mov      cx, word ptr [si + 0xa]
000B36:  2B C8                         sub      cx, ax
000B38:  56                            push     si
000B39:  7E 33                         jle      0xb6e
000B3B:  8B 77 04                      mov      si, word ptr [bx + 4]
000B3E:  90                            nop
000B3F:  2B 44 0A                      sub      ax, word ptr [si + 0xa]
000B42:  75 3C                         jne      0xb80
000B44:  8B 44 42                      mov      ax, word ptr [si + 0x42]
000B47:  8A DC                         mov      bl, ah
000B49:  8B 54 44                      mov      dx, word ptr [si + 0x44]
000B4C:  8A FE                         mov      bh, dh
000B4E:  8B 6C 52                      mov      bp, word ptr [si + 0x52]
000B51:  90                            nop
000B52:  C5 74 54                      lds      si, ptr [si + 0x54]
000B55:  03 C5                         add      ax, bp
000B57:  8A 2F                         mov      ch, byte ptr [bx]
000B59:  03 D6                         add      dx, si
000B5B:  8A DC                         mov      bl, ah
000B5D:  26 88 2D                      mov      byte ptr es:[di], ch
000B60:  83 C7 50                      add      di, 0x50
000B63:  FE C9                         dec      cl
000B65:  8A FE                         mov      bh, dh
000B67:  75 EC                         jne      0xb55
000B69:  64 8E 1E 06 00                mov      ds, word ptr fs:[6]
000B6E:  5B                            pop      bx
000B6F:  F7 47 02 01 80                test     word ptr [bx + 2], 0x8001
000B74:  74 B7                         je       0xb2d
000B76:  79 96                         jns      0xb0e
000B78:  E9 16 01                      jmp      0xc91
000B7B:  87 DB                         xchg     bx, bx
000B7D:  87 DB                         xchg     bx, bx
000B7F:  90                            nop
000B80:  8B 54 54                      mov      dx, word ptr [si + 0x54]
000B83:  8B 6C 52                      mov      bp, word ptr [si + 0x52]
000B86:  0F AF D0                      imul     dx, ax
000B89:  0F AF C5                      imul     ax, bp
000B8C:  03 54 44                      add      dx, word ptr [si + 0x44]
000B8F:  03 44 42                      add      ax, word ptr [si + 0x42]
000B92:  8A FE                         mov      bh, dh
000B94:  8A DC                         mov      bl, ah
000B96:  C5 74 54                      lds      si, ptr [si + 0x54]
000B99:  8A 2F                         mov      ch, byte ptr [bx]
000B9B:  03 C5                         add      ax, bp
000B9D:  26 88 2D                      mov      byte ptr es:[di], ch
000BA0:  03 D6                         add      dx, si
000BA2:  83 C7 50                      add      di, 0x50
000BA5:  FE C9                         dec      cl
000BA7:  8A DC                         mov      bl, ah
000BA9:  8A FE                         mov      bh, dh
000BAB:  75 EC                         jne      0xb99
000BAD:  5B                            pop      bx
000BAE:  64 8E 1E 06 00                mov      ds, word ptr fs:[6]
000BB3:  F7 47 02 01 80                test     word ptr [bx + 2], 0x8001
000BB8:  0F 84 71 FF                   je       0xb2d
000BBC:  0F 89 4E FF                   jns      0xb0e
000BC0:  E9 CE 00                      jmp      0xc91
000BC3:  3E 89 7E 06                   mov      word ptr ds:[bp + 6], di
000BC7:  8B EF                         mov      bp, di
000BC9:  3E C7 46 02 00 80             mov      word ptr ds:[bp + 2], 0x8000
000BCF:  BB 64 09                      mov      bx, 0x964
000BD2:  FF 26 7E 06                   jmp      word ptr [0x67e]
000BD6:  64 8E 06 14 00                mov      es, word ptr fs:[0x14]
000BDB:  F7 47 02 01 80                test     word ptr [bx + 2], 0x8001
000BE0:  0F 88 AD 00                   js       0xc91
000BE4:  74 1E                         je       0xc04
000BE6:  8B 3E 80 06                   mov      di, word ptr [0x680]
000BEA:  8B 5F 06                      mov      bx, word ptr [bx + 6]
000BED:  8B 47 0A                      mov      ax, word ptr [bx + 0xa]
000BF0:  8B C8                         mov      cx, ax
000BF2:  C1 E0 06                      shl      ax, 6
000BF5:  02 E1                         add      ah, cl
000BF7:  03 F8                         add      di, ax
000BF9:  F7 47 02 01 80                test     word ptr [bx + 2], 0x8001
000BFE:  0F 88 8F 00                   js       0xc91
000C02:  75 E2                         jne      0xbe6
000C04:  8B 77 06                      mov      si, word ptr [bx + 6]
000C07:  8B 47 0A                      mov      ax, word ptr [bx + 0xa]
000C0A:  8B 4C 0A                      mov      cx, word ptr [si + 0xa]
000C0D:  2B C8                         sub      cx, ax
000C0F:  56                            push     si
000C10:  7E 32                         jle      0xc44
000C12:  8B 77 04                      mov      si, word ptr [bx + 4]
000C15:  2B 44 0A                      sub      ax, word ptr [si + 0xa]
000C18:  75 36                         jne      0xc50
000C1A:  8B 44 42                      mov      ax, word ptr [si + 0x42]
000C1D:  8A DC                         mov      bl, ah
000C1F:  8B 54 44                      mov      dx, word ptr [si + 0x44]
000C22:  8A FE                         mov      bh, dh
000C24:  8B 6C 52                      mov      bp, word ptr [si + 0x52]
000C27:  C5 74 54                      lds      si, ptr [si + 0x54]
000C2A:  03 C5                         add      ax, bp
000C2C:  03 D6                         add      dx, si
000C2E:  8A 2F                         mov      ch, byte ptr [bx]
000C30:  8A DC                         mov      bl, ah
000C32:  26 88 2D                      mov      byte ptr es:[di], ch
000C35:  81 C7 40 01                   add      di, 0x140
000C39:  FE C9                         dec      cl
000C3B:  8A FE                         mov      bh, dh
000C3D:  75 EB                         jne      0xc2a
000C3F:  64 8E 1E 06 00                mov      ds, word ptr fs:[6]
000C44:  5B                            pop      bx
000C45:  F7 47 02 01 80                test     word ptr [bx + 2], 0x8001
000C4A:  74 B8                         je       0xc04
000C4C:  79 98                         jns      0xbe6
000C4E:  EB 41                         jmp      0xc91
000C50:  8B 54 54                      mov      dx, word ptr [si + 0x54]
000C53:  8B 6C 52                      mov      bp, word ptr [si + 0x52]
000C56:  0F AF D0                      imul     dx, ax
000C59:  0F AF C5                      imul     ax, bp
000C5C:  03 54 44                      add      dx, word ptr [si + 0x44]
000C5F:  03 44 42                      add      ax, word ptr [si + 0x42]
000C62:  8A FE                         mov      bh, dh
000C64:  8A DC                         mov      bl, ah
000C66:  C5 74 54                      lds      si, ptr [si + 0x54]
000C69:  8A 2F                         mov      ch, byte ptr [bx]
000C6B:  03 C5                         add      ax, bp
000C6D:  26 88 2D                      mov      byte ptr es:[di], ch
000C70:  03 D6                         add      dx, si
000C72:  81 C7 40 01                   add      di, 0x140
000C76:  FE C9                         dec      cl
000C78:  8A DC                         mov      bl, ah
000C7A:  8A FE                         mov      bh, dh
000C7C:  75 EB                         jne      0xc69
000C7E:  5B                            pop      bx
000C7F:  64 8E 1E 06 00                mov      ds, word ptr fs:[6]
000C84:  F7 47 02 01 80                test     word ptr [bx + 2], 0x8001
000C89:  0F 84 77 FF                   je       0xc04
000C8D:  0F 89 55 FF                   jns      0xbe6
000C91:  8B 36 64 09                   mov      si, word ptr [0x964]
000C95:  FF 4C 2E                      dec      word ptr [si + 0x2e]
000C98:  78 2D                         js       0xcc7
000C9A:  8B 44 4A                      mov      ax, word ptr [si + 0x4a]
000C9D:  8B 5C 4C                      mov      bx, word ptr [si + 0x4c]
000CA0:  66 8B 4C 0C                   mov      ecx, dword ptr [si + 0xc]
000CA4:  66 8B 54 24                   mov      edx, dword ptr [si + 0x24]
000CA8:  01 44 42                      add      word ptr [si + 0x42], ax
000CAB:  01 5C 44                      add      word ptr [si + 0x44], bx
000CAE:  66 01 4C 08                   add      dword ptr [si + 8], ecx
000CB2:  66 01 54 20                   add      dword ptr [si + 0x20], edx
000CB6:  8B DE                         mov      bx, si
000CB8:  66 8B 4C 1C                   mov      ecx, dword ptr [si + 0x1c]
000CBC:  8B 37                         mov      si, word ptr [bx]
000CBE:  66 01 4F 18                   add      dword ptr [bx + 0x18], ecx
000CC2:  FF 4C 2E                      dec      word ptr [si + 0x2e]
000CC5:  79 D3                         jns      0xc9a
000CC7:  FF 64 2C                      jmp      word ptr [si + 0x2c]
000CCA:  66 8B 44 32                   mov      eax, dword ptr [si + 0x32]
000CCE:  66 8B 4C 36                   mov      ecx, dword ptr [si + 0x36]
000CD2:  66 8B 54 46                   mov      edx, dword ptr [si + 0x46]
000CD6:  66 8B 7C 4E                   mov      edi, dword ptr [si + 0x4e]
000CDA:  66 89 44 08                   mov      dword ptr [si + 8], eax
000CDE:  66 89 4C 0C                   mov      dword ptr [si + 0xc], ecx
000CE2:  66 89 54 42                   mov      dword ptr [si + 0x42], edx
000CE6:  66 89 7C 4A                   mov      dword ptr [si + 0x4a], edi
000CEA:  66 8B 44 3A                   mov      eax, dword ptr [si + 0x3a]
000CEE:  66 8B 4C 3E                   mov      ecx, dword ptr [si + 0x3e]
000CF2:  66 89 44 20                   mov      dword ptr [si + 0x20], eax
000CF6:  66 89 4C 24                   mov      dword ptr [si + 0x24], ecx
000CFA:  8B 44 30                      mov      ax, word ptr [si + 0x30]
000CFD:  89 44 2E                      mov      word ptr [si + 0x2e], ax
000D00:  C7 44 2C 5E 0D                mov      word ptr [si + 0x2c], 0xd5e
000D05:  8B DE                         mov      bx, si
000D07:  66 8B 4C 1C                   mov      ecx, dword ptr [si + 0x1c]
000D0B:  8B 37                         mov      si, word ptr [bx]
000D0D:  66 01 4F 18                   add      dword ptr [bx + 0x18], ecx
000D11:  FF 4C 2E                      dec      word ptr [si + 0x2e]
000D14:  79 84                         jns      0xc9a
000D16:  FF 64 2C                      jmp      word ptr [si + 0x2c]
000D19:  66 8B 4C 0C                   mov      ecx, dword ptr [si + 0xc]
000D1D:  66 8B 54 24                   mov      edx, dword ptr [si + 0x24]
000D21:  8B 44 4A                      mov      ax, word ptr [si + 0x4a]
000D24:  8B 5C 4C                      mov      bx, word ptr [si + 0x4c]
000D27:  66 01 4C 08                   add      dword ptr [si + 8], ecx
000D2B:  66 01 54 20                   add      dword ptr [si + 0x20], edx
000D2F:  01 44 42                      add      word ptr [si + 0x42], ax
000D32:  01 5C 44                      add      word ptr [si + 0x44], bx
000D35:  8B DE                         mov      bx, si
000D37:  66 8B 44 32                   mov      eax, dword ptr [si + 0x32]
000D3B:  66 8B 4C 36                   mov      ecx, dword ptr [si + 0x36]
000D3F:  66 89 44 18                   mov      dword ptr [si + 0x18], eax
000D43:  66 89 4C 1C                   mov      dword ptr [si + 0x1c], ecx
000D47:  8B 37                         mov      si, word ptr [bx]
000D49:  8B 47 30                      mov      ax, word ptr [bx + 0x30]
000D4C:  89 47 2E                      mov      word ptr [bx + 0x2e], ax
000D4F:  C7 47 2C 5E 0D                mov      word ptr [bx + 0x2c], 0xd5e
000D54:  FF 4C 2E                      dec      word ptr [si + 0x2e]
000D57:  0F 89 3F FF                   jns      0xc9a
000D5B:  FF 64 2C                      jmp      word ptr [si + 0x2c]
000D5E:  8B 5C 10                      mov      bx, word ptr [si + 0x10]
000D61:  8B 3C                         mov      di, word ptr [si]
000D63:  A1 08 09                      mov      ax, word ptr [0x908]
000D66:  89 3F                         mov      word ptr [bx], di
000D68:  89 5D 10                      mov      word ptr [di + 0x10], bx
000D6B:  89 04                         mov      word ptr [si], ax
000D6D:  89 36 08 09                   mov      word ptr [0x908], si
000D71:  8B F7                         mov      si, di
000D73:  FF 4D 2E                      dec      word ptr [di + 0x2e]
000D76:  0F 89 20 FF                   jns      0xc9a
000D7A:  FF 64 2C                      jmp      word ptr [si + 0x2c]
