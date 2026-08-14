; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x008bab
; seg_off: 071e:13cb
; group: seg_071e
; provenance: recursive_graph
; label: name_area_palette_effect_update
; label_comment: Parsed-name-area palette animation. DS:0x27F1 holds ten near pointers to packed streams; each stream starts with {operation:u8, frame_count:u8}, followed by {x,y,width,height} words. The initial stream is deterministic, later streams use blood_prng_next(9)+1, and operations collapse, brighten, cycle, or darken palette indices 0xE0..0xEF in GS:[0x5221]. Natural C and direct vectors: func_008bab_name_area_palette_effect_update.c and func_8bab_natural.json
; byte_count: 235
; boundary: cfg_blocks_24_terminals_3
; terminal: jmp 0x8c8d:2, ret:1
; direct_callees: none
; indirect_calls: 1
; routine_bytes_sha256: 6d470b4ca2cd6f54bcec1b05dd688dc0ee0bfc6119dbf7818b57a4eaed884375

008BAB:  06                           push     es
008BAC:  57                           push     di
008BAD:  1E                           push     ds
008BAE:  56                           push     si
008BAF:  53                           push     bx
008BB0:  51                           push     cx
008BB1:  52                           push     dx
008BB2:  55                           push     bp
008BB3:  F6 06 E8 27 01               test     byte ptr [0x27e8], 1
008BB8:  0F 84 D1 00                  je       0x8c8d
008BBC:  F6 06 E9 27 01               test     byte ptr [0x27e9], 1
008BC1:  74 11                        je       0x8bd4
008BC3:  8B 36 F1 27                  mov      si, word ptr [0x27f1]
008BC7:  AD                           lodsw    ax, word ptr [si]
008BC8:  A3 EF 27                     mov      word ptr [0x27ef], ax
008BCB:  89 36 ED 27                  mov      word ptr [0x27ed], si
008BCF:  C6 06 E9 27 00               mov      byte ptr [0x27e9], 0
008BD4:  8B 36 ED 27                  mov      si, word ptr [0x27ed]
008BD8:  A0 F0 27                     mov      al, byte ptr [0x27f0]
008BDB:  0A C0                        or       al, al
008BDD:  75 16                        jne      0x8bf5
008BDF:  B8 09 00                     mov      ax, 9
008BE2:  9A 02 0B CE 01               lcall    0x1ce, 0xb02
008BE7:  40                           inc      ax
008BE8:  03 C0                        add      ax, ax
008BEA:  BE F1 27                     mov      si, 0x27f1
008BED:  03 F0                        add      si, ax
008BEF:  8B 34                        mov      si, word ptr [si]
008BF1:  AD                           lodsw    ax, word ptr [si]
008BF2:  A3 EF 27                     mov      word ptr [0x27ef], ax
008BF5:  FE 0E F0 27                  dec      byte ptr [0x27f0]
008BF9:  C4 3E 21 52                  les      di, ptr [0x5221]
008BFD:  AD                           lodsw    ax, word ptr [si]
008BFE:  03 F8                        add      di, ax
008C00:  AD                           lodsw    ax, word ptr [si]
008C01:  8B D8                        mov      bx, ax
008C03:  86 C4                        xchg     ah, al
008C05:  C1 E3 06                     shl      bx, 6
008C08:  03 C3                        add      ax, bx
008C0A:  03 F8                        add      di, ax
008C0C:  AD                           lodsw    ax, word ptr [si]
008C0D:  BA 40 01                     mov      dx, 0x140
008C10:  2B D0                        sub      dx, ax
008C12:  8B D8                        mov      bx, ax
008C14:  AD                           lodsw    ax, word ptr [si]
008C15:  8B E8                        mov      bp, ax
008C17:  89 36 ED 27                  mov      word ptr [0x27ed], si
008C1B:  B7 E0                        mov      bh, 0xe0
008C1D:  32 ED                        xor      ch, ch
008C1F:  06                           push     es
008C20:  1F                           pop      ds
008C21:  65 A0 EF 27                  mov      al, byte ptr gs:[0x27ef]
008C25:  0A C0                        or       al, al
008C27:  74 4E                        je       0x8c77
008C29:  FE C8                        dec      al
008C2B:  74 48                        je       0x8c75
008C2D:  FE C8                        dec      al
008C2F:  74 20                        je       0x8c51
008C31:  8A CB                        mov      cl, bl
008C33:  8B F7                        mov      si, di
008C35:  AC                           lodsb    al, byte ptr [si]
008C36:  8A E7                        mov      ah, bh
008C38:  34 E0                        xor      al, 0xe0
008C3A:  3C 0F                        cmp      al, 0xf
008C3C:  77 09                        ja       0x8c47
008C3E:  FE C8                        dec      al
008C40:  78 02                        js       0x8c44
008C42:  02 E0                        add      ah, al
008C44:  26 88 25                     mov      byte ptr es:[di], ah
008C47:  47                           inc      di
008C48:  E2 EB                        loop     0x8c35
008C4A:  03 FA                        add      di, dx
008C4C:  4D                           dec      bp
008C4D:  75 E2                        jne      0x8c31
008C4F:  EB 3C                        jmp      0x8c8d
008C51:  8A CB                        mov      cl, bl
008C53:  8B F7                        mov      si, di
008C55:  AC                           lodsb    al, byte ptr [si]
008C56:  8A E7                        mov      ah, bh
008C58:  34 E0                        xor      al, 0xe0
008C5A:  3C 0F                        cmp      al, 0xf
008C5C:  73 0D                        jae      0x8c6b
008C5E:  3C 0E                        cmp      al, 0xe
008C60:  74 09                        je       0x8c6b
008C62:  04 02                        add      al, 2
008C64:  24 0F                        and      al, 0xf
008C66:  02 E0                        add      ah, al
008C68:  26 88 25                     mov      byte ptr es:[di], ah
008C6B:  47                           inc      di
008C6C:  E2 E7                        loop     0x8c55
008C6E:  03 FA                        add      di, dx
008C70:  4D                           dec      bp
008C71:  75 DE                        jne      0x8c51
008C73:  EB 18                        jmp      0x8c8d
008C75:  B7 EF                        mov      bh, 0xef
008C77:  8A CB                        mov      cl, bl
008C79:  8B F7                        mov      si, di
008C7B:  AC                           lodsb    al, byte ptr [si]
008C7C:  34 E0                        xor      al, 0xe0
008C7E:  3C 0F                        cmp      al, 0xf
008C80:  77 03                        ja       0x8c85
008C82:  26 88 3D                     mov      byte ptr es:[di], bh
008C85:  47                           inc      di
008C86:  E2 F3                        loop     0x8c7b
008C88:  03 FA                        add      di, dx
008C8A:  4D                           dec      bp
008C8B:  75 EA                        jne      0x8c77
008C8D:  5D                           pop      bp
008C8E:  5A                           pop      dx
008C8F:  59                           pop      cx
008C90:  5B                           pop      bx
008C91:  5E                           pop      si
008C92:  1F                           pop      ds
008C93:  5F                           pop      di
008C94:  07                           pop      es
008C95:  C3                           ret     
