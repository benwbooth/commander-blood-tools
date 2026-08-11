; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x0027e9
; seg_off: 01ce:0509
; group: seg_01ce
; provenance: recursive_graph, relocation_proven_far_transfer_target
; label: dos_set_drive_and_chdir
; label_comment: DRIVE + DIRECTORY setup, NOT character output (audit-fixes #351 corrects a label that was wrong in its interrupt, its function and its name). Gated on [0x0AE0]&1: `mov ah,0x0E / mov dl,[0x01B9] / int 0x21` @0x27F7..0x27FD is DOS SELECT DEFAULT DRIVE with DL = the drive byte -- the old label read AH=0x0E as a BIOS teletype call and named the wrong interrupt; the instruction here is INT 21h. It then does `mov dx,0x01DA / mov ah,0x3B` @0x27FF, DOS CHDIR to the path at DS:0x01DA. So this is the launch-path setup (cf. the game's `WRIC:\cblood\` argument), not console output.
; incoming: call@0x001332->01ce:0509
; incoming: call@0x001561->01ce:0509
; byte_count: 38
; boundary: cfg_blocks_3_terminals_1
; terminal: retf:1
; direct_callees: none
; indirect_calls: 0
; routine_bytes_sha256: eb6b6b6954b87081c3141b81b9cd0507f5065eabb570dd365a600725a3cd650c

0027E9:  50                           push     ax
0027EA:  1E                           push     ds
0027EB:  52                           push     dx
0027EC:  8C E8                        mov      ax, gs
0027EE:  8E D8                        mov      ds, ax
0027F0:  F6 06 E0 0A 01               test     byte ptr [0xae0], 1
0027F5:  74 14                        je       0x280b
0027F7:  B4 0E                        mov      ah, 0xe
0027F9:  8A 16 B9 01                  mov      dl, byte ptr [0x1b9]
0027FD:  CD 21                        int      0x21
0027FF:  BA DA 01                     mov      dx, 0x1da
002802:  B4 3B                        mov      ah, 0x3b
002804:  CD 21                        int      0x21
002806:  C6 06 E0 0A 00               mov      byte ptr [0xae0], 0
00280B:  5A                           pop      dx
00280C:  1F                           pop      ds
00280D:  58                           pop      ax
00280E:  CB                           retf    
