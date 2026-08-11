; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x00099f
; seg_off: 0000:039f
; group: seg_0000
; provenance: recursive_graph
; label: get_ems_int_vector
; label_comment: startup: int21h ax=0x3567 (get interrupt vector 0x67 = the EMS driver entry); di=0xa. EMS (int 67h) detection/setup - the large-memory banking the gs:0xd8c/ems_paged_read subsystem uses
; byte_count: 250
; boundary: cfg_blocks_25_terminals_1
; terminal: retf:1
; direct_callees: none
; indirect_calls: 4
; routine_bytes_sha256: b54827c41368004235c8a04276c50ac29230fed5e95bd77460c7310bee245314

00099F:  66 50                        push     eax
0009A1:  53                           push     bx
0009A2:  1E                           push     ds
0009A3:  56                           push     si
0009A4:  06                           push     es
0009A5:  57                           push     di
0009A6:  B8 67 35                     mov      ax, 0x3567
0009A9:  CD 21                        int      0x21
0009AB:  BF 0A 00                     mov      di, 0xa
0009AE:  8C C8                        mov      ax, cs
0009B0:  8E D8                        mov      ds, ax
0009B2:  BE 97 03                     mov      si, 0x397
0009B5:  B9 08 00                     mov      cx, 8
0009B8:  F3 A6                        repe cmpsb byte ptr [si], byte ptr es:[di]
0009BA:  75 51                        jne      0xa0d
0009BC:  B4 40                        mov      ah, 0x40
0009BE:  CD 67                        int      0x67
0009C0:  0A E4                        or       ah, ah
0009C2:  75 49                        jne      0xa0d
0009C4:  BB 04 00                     mov      bx, 4
0009C7:  B4 43                        mov      ah, 0x43
0009C9:  CD 67                        int      0x67
0009CB:  0A E4                        or       ah, ah
0009CD:  75 05                        jne      0x9d4
0009CF:  65 89 16 64 0A               mov      word ptr gs:[0xa64], dx
0009D4:  BB 10 00                     mov      bx, 0x10
0009D7:  B4 43                        mov      ah, 0x43
0009D9:  CD 67                        int      0x67
0009DB:  0A E4                        or       ah, ah
0009DD:  75 05                        jne      0x9e4
0009DF:  65 89 16 58 0A               mov      word ptr gs:[0xa58], dx
0009E4:  BB 10 00                     mov      bx, 0x10
0009E7:  B4 43                        mov      ah, 0x43
0009E9:  CD 67                        int      0x67
0009EB:  0A E4                        or       ah, ah
0009ED:  75 05                        jne      0x9f4
0009EF:  65 89 16 5C 0A               mov      word ptr gs:[0xa5c], dx
0009F4:  BB 5A 00                     mov      bx, 0x5a
0009F7:  B4 43                        mov      ah, 0x43
0009F9:  CD 67                        int      0x67
0009FB:  0A E4                        or       ah, ah
0009FD:  75 05                        jne      0xa04
0009FF:  65 89 16 60 0A               mov      word ptr gs:[0xa60], dx
000A04:  B4 41                        mov      ah, 0x41
000A06:  CD 67                        int      0x67
000A08:  65 89 1E 66 0A               mov      word ptr gs:[0xa66], bx
000A0D:  B8 00 43                     mov      ax, 0x4300
000A10:  CD 2F                        int      0x2f
000A12:  3C 80                        cmp      al, 0x80
000A14:  75 7B                        jne      0xa91
000A16:  B8 10 43                     mov      ax, 0x4310
000A19:  CD 2F                        int      0x2f
000A1B:  65 89 1E 4A 0A               mov      word ptr gs:[0xa4a], bx
000A20:  65 8C 06 4C 0A               mov      word ptr gs:[0xa4c], es
000A25:  65 83 3E 64 0A FF            cmp      word ptr gs:[0xa64], -1
000A2B:  75 13                        jne      0xa40
000A2D:  BA 40 00                     mov      dx, 0x40
000A30:  B4 09                        mov      ah, 9
000A32:  65 FF 1E 4A 0A               lcall    gs:[0xa4a]
000A37:  0B C0                        or       ax, ax
000A39:  74 05                        je       0xa40
000A3B:  65 89 16 62 0A               mov      word ptr gs:[0xa62], dx
000A40:  65 83 3E 58 0A FF            cmp      word ptr gs:[0xa58], -1
000A46:  75 13                        jne      0xa5b
000A48:  BA 00 01                     mov      dx, 0x100
000A4B:  B4 09                        mov      ah, 9
000A4D:  65 FF 1E 4A 0A               lcall    gs:[0xa4a]
000A52:  0B C0                        or       ax, ax
000A54:  74 05                        je       0xa5b
000A56:  65 89 16 56 0A               mov      word ptr gs:[0xa56], dx
000A5B:  65 83 3E 5C 0A FF            cmp      word ptr gs:[0xa5c], -1
000A61:  75 13                        jne      0xa76
000A63:  BA 00 01                     mov      dx, 0x100
000A66:  B4 09                        mov      ah, 9
000A68:  65 FF 1E 4A 0A               lcall    gs:[0xa4a]
000A6D:  0B C0                        or       ax, ax
000A6F:  74 05                        je       0xa76
000A71:  65 89 16 5A 0A               mov      word ptr gs:[0xa5a], dx
000A76:  65 83 3E 60 0A FF            cmp      word ptr gs:[0xa60], -1
000A7C:  75 13                        jne      0xa91
000A7E:  BA A0 05                     mov      dx, 0x5a0
000A81:  B4 09                        mov      ah, 9
000A83:  65 FF 1E 4A 0A               lcall    gs:[0xa4a]
000A88:  0B C0                        or       ax, ax
000A8A:  74 05                        je       0xa91
000A8C:  65 89 16 5E 0A               mov      word ptr gs:[0xa5e], dx
000A91:  5F                           pop      di
000A92:  07                           pop      es
000A93:  5E                           pop      si
000A94:  1F                           pop      ds
000A95:  5B                           pop      bx
000A96:  66 58                        pop      eax
000A98:  CB                           retf    
