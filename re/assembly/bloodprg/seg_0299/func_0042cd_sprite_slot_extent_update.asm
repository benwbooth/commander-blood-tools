; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x0042cd
; seg_off: 0299:133d
; group: seg_0299
; provenance: recursive_graph, relocation_proven_far_transfer_target
; label: sprite_slot_extent_update
; label_comment: updates sprite slot +0x0C/+0x0E extent words and sets dirty/source-change bits on change || ALSO RECORDED as `subobject_sprite_render`: 0x299:0x133D (called from entity_draw with cx,dx = scaled coords): the SUB-OBJECT SPRITE renderer — the candidate consumer of the world payload's 0x80|node reference walks (sprite-strip descriptors). Pinned as the ext-consumer trace's next hop || ALSO RECORDED as `entity_dirty_track`: CORRECTED (was mis-labeled set-target/stepper): [bx+0xC]/[bx+0xE] = the entity's LAST screen position; entity_draw compares the camera-derived scaled coords (cx,dx) to it and sets redraw flags (0x12) on change = DIRTY-RECT tracking. Entities are STATIC in world space (drawn at their .ext positions); only the camera moves. There is NO entity walk/stepper — that gated row was a misreading and is RETIRED || ALSO RECORDED as `sprite_slot_set_extent`: SPRITE-SLOT EXTENT SETTER (CORRECTED - was mislabelled entity_step_pinned 'the per-frame position STEPPER'). Called as lcall 0x299:0x133D from the nav projector 0x9CD6 with AX=object id, CX=scaled width, DX=scaled height (computed just above at 0x9CB2..0x9CCC as source dim * depth_scale then shrd 0xA). bx=0x6212+(id<<5); `test al,0x81` active gate; lds si,[bp+4] = the SOURCE descriptor. If CX==[si] AND DX==[si+2] (scaled == source): `btr ax,4` clears the EXTENT-CHANGED bit and, ONLY if it had been set (CF), `or al,2` marks dirty. Otherwise if CX/DX differ from gs:[bx+0xC]/gs:[bx+0xE]: `or al,0x12` (dirty 0x02 | extent-changed 0x10) and store the new extent at +0xC/+0xE. So entity fields +0x0C/+0x0E are the sprite EXTENT (w/h). Ported exactly: ship3d.rs update_ship_3d_sprite_slot_extent (EXTENT_CHANGED_FLAG 0x0010) || MERGED 2026-07-25 (audit-fixes #184): one address under several names, folded by union.
; incoming: call@0x00926d->0299:133d
; incoming: call@0x009cd6->0299:133d
; byte_count: 73
; boundary: cfg_blocks_9_terminals_2
; terminal: jmp 0x430d:1, retf:1
; direct_callees: none
; indirect_calls: 0
; cxx_source: re/borland/bloodprg/seg_0299/func_0042cd_sprite_slot_extent_update.cpp
; routine_bytes_sha256: 463f6e4fbb383556b88c63c4c2bc5d4cc6a37ee1694dc2a29da80b8395dfb225

0042CD:  66 50                        push     eax
0042CF:  53                           push     bx
0042D0:  1E                           push     ds
0042D1:  56                           push     si
0042D2:  C1 E0 05                     shl      ax, 5
0042D5:  BB 12 62                     mov      bx, 0x6212
0042D8:  03 D8                        add      bx, ax
0042DA:  65 8B 07                     mov      ax, word ptr gs:[bx]
0042DD:  A8 81                        test     al, 0x81
0042DF:  74 2C                        je       0x430d
0042E1:  C5 76 04                     lds      si, ptr [bp + 4]
0042E4:  3B 0C                        cmp      cx, word ptr [si]
0042E6:  75 0F                        jne      0x42f7
0042E8:  3B 54 02                     cmp      dx, word ptr [si + 2]
0042EB:  75 0A                        jne      0x42f7
0042ED:  0F BA F0 04                  btr      ax, 4
0042F1:  73 1A                        jae      0x430d
0042F3:  0C 02                        or       al, 2
0042F5:  EB 16                        jmp      0x430d
0042F7:  65 3B 4F 0C                  cmp      cx, word ptr gs:[bx + 0xc]
0042FB:  75 06                        jne      0x4303
0042FD:  65 3B 57 0E                  cmp      dx, word ptr gs:[bx + 0xe]
004301:  74 0A                        je       0x430d
004303:  0C 12                        or       al, 0x12
004305:  65 89 4F 0C                  mov      word ptr gs:[bx + 0xc], cx
004309:  65 89 57 0E                  mov      word ptr gs:[bx + 0xe], dx
00430D:  65 89 07                     mov      word ptr gs:[bx], ax
004310:  5E                           pop      si
004311:  1F                           pop      ds
004312:  5B                           pop      bx
004313:  66 58                        pop      eax
004315:  CB                           retf    
