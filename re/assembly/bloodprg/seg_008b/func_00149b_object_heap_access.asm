; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x00149b
; seg_off: 008b:05eb
; group: seg_008b
; provenance: recursive_graph
; label: object_heap_access
; label_comment: runtime object-heap accessor: es=[0x6726] (object heap segment), lds si,[0x672c] (lookup table); di=[si+0x10]; test es:[di],0x118 (object flags). Reads a live object from the runtime heap via the 0x672c lookup - the object-instance heap access (es:0x6726 is the runtime object heap)
; byte_count: 47
; boundary: cfg_blocks_6_terminals_1
; terminal: ret:1
; direct_callees: none
; indirect_calls: 0
; routine_bytes_sha256: 23fc2a908b47847ad46c411b0ea3eac0447641651b0864c4ab0c1a2544d70439

00149B:  06                           push     es
00149C:  57                           push     di
00149D:  1E                           push     ds
00149E:  56                           push     si
00149F:  8E 06 26 67                  mov      es, word ptr [0x6726]
0014A3:  C5 36 2C 67                  lds      si, ptr [0x672c]
0014A7:  8B 7C 10                     mov      di, word ptr [si + 0x10]
0014AA:  26 F7 05 18 01               test     word ptr es:[di], 0x118
0014AF:  74 0B                        je       0x14bc
0014B1:  26 F6 45 02 02               test     byte ptr es:[di + 2], 2
0014B6:  74 04                        je       0x14bc
0014B8:  26 FE 45 14                  inc      byte ptr es:[di + 0x14]
0014BC:  83 C6 14                     add      si, 0x14
0014BF:  83 7C 12 01                  cmp      word ptr [si + 0x12], 1
0014C3:  74 E2                        je       0x14a7
0014C5:  5E                           pop      si
0014C6:  1F                           pop      ds
0014C7:  5F                           pop      di
0014C8:  07                           pop      es
0014C9:  C3                           ret     
