; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x0017af
; seg_off: 008b:08ff
; group: seg_008b
; provenance: recursive_graph
; label: page_offset_helper
; label_comment: screen page-offset helper (3 calls): ax=[0x5219]; if negative ax=0 else ax += 0x4000. Adds the VGA page offset (0x4000) to the screen-buffer offset when valid
; byte_count: 42
; boundary: cfg_blocks_7_terminals_3
; terminal: jmp 0x17bd:1, jmp 0x17ce:1, ret:1
; direct_callees: none
; indirect_calls: 0
; routine_bytes_sha256: 3669a5ba5a7f728a031ad39d931654f2c8b4a52ef374246fa174195b948253db

0017AF:  A1 19 52                     mov      ax, word ptr [0x5219]
0017B2:  0B C0                        or       ax, ax
0017B4:  78 05                        js       0x17bb
0017B6:  05 00 40                     add      ax, 0x4000
0017B9:  EB 02                        jmp      0x17bd
0017BB:  33 C0                        xor      ax, ax
0017BD:  A3 19 52                     mov      word ptr [0x5219], ax
0017C0:  A1 1D 52                     mov      ax, word ptr [0x521d]
0017C3:  0B C0                        or       ax, ax
0017C5:  78 05                        js       0x17cc
0017C7:  05 00 40                     add      ax, 0x4000
0017CA:  EB 02                        jmp      0x17ce
0017CC:  33 C0                        xor      ax, ax
0017CE:  A3 1D 52                     mov      word ptr [0x521d], ax
0017D1:  8B 16 9E 0A                  mov      dx, word ptr [0xa9e]
0017D5:  B0 0C                        mov      al, 0xc
0017D7:  EF                           out      dx, ax
0017D8:  C3                           ret     
