; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x00555b
; seg_off: 04da:01bb
; group: seg_04da
; provenance: recursive_graph, relocation_proven_far_transfer_target
; label: vm_record_state_proc
; label_comment: VM record-state processor: bp=0x6d3e; les di,gs:[0x672c]; lds si,gs:[0x6724]; cmp es:[di+0x12],1. Processes the object/line-record state (reads field +0x12) during the dialogue/presentation update
; incoming: call@0x0010e3->04da:01bb
; incoming: call@0x001d43->04da:01bb
; byte_count: 73
; boundary: cfg_blocks_6_terminals_2
; terminal: jmp 0x556f:1, retf:1
; direct_callees: 0x006023
; indirect_calls: 0
; routine_bytes_sha256: 9c41f2e1af557f037491e14e31b74b101ad43fb38a8a74d70dca5205152203b6

00555B:  50                           push     ax
00555C:  53                           push     bx
00555D:  06                           push     es
00555E:  57                           push     di
00555F:  1E                           push     ds
005560:  56                           push     si
005561:  55                           push     bp
005562:  BD 3E 6D                     mov      bp, 0x6d3e
005565:  65 C4 3E 2C 67               les      di, ptr gs:[0x672c]
00556A:  65 C5 36 24 67               lds      si, ptr gs:[0x6724]
00556F:  26 83 7D 12 01               cmp      word ptr es:[di + 0x12], 1
005574:  75 26                        jne      0x559c
005576:  26 8B 75 10                  mov      si, word ptr es:[di + 0x10]
00557A:  8B 1C                        mov      bx, word ptr [si]
00557C:  B8 11 00                     mov      ax, 0x11
00557F:  E8 A1 0A                     call     0x6023
005582:  66 98                        cwde    
005584:  67 83 3C 30 FF               cmp      word ptr [eax + esi], -1
005589:  75 0C                        jne      0x5597
00558B:  89 76 00                     mov      word ptr [bp], si
00558E:  83 C5 02                     add      bp, 2
005591:  83 7E 00 FF                  cmp      word ptr [bp], -1
005595:  74 05                        je       0x559c
005597:  83 C7 14                     add      di, 0x14
00559A:  EB D3                        jmp      0x556f
00559C:  5D                           pop      bp
00559D:  5E                           pop      si
00559E:  1F                           pop      ds
00559F:  5F                           pop      di
0055A0:  07                           pop      es
0055A1:  5B                           pop      bx
0055A2:  58                           pop      ax
0055A3:  CB                           retf    
