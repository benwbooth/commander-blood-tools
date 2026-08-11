; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x007684
; seg_off: 04da:22e4
; group: seg_04da
; provenance: static_dispatch_table_target
; label: dlg_line_asset_table_fill
; label_comment: THE FILL for the per-line asset table (completes the chain from A6 b3). di = the cursor gs:[0x1FAF], which 0x7447 seeded at 0x1FB5+0x26 -- i.e. already AT entry+2, exactly where the reader 0x9D6E looks. Per source byte: LODSB + CBW; if NEGATIVE store the sign-extended value unchanged (so 0xFF becomes 0xFFFF, the 'no asset' sentinel the reader tests at 0x9D71); otherwise store (byte-1)*16 (DEC AX; SHL AX,4). Then STOSW + `add di,2` advances a full 4-byte stride to the next entry's +2. The *16 means the stored id is a BYTE OFFSET into a 16-byte-stride NAME TABLE -- the same stride as the sprite filename table at DS:0x0669 -- so a per-line asset is a filename reference, not an ordinal
; incoming: byte_parser_dispatch_74e5:byte_0x07
; byte_count: 54
; boundary: cfg_blocks_7_terminals_2
; terminal: jmp 0x76a8:1, ret:1
; direct_callees: none
; indirect_calls: 0
; cxx_source: re/borland/bloodprg/seg_04da/func_007684_dlg_line_asset_table_fill.cpp
; routine_bytes_sha256: 8775c9c9dfe17ef427907c6adb0c3d9dbc00ba2fd8edc1951114fe97ce6a6478

007684:  65 8B 3E AF 1F               mov      di, word ptr gs:[0x1faf]
007689:  AC                           lodsb    al, byte ptr [si]
00768A:  98                           cwde    
00768B:  78 07                        js       0x7694
00768D:  48                           dec      ax
00768E:  C1 E0 04                     shl      ax, 4
007691:  05 D7 0D                     add      ax, 0xdd7
007694:  AB                           stosw    word ptr es:[di], ax
007695:  83 C7 02                     add      di, 2
007698:  65 89 3E AF 1F               mov      word ptr gs:[0x1faf], di
00769D:  65 8B 3E AD 1F               mov      di, word ptr gs:[0x1fad]
0076A2:  65 83 06 AD 1F 1A            add      word ptr gs:[0x1fad], 0x1a
0076A8:  AC                           lodsb    al, byte ptr [si]
0076A9:  0A C0                        or       al, al
0076AB:  78 07                        js       0x76b4
0076AD:  3C 20                        cmp      al, 0x20
0076AF:  72 03                        jb       0x76b4
0076B1:  AA                           stosb    byte ptr es:[di], al
0076B2:  EB F4                        jmp      0x76a8
0076B4:  4E                           dec      si
0076B5:  26 C6 05 00                  mov      byte ptr es:[di], 0
0076B9:  C3                           ret     
