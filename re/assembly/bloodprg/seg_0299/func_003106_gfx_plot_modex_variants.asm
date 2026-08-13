; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x003106
; seg_off: 0299:0176
; group: seg_0299
; provenance: recursive_graph, relocation_proven_far_transfer_target
; label: square_caps_text_draw_display
; label_comment: Variable-width square-caps text renderer. DS:SI=text, BX=x, DX=y, AL=color. Clears GS:0x27CD, clips the 10-row glyphs against GS:0x5239/0x523B, maps characters through GS:0x7362, reads signed advances from GS:0x7412, expands ten big-endian 16-bit glyph rows from SS:0x7442+index*20 onto GS:[0x5221] with transparent backgrounds, and preserves all 16-bit inputs. Natural C and raw vectors: re/source/bloodprg/candidates/seg_0299/func_003106_square_caps_text_draw_display.c and re/tools/oracle_vectors/func_3106_natural.json
; incoming: call@0x001507->0299:0176
; incoming: call@0x001515->0299:0176
; incoming: call@0x001520->0299:0176
; incoming: call@0x001e4f->0299:0176
; incoming: call@0x008597->0299:0176
; incoming: call@0x0085ce->0299:0176
; byte_count: 140
; boundary: cfg_blocks_11_terminals_3
; terminal: jmp 0x3142:1, jmp 0x3166:1, retf:1
; direct_callees: none
; indirect_calls: 0
; routine_bytes_sha256: 2658c6fa6895f1c5bb2d16742d0980db5660d63384cf926bc7efb42b0d4f049d

003106:  50                           push     ax
003107:  53                           push     bx
003108:  51                           push     cx
003109:  52                           push     dx
00310A:  56                           push     si
00310B:  06                           push     es
00310C:  57                           push     di
00310D:  55                           push     bp
00310E:  65 C7 06 CD 27 00 00         mov      word ptr gs:[0x27cd], 0
003115:  65 3B 16 3B 52               cmp      dx, word ptr gs:[0x523b]
00311A:  77 6D                        ja       0x3189
00311C:  65 8B 0E 39 52               mov      cx, word ptr gs:[0x5239]
003121:  83 E9 0A                     sub      cx, 0xa
003124:  3B D1                        cmp      dx, cx
003126:  7E 61                        jle      0x3189
003128:  50                           push     ax
003129:  65 C4 3E 21 52               les      di, ptr gs:[0x5221]
00312E:  8B C2                        mov      ax, dx
003130:  86 C4                        xchg     ah, al
003132:  C1 E2 06                     shl      dx, 6
003135:  03 C2                        add      ax, dx
003137:  03 C3                        add      ax, bx
003139:  03 F8                        add      di, ax
00313B:  5A                           pop      dx
00313C:  BB 62 73                     mov      bx, 0x7362
00313F:  66 33 C0                     xor      eax, eax
003142:  AC                           lodsb    al, byte ptr [si]
003143:  0A C0                        or       al, al
003145:  74 42                        je       0x3189
003147:  65 D7                        xlatb   
003149:  98                           cwde    
00314A:  67 65 8A B0 12 74 00 00      mov      dh, byte ptr gs:[eax + 0x7412]
003152:  BD 42 74                     mov      bp, 0x7442
003155:  B9 14 00                     mov      cx, 0x14
003158:  F6 E1                        mul      cl
00315A:  03 E8                        add      bp, ax
00315C:  B9 0A 00                     mov      cx, 0xa
00315F:  57                           push     di
003160:  8B 46 00                     mov      ax, word ptr [bp]
003163:  86 C4                        xchg     ah, al
003165:  57                           push     di
003166:  D1 E0                        shl      ax, 1
003168:  73 03                        jae      0x316d
00316A:  26 88 15                     mov      byte ptr es:[di], dl
00316D:  74 03                        je       0x3172
00316F:  47                           inc      di
003170:  EB F4                        jmp      0x3166
003172:  5F                           pop      di
003173:  81 C7 40 01                  add      di, 0x140
003177:  83 C5 02                     add      bp, 2
00317A:  E2 E4                        loop     0x3160
00317C:  5F                           pop      di
00317D:  8A C6                        mov      al, dh
00317F:  98                           cwde    
003180:  03 F8                        add      di, ax
003182:  65 01 06 CD 27               add      word ptr gs:[0x27cd], ax
003187:  EB B9                        jmp      0x3142
003189:  5D                           pop      bp
00318A:  5F                           pop      di
00318B:  07                           pop      es
00318C:  5E                           pop      si
00318D:  5A                           pop      dx
00318E:  59                           pop      cx
00318F:  5B                           pop      bx
003190:  58                           pop      ax
003191:  CB                           retf    
