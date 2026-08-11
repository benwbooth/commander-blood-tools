; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x00339e
; seg_off: 0299:040e
; group: seg_0299
; provenance: recursive_graph, relocation_proven_far_transfer_target
; label: window_blit_entry
; label_comment: ALSO RECORDED as `blit_coord_guard`: blit coordinate guard: or dx,dx; je/js skip (reject zero-or-negative coordinate); or bp,bp. Entry guard shared by the clipped-blit family (also at 0x3b85, 0x3c6c) that culls off-screen/degenerate spans before drawing || the clipped blit the panel's window chrome goes through (0x299:0x40E; bx=x cx=y dx=w bp=h si=[0xAC8]=0x5F11 the source handle). Shares the blit_coord_guard prologue. UNDECODED: what [0xAC8] resolves to -- the port draws the panel TEXT (vm.rs location_panel_rows) but not yet its chrome || MERGED 2026-07-25 (audit-fixes #184): one address under several names, folded by union.
; incoming: call@0x001eb1->0299:040e
; incoming: call@0x0078c4->0299:040e
; incoming: call@0x007a84->0299:040e
; incoming: call@0x007aaa->0299:040e
; incoming: call@0x007ad1->0299:040e
; incoming: call@0x0084dc->0299:040e
; incoming: call@0x009156->0299:040e
; byte_count: 138
; boundary: cfg_blocks_20_terminals_1
; terminal: retf:1
; direct_callees: none
; indirect_calls: 0
; cxx_source: re/borland/bloodprg/seg_0299/func_00339e_window_blit_entry.cpp
; routine_bytes_sha256: 3edf4d119c626bdaff043093ff0bb74d417875603674e4fc430e5e584d0658ac

00339E:  50                           push     ax
00339F:  53                           push     bx
0033A0:  51                           push     cx
0033A1:  52                           push     dx
0033A2:  55                           push     bp
0033A3:  06                           push     es
0033A4:  57                           push     di
0033A5:  56                           push     si
0033A6:  0B D2                        or       dx, dx
0033A8:  74 75                        je       0x341f
0033AA:  78 73                        js       0x341f
0033AC:  0B ED                        or       bp, bp
0033AE:  74 6F                        je       0x341f
0033B0:  78 6D                        js       0x341f
0033B2:  8B C3                        mov      ax, bx
0033B4:  2B 06 35 52                  sub      ax, word ptr [0x5235]
0033B8:  79 0A                        jns      0x33c4
0033BA:  03 D0                        add      dx, ax
0033BC:  78 61                        js       0x341f
0033BE:  74 5F                        je       0x341f
0033C0:  8B 1E 35 52                  mov      bx, word ptr [0x5235]
0033C4:  8B C3                        mov      ax, bx
0033C6:  03 C2                        add      ax, dx
0033C8:  2B 06 37 52                  sub      ax, word ptr [0x5237]
0033CC:  78 06                        js       0x33d4
0033CE:  2B D0                        sub      dx, ax
0033D0:  78 4D                        js       0x341f
0033D2:  74 4B                        je       0x341f
0033D4:  8B C1                        mov      ax, cx
0033D6:  2B 06 39 52                  sub      ax, word ptr [0x5239]
0033DA:  79 0A                        jns      0x33e6
0033DC:  03 E8                        add      bp, ax
0033DE:  78 3F                        js       0x341f
0033E0:  74 3D                        je       0x341f
0033E2:  8B 0E 39 52                  mov      cx, word ptr [0x5239]
0033E6:  8B C1                        mov      ax, cx
0033E8:  03 C5                        add      ax, bp
0033EA:  2B 06 37 52                  sub      ax, word ptr [0x5237]
0033EE:  78 06                        js       0x33f6
0033F0:  2B E8                        sub      bp, ax
0033F2:  78 2B                        js       0x341f
0033F4:  74 29                        je       0x341f
0033F6:  C4 3E 21 52                  les      di, ptr [0x5221]
0033FA:  8B C1                        mov      ax, cx
0033FC:  86 C4                        xchg     ah, al
0033FE:  C1 E1 06                     shl      cx, 6
003401:  03 C1                        add      ax, cx
003403:  03 C3                        add      ax, bx
003405:  03 F8                        add      di, ax
003407:  8B DE                        mov      bx, si
003409:  8B CD                        mov      cx, bp
00340B:  BD 40 01                     mov      bp, 0x140
00340E:  2B EA                        sub      bp, dx
003410:  51                           push     cx
003411:  8B CA                        mov      cx, dx
003413:  26 8A 05                     mov      al, byte ptr es:[di]
003416:  D7                           xlatb   
003417:  AA                           stosb    byte ptr es:[di], al
003418:  E2 F9                        loop     0x3413
00341A:  03 FD                        add      di, bp
00341C:  59                           pop      cx
00341D:  E2 F1                        loop     0x3410
00341F:  5E                           pop      si
003420:  5F                           pop      di
003421:  07                           pop      es
003422:  5D                           pop      bp
003423:  5A                           pop      dx
003424:  59                           pop      cx
003425:  5B                           pop      bx
003426:  58                           pop      ax
003427:  CB                           retf    
