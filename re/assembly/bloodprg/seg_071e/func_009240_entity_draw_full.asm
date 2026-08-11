; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x009240
; seg_off: 071e:1a60
; group: seg_071e
; provenance: recursive_graph
; label: entity_draw_full
; label_comment: entity render (FULL): les di,[0x6212+4] (entity data far ptr); scale bh=(3*[0x2789])/2+1; x=es:[di]*bh>>4 (cx), y=es:[di+2]*bh>>4 (dx); lcall 0x299:0x133d (draw at cx,dx). So the entity's DATA (the .ext-provided object data at record +0x04) is a COORDINATE record: +0x00 = x, +0x02 = y, scaled by the [0x2789] zoom factor. Connects the .ext object data -> entity_object_table -> scaled screen draw. The object-position render path || ALSO RECORDED as `entity_draw`: entity render/process (2 calls): si=0x6212 (entity_object_table); les di,[si+4] (the entity's data far pointer at record +0x04/+0x06); bh=[0x2789]; al=3 (draw mode). Reads a loaded entity's data pointer + renders it - the object-consumer/draw that walks entity_object_table (the per-object draw sought earlier). Connects the object-instance system to the render path || MERGED 2026-07-25 (#186): one address, several names, folded by union.
; byte_count: 99
; boundary: cfg_blocks_1_terminals_1
; terminal: ret:1
; direct_callees: none
; indirect_calls: 2
; cxx_source: re/borland/bloodprg/seg_071e/func_009240_entity_draw_full.cpp
; routine_bytes_sha256: 8a64f53286b9dab1afe8a31223091c0e2037b2c5bf5bc10e0da3c83c0b9c8462

009240:  06                           push     es
009241:  BE 12 62                     mov      si, 0x6212
009244:  C4 7C 04                     les      di, ptr [si + 4]
009247:  8A 3E 89 27                  mov      bh, byte ptr [0x2789]
00924B:  B0 03                        mov      al, 3
00924D:  F6 E7                        mul      bh
00924F:  8A F8                        mov      bh, al
009251:  D0 EF                        shr      bh, 1
009253:  FE C7                        inc      bh
009255:  53                           push     bx
009256:  26 8B 05                     mov      ax, word ptr es:[di]
009259:  F6 E7                        mul      bh
00925B:  C1 E8 04                     shr      ax, 4
00925E:  8B C8                        mov      cx, ax
009260:  26 8B 45 02                  mov      ax, word ptr es:[di + 2]
009264:  F6 E7                        mul      bh
009266:  C1 E8 04                     shr      ax, 4
009269:  8B D0                        mov      dx, ax
00926B:  33 C0                        xor      ax, ax
00926D:  9A 3D 13 99 02               lcall    0x299, 0x133d
009272:  5A                           pop      dx
009273:  B2 0D                        mov      dl, 0xd
009275:  8B 1E AB 2A                  mov      bx, word ptr [0x2aab]
009279:  A1 80 27                     mov      ax, word ptr [0x2780]
00927C:  2B 06 7E 27                  sub      ax, word ptr [0x277e]
009280:  2B C3                        sub      ax, bx
009282:  F6 FA                        idiv     dl
009284:  F6 EE                        imul     dh
009286:  03 D8                        add      bx, ax
009288:  8B 0E AD 2A                  mov      cx, word ptr [0x2aad]
00928C:  A1 82 27                     mov      ax, word ptr [0x2782]
00928F:  83 C0 0A                     add      ax, 0xa
009292:  2B C1                        sub      ax, cx
009294:  F6 FA                        idiv     dl
009296:  F6 EE                        imul     dh
009298:  03 C8                        add      cx, ax
00929A:  33 C0                        xor      ax, ax
00929C:  9A 7D 12 99 02               lcall    0x299, 0x127d
0092A1:  07                           pop      es
0092A2:  C3                           ret     
