; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x00287b
; seg_off: 01ce:059b
; group: seg_01ce
; provenance: recursive_graph, relocation_proven_far_transfer_target
; label: resource_load_by_id
; label_comment: SEG 0x1ce:0x59b: load resource by ID AX. ds=fs; si=0x0c04+AX*16 (FS:0x0c04 name-table 16-byte entry = filename); call 0x28ca (lookup/already-loaded), lcall 0x4b9:0 (alloc slot), call 0x2abb (open+read file into segment). Returns ax=success. THE entry point for loading worlds/scripts: e.g. load venusia = resource_load_by_id(25). Called per-resource by vm_resource_profile_select 0x53da
; incoming: call@0x000f90->01ce:059b
; incoming: call@0x0053dc->01ce:059b
; byte_count: 79
; boundary: cfg_blocks_7_terminals_2
; terminal: jmp 0x28c2:1, retf:1
; direct_callees: 0x0028ca, 0x002abb
; indirect_calls: 1
; cxx_source: re/borland/bloodprg/seg_01ce/func_00287b_resource_load_by_id.cpp
; routine_bytes_sha256: 2ef711193074fd2dbc1bb93c3f9cf3698fe854d1c77940b18eecb5dad2b5dd91

00287B:  53                           push     bx
00287C:  1E                           push     ds
00287D:  56                           push     si
00287E:  06                           push     es
00287F:  57                           push     di
002880:  66 55                        push     ebp
002882:  8C E3                        mov      bx, fs
002884:  8E DB                        mov      ds, bx
002886:  BE 04 0C                     mov      si, 0xc04
002889:  8B D8                        mov      bx, ax
00288B:  C1 E3 04                     shl      bx, 4
00288E:  03 F3                        add      si, bx
002890:  8B FE                        mov      di, si
002892:  0E                           push     cs
002893:  E8 34 00                     call     0x28ca
002896:  66 0B ED                     or       ebp, ebp
002899:  74 24                        je       0x28bf
00289B:  9A 00 00 B9 04               lcall    0x4b9, 0
0028A0:  0B C0                        or       ax, ax
0028A2:  78 1B                        js       0x28bf
0028A4:  75 11                        jne      0x28b7
0028A6:  87 F7                        xchg     di, si
0028A8:  1E                           push     ds
0028A9:  07                           pop      es
0028AA:  8C E3                        mov      bx, fs
0028AC:  8E DB                        mov      ds, bx
0028AE:  0E                           push     cs
0028AF:  E8 09 02                     call     0x2abb
0028B2:  66 0B C0                     or       eax, eax
0028B5:  74 08                        je       0x28bf
0028B7:  66 B8 01 00 00 00            mov      eax, 1
0028BD:  EB 03                        jmp      0x28c2
0028BF:  66 33 C0                     xor      eax, eax
0028C2:  66 5D                        pop      ebp
0028C4:  5F                           pop      di
0028C5:  07                           pop      es
0028C6:  5E                           pop      si
0028C7:  1F                           pop      ds
0028C8:  5B                           pop      bx
0028C9:  CB                           retf    
