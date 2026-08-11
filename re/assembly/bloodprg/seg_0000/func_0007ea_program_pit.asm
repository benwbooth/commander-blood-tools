; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x0007ea
; seg_off: 0000:01ea
; group: seg_0000
; provenance: recursive_graph
; label: program_pit
; label_comment: PIT RESTORE, not a speed-up (audit-fixes #349 corrects two errors in the previous label). cli; mov al,0x36 / out 0x43,al (channel 0, mode 3 square wave); mov al,0xff then out 0x40,al TWICE -- the raw bytes are `e6 40 e6 40` at 0x07F4/0x07F6, so the port is 0x40 (PIT CHANNEL 0, the system timer), NOT 0x42 (the PC speaker) as this label used to say. Writing 0xFF as both divisor halves gives 0xFFFF, the SLOWEST divisor = the DEFAULT ~18.2Hz -- so this RESTORES the stock tick rate rather than making it faster. Then gs:[0xB21]=0 clears the timer-active flag, sti, and the original INT 08h vector is restored from gs:[0xB1D]/[0xB1F]. The teardown counterpart of 0x079C. src/recomp/io_lift.rs func_7ea had both details right.
; byte_count: 41
; boundary: cfg_blocks_1_terminals_1
; terminal: retf:1
; direct_callees: none
; indirect_calls: 0
; cxx_source: re/borland/bloodprg/seg_0000/func_0007ea_program_pit.cpp
; routine_bytes_sha256: 6b36e730d0da3c9952d868446201d9934b9cfdde8c22f91040d445df1ec00867

0007EA:  50                           push     ax
0007EB:  52                           push     dx
0007EC:  1E                           push     ds
0007ED:  FA                           cli     
0007EE:  B0 36                        mov      al, 0x36
0007F0:  E6 43                        out      0x43, al
0007F2:  B0 FF                        mov      al, 0xff
0007F4:  E6 40                        out      0x40, al
0007F6:  E6 40                        out      0x40, al
0007F8:  65 C6 06 21 0B 00            mov      byte ptr gs:[0xb21], 0
0007FE:  FB                           sti     
0007FF:  65 A1 1F 0B                  mov      ax, word ptr gs:[0xb1f]
000803:  8E D8                        mov      ds, ax
000805:  65 8B 16 1D 0B               mov      dx, word ptr gs:[0xb1d]
00080A:  B8 08 25                     mov      ax, 0x2508
00080D:  CD 21                        int      0x21
00080F:  1F                           pop      ds
000810:  5A                           pop      dx
000811:  58                           pop      ax
000812:  CB                           retf    
