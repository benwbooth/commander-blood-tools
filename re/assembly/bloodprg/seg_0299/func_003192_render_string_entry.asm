; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x003192
; seg_off: 0299:0202
; group: seg_0299
; provenance: recursive_graph, relocation_proven_far_transfer_target
; label: render_string_entry
; label_comment: STRING DRAW entry (0x299:0x202): zeroes gs:0x27CD, clips, then runs render_string 0x31C8. gs:0x27CD accumulates the drawn width -- but ONLY over glyphs (`add word gs:[0x27cd],ax` @0x3215). The SPACE path (`add di,6 / jmp` @0x31D7) moves the pen WITHOUT touching it, and unmapped bytes are skipped entirely, so the reported width is not the pen distance. Ported: font.rs game_font_drawn_width || NARROWER EARLIER READING `clipped_blit_w8_a`: clipped blit: cmp dx,gs:[0x523b] (clip Y to screen height); reject if below; cx=gs:[0x5239]-8 (clip X to width, 8-px span). Bounds-checked span copy into the display page || MERGED 2026-07-25 (audit-fixes #133): one address, two names, the shorter describing a prologue or a single facet. Kept because a narrow reading records a true observation; renamed away because it is not what the routine IS.
; incoming: call@0x008feb->0299:0202
; incoming: call@0x009183->0299:0202
; incoming: call@0x009199->0299:0202
; incoming: call@0x0091a8->0299:0202
; incoming: call@0x0091e4->0299:0202
; byte_count: 147
; boundary: cfg_blocks_14_terminals_4
; terminal: jmp 0x31ce:2, jmp 0x31fb:1, retf:1
; direct_callees: none
; indirect_calls: 0
; routine_bytes_sha256: 2c83af0acf062fa3fa48bb64c99d7af3f1e36640ac25ef84cf9a0ddf4d06d3b2

003192:  50                           push     ax
003193:  53                           push     bx
003194:  51                           push     cx
003195:  52                           push     dx
003196:  56                           push     si
003197:  06                           push     es
003198:  57                           push     di
003199:  55                           push     bp
00319A:  65 C7 06 CD 27 00 00         mov      word ptr gs:[0x27cd], 0
0031A1:  65 3B 16 3B 52               cmp      dx, word ptr gs:[0x523b]
0031A6:  77 74                        ja       0x321c
0031A8:  65 8B 0E 39 52               mov      cx, word ptr gs:[0x5239]
0031AD:  83 E9 08                     sub      cx, 8
0031B0:  3B D1                        cmp      dx, cx
0031B2:  7E 68                        jle      0x321c
0031B4:  50                           push     ax
0031B5:  65 C4 3E 21 52               les      di, ptr gs:[0x5221]
0031BA:  8B C2                        mov      ax, dx
0031BC:  86 C4                        xchg     ah, al
0031BE:  C1 E2 06                     shl      dx, 6
0031C1:  03 C2                        add      ax, dx
0031C3:  03 C3                        add      ax, bx
0031C5:  03 F8                        add      di, ax
0031C7:  5A                           pop      dx
0031C8:  BB 02 78                     mov      bx, 0x7802
0031CB:  66 33 C0                     xor      eax, eax
0031CE:  AC                           lodsb    al, byte ptr [si]
0031CF:  0A C0                        or       al, al
0031D1:  74 49                        je       0x321c
0031D3:  3C 20                        cmp      al, 0x20
0031D5:  75 05                        jne      0x31dc
0031D7:  83 C7 06                     add      di, 6
0031DA:  EB F2                        jmp      0x31ce
0031DC:  65 D7                        xlatb   
0031DE:  0A C0                        or       al, al
0031E0:  78 EC                        js       0x31ce
0031E2:  98                           cwde    
0031E3:  67 65 8A B0 B2 78 00 00      mov      dh, byte ptr gs:[eax + 0x78b2]
0031EB:  BD 08 79                     mov      bp, 0x7908
0031EE:  C1 E0 03                     shl      ax, 3
0031F1:  03 E8                        add      bp, ax
0031F3:  B9 08 00                     mov      cx, 8
0031F6:  57                           push     di
0031F7:  8A 46 00                     mov      al, byte ptr [bp]
0031FA:  57                           push     di
0031FB:  D0 E0                        shl      al, 1
0031FD:  73 03                        jae      0x3202
0031FF:  26 88 15                     mov      byte ptr es:[di], dl
003202:  74 03                        je       0x3207
003204:  47                           inc      di
003205:  EB F4                        jmp      0x31fb
003207:  5F                           pop      di
003208:  81 C7 40 01                  add      di, 0x140
00320C:  45                           inc      bp
00320D:  E2 E8                        loop     0x31f7
00320F:  5F                           pop      di
003210:  8A C6                        mov      al, dh
003212:  98                           cwde    
003213:  03 F8                        add      di, ax
003215:  65 01 06 CD 27               add      word ptr gs:[0x27cd], ax
00321A:  EB B2                        jmp      0x31ce
00321C:  5D                           pop      bp
00321D:  5F                           pop      di
00321E:  07                           pop      es
00321F:  5E                           pop      si
003220:  5A                           pop      dx
003221:  59                           pop      cx
003222:  5B                           pop      bx
003223:  58                           pop      ax
003224:  CB                           retf    
