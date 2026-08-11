; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x00bd8d
; seg_off: 0b1b:05dd
; group: seg_0b1b
; provenance: recursive_graph
; label: ems_page_offset_split
; label_comment: EMS address split: cx=ax>>2; ax<<=0xe (14) -> dx. Splits a value into an EMS 16KB-page number + offset (the banked-memory addressing math)
; byte_count: 42
; boundary: cfg_blocks_1_terminals_1
; terminal: ret:1
; direct_callees: none
; indirect_calls: 0
; routine_bytes_sha256: 4ad91abe66caeda36b5ba7f2714f2dacabcddd3192b084b7f96c6b28fa9e9cf5

00BD8D:  1E                           push     ds
00BD8E:  50                           push     ax
00BD8F:  53                           push     bx
00BD90:  51                           push     cx
00BD91:  52                           push     dx
00BD92:  8B C8                        mov      cx, ax
00BD94:  C1 E9 02                     shr      cx, 2
00BD97:  C1 E0 0E                     shl      ax, 0xe
00BD9A:  8B D0                        mov      dx, ax
00BD9C:  65 8B 1E 49 0C               mov      bx, word ptr gs:[0xc49]
00BDA1:  B8 00 42                     mov      ax, 0x4200
00BDA4:  CD 21                        int      0x21
00BDA6:  B9 00 40                     mov      cx, 0x4000
00BDA9:  B4 3F                        mov      ah, 0x3f
00BDAB:  06                           push     es
00BDAC:  1F                           pop      ds
00BDAD:  8B D7                        mov      dx, di
00BDAF:  CD 21                        int      0x21
00BDB1:  5A                           pop      dx
00BDB2:  59                           pop      cx
00BDB3:  5B                           pop      bx
00BDB4:  58                           pop      ax
00BDB5:  1F                           pop      ds
00BDB6:  C3                           ret     
