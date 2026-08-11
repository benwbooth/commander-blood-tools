; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x003e70
; seg_off: 0299:0ee0
; group: seg_0299
; provenance: recursive_graph, relocation_proven_far_transfer_target
; label: vga_graphics_controller_setup
; label_comment: program the VGA graphics controller: dx=0x3ce (GC index/data port); ax=4 (read map select / write mode). Sets the mode-X plane/write-mode for planar VGA access
; incoming: call@0x008d14->0299:0ee0
; byte_count: 94
; boundary: cfg_blocks_1_terminals_1
; terminal: retf:1
; direct_callees: none
; indirect_calls: 0
; routine_bytes_sha256: fccf6f984ecd00f605ffb78207e8357ee8105cd23fc3476a6b8f2a293da4f057

003E70:  50                           push     ax
003E71:  53                           push     bx
003E72:  51                           push     cx
003E73:  52                           push     dx
003E74:  56                           push     si
003E75:  57                           push     di
003E76:  55                           push     bp
003E77:  FC                           cld     
003E78:  BA CE 03                     mov      dx, 0x3ce
003E7B:  8B DE                        mov      bx, si
003E7D:  8B EF                        mov      bp, di
003E7F:  B8 04 00                     mov      ax, 4
003E82:  EF                           out      dx, ax
003E83:  B9 80 3E                     mov      cx, 0x3e80
003E86:  A4                           movsb    byte ptr es:[di], byte ptr [si]
003E87:  83 C7 03                     add      di, 3
003E8A:  E2 FA                        loop     0x3e86
003E8C:  8B FD                        mov      di, bp
003E8E:  47                           inc      di
003E8F:  8B F3                        mov      si, bx
003E91:  B8 04 01                     mov      ax, 0x104
003E94:  EF                           out      dx, ax
003E95:  B9 80 3E                     mov      cx, 0x3e80
003E98:  A4                           movsb    byte ptr es:[di], byte ptr [si]
003E99:  83 C7 03                     add      di, 3
003E9C:  E2 FA                        loop     0x3e98
003E9E:  8B FD                        mov      di, bp
003EA0:  83 C7 02                     add      di, 2
003EA3:  8B F3                        mov      si, bx
003EA5:  B8 04 02                     mov      ax, 0x204
003EA8:  EF                           out      dx, ax
003EA9:  B9 80 3E                     mov      cx, 0x3e80
003EAC:  A4                           movsb    byte ptr es:[di], byte ptr [si]
003EAD:  83 C7 03                     add      di, 3
003EB0:  E2 FA                        loop     0x3eac
003EB2:  8B FD                        mov      di, bp
003EB4:  83 C7 03                     add      di, 3
003EB7:  8B F3                        mov      si, bx
003EB9:  B8 04 03                     mov      ax, 0x304
003EBC:  EF                           out      dx, ax
003EBD:  B9 80 3E                     mov      cx, 0x3e80
003EC0:  A4                           movsb    byte ptr es:[di], byte ptr [si]
003EC1:  83 C7 03                     add      di, 3
003EC4:  E2 FA                        loop     0x3ec0
003EC6:  5D                           pop      bp
003EC7:  5F                           pop      di
003EC8:  5E                           pop      si
003EC9:  5A                           pop      dx
003ECA:  59                           pop      cx
003ECB:  5B                           pop      bx
003ECC:  58                           pop      ax
003ECD:  CB                           retf    
