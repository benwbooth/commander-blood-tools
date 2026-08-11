; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x000d61
; seg_off: 0000:0761
; group: seg_0000
; provenance: recursive_graph
; label: print_string_dos
; label_comment: DOS string print (2 calls): ah=2; loop lodsb al; while al!=0 int21h (char output). Prints the null-terminated string at DS:SI via DOS teletype - a debug/console text helper
; byte_count: 20
; boundary: cfg_blocks_4_terminals_2
; terminal: jmp 0xd66:1, retf:1
; direct_callees: none
; indirect_calls: 0
; cxx_source: re/borland/bloodprg/seg_0000/func_000d61_print_string_dos.cpp
; routine_bytes_sha256: 6d8c996dedfc2684eee31e8454f331807f9a5ae4a08e17f8db65e46c84b9115f

000D61:  50                           push     ax
000D62:  52                           push     dx
000D63:  56                           push     si
000D64:  B4 02                        mov      ah, 2
000D66:  AC                           lodsb    al, byte ptr [si]
000D67:  0A C0                        or       al, al
000D69:  74 06                        je       0xd71
000D6B:  8A D0                        mov      dl, al
000D6D:  CD 21                        int      0x21
000D6F:  EB F5                        jmp      0xd66
000D71:  5E                           pop      si
000D72:  5A                           pop      dx
000D73:  58                           pop      ax
000D74:  CB                           retf    
