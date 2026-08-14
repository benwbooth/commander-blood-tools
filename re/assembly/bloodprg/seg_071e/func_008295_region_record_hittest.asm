; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x008295
; seg_off: 071e:0ab5
; group: seg_071e
; provenance: recursive_graph, relocation_proven_far_transfer_target
; label: region_record_hittest
; label_comment: consumes an SS:BP {x,y,width,height} signed rectangle in cursor coordinates, gated by DS:0x0a3e bit 0, and returns an inclusive hit in carry. ui_region_31_poll 0x82c3 repeatedly passes SS:0x65fa; other callers pass independent rectangles.
; incoming: call@0x001528->071e:0ab5
; incoming: call@0x001538->071e:0ab5
; byte_count: 46
; boundary: cfg_blocks_8_terminals_2
; terminal: jmp 0x82c1:1, retf:1
; direct_callees: none
; indirect_calls: 0
; routine_bytes_sha256: 5ba395485dab841fbe4d86bba408b4f61c85444c2e858b94ec645f07bafec3b2

008295:  50                           push     ax
008296:  F6 06 3E 0A 01               test     byte ptr [0xa3e], 1
00829B:  74 23                        je       0x82c0
00829D:  A1 2A 0A                     mov      ax, word ptr [0xa2a]
0082A0:  3B 46 00                     cmp      ax, word ptr [bp]
0082A3:  7C 1B                        jl       0x82c0
0082A5:  2B 46 04                     sub      ax, word ptr [bp + 4]
0082A8:  3B 46 00                     cmp      ax, word ptr [bp]
0082AB:  7F 13                        jg       0x82c0
0082AD:  A1 2C 0A                     mov      ax, word ptr [0xa2c]
0082B0:  3B 46 02                     cmp      ax, word ptr [bp + 2]
0082B3:  7C 0B                        jl       0x82c0
0082B5:  2B 46 06                     sub      ax, word ptr [bp + 6]
0082B8:  3B 46 02                     cmp      ax, word ptr [bp + 2]
0082BB:  7F 03                        jg       0x82c0
0082BD:  F9                           stc     
0082BE:  EB 01                        jmp      0x82c1
0082C0:  F8                           clc     
0082C1:  58                           pop      ax
0082C2:  CB                           retf    
