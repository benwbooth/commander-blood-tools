; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x002b6b
; seg_off: 01ce:088b
; group: seg_01ce
; provenance: recursive_graph, relocation_proven_far_transfer_target
; label: file_create
; label_comment: DOS file create: call 0x27c3 (path setup); store gs:[0xa92]=eax; ax=0x3c00; int21h (DOS create/truncate file). Creates a file (temp/save/scratch)
; incoming: call@0x001cb2->01ce:088b
; byte_count: 131
; boundary: cfg_blocks_8_terminals_2
; terminal: jmp 0x2be5:1, retf:1
; direct_callees: 0x0027c3
; indirect_calls: 0
; cxx_source: re/borland/bloodprg/seg_01ce/func_002b6b_file_create.cpp
; routine_bytes_sha256: 0b549eeef7b59edae933c92a723fb2754c842b432857f60eca52fb2aaac88b71

002B6B:  53                           push     bx
002B6C:  51                           push     cx
002B6D:  06                           push     es
002B6E:  57                           push     di
002B6F:  1E                           push     ds
002B70:  56                           push     si
002B71:  66 55                        push     ebp
002B73:  0E                           push     cs
002B74:  E8 4C FC                     call     0x27c3
002B77:  66 65 A3 92 0A               mov      dword ptr gs:[0xa92], eax
002B7C:  66 8B E8                     mov      ebp, eax
002B7F:  B8 00 3C                     mov      ax, 0x3c00
002B82:  33 C9                        xor      cx, cx
002B84:  8B D6                        mov      dx, si
002B86:  CD 21                        int      0x21
002B88:  72 58                        jb       0x2be2
002B8A:  65 A3 84 0A                  mov      word ptr gs:[0xa84], ax
002B8E:  8C C0                        mov      ax, es
002B90:  8E D8                        mov      ds, ax
002B92:  8B D7                        mov      dx, di
002B94:  65 8B 0E 92 0A               mov      cx, word ptr gs:[0xa92]
002B99:  65 A1 94 0A                  mov      ax, word ptr gs:[0xa94]
002B9D:  0B C0                        or       ax, ax
002B9F:  74 03                        je       0x2ba4
002BA1:  B9 00 7D                     mov      cx, 0x7d00
002BA4:  B8 00 40                     mov      ax, 0x4000
002BA7:  65 8B 1E 84 0A               mov      bx, word ptr gs:[0xa84]
002BAC:  CD 21                        int      0x21
002BAE:  65 29 06 92 0A               sub      word ptr gs:[0xa92], ax
002BB3:  65 83 1E 94 0A 00            sbb      word ptr gs:[0xa94], 0
002BB9:  8B D8                        mov      bx, ax
002BBB:  C1 EB 04                     shr      bx, 4
002BBE:  83 E0 0F                     and      ax, 0xf
002BC1:  8C D9                        mov      cx, ds
002BC3:  03 CB                        add      cx, bx
002BC5:  8E D9                        mov      ds, cx
002BC7:  03 D0                        add      dx, ax
002BC9:  66 65 A1 92 0A               mov      eax, dword ptr gs:[0xa92]
002BCE:  66 0B C0                     or       eax, eax
002BD1:  75 C1                        jne      0x2b94
002BD3:  B8 00 3E                     mov      ax, 0x3e00
002BD6:  65 8B 1E 84 0A               mov      bx, word ptr gs:[0xa84]
002BDB:  CD 21                        int      0x21
002BDD:  66 8B C5                     mov      eax, ebp
002BE0:  EB 03                        jmp      0x2be5
002BE2:  66 33 C0                     xor      eax, eax
002BE5:  66 5D                        pop      ebp
002BE7:  5E                           pop      si
002BE8:  1F                           pop      ds
002BE9:  5F                           pop      di
002BEA:  07                           pop      es
002BEB:  59                           pop      cx
002BEC:  5B                           pop      bx
002BED:  CB                           retf    
