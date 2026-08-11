; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x0024b2
; seg_off: 01ce:01d2
; group: seg_01ce
; provenance: relocation_proven_far_transfer_target
; label: cs_table_read_1c6
; label_comment: cs-table read: ds=cs; si=0x1c6; si+=0xb; cx=0xa (10 entries). Reads a 10-entry constant table at cs:0x1c6+0xb (sibling of cs_data_ptr_setup 0x24eb)
; incoming: call@0x000e27->01ce:01d2
; byte_count: 57
; boundary: cfg_blocks_5_terminals_1
; terminal: retf:1
; direct_callees: none
; indirect_calls: 0
; routine_bytes_sha256: 9883d69c45c5c81c34c7e1ac68698e5d8629a16537d226b69843d8509fa088f0

0024B2:  50                           push     ax
0024B3:  51                           push     cx
0024B4:  52                           push     dx
0024B5:  57                           push     di
0024B6:  1E                           push     ds
0024B7:  56                           push     si
0024B8:  8C CA                        mov      dx, cs
0024BA:  8E DA                        mov      ds, dx
0024BC:  BE C6 01                     mov      si, 0x1c6
0024BF:  83 C6 0B                     add      si, 0xb
0024C2:  B9 0A 00                     mov      cx, 0xa
0024C5:  0B C0                        or       ax, ax
0024C7:  79 07                        jns      0x24d0
0024C9:  26 C6 05 2D                  mov      byte ptr es:[di], 0x2d
0024CD:  47                           inc      di
0024CE:  F7 D8                        neg      ax
0024D0:  4E                           dec      si
0024D1:  33 D2                        xor      dx, dx
0024D3:  F7 F1                        div      cx
0024D5:  83 C2 30                     add      dx, 0x30
0024D8:  88 14                        mov      byte ptr [si], dl
0024DA:  0B C0                        or       ax, ax
0024DC:  75 F2                        jne      0x24d0
0024DE:  AC                           lodsb    al, byte ptr [si]
0024DF:  AA                           stosb    byte ptr es:[di], al
0024E0:  0A C0                        or       al, al
0024E2:  75 FA                        jne      0x24de
0024E4:  5E                           pop      si
0024E5:  1F                           pop      ds
0024E6:  5F                           pop      di
0024E7:  5A                           pop      dx
0024E8:  59                           pop      cx
0024E9:  58                           pop      ax
0024EA:  CB                           retf    
