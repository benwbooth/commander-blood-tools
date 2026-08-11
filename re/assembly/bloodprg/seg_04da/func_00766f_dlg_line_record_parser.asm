; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x00766f
; seg_off: 04da:22cf
; group: seg_04da
; provenance: static_dispatch_table_target
; label: dlg_line_record_parser
; label_comment: PER-LINE RECORD PARSER — the source of the per-line asset table. Reads one record from ds:si and fans it into THREE destinations: (1) a NAME copied to DS:0x24C6 (0x766F..0x767E, LODSB with a js/cmp 0x20 terminator like the other name copiers); (2) the ASSET ID at 0x7684, stored as (byte-1)*16 or a sign-extended passthrough, through the cursor DS:0x1FAF into the 4-byte-stride table DS:0x1FB5; (3) a further field at 0x769D through the cursor DS:0x1FAD, which advances 0x1A (26) bytes per record. Immediately preceded at 0x7667 by `mov ax,1; lcall 0xb1b:0x855` — the SND bank loader with AX!=0, the mode that preserves the table and may write son.snd
; incoming: byte_parser_dispatch_74e5:byte_0x10
; byte_count: 21
; boundary: cfg_blocks_5_terminals_2
; terminal: jmp 0x7672:1, ret:1
; direct_callees: none
; indirect_calls: 0
; cxx_source: re/borland/bloodprg/seg_04da/func_00766f_dlg_line_record_parser.cpp
; routine_bytes_sha256: 563a3edc7b8e95fd2ba6c8d95374d3c6a92c93d610460c31c04a7372bc5fb205

00766F:  BF C6 24                     mov      di, 0x24c6
007672:  AC                           lodsb    al, byte ptr [si]
007673:  0A C0                        or       al, al
007675:  78 07                        js       0x767e
007677:  3C 20                        cmp      al, 0x20
007679:  72 03                        jb       0x767e
00767B:  AA                           stosb    byte ptr es:[di], al
00767C:  EB F4                        jmp      0x7672
00767E:  4E                           dec      si
00767F:  26 C6 05 00                  mov      byte ptr es:[di], 0
007683:  C3                           ret     
