; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x00ab25
; seg_off: 0971:0e15
; group: seg_0971
; provenance: recursive_graph
; label: resource_payload_decode_rect
; label_comment: Rectangular transparent-pixel form of the AD decoder: expands staged values through AABC, then consumes MSB-first controls under two flag-selected run layouts while splitting literals and runs across masked-width scanlines. Zero advances without drawing; AD96 advances rows and exits after the final row.
; byte_count: 1136
; boundary: cfg_blocks_179_terminals_48
; terminal: jmp 0xabbf:11, jmp 0xabe8:3, jmp 0xabf5:3, jmp 0xac35:2, jmp 0xac86:1, jmp 0xad03:1, jmp 0xad4f:1, jmp 0xadb2:1, jmp 0xadb4:11, jmp 0xadc3:1, jmp 0xaddd:3, jmp 0xadec:1, jmp 0xadfd:1, jmp 0xae43:1, jmp 0xae9c:3, jmp 0xaee7:2, ret:2
; direct_callees: 0x00aabc, 0x00ad96
; indirect_calls: 0
; routine_bytes_sha256: ba6707b90afc944be901c66398b2f1272535c6c5502dd573b7d17160404379cf

00AB25:  1E                           push     ds
00AB26:  65 8E 06 BE 0A               mov      es, word ptr gs:[0xabe]
00AB2B:  65 C7 06 A0 0A 03 00         mov      word ptr gs:[0xaa0], 3
00AB32:  8B DE                        mov      bx, si
00AB34:  8A 44 04                     mov      al, byte ptr [si + 4]
00AB37:  83 C6 06                     add      si, 6
00AB3A:  33 D2                        xor      dx, dx
00AB3C:  33 C9                        xor      cx, cx
00AB3E:  A8 04                        test     al, 4
00AB40:  75 08                        jne      0xab4a
00AB42:  8B 14                        mov      dx, word ptr [si]
00AB44:  8B 4C 02                     mov      cx, word ptr [si + 2]
00AB47:  83 C6 04                     add      si, 4
00AB4A:  50                           push     ax
00AB4B:  51                           push     cx
00AB4C:  52                           push     dx
00AB4D:  24 40                        and      al, 0x40
00AB4F:  02 C0                        add      al, al
00AB51:  2E A2 DD 0D                  mov      byte ptr cs:[0xddd], al
00AB55:  2E A2 0D 0E                  mov      byte ptr cs:[0xe0d], al
00AB59:  8B EF                        mov      bp, di
00AB5B:  03 2F                        add      bp, word ptr [bx]
00AB5D:  57                           push     di
00AB5E:  8B FD                        mov      di, bp
00AB60:  2B 7F 02                     sub      di, word ptr [bx + 2]
00AB63:  57                           push     di
00AB64:  E8 55 FF                     call     0xaabc
00AB67:  5E                           pop      si
00AB68:  5F                           pop      di
00AB69:  8B C3                        mov      ax, bx
00AB6B:  5A                           pop      dx
00AB6C:  5B                           pop      bx
00AB6D:  50                           push     ax
00AB6E:  65 03 1E A7 1F               add      bx, word ptr gs:[0x1fa7]
00AB73:  8B C3                        mov      ax, bx
00AB75:  86 C4                        xchg     ah, al
00AB77:  C1 E3 06                     shl      bx, 6
00AB7A:  03 C3                        add      ax, bx
00AB7C:  03 C2                        add      ax, dx
00AB7E:  8B F8                        mov      di, ax
00AB80:  5B                           pop      bx
00AB81:  58                           pop      ax
00AB82:  C8 10 00 00                  enter    0x10, 0
00AB86:  50                           push     ax
00AB87:  C7 46 FE 00 00               mov      word ptr [bp - 2], 0
00AB8C:  65 A1 A6 0D                  mov      ax, word ptr gs:[0xda6]
00AB90:  32 E4                        xor      ah, ah
00AB92:  3C 82                        cmp      al, 0x82
00AB94:  76 02                        jbe      0xab98
00AB96:  B0 82                        mov      al, 0x82
00AB98:  89 46 FA                     mov      word ptr [bp - 6], ax
00AB9B:  65 8B 0E A4 0D               mov      cx, word ptr gs:[0xda4]
00ABA0:  81 E1 FF 01                  and      cx, 0x1ff
00ABA4:  89 4E F6                     mov      word ptr [bp - 0xa], cx
00ABA7:  1E                           push     ds
00ABA8:  06                           push     es
00ABA9:  1F                           pop      ds
00ABAA:  0F A1                        pop      fs
00ABAC:  65 8E 06 23 52               mov      es, word ptr gs:[0x5223]
00ABB1:  58                           pop      ax
00ABB2:  BA 00 80                     mov      dx, 0x8000
00ABB5:  3C 80                        cmp      al, 0x80
00ABB7:  72 15                        jb       0xabce
00ABB9:  E9 F6 01                     jmp      0xadb2
00ABBC:  C9                           leave   
00ABBD:  1F                           pop      ds
00ABBE:  C3                           ret     
00ABBF:  FE 4E FA                     dec      byte ptr [bp - 6]
00ABC2:  74 F8                        je       0xabbc
00ABC4:  8B 7E F8                     mov      di, word ptr [bp - 8]
00ABC7:  81 C7 40 01                  add      di, 0x140
00ABCB:  8B 4E F6                     mov      cx, word ptr [bp - 0xa]
00ABCE:  89 7E F8                     mov      word ptr [bp - 8], di
00ABD1:  EB 15                        jmp      0xabe8
00ABD3:  64 8B 17                     mov      dx, word ptr fs:[bx]
00ABD6:  43                           inc      bx
00ABD7:  13 D2                        adc      dx, dx
00ABD9:  43                           inc      bx
00ABDA:  72 12                        jb       0xabee
00ABDC:  AC                           lodsb    al, byte ptr [si]
00ABDD:  0A C0                        or       al, al
00ABDF:  74 03                        je       0xabe4
00ABE1:  26 88 05                     mov      byte ptr es:[di], al
00ABE4:  47                           inc      di
00ABE5:  49                           dec      cx
00ABE6:  74 D7                        je       0xabbf
00ABE8:  03 D2                        add      dx, dx
00ABEA:  73 F0                        jae      0xabdc
00ABEC:  74 E5                        je       0xabd3
00ABEE:  AC                           lodsb    al, byte ptr [si]
00ABEF:  8A E0                        mov      ah, al
00ABF1:  03 D2                        add      dx, dx
00ABF3:  72 3A                        jb       0xac2f
00ABF5:  83 F9 02                     cmp      cx, 2
00ABF8:  72 11                        jb       0xac0b
00ABFA:  0A C0                        or       al, al
00ABFC:  74 03                        je       0xac01
00ABFE:  26 89 05                     mov      word ptr es:[di], ax
00AC01:  83 C7 02                     add      di, 2
00AC04:  83 E9 02                     sub      cx, 2
00AC07:  75 DF                        jne      0xabe8
00AC09:  EB B4                        jmp      0xabbf
00AC0B:  0A C0                        or       al, al
00AC0D:  74 0C                        je       0xac1b
00AC0F:  26 88 05                     mov      byte ptr es:[di], al
00AC12:  E8 81 01                     call     0xad96
00AC15:  AA                           stosb    byte ptr es:[di], al
00AC16:  49                           dec      cx
00AC17:  75 CF                        jne      0xabe8
00AC19:  EB A4                        jmp      0xabbf
00AC1B:  E8 78 01                     call     0xad96
00AC1E:  47                           inc      di
00AC1F:  49                           dec      cx
00AC20:  75 C6                        jne      0xabe8
00AC22:  EB 9B                        jmp      0xabbf
00AC24:  64 8B 17                     mov      dx, word ptr fs:[bx]
00AC27:  43                           inc      bx
00AC28:  13 D2                        adc      dx, dx
00AC2A:  43                           inc      bx
00AC2B:  72 04                        jb       0xac31
00AC2D:  EB C6                        jmp      0xabf5
00AC2F:  74 F3                        je       0xac24
00AC31:  03 D2                        add      dx, dx
00AC33:  72 4B                        jb       0xac80
00AC35:  83 F9 02                     cmp      cx, 2
00AC38:  76 16                        jbe      0xac50
00AC3A:  0A C0                        or       al, al
00AC3C:  74 07                        je       0xac45
00AC3E:  26 88 05                     mov      byte ptr es:[di], al
00AC41:  26 89 45 01                  mov      word ptr es:[di + 1], ax
00AC45:  83 C7 03                     add      di, 3
00AC48:  83 E9 03                     sub      cx, 3
00AC4B:  75 9B                        jne      0xabe8
00AC4D:  E9 6F FF                     jmp      0xabbf
00AC50:  74 0C                        je       0xac5e
00AC52:  0A C0                        or       al, al
00AC54:  74 03                        je       0xac59
00AC56:  26 88 05                     mov      byte ptr es:[di], al
00AC59:  E8 3A 01                     call     0xad96
00AC5C:  EB 97                        jmp      0xabf5
00AC5E:  0A C0                        or       al, al
00AC60:  74 0B                        je       0xac6d
00AC62:  26 89 05                     mov      word ptr es:[di], ax
00AC65:  E8 2E 01                     call     0xad96
00AC68:  AA                           stosb    byte ptr es:[di], al
00AC69:  49                           dec      cx
00AC6A:  E9 7B FF                     jmp      0xabe8
00AC6D:  E8 26 01                     call     0xad96
00AC70:  47                           inc      di
00AC71:  49                           dec      cx
00AC72:  E9 73 FF                     jmp      0xabe8
00AC75:  64 8B 17                     mov      dx, word ptr fs:[bx]
00AC78:  43                           inc      bx
00AC79:  13 D2                        adc      dx, dx
00AC7B:  43                           inc      bx
00AC7C:  72 04                        jb       0xac82
00AC7E:  EB B5                        jmp      0xac35
00AC80:  74 F3                        je       0xac75
00AC82:  03 D2                        add      dx, dx
00AC84:  72 6A                        jb       0xacf0
00AC86:  83 F9 03                     cmp      cx, 3
00AC89:  76 18                        jbe      0xaca3
00AC8B:  0A C0                        or       al, al
00AC8D:  74 07                        je       0xac96
00AC8F:  26 89 05                     mov      word ptr es:[di], ax
00AC92:  26 89 45 02                  mov      word ptr es:[di + 2], ax
00AC96:  83 C7 04                     add      di, 4
00AC99:  83 E9 04                     sub      cx, 4
00AC9C:  0F 85 48 FF                  jne      0xabe8
00ACA0:  E9 1C FF                     jmp      0xabbf
00ACA3:  74 1D                        je       0xacc2
00ACA5:  49                           dec      cx
00ACA6:  74 0D                        je       0xacb5
00ACA8:  0A C0                        or       al, al
00ACAA:  74 03                        je       0xacaf
00ACAC:  26 89 05                     mov      word ptr es:[di], ax
00ACAF:  E8 E4 00                     call     0xad96
00ACB2:  E9 40 FF                     jmp      0xabf5
00ACB5:  0A C0                        or       al, al
00ACB7:  74 03                        je       0xacbc
00ACB9:  26 88 05                     mov      byte ptr es:[di], al
00ACBC:  E8 D7 00                     call     0xad96
00ACBF:  E9 73 FF                     jmp      0xac35
00ACC2:  0A C0                        or       al, al
00ACC4:  74 13                        je       0xacd9
00ACC6:  26 88 05                     mov      byte ptr es:[di], al
00ACC9:  26 89 45 01                  mov      word ptr es:[di + 1], ax
00ACCD:  E8 C6 00                     call     0xad96
00ACD0:  AA                           stosb    byte ptr es:[di], al
00ACD1:  49                           dec      cx
00ACD2:  0F 85 12 FF                  jne      0xabe8
00ACD6:  E9 E6 FE                     jmp      0xabbf
00ACD9:  E8 BA 00                     call     0xad96
00ACDC:  47                           inc      di
00ACDD:  49                           dec      cx
00ACDE:  0F 85 06 FF                  jne      0xabe8
00ACE2:  E9 DA FE                     jmp      0xabbf
00ACE5:  64 8B 17                     mov      dx, word ptr fs:[bx]
00ACE8:  43                           inc      bx
00ACE9:  13 D2                        adc      dx, dx
00ACEB:  43                           inc      bx
00ACEC:  72 04                        jb       0xacf2
00ACEE:  EB 96                        jmp      0xac86
00ACF0:  74 F3                        je       0xace5
00ACF2:  87 56 FE                     xchg     word ptr [bp - 2], dx
00ACF5:  83 FA 04                     cmp      dx, 4
00ACF8:  72 41                        jb       0xad3b
00ACFA:  75 07                        jne      0xad03
00ACFC:  64 8A 17                     mov      dl, byte ptr fs:[bx]
00ACFF:  43                           inc      bx
00AD00:  83 C2 14                     add      dx, 0x14
00AD03:  3B D1                        cmp      dx, cx
00AD05:  76 0D                        jbe      0xad14
00AD07:  2B D1                        sub      dx, cx
00AD09:  0A C0                        or       al, al
00AD0B:  74 02                        je       0xad0f
00AD0D:  F3 AA                        rep stosb byte ptr es:[di], al
00AD0F:  E8 84 00                     call     0xad96
00AD12:  EB EF                        jmp      0xad03
00AD14:  0A C0                        or       al, al
00AD16:  74 12                        je       0xad2a
00AD18:  87 D1                        xchg     cx, dx
00AD1A:  2B D1                        sub      dx, cx
00AD1C:  F3 AA                        rep stosb byte ptr es:[di], al
00AD1E:  87 D1                        xchg     cx, dx
00AD20:  87 56 FE                     xchg     word ptr [bp - 2], dx
00AD23:  0F 85 C1 FE                  jne      0xabe8
00AD27:  E9 95 FE                     jmp      0xabbf
00AD2A:  03 FA                        add      di, dx
00AD2C:  2B CA                        sub      cx, dx
00AD2E:  BA 00 00                     mov      dx, 0
00AD31:  87 56 FE                     xchg     word ptr [bp - 2], dx
00AD34:  0F 85 B0 FE                  jne      0xabe8
00AD38:  E9 84 FE                     jmp      0xabbf
00AD3B:  64 8A 17                     mov      dl, byte ptr fs:[bx]
00AD3E:  43                           inc      bx
00AD3F:  52                           push     dx
00AD40:  C0 EA 04                     shr      dl, 4
00AD43:  75 07                        jne      0xad4c
00AD45:  64 8A 17                     mov      dl, byte ptr fs:[bx]
00AD48:  43                           inc      bx
00AD49:  83 C2 10                     add      dx, 0x10
00AD4C:  83 C2 04                     add      dx, 4
00AD4F:  3B D1                        cmp      dx, cx
00AD51:  76 0D                        jbe      0xad60
00AD53:  2B D1                        sub      dx, cx
00AD55:  0A C0                        or       al, al
00AD57:  74 02                        je       0xad5b
00AD59:  F3 AA                        rep stosb byte ptr es:[di], al
00AD5B:  E8 38 00                     call     0xad96
00AD5E:  EB EF                        jmp      0xad4f
00AD60:  0A C0                        or       al, al
00AD62:  74 1B                        je       0xad7f
00AD64:  87 D1                        xchg     cx, dx
00AD66:  2B D1                        sub      dx, cx
00AD68:  F3 AA                        rep stosb byte ptr es:[di], al
00AD6A:  8B CA                        mov      cx, dx
00AD6C:  5A                           pop      dx
00AD6D:  80 E2 0F                     and      dl, 0xf
00AD70:  83 C2 04                     add      dx, 4
00AD73:  87 56 FE                     xchg     word ptr [bp - 2], dx
00AD76:  0B C9                        or       cx, cx
00AD78:  0F 85 6C FE                  jne      0xabe8
00AD7C:  E9 40 FE                     jmp      0xabbf
00AD7F:  2B CA                        sub      cx, dx
00AD81:  03 FA                        add      di, dx
00AD83:  5A                           pop      dx
00AD84:  80 E2 0F                     and      dl, 0xf
00AD87:  83 C2 04                     add      dx, 4
00AD8A:  87 56 FE                     xchg     word ptr [bp - 2], dx
00AD8D:  0B C9                        or       cx, cx
00AD8F:  0F 85 55 FE                  jne      0xabe8
00AD93:  E9 29 FE                     jmp      0xabbf
; -- non-contiguous block: next 0x00adaf --
00ADAF:  C9                           leave   
00ADB0:  1F                           pop      ds
00ADB1:  C3                           ret     
00ADB2:  EB 0F                        jmp      0xadc3
00ADB4:  FE 4E FA                     dec      byte ptr [bp - 6]
00ADB7:  74 F6                        je       0xadaf
00ADB9:  8B 7E F8                     mov      di, word ptr [bp - 8]
00ADBC:  81 C7 40 01                  add      di, 0x140
00ADC0:  8B 4E F6                     mov      cx, word ptr [bp - 0xa]
00ADC3:  89 7E F8                     mov      word ptr [bp - 8], di
00ADC6:  EB 15                        jmp      0xaddd
00ADC8:  64 8B 17                     mov      dx, word ptr fs:[bx]
00ADCB:  43                           inc      bx
00ADCC:  13 D2                        adc      dx, dx
00ADCE:  43                           inc      bx
00ADCF:  72 12                        jb       0xade3
00ADD1:  AC                           lodsb    al, byte ptr [si]
00ADD2:  0A C0                        or       al, al
00ADD4:  74 03                        je       0xadd9
00ADD6:  26 88 05                     mov      byte ptr es:[di], al
00ADD9:  47                           inc      di
00ADDA:  49                           dec      cx
00ADDB:  74 D7                        je       0xadb4
00ADDD:  03 D2                        add      dx, dx
00ADDF:  73 F0                        jae      0xadd1
00ADE1:  74 E5                        je       0xadc8
00ADE3:  AC                           lodsb    al, byte ptr [si]
00ADE4:  8A E0                        mov      ah, al
00ADE6:  03 D2                        add      dx, dx
00ADE8:  0F 82 AA 00                  jb       0xae96
00ADEC:  87 56 FE                     xchg     word ptr [bp - 2], dx
00ADEF:  83 FA 04                     cmp      dx, 4
00ADF2:  72 3B                        jb       0xae2f
00ADF4:  75 07                        jne      0xadfd
00ADF6:  64 8A 17                     mov      dl, byte ptr fs:[bx]
00ADF9:  43                           inc      bx
00ADFA:  83 C2 14                     add      dx, 0x14
00ADFD:  3B D1                        cmp      dx, cx
00ADFF:  76 0D                        jbe      0xae0e
00AE01:  2B D1                        sub      dx, cx
00AE03:  0A C0                        or       al, al
00AE05:  74 02                        je       0xae09
00AE07:  F3 AA                        rep stosb byte ptr es:[di], al
00AE09:  E8 8A FF                     call     0xad96
00AE0C:  EB EF                        jmp      0xadfd
00AE0E:  0A C0                        or       al, al
00AE10:  74 0F                        je       0xae21
00AE12:  87 D1                        xchg     cx, dx
00AE14:  2B D1                        sub      dx, cx
00AE16:  F3 AA                        rep stosb byte ptr es:[di], al
00AE18:  87 D1                        xchg     cx, dx
00AE1A:  87 56 FE                     xchg     word ptr [bp - 2], dx
00AE1D:  75 BE                        jne      0xaddd
00AE1F:  EB 93                        jmp      0xadb4
00AE21:  03 FA                        add      di, dx
00AE23:  2B CA                        sub      cx, dx
00AE25:  BA 00 00                     mov      dx, 0
00AE28:  87 56 FE                     xchg     word ptr [bp - 2], dx
00AE2B:  75 B0                        jne      0xaddd
00AE2D:  EB 85                        jmp      0xadb4
00AE2F:  64 8A 17                     mov      dl, byte ptr fs:[bx]
00AE32:  43                           inc      bx
00AE33:  52                           push     dx
00AE34:  C0 EA 04                     shr      dl, 4
00AE37:  75 07                        jne      0xae40
00AE39:  64 8A 17                     mov      dl, byte ptr fs:[bx]
00AE3C:  43                           inc      bx
00AE3D:  83 C2 10                     add      dx, 0x10
00AE40:  83 C2 04                     add      dx, 4
00AE43:  3B D1                        cmp      dx, cx
00AE45:  76 0D                        jbe      0xae54
00AE47:  2B D1                        sub      dx, cx
00AE49:  0A C0                        or       al, al
00AE4B:  74 02                        je       0xae4f
00AE4D:  F3 AA                        rep stosb byte ptr es:[di], al
00AE4F:  E8 44 FF                     call     0xad96
00AE52:  EB EF                        jmp      0xae43
00AE54:  0A C0                        or       al, al
00AE56:  74 1B                        je       0xae73
00AE58:  87 D1                        xchg     cx, dx
00AE5A:  2B D1                        sub      dx, cx
00AE5C:  F3 AA                        rep stosb byte ptr es:[di], al
00AE5E:  8B CA                        mov      cx, dx
00AE60:  5A                           pop      dx
00AE61:  80 E2 0F                     and      dl, 0xf
00AE64:  83 C2 04                     add      dx, 4
00AE67:  87 56 FE                     xchg     word ptr [bp - 2], dx
00AE6A:  0B C9                        or       cx, cx
00AE6C:  0F 85 6D FF                  jne      0xaddd
00AE70:  E9 41 FF                     jmp      0xadb4
00AE73:  2B CA                        sub      cx, dx
00AE75:  03 FA                        add      di, dx
00AE77:  5A                           pop      dx
00AE78:  80 E2 0F                     and      dl, 0xf
00AE7B:  83 C2 04                     add      dx, 4
00AE7E:  87 56 FE                     xchg     word ptr [bp - 2], dx
00AE81:  0B C9                        or       cx, cx
00AE83:  0F 85 56 FF                  jne      0xaddd
00AE87:  E9 2A FF                     jmp      0xadb4
00AE8A:  64 8B 17                     mov      dx, word ptr fs:[bx]
00AE8D:  43                           inc      bx
00AE8E:  13 D2                        adc      dx, dx
00AE90:  43                           inc      bx
00AE91:  72 05                        jb       0xae98
00AE93:  E9 56 FF                     jmp      0xadec
00AE96:  74 F2                        je       0xae8a
00AE98:  03 D2                        add      dx, dx
00AE9A:  72 45                        jb       0xaee1
00AE9C:  83 F9 02                     cmp      cx, 2
00AE9F:  72 14                        jb       0xaeb5
00AEA1:  0A C0                        or       al, al
00AEA3:  74 03                        je       0xaea8
00AEA5:  26 89 05                     mov      word ptr es:[di], ax
00AEA8:  83 C7 02                     add      di, 2
00AEAB:  83 E9 02                     sub      cx, 2
00AEAE:  0F 85 2B FF                  jne      0xaddd
00AEB2:  E9 FF FE                     jmp      0xadb4
00AEB5:  0A C0                        or       al, al
00AEB7:  74 0F                        je       0xaec8
00AEB9:  26 88 05                     mov      byte ptr es:[di], al
00AEBC:  E8 D7 FE                     call     0xad96
00AEBF:  AA                           stosb    byte ptr es:[di], al
00AEC0:  49                           dec      cx
00AEC1:  0F 85 18 FF                  jne      0xaddd
00AEC5:  E9 EC FE                     jmp      0xadb4
00AEC8:  E8 CB FE                     call     0xad96
00AECB:  47                           inc      di
00AECC:  49                           dec      cx
00AECD:  0F 85 0C FF                  jne      0xaddd
00AED1:  E9 E0 FE                     jmp      0xadb4
00AED4:  64 8B 17                     mov      dx, word ptr fs:[bx]
00AED7:  83 C3 02                     add      bx, 2
00AEDA:  F9                           stc     
00AEDB:  13 D2                        adc      dx, dx
00AEDD:  72 04                        jb       0xaee3
00AEDF:  EB BB                        jmp      0xae9c
00AEE1:  74 F1                        je       0xaed4
00AEE3:  03 D2                        add      dx, dx
00AEE5:  72 4D                        jb       0xaf34
00AEE7:  83 F9 02                     cmp      cx, 2
00AEEA:  76 18                        jbe      0xaf04
00AEEC:  0A C0                        or       al, al
00AEEE:  74 07                        je       0xaef7
00AEF0:  26 88 05                     mov      byte ptr es:[di], al
00AEF3:  26 89 45 01                  mov      word ptr es:[di + 1], ax
00AEF7:  83 C7 03                     add      di, 3
00AEFA:  83 E9 03                     sub      cx, 3
00AEFD:  0F 85 DC FE                  jne      0xaddd
00AF01:  E9 B0 FE                     jmp      0xadb4
00AF04:  74 0C                        je       0xaf12
00AF06:  0A C0                        or       al, al
00AF08:  74 03                        je       0xaf0d
00AF0A:  26 88 05                     mov      byte ptr es:[di], al
00AF0D:  E8 86 FE                     call     0xad96
00AF10:  EB 8A                        jmp      0xae9c
00AF12:  0A C0                        or       al, al
00AF14:  74 0B                        je       0xaf21
00AF16:  26 89 05                     mov      word ptr es:[di], ax
00AF19:  E8 7A FE                     call     0xad96
00AF1C:  AA                           stosb    byte ptr es:[di], al
00AF1D:  49                           dec      cx
00AF1E:  E9 BC FE                     jmp      0xaddd
00AF21:  E8 72 FE                     call     0xad96
00AF24:  47                           inc      di
00AF25:  49                           dec      cx
00AF26:  E9 B4 FE                     jmp      0xaddd
00AF29:  64 8B 17                     mov      dx, word ptr fs:[bx]
00AF2C:  43                           inc      bx
00AF2D:  13 D2                        adc      dx, dx
00AF2F:  43                           inc      bx
00AF30:  72 04                        jb       0xaf36
00AF32:  EB B3                        jmp      0xaee7
00AF34:  74 F3                        je       0xaf29
00AF36:  83 F9 03                     cmp      cx, 3
00AF39:  76 18                        jbe      0xaf53
00AF3B:  0A C0                        or       al, al
00AF3D:  74 07                        je       0xaf46
00AF3F:  26 89 05                     mov      word ptr es:[di], ax
00AF42:  26 89 45 02                  mov      word ptr es:[di + 2], ax
00AF46:  83 C7 04                     add      di, 4
00AF49:  83 E9 04                     sub      cx, 4
00AF4C:  0F 85 8D FE                  jne      0xaddd
00AF50:  E9 61 FE                     jmp      0xadb4
00AF53:  74 1D                        je       0xaf72
00AF55:  49                           dec      cx
00AF56:  74 0D                        je       0xaf65
00AF58:  0A C0                        or       al, al
00AF5A:  74 03                        je       0xaf5f
00AF5C:  26 89 05                     mov      word ptr es:[di], ax
00AF5F:  E8 34 FE                     call     0xad96
00AF62:  E9 37 FF                     jmp      0xae9c
00AF65:  0A C0                        or       al, al
00AF67:  74 03                        je       0xaf6c
00AF69:  26 88 05                     mov      byte ptr es:[di], al
00AF6C:  E8 27 FE                     call     0xad96
00AF6F:  E9 75 FF                     jmp      0xaee7
00AF72:  0A C0                        or       al, al
00AF74:  74 13                        je       0xaf89
00AF76:  26 88 05                     mov      byte ptr es:[di], al
00AF79:  26 89 45 01                  mov      word ptr es:[di + 1], ax
00AF7D:  E8 16 FE                     call     0xad96
00AF80:  AA                           stosb    byte ptr es:[di], al
00AF81:  49                           dec      cx
00AF82:  0F 85 57 FE                  jne      0xaddd
00AF86:  E9 2B FE                     jmp      0xadb4
00AF89:  E8 0A FE                     call     0xad96
00AF8C:  47                           inc      di
00AF8D:  49                           dec      cx
00AF8E:  0F 85 4B FE                  jne      0xaddd
00AF92:  E9 1F FE                     jmp      0xadb4
