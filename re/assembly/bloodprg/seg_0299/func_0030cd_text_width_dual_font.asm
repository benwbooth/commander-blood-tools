; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x0030cd
; seg_off: 0299:013d
; group: seg_0299
; provenance: recursive_graph, relocation_proven_far_transfer_target
; label: text_width_dual_font
; label_comment: STRING WIDTH, TWO FACES selected by AX on entry: ax==0 -> xlat DS:0x7362 + advances DS:0x7412 (square caps); ax!=0 -> xlat DS:0x7802 + advances DS:0x78B2 (the main font). Loop: lodsb / xlatb / `add dl,gs:[eax+edi]` / `adc dh,0`, then `sub ax,2` @0x30FE. UNLIKE render_string 0x3192 it has NO space case and NO 0xFF skip, so an unmapped char adds the byte at advances+0xFF (inside the glyph rows) -- and a space adds its table advance rather than a fixed 6. Callers: 0x846C ax=0 (save-slot widget), 0x7329 ax=1 (the pixel wrap layout), 0x8FCD ax=1 || CORRECTS the earlier name `divmod_setup_30e2` ("arithmetic setup: xor dx,dx; test ax; branch to 0x30e2 -- zero/sign guard before a 32-bit divide in the blit-address math"), which read the AX test as a divide guard. There is no divide: `or ax,ax / jne 0x30E2` SELECTS A FONT, and `xor dx,dx` zeroes the width accumulator. MERGED 2026-07-27 (audit-fixes #583).
; incoming: call@0x007329->0299:013d
; incoming: call@0x00846c->0299:013d
; incoming: call@0x008fcd->0299:013d
; byte_count: 57
; boundary: cfg_blocks_6_terminals_3
; terminal: jmp 0x30eb:2, retf:1
; direct_callees: none
; indirect_calls: 0
; routine_bytes_sha256: 9ac3b22669fd6fa9c42530b56ea40a5ff96c3dab207d403aceb72bdb27256748

0030CD:  53                           push     bx
0030CE:  52                           push     dx
0030CF:  57                           push     di
0030D0:  56                           push     si
0030D1:  33 D2                        xor      dx, dx
0030D3:  0B C0                        or       ax, ax
0030D5:  75 0B                        jne      0x30e2
0030D7:  66 33 C0                     xor      eax, eax
0030DA:  BB 62 73                     mov      bx, 0x7362
0030DD:  BF 12 74                     mov      di, 0x7412
0030E0:  EB 09                        jmp      0x30eb
0030E2:  66 33 C0                     xor      eax, eax
0030E5:  BB 02 78                     mov      bx, 0x7802
0030E8:  BF B2 78                     mov      di, 0x78b2
0030EB:  AC                           lodsb    al, byte ptr [si]
0030EC:  0A C0                        or       al, al
0030EE:  74 0C                        je       0x30fc
0030F0:  65 D7                        xlatb   
0030F2:  67 65 02 14 38               add      dl, byte ptr gs:[eax + edi]
0030F7:  80 D6 00                     adc      dh, 0
0030FA:  EB EF                        jmp      0x30eb
0030FC:  8B C2                        mov      ax, dx
0030FE:  83 E8 02                     sub      ax, 2
003101:  5E                           pop      si
003102:  5F                           pop      di
003103:  5A                           pop      dx
003104:  5B                           pop      bx
003105:  CB                           retf    
