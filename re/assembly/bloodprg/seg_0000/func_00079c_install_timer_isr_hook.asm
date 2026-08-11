; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x00079c
; seg_off: 0000:019c
; group: seg_0000
; provenance: recursive_graph
; label: install_timer_isr_hook
; label_comment: ALSO RECORDED as `save_timer_vector`: startup: int21h ax=0x3508 (get interrupt vector 8 = the system timer); store the original handler seg:off to gs:[0xb1d]/[0xb1f] so it can be chained/restored. Hooks the timer interrupt for the game's tick || timer ISR install: get INT 08h vector (int 21h AX=3508) -> save original to gs:[0xb1d]/[0xb1f]; then set INT 08h (int 21h AH=25, DS:DX=cs:0x213) to the game's own timer handler. Hooks the PIT timer, chaining to the saved vector || MERGED 2026-07-25 (audit-fixes #184): one address under several names, folded by union.
; byte_count: 78
; boundary: cfg_blocks_1_terminals_1
; terminal: retf:1
; direct_callees: none
; indirect_calls: 0
; routine_bytes_sha256: f7eee5da7fe0d1a069465c1e93514586ad07e3e5bfe69a4786afae3d595347ba

00079C:  50                           push     ax
00079D:  53                           push     bx
00079E:  52                           push     dx
00079F:  06                           push     es
0007A0:  1E                           push     ds
0007A1:  B8 08 35                     mov      ax, 0x3508
0007A4:  CD 21                        int      0x21
0007A6:  65 89 1E 1D 0B               mov      word ptr gs:[0xb1d], bx
0007AB:  65 8C 06 1F 0B               mov      word ptr gs:[0xb1f], es
0007B0:  B4 25                        mov      ah, 0x25
0007B2:  8C CB                        mov      bx, cs
0007B4:  8E DB                        mov      ds, bx
0007B6:  BA 13 02                     mov      dx, 0x213
0007B9:  CD 21                        int      0x21
0007BB:  FA                           cli     
0007BC:  B0 36                        mov      al, 0x36
0007BE:  E6 43                        out      0x43, al
0007C0:  B8 46 17                     mov      ax, 0x1746
0007C3:  E6 40                        out      0x40, al
0007C5:  8A C4                        mov      al, ah
0007C7:  E6 40                        out      0x40, al
0007C9:  65 C6 06 21 0B 01            mov      byte ptr gs:[0xb21], 1
0007CF:  65 C6 06 22 0B 0B            mov      byte ptr gs:[0xb22], 0xb
0007D5:  65 C7 06 27 0B 19 00         mov      word ptr gs:[0xb27], 0x19
0007DC:  65 C7 06 25 0B 03 00         mov      word ptr gs:[0xb25], 3
0007E3:  FB                           sti     
0007E4:  1F                           pop      ds
0007E5:  07                           pop      es
0007E6:  5A                           pop      dx
0007E7:  5B                           pop      bx
0007E8:  58                           pop      ax
0007E9:  CB                           retf    
