; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x002693
; seg_off: 01ce:03b3
; group: seg_01ce
; provenance: recursive_graph, relocation_proven_far_transfer_target
; label: path_builder_gs_relative
; label_comment: gs-relative filename->path builder (5 calls): given DX=filename offset in the gs string segment, assemble the full path into DS:0x259 (prepend dir, append .ext). Used by resource_file_load 0x2abb - the reason filename-offset searches fail (names are gs-relative + path-assembled)
; incoming: call@0x000f6b->01ce:03b3
; incoming: call@0x003fdf->01ce:03b3
; incoming: call@0x007459->01ce:03b3
; incoming: call@0x0075bc->01ce:03b3
; incoming: call@0x009fcb->01ce:03b3
; incoming: call@0x00bddb->01ce:03b3
; incoming: call@0x00c020->01ce:03b3
; byte_count: 60
; boundary: cfg_blocks_7_terminals_2
; terminal: jmp 0x26ca:1, retf:1
; direct_callees: 0x0025a4, 0x0026cf, 0x0027c3, 0x0027e9
; indirect_calls: 0
; routine_bytes_sha256: f13641713a2580b5495d510e887af229eda951055dcb968d1e6cf7c34b27485c

002693:  50                           push     ax
002694:  06                           push     es
002695:  56                           push     si
002696:  57                           push     di
002697:  65 C6 06 E2 0A 00            mov      byte ptr gs:[0xae2], 0
00269D:  65 F6 06 E1 0A 01            test     byte ptr gs:[0xae1], 1
0026A3:  75 21                        jne      0x26c6
0026A5:  8C E8                        mov      ax, gs
0026A7:  8E C0                        mov      es, ax
0026A9:  8B F2                        mov      si, dx
0026AB:  BF 59 02                     mov      di, 0x259
0026AE:  0E                           push     cs
0026AF:  E8 F2 FE                     call     0x25a4
0026B2:  72 12                        jb       0x26c6
0026B4:  83 C7 10                     add      di, 0x10
0026B7:  26 80 3D 00                  cmp      byte ptr es:[di], 0
0026BB:  75 F1                        jne      0x26ae
0026BD:  0E                           push     cs
0026BE:  E8 28 01                     call     0x27e9
0026C1:  E8 0B 00                     call     0x26cf
0026C4:  EB 04                        jmp      0x26ca
0026C6:  0E                           push     cs
0026C7:  E8 F9 00                     call     0x27c3
0026CA:  5F                           pop      di
0026CB:  5E                           pop      si
0026CC:  07                           pop      es
0026CD:  58                           pop      ax
0026CE:  CB                           retf    
