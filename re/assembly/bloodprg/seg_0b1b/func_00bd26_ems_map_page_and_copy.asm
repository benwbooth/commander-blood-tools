; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x00bd26
; seg_off: 0b1b:0576
; group: seg_0b1b
; provenance: recursive_graph
; label: ems_map_page_and_copy
; label_comment: EMS PAGE MAP + 16KB COPY, not a DOS IOCTL (audit-fixes #351). `mov ax,0x4400` @0xBD3B looks like the DOS IOCTL function and the old label read it that way -- but the very next instruction is `int 0x67` @0xBD3E, the EMS entry point, so AH=0x44 is the EMS MAP-HANDLE-PAGE function with BX = the handle (from AX) and DX = gs:[0x0A60]. It is followed by `mov cx,0x1000 / rep movsd` @0xBD40, copying 0x1000 dwords = 16384 bytes = EXACTLY one EMS page. The AX value is the same in both APIs; only the interrupt distinguishes them.
; byte_count: 40
; boundary: cfg_blocks_1_terminals_1
; terminal: ret:1
; direct_callees: none
; indirect_calls: 0
; routine_bytes_sha256: db15b3cda28ad812dac847298327127233ee695c460da0f3a6ff517b2ba5a40e

00BD26:  1E                           push     ds
00BD27:  56                           push     si
00BD28:  57                           push     di
00BD29:  50                           push     ax
00BD2A:  53                           push     bx
00BD2B:  51                           push     cx
00BD2C:  52                           push     dx
00BD2D:  65 8E 1E 66 0A               mov      ds, word ptr gs:[0xa66]
00BD32:  33 F6                        xor      si, si
00BD34:  8B D8                        mov      bx, ax
00BD36:  65 8B 16 60 0A               mov      dx, word ptr gs:[0xa60]
00BD3B:  B8 00 44                     mov      ax, 0x4400
00BD3E:  CD 67                        int      0x67
00BD40:  B9 00 10                     mov      cx, 0x1000
00BD43:  F3 66 A5                     rep movsd dword ptr es:[di], dword ptr [si]
00BD46:  5A                           pop      dx
00BD47:  59                           pop      cx
00BD48:  5B                           pop      bx
00BD49:  58                           pop      ax
00BD4A:  5F                           pop      di
00BD4B:  5E                           pop      si
00BD4C:  1F                           pop      ds
00BD4D:  C3                           ret     
