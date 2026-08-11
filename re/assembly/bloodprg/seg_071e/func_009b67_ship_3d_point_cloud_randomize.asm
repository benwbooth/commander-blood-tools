; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x009b67
; seg_off: 071e:2387
; group: seg_071e
; provenance: recursive_graph, relocation_proven_far_transfer_target
; label: ship_3d_point_cloud_randomize
; label_comment: initializes 1000 point-cloud records at DS:0x2FC1 with random x/y/z words
; incoming: call@0x000fd3->071e:2387
; byte_count: 49
; boundary: cfg_blocks_1_terminals_1
; terminal: retf:1
; direct_callees: none
; indirect_calls: 3
; routine_bytes_sha256: 8e518bfaaeff24ed55e4ebbdaec0ea0e13b3ee7810410bcd5a1e23d47938df31

009B67:  06                           push     es
009B68:  57                           push     di
009B69:  50                           push     ax
009B6A:  B9 E8 03                     mov      cx, 0x3e8
009B6D:  8C E8                        mov      ax, gs
009B6F:  8E C0                        mov      es, ax
009B71:  BF C1 2F                     mov      di, 0x2fc1
009B74:  B8 FF FF                     mov      ax, 0xffff
009B77:  9A 02 0B CE 01               lcall    0x1ce, 0xb02
009B7C:  AB                           stosw    word ptr es:[di], ax
009B7D:  B8 FF FF                     mov      ax, 0xffff
009B80:  9A 02 0B CE 01               lcall    0x1ce, 0xb02
009B85:  AB                           stosw    word ptr es:[di], ax
009B86:  B8 FF FF                     mov      ax, 0xffff
009B89:  9A 02 0B CE 01               lcall    0x1ce, 0xb02
009B8E:  AB                           stosw    word ptr es:[di], ax
009B8F:  83 C7 02                     add      di, 2
009B92:  E2 E0                        loop     0x9b74
009B94:  58                           pop      ax
009B95:  5F                           pop      di
009B96:  07                           pop      es
009B97:  CB                           retf    
