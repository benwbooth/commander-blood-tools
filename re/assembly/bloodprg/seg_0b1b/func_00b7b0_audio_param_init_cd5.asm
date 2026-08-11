; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x00b7b0
; seg_off: 0b1b:0000
; group: seg_0b1b
; provenance: recursive_graph, relocation_proven_far_transfer_target
; label: audio_param_init_cd5
; label_comment: audio param init: ds=gs; di=0xcd5; cx=9; [di]=ax fill. Initialises the 9-word audio parameter block at gs:0xcd5 (adjacent to the driver callback ptr gs:0xcdf)
; incoming: call@0x000fb0->0b1b:0000
; byte_count: 51
; boundary: cfg_blocks_1_terminals_1
; terminal: retf:1
; direct_callees: none
; indirect_calls: 1
; routine_bytes_sha256: a4c1c0b88b5a0946e63d332601244eae5acfc57d56ab020bb232920255f67b3c

00B7B0:  53                           push     bx
00B7B1:  51                           push     cx
00B7B2:  52                           push     dx
00B7B3:  06                           push     es
00B7B4:  57                           push     di
00B7B5:  1E                           push     ds
00B7B6:  56                           push     si
00B7B7:  55                           push     bp
00B7B8:  8C EB                        mov      bx, gs
00B7BA:  8E DB                        mov      ds, bx
00B7BC:  BF D5 0C                     mov      di, 0xcd5
00B7BF:  B9 09 00                     mov      cx, 9
00B7C2:  89 05                        mov      word ptr [di], ax
00B7C4:  83 C7 04                     add      di, 4
00B7C7:  E2 F9                        loop     0xb7c2
00B7C9:  B8 1D 01                     mov      ax, 0x11d
00B7CC:  A3 EC 0A                     mov      word ptr [0xaec], ax
00B7CF:  8C 0E EE 0A                  mov      word ptr [0xaee], cs
00B7D3:  A1 45 0C                     mov      ax, word ptr [0xc45]
00B7D6:  FF 1E D3 0C                  lcall    [0xcd3]
00B7DA:  5D                           pop      bp
00B7DB:  5E                           pop      si
00B7DC:  1F                           pop      ds
00B7DD:  5F                           pop      di
00B7DE:  07                           pop      es
00B7DF:  5A                           pop      dx
00B7E0:  59                           pop      cx
00B7E1:  5B                           pop      bx
00B7E2:  CB                           retf    
