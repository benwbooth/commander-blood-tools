; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x0027c3
; seg_off: 01ce:04e3
; group: seg_01ce
; provenance: recursive_graph, relocation_proven_far_transfer_target
; label: set_ds_gs_check_ae0
; label_comment: gs-setup helper (2 calls): ds=gs; test [0xae0]&1 -> branch. Establishes DS=the gs data segment then checks the [0xae0] mode flag
; incoming: call@0x0012ed->01ce:04e3
; incoming: call@0x0015e5->01ce:04e3
; incoming: call@0x00174d->01ce:04e3
; incoming: call@0x001c3f->01ce:04e3
; incoming: call@0x001cc3->01ce:04e3
; incoming: call@0x0075a1->01ce:04e3
; incoming: call@0x00bf8d->01ce:04e3
; incoming: call@0x00c197->01ce:04e3
; byte_count: 38
; boundary: cfg_blocks_3_terminals_1
; terminal: retf:1
; direct_callees: none
; indirect_calls: 0
; cxx_source: re/borland/bloodprg/seg_01ce/func_0027c3_set_ds_gs_check_ae0.cpp
; routine_bytes_sha256: 13fabd2f516c822ed776e53c3c177d99d5ab54d5e02bbb9a6d557d256693426a

0027C3:  50                           push     ax
0027C4:  1E                           push     ds
0027C5:  52                           push     dx
0027C6:  8C E8                        mov      ax, gs
0027C8:  8E D8                        mov      ds, ax
0027CA:  F6 06 E0 0A 01               test     byte ptr [0xae0], 1
0027CF:  75 14                        jne      0x27e5
0027D1:  B4 0E                        mov      ah, 0xe
0027D3:  8A 16 B8 01                  mov      dl, byte ptr [0x1b8]
0027D7:  CD 21                        int      0x21
0027D9:  BA BA 01                     mov      dx, 0x1ba
0027DC:  B4 3B                        mov      ah, 0x3b
0027DE:  CD 21                        int      0x21
0027E0:  C6 06 E0 0A 01               mov      byte ptr [0xae0], 1
0027E5:  5A                           pop      dx
0027E6:  1F                           pop      ds
0027E7:  58                           pop      ax
0027E8:  CB                           retf    
