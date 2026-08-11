; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x00bd4e
; seg_off: 0b1b:059e
; group: seg_0b1b
; provenance: recursive_graph
; label: ems_buffer_setup
; label_comment: EMS buffer setup: ds=bx; si=0xa6c; [si]=0x4000 (16KB EMS page size); bx=[0xa5e]. Initializes an EMS page-mapped buffer
; byte_count: 63
; boundary: cfg_blocks_1_terminals_1
; terminal: ret:1
; direct_callees: none
; indirect_calls: 1
; routine_bytes_sha256: 264ce031bf2411ec0dd052e40c1d8d50e38c0e05d7c0816f4b7cb86f6089a023

00BD4E:  1E                           push     ds
00BD4F:  56                           push     si
00BD50:  53                           push     bx
00BD51:  50                           push     ax
00BD52:  8C EB                        mov      bx, gs
00BD54:  8E DB                        mov      ds, bx
00BD56:  BE 6C 0A                     mov      si, 0xa6c
00BD59:  66 C7 04 00 40 00 00         mov      dword ptr [si], 0x4000
00BD60:  8B 1E 5E 0A                  mov      bx, word ptr [0xa5e]
00BD64:  89 5C 04                     mov      word ptr [si + 4], bx
00BD67:  66 0F B7 C0                  movzx    eax, ax
00BD6B:  66 C1 E0 0E                  shl      eax, 0xe
00BD6F:  66 89 44 06                  mov      dword ptr [si + 6], eax
00BD73:  C7 44 0A 00 00               mov      word ptr [si + 0xa], 0
00BD78:  89 7C 0C                     mov      word ptr [si + 0xc], di
00BD7B:  8C 44 0E                     mov      word ptr [si + 0xe], es
00BD7E:  66 B8 00 0B 00 00            mov      eax, 0xb00
00BD84:  FF 1E 4A 0A                  lcall    [0xa4a]
00BD88:  58                           pop      ax
00BD89:  5B                           pop      bx
00BD8A:  5E                           pop      si
00BD8B:  1F                           pop      ds
00BD8C:  C3                           ret     
