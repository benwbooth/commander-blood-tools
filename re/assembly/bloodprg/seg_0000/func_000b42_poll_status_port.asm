; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x000b42
; seg_off: 0000:0542
; group: seg_0000
; provenance: recursive_graph
; label: poll_status_port
; label_comment: poll a hardware status bit: dx=base+6; in al,dx; and al,8 -> mask status bit 3. Polls the VGA vertical-retrace status (or sound-card status) for timing/sync. Also 0x0b4e
; byte_count: 149
; boundary: cfg_blocks_13_terminals_2
; terminal: jmp 0xbcb:1, retf:1
; direct_callees: none
; indirect_calls: 0
; routine_bytes_sha256: 014e4328d0f2ac17161080435af22e14ad3000551ea78d401f72831a08af52fa

000B42:  50                           push     ax
000B43:  53                           push     bx
000B44:  52                           push     dx
000B45:  51                           push     cx
000B46:  65 8B 16 9E 0A               mov      dx, word ptr gs:[0xa9e]
000B4B:  83 C2 06                     add      dx, 6
000B4E:  EC                           in       al, dx
000B4F:  24 08                        and      al, 8
000B51:  8A E0                        mov      ah, al
000B53:  65 C7 06 35 0B 02 00         mov      word ptr gs:[0xb35], 2
000B5A:  65 F7 06 35 0B 03 00         test     word ptr gs:[0xb35], 3
000B61:  74 68                        je       0xbcb
000B63:  EC                           in       al, dx
000B64:  24 08                        and      al, 8
000B66:  32 C4                        xor      al, ah
000B68:  74 F0                        je       0xb5a
000B6A:  65 FE 06 12 0B               inc      byte ptr gs:[0xb12]
000B6F:  E4 61                        in       al, 0x61
000B71:  0C 01                        or       al, 1
000B73:  E6 61                        out      0x61, al
000B75:  B0 B0                        mov      al, 0xb0
000B77:  E6 43                        out      0x43, al
000B79:  B0 FF                        mov      al, 0xff
000B7B:  E6 42                        out      0x42, al
000B7D:  E6 42                        out      0x42, al
000B7F:  EC                           in       al, dx
000B80:  24 08                        and      al, 8
000B82:  0F 95 C1                     setne    cl
000B85:  8A E0                        mov      ah, al
000B87:  EC                           in       al, dx
000B88:  24 08                        and      al, 8
000B8A:  32 C4                        xor      al, ah
000B8C:  74 F9                        je       0xb87
000B8E:  B0 80                        mov      al, 0x80
000B90:  E6 43                        out      0x43, al
000B92:  E4 42                        in       al, 0x42
000B94:  8A D8                        mov      bl, al
000B96:  E4 42                        in       al, 0x42
000B98:  8A F8                        mov      bh, al
000B9A:  F7 DB                        neg      bx
000B9C:  EC                           in       al, dx
000B9D:  24 08                        and      al, 8
000B9F:  8A E0                        mov      ah, al
000BA1:  EC                           in       al, dx
000BA2:  24 08                        and      al, 8
000BA4:  32 C4                        xor      al, ah
000BA6:  74 F9                        je       0xba1
000BA8:  B0 80                        mov      al, 0x80
000BAA:  E6 43                        out      0x43, al
000BAC:  E4 42                        in       al, 0x42
000BAE:  8A E0                        mov      ah, al
000BB0:  E4 42                        in       al, 0x42
000BB2:  86 C4                        xchg     ah, al
000BB4:  F7 D8                        neg      ax
000BB6:  2B C3                        sub      ax, bx
000BB8:  3B C3                        cmp      ax, bx
000BBA:  7F 06                        jg       0xbc2
000BBC:  0A C9                        or       cl, cl
000BBE:  74 06                        je       0xbc6
000BC0:  EB 09                        jmp      0xbcb
000BC2:  0A C9                        or       cl, cl
000BC4:  74 05                        je       0xbcb
000BC6:  65 FE 06 12 0B               inc      byte ptr gs:[0xb12]
000BCB:  65 C7 06 25 0B 03 00         mov      word ptr gs:[0xb25], 3
000BD2:  59                           pop      cx
000BD3:  5A                           pop      dx
000BD4:  5B                           pop      bx
000BD5:  58                           pop      ax
000BD6:  CB                           retf    
