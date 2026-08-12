; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x007684
; seg_off: 04da:22e4
; group: seg_04da
; provenance: static_dispatch_table_target
; label: dlg_line_asset_table_fill
; label_comment: Fills one per-line asset-table entry and its detail string. DI starts at the GS:0x1FAF cursor, seeded by 0x7447 at entry+2. LODSB/CBW sign-extends the id, but CBW leaves flags unchanged: through the sole proven caller, opcode 0x07 makes the dispatcher's ADD AX,AX leave SF clear, so JS is not taken and every shipped id follows 0x0DD7+(id-1)*16 modulo 16 bits (including 0xFF -> 0x0DB7). Only an out-of-contract direct entry with SF set stores the sign-extended id unchanged. STOSW plus ADD DI,2 advances the four-byte entry stride; GS:0x1FAD supplies a separate detail cursor advanced by 0x1A, and bytes 0x20..0x7F are copied without consuming the stopping byte.
; incoming: byte_parser_dispatch_74e5:byte_0x07
; byte_count: 54
; boundary: cfg_blocks_7_terminals_2
; terminal: jmp 0x76a8:1, ret:1
; direct_callees: none
; indirect_calls: 0
; routine_bytes_sha256: 8775c9c9dfe17ef427907c6adb0c3d9dbc00ba2fd8edc1951114fe97ce6a6478

007684:  65 8B 3E AF 1F               mov      di, word ptr gs:[0x1faf]
007689:  AC                           lodsb    al, byte ptr [si]
00768A:  98                           cbw
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
