; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x00420d
; seg_off: 0299:127d
; group: seg_0299
; provenance: recursive_graph, relocation_proven_far_transfer_target
; label: sprite_slot_position_update
; label_comment: updates sprite slot +0x08/+0x0A screen position words and sets dirty bit on change || ALSO RECORDED as `sprite_slot_set_draw_position`: SPRITE-SLOT DRAW-POSITION SETTER (CORRECTED - was mislabelled entity_flag_set_if_field8_mismatch, which called +8 'a comparable id/group/target'). Called as lcall 0x299:0x127D from the nav projector 0x9CEF with AX=object id, BX=draw x, CX=draw y. bx=0x6212+(id<<5); ax=gs:[bx] flags; `test al,0x81` requires ACTIVE(0x80)+bit0 else return; then INDEPENDENTLY: if bx != gs:[bx+8] set dirty `or al,2` and store x at +8; if cx != gs:[bx+0xA] set dirty and store y at +0xA; finally write the flags back. So entity fields +0x08/+0x0A ARE the sprite draw X/Y, and 0x02 is the DIRTY bit. Ported exactly: ship3d.rs update_ship_3d_sprite_slot_position (ACTIVE_MASK 0x0081, DIRTY_FLAG 0x0002) || MERGED 2026-07-25 (audit-fixes #184): one address under several names, folded by union.
; incoming: call@0x00929c->0299:127d
; incoming: call@0x009cef->0299:127d
; byte_count: 51
; boundary: cfg_blocks_6_terminals_1
; terminal: retf:1
; direct_callees: none
; indirect_calls: 0
; routine_bytes_sha256: b06aba7a8862bde0678fdb2cab5e6e25126e627df1a0c73bf442be4dd583b6df

00420D:  50                           push     ax
00420E:  53                           push     bx
00420F:  52                           push     dx
004210:  C1 E0 05                     shl      ax, 5
004213:  8B D3                        mov      dx, bx
004215:  BB 12 62                     mov      bx, 0x6212
004218:  03 D8                        add      bx, ax
00421A:  65 8B 07                     mov      ax, word ptr gs:[bx]
00421D:  A8 81                        test     al, 0x81
00421F:  74 18                        je       0x4239
004221:  65 3B 57 08                  cmp      dx, word ptr gs:[bx + 8]
004225:  74 06                        je       0x422d
004227:  0C 02                        or       al, 2
004229:  65 89 57 08                  mov      word ptr gs:[bx + 8], dx
00422D:  65 3B 4F 0A                  cmp      cx, word ptr gs:[bx + 0xa]
004231:  74 06                        je       0x4239
004233:  0C 02                        or       al, 2
004235:  65 89 4F 0A                  mov      word ptr gs:[bx + 0xa], cx
004239:  65 89 07                     mov      word ptr gs:[bx], ax
00423C:  5A                           pop      dx
00423D:  5B                           pop      bx
00423E:  58                           pop      ax
00423F:  CB                           retf    
