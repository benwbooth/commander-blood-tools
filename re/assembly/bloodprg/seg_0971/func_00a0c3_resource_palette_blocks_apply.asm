; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x00a0c3
; seg_off: 0971:03b3
; group: seg_0971
; provenance: recursive_graph
; label: resource_palette_blocks_apply
; label_comment: NAME CORRECTION (was draw_cleanup_set_dirty, which described only the first FIVE instructions and called the whole thing a draw epilogue). THE RESOURCE PALETTE-BLOCK APPLIER: one routine 0xA0C3..0xA116, ret at 0xA116. Prologue: push ax/bx/cx/dx; push ds/push es/pop ds/pop es @0xA0C7 SWAPS ds and es so lodsw reads the caller's stream and rep movsb writes the data segment; dx=si saves the stream start; gs:[0x5b55]=1 marks the screen dirty. LOOP 0xA0D3..0xA0EC: lodsw -> al=start, ah=count; terminator is cmp ax,-1 @0xA0D4, a WORD compare (both bytes 0xFF); di=0x5251+start*3 (bl=3, mul bl); cx=count*3 (mov al,bl / mul bh @0xA0E6); rep movsb copies RAW 6-bit DAC triples into live_palette DS:0x5251. count==0 gives cx=0 and copies ZERO entries, NOT 256. No clamp: start+count>256 would run past the 768-byte buffer. CALLERS 0xA062 (resource_switch) and 0xA780 (list_d8c_init) -- a RESOURCE path, not a draw path, which is what makes the old name wrong. Tail 0xA0EE..0xA115 calls flag_gated_2751 and adjusts gs:0xDAF by the bytes consumed -- NOT read line by line. PORT: src/hnm.rs parse_palette_block
; byte_count: 84
; boundary: cfg_blocks_6_terminals_2
; terminal: jmp 0xa0d3:1, ret:1
; direct_callees: 0x00a117
; indirect_calls: 0
; cxx_source: re/borland/bloodprg/seg_0971/func_00a0c3_resource_palette_blocks_apply.cpp
; routine_bytes_sha256: 4721d1394bf610e0c221c0ec8a92ff143d65247d3275a157c4bde8906f0918ef

00A0C3:  50                           push     ax
00A0C4:  53                           push     bx
00A0C5:  51                           push     cx
00A0C6:  52                           push     dx
00A0C7:  1E                           push     ds
00A0C8:  06                           push     es
00A0C9:  1F                           pop      ds
00A0CA:  07                           pop      es
00A0CB:  8B D6                        mov      dx, si
00A0CD:  65 C6 06 55 5B 01            mov      byte ptr gs:[0x5b55], 1
00A0D3:  AD                           lodsw    ax, word ptr [si]
00A0D4:  83 F8 FF                     cmp      ax, -1
00A0D7:  74 15                        je       0xa0ee
00A0D9:  BF 51 52                     mov      di, 0x5251
00A0DC:  8A FC                        mov      bh, ah
00A0DE:  B3 03                        mov      bl, 3
00A0E0:  F6 E3                        mul      bl
00A0E2:  03 F8                        add      di, ax
00A0E4:  8A C3                        mov      al, bl
00A0E6:  F6 E7                        mul      bh
00A0E8:  8B C8                        mov      cx, ax
00A0EA:  F3 A4                        rep movsb byte ptr es:[di], byte ptr [si]
00A0EC:  EB E5                        jmp      0xa0d3
00A0EE:  E8 26 00                     call     0xa117
00A0F1:  65 A1 60 0D                  mov      ax, word ptr gs:[0xd60]
00A0F5:  0B C0                        or       ax, ax
00A0F7:  75 15                        jne      0xa10e
00A0F9:  8B C6                        mov      ax, si
00A0FB:  2B C2                        sub      ax, dx
00A0FD:  92                           xchg     dx, ax
00A0FE:  65 A1 AF 0D                  mov      ax, word ptr gs:[0xdaf]
00A102:  2B C2                        sub      ax, dx
00A104:  C1 E8 02                     shr      ax, 2
00A107:  83 E8 02                     sub      ax, 2
00A10A:  65 A3 AF 0D                  mov      word ptr gs:[0xdaf], ax
00A10E:  06                           push     es
00A10F:  1E                           push     ds
00A110:  07                           pop      es
00A111:  1F                           pop      ds
00A112:  5A                           pop      dx
00A113:  59                           pop      cx
00A114:  5B                           pop      bx
00A115:  58                           pop      ax
00A116:  C3                           ret     
