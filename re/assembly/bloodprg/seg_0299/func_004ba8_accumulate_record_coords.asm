; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x004ba8
; seg_off: 0299:1c18
; group: seg_0299
; provenance: static_dispatch_table_target
; label: accumulate_record_coords
; label_comment: coordinate accumulation (family 0x4ba8/0x4cd6): lds si,[di+4] (record data ptr); ax+=[si+4], bx+=[si+6], dx+=[si+4] - sums the x/y coordinate fields across records (bounds/centroid/offset accumulation for object/geometry positioning)
; incoming: sprite_blitter_candidates:blit_2
; byte_count: 302
; boundary: cfg_blocks_26_terminals_4
; terminal: jmp 0x4ccb:3, ret:1
; direct_callees: none
; indirect_calls: 0
; routine_bytes_sha256: 9f0855d33b09f0236b5e3beac6450c56c95ea69b8ebb4bfdd07bb77071c8934b

004BA8:  66 50                        push     eax
004BAA:  53                           push     bx
004BAB:  51                           push     cx
004BAC:  52                           push     dx
004BAD:  06                           push     es
004BAE:  57                           push     di
004BAF:  1E                           push     ds
004BB0:  56                           push     si
004BB1:  55                           push     bp
004BB2:  C5 75 04                     lds      si, ptr [di + 4]
004BB5:  03 44 04                     add      ax, word ptr [si + 4]
004BB8:  03 5C 06                     add      bx, word ptr [si + 6]
004BBB:  03 54 04                     add      dx, word ptr [si + 4]
004BBE:  03 6C 06                     add      bp, word ptr [si + 6]
004BC1:  FF 34                        push     word ptr [si]
004BC3:  52                           push     dx
004BC4:  26 8B 4D 0E                  mov      cx, word ptr es:[di + 0xe]
004BC8:  8B C3                        mov      ax, bx
004BCA:  26 2B 45 1C                  sub      ax, word ptr es:[di + 0x1c]
004BCE:  79 14                        jns      0x4be4
004BD0:  F7 D8                        neg      ax
004BD2:  2B C8                        sub      cx, ax
004BD4:  2E F6 06 E0 14 01            test     byte ptr cs:[0x14e0], 1
004BDA:  75 04                        jne      0x4be0
004BDC:  F7 24                        mul      word ptr [si]
004BDE:  03 F0                        add      si, ax
004BE0:  26 8B 5D 1C                  mov      bx, word ptr es:[di + 0x1c]
004BE4:  8B C5                        mov      ax, bp
004BE6:  26 2B 45 1E                  sub      ax, word ptr es:[di + 0x1e]
004BEA:  78 0E                        js       0x4bfa
004BEC:  2B C8                        sub      cx, ax
004BEE:  2E F6 06 E0 14 01            test     byte ptr cs:[0x14e0], 1
004BF4:  74 04                        je       0x4bfa
004BF6:  F7 24                        mul      word ptr [si]
004BF8:  03 F0                        add      si, ax
004BFA:  26 8B 6D 0C                  mov      bp, word ptr es:[di + 0xc]
004BFE:  26 8B 55 08                  mov      dx, word ptr es:[di + 8]
004C02:  03 54 04                     add      dx, word ptr [si + 4]
004C05:  8B C2                        mov      ax, dx
004C07:  26 2B 45 18                  sub      ax, word ptr es:[di + 0x18]
004C0B:  79 10                        jns      0x4c1d
004C0D:  03 E8                        add      bp, ax
004C0F:  2E F6 06 DF 14 01            test     byte ptr cs:[0x14df], 1
004C15:  75 02                        jne      0x4c19
004C17:  2B F0                        sub      si, ax
004C19:  26 8B 55 18                  mov      dx, word ptr es:[di + 0x18]
004C1D:  58                           pop      ax
004C1E:  26 2B 45 1A                  sub      ax, word ptr es:[di + 0x1a]
004C22:  78 0C                        js       0x4c30
004C24:  2B E8                        sub      bp, ax
004C26:  2E F6 06 DF 14 01            test     byte ptr cs:[0x14df], 1
004C2C:  74 02                        je       0x4c30
004C2E:  03 F0                        add      si, ax
004C30:  65 C4 3E 21 52               les      di, ptr gs:[0x5221]
004C35:  2E F6 06 E0 14 01            test     byte ptr cs:[0x14e0], 1
004C3B:  74 03                        je       0x4c40
004C3D:  03 D9                        add      bx, cx
004C3F:  4B                           dec      bx
004C40:  8B C3                        mov      ax, bx
004C42:  86 C4                        xchg     ah, al
004C44:  C1 E3 06                     shl      bx, 6
004C47:  03 C3                        add      ax, bx
004C49:  03 F8                        add      di, ax
004C4B:  03 FA                        add      di, dx
004C4D:  BA 40 01                     mov      dx, 0x140
004C50:  2B D5                        sub      dx, bp
004C52:  5B                           pop      bx
004C53:  2B DD                        sub      bx, bp
004C55:  87 DD                        xchg     bp, bx
004C57:  83 C6 08                     add      si, 8
004C5A:  32 ED                        xor      ch, ch
004C5C:  2E A1 DF 14                  mov      ax, word ptr cs:[0x14df]
004C60:  0A E4                        or       ah, ah
004C62:  74 06                        je       0x4c6a
004C64:  03 D3                        add      dx, bx
004C66:  03 D3                        add      dx, bx
004C68:  F7 DA                        neg      dx
004C6A:  0A C0                        or       al, al
004C6C:  75 45                        jne      0x4cb3
004C6E:  8A E3                        mov      ah, bl
004C70:  80 E4 03                     and      ah, 3
004C73:  74 1A                        je       0x4c8f
004C75:  C1 EB 02                     shr      bx, 2
004C78:  74 29                        je       0x4ca3
004C7A:  86 C8                        xchg     al, cl
004C7C:  8A CB                        mov      cl, bl
004C7E:  F3 66 A5                     rep movsd dword ptr es:[di], dword ptr [si]
004C81:  8A CC                        mov      cl, ah
004C83:  F3 A4                        rep movsb byte ptr es:[di], byte ptr [si]
004C85:  03 FA                        add      di, dx
004C87:  03 F5                        add      si, bp
004C89:  86 C8                        xchg     al, cl
004C8B:  E2 ED                        loop     0x4c7a
004C8D:  EB 3C                        jmp      0x4ccb
004C8F:  C1 EB 02                     shr      bx, 2
004C92:  86 C8                        xchg     al, cl
004C94:  8A CB                        mov      cl, bl
004C96:  F3 66 A5                     rep movsd dword ptr es:[di], dword ptr [si]
004C99:  03 FA                        add      di, dx
004C9B:  03 F5                        add      si, bp
004C9D:  86 C8                        xchg     al, cl
004C9F:  E2 F1                        loop     0x4c92
004CA1:  EB 28                        jmp      0x4ccb
004CA3:  86 C8                        xchg     al, cl
004CA5:  8A CC                        mov      cl, ah
004CA7:  F3 A4                        rep movsb byte ptr es:[di], byte ptr [si]
004CA9:  03 FA                        add      di, dx
004CAB:  03 F5                        add      si, bp
004CAD:  86 C8                        xchg     al, cl
004CAF:  E2 F2                        loop     0x4ca3
004CB1:  EB 18                        jmp      0x4ccb
004CB3:  03 D3                        add      dx, bx
004CB5:  03 D3                        add      dx, bx
004CB7:  03 FB                        add      di, bx
004CB9:  4F                           dec      di
004CBA:  51                           push     cx
004CBB:  8B CB                        mov      cx, bx
004CBD:  AC                           lodsb    al, byte ptr [si]
004CBE:  26 88 05                     mov      byte ptr es:[di], al
004CC1:  4F                           dec      di
004CC2:  E2 F9                        loop     0x4cbd
004CC4:  03 FA                        add      di, dx
004CC6:  03 F5                        add      si, bp
004CC8:  59                           pop      cx
004CC9:  E2 EF                        loop     0x4cba
004CCB:  5D                           pop      bp
004CCC:  5E                           pop      si
004CCD:  1F                           pop      ds
004CCE:  5F                           pop      di
004CCF:  07                           pop      es
004CD0:  5A                           pop      dx
004CD1:  59                           pop      cx
004CD2:  5B                           pop      bx
004CD3:  66 58                        pop      eax
004CD5:  C3                           ret     
