; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x0041d1
; seg_off: 0299:1241
; group: seg_0299
; provenance: recursive_graph, relocation_proven_far_transfer_target
; label: entity_flag_state_transition
; label_comment: AX=object id: read flags gs:[0x6212+id<<5]; if active (bit7/0x80 set, js) AND bit0(0x01) set, clear bit0 and set bit1 (al=(al&0xfe)|2); store back. Per-object flag state machine - entity flags +0x00 bits: 0x80=active, 0x01/0x02 = state (0x83 init = active+state0+bit1). Transitions state0->state-advance
; incoming: call@0x00186f->0299:1241
; incoming: call@0x001877->0299:1241
; incoming: call@0x0059dc->0299:1241
; incoming: call@0x0059e4->0299:1241
; incoming: call@0x005e49->0299:1241
; incoming: call@0x007001->0299:1241
; incoming: call@0x007a22->0299:1241
; incoming: call@0x007dff->0299:1241
; incoming: call@0x007f5a->0299:1241
; incoming: call@0x007f82->0299:1241
; incoming: call@0x007fc7->0299:1241
; incoming: call@0x007fcf->0299:1241
; incoming: call@0x007ffd->0299:1241
; incoming: call@0x008041->0299:1241
; incoming: call@0x008068->0299:1241
; incoming: call@0x0080b6->0299:1241
; incoming: call@0x008134->0299:1241
; incoming: call@0x0081c8->0299:1241
; incoming: call@0x008253->0299:1241
; incoming: call@0x008aed->0299:1241
; incoming: call@0x008b44->0299:1241
; incoming: call@0x008dac->0299:1241
; incoming: call@0x008dfd->0299:1241
; incoming: call@0x008e0d->0299:1241
; incoming: call@0x008e18->0299:1241
; incoming: call@0x008eb7->0299:1241
; incoming: call@0x008ec7->0299:1241
; incoming: call@0x00905b->0299:1241
; incoming: call@0x00906b->0299:1241
; incoming: call@0x009212->0299:1241
; incoming: call@0x0095f5->0299:1241
; incoming: call@0x00afb9->0299:1241
; incoming: call@0x00afc1->0299:1241
; byte_count: 31
; boundary: cfg_blocks_4_terminals_1
; terminal: retf:1
; direct_callees: none
; indirect_calls: 0
; cxx_source: re/borland/bloodprg/seg_0299/func_0041d1_entity_flag_state_transition.cpp
; routine_bytes_sha256: 07642a26707bc535c2ccc6c1b6f3ac29aaa621814a3dcdd06be4704def6872d5

0041D1:  50                           push     ax
0041D2:  53                           push     bx
0041D3:  C1 E0 05                     shl      ax, 5
0041D6:  BB 12 62                     mov      bx, 0x6212
0041D9:  03 D8                        add      bx, ax
0041DB:  65 8B 07                     mov      ax, word ptr gs:[bx]
0041DE:  0A C0                        or       al, al
0041E0:  79 08                        jns      0x41ea
0041E2:  A8 01                        test     al, 1
0041E4:  74 04                        je       0x41ea
0041E6:  24 FE                        and      al, 0xfe
0041E8:  0C 02                        or       al, 2
0041EA:  65 89 07                     mov      word ptr gs:[bx], ax
0041ED:  5B                           pop      bx
0041EE:  58                           pop      ax
0041EF:  CB                           retf    
