; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x006462
; seg_off: 04da:10c2
; group: seg_04da
; provenance: direct_call_target, label_csv_target, manual_binary_boundary
; label: vm_branch
; label_comment: VM branch helper: pop one word from the A0/A1 branch stack, put it in SI as the script PC, and clear query mode gs:[0x67ad]
; incoming: call@0x00649c->0x006462
; incoming: call@0x0064a8->0x006462
; incoming: call@0x0064b4->0x006462
; incoming: call@0x00650c->0x006462
; incoming: call@0x006555->0x006462
; incoming: call@0x006592->0x006462
; incoming: call@0x0065c9->0x006462
; incoming: call@0x006601->0x006462
; incoming: call@0x0068df->0x006462
; incoming: call@0x006928->0x006462
; incoming: call@0x006931->0x006462
; incoming: call@0x006977->0x006462
; incoming: call@0x006980->0x006462
; incoming: call@0x006a02->0x006462
; incoming: call@0x006a0e->0x006462
; incoming: call@0x006add->0x006462
; incoming: call@0x006ae6->0x006462
; incoming: call@0x006b28->0x006462
; incoming: call@0x006c75->0x006462
; incoming: call@0x006d13->0x006462
; incoming: call@0x006d7b->0x006462
; incoming: call@0x006dca->0x006462
; incoming: call@0x006e2f->0x006462
; incoming: call@0x006ee9->0x006462
; incoming: call@0x006f5d->0x006462
; incoming: call@0x006fb4->0x006462
; byte_count: 25
; boundary: cfg_blocks_1_terminals_1
; terminal: ret:1
; direct_callees: none
; indirect_calls: 0
; routine_bytes_sha256: 8ae7c0d81219b5b709d7205b28bf011636496f025b4afdaf694c82aa77575d0d

006462:  55                           push     bp
006463:  65 83 2E 84 68 02            sub      word ptr gs:[0x6884], 2
006469:  65 A1 84 68                  mov      ax, word ptr gs:[0x6884]
00646D:  8B E8                        mov      bp, ax
00646F:  8B B6 20 68                  mov      si, word ptr [bp + 0x6820]
006473:  65 C6 06 AD 67 00            mov      byte ptr gs:[0x67ad], 0
006479:  5D                           pop      bp
00647A:  C3                           ret
