; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x00792d
; seg_off: 071e:014d
; group: seg_071e
; provenance: recursive_graph
; label: camera_nav_update
; label_comment: camera/nav per-frame update: test [0x278a]&1; al=[0x27df] (the camera-approach FSM phase, ship3d.rs Ship3dCameraApproach). Steps the ship-nav camera state
; byte_count: 184
; boundary: cfg_blocks_9_terminals_2
; terminal: jmp 0x79df:1, ret:1
; direct_callees: 0x0082c3
; indirect_calls: 0
; routine_bytes_sha256: de36e7ef9719c00e2e30248c395eb1bb6d6f39e379954e2d0cdc9d6a7c74cd2d

00792D:  50                           push     ax
00792E:  1E                           push     ds
00792F:  56                           push     si
007930:  06                           push     es
007931:  57                           push     di
007932:  F6 06 8A 27 01               test     byte ptr [0x278a], 1
007937:  0F 85 A4 00                  jne      0x79df
00793B:  A0 DF 27                     mov      al, byte ptr [0x27df]
00793E:  0A C0                        or       al, al
007940:  0F 85 9B 00                  jne      0x79df
007944:  8E 06 26 67                  mov      es, word ptr [0x6726]
007948:  8B 3E 52 67                  mov      di, word ptr [0x6752]
00794C:  26 8B 7D 16                  mov      di, word ptr es:[di + 0x16]
007950:  26 F7 05 18 00               test     word ptr es:[di], 0x18
007955:  0F 84 86 00                  je       0x79df
007959:  0E                           push     cs
00795A:  E8 66 09                     call     0x82c3
00795D:  83 F8 1F                     cmp      ax, 0x1f
007960:  75 7D                        jne      0x79df
007962:  C7 06 32 0A 0C 00            mov      word ptr [0xa32], 0xc
007968:  26 83 7D 14 00               cmp      word ptr es:[di + 0x14], 0
00796D:  75 13                        jne      0x7982
00796F:  80 0E 93 27 04               or       byte ptr [0x2793], 4
007974:  A0 63 2A                     mov      al, byte ptr [0x2a63]
007977:  A8 02                        test     al, 2
007979:  75 64                        jne      0x79df
00797B:  0C 08                        or       al, 8
00797D:  A2 63 2A                     mov      byte ptr [0x2a63], al
007980:  EB 5D                        jmp      0x79df
007982:  8C E8                        mov      ax, gs
007984:  8E C0                        mov      es, ax
007986:  BF 51 58                     mov      di, 0x5851
007989:  B9 C0 00                     mov      cx, 0xc0
00798C:  66 33 C0                     xor      eax, eax
00798F:  F3 66 AB                     rep stosd dword ptr es:[di], eax
007992:  BF 51 55                     mov      di, 0x5551
007995:  BE 51 52                     mov      si, 0x5251
007998:  B9 C0 00                     mov      cx, 0xc0
00799B:  F3 66 A5                     rep movsd dword ptr es:[di], dword ptr [si]
00799E:  C7 06 4F 52 00 00            mov      word ptr [0x524f], 0
0079A4:  C7 06 4D 52 14 00            mov      word ptr [0x524d], 0x14
0079AA:  C6 06 51 5B 00               mov      byte ptr [0x5b51], 0
0079AF:  C6 06 52 5B FF               mov      byte ptr [0x5b52], 0xff
0079B4:  C7 06 93 27 00 00            mov      word ptr [0x2793], 0
0079BA:  C7 06 F3 24 05 00            mov      word ptr [0x24f3], 5
0079C0:  C6 06 35 25 01               mov      byte ptr [0x2535], 1
0079C5:  C6 06 BB 67 00               mov      byte ptr [0x67bb], 0
0079CA:  C6 06 2D 25 00               mov      byte ptr [0x252d], 0
0079CF:  C7 06 27 25 00 00            mov      word ptr [0x2527], 0
0079D5:  C6 06 2F 25 00               mov      byte ptr [0x252f], 0
0079DA:  C6 06 29 25 00               mov      byte ptr [0x2529], 0
0079DF:  5F                           pop      di
0079E0:  07                           pop      es
0079E1:  5E                           pop      si
0079E2:  1F                           pop      ds
0079E3:  58                           pop      ax
0079E4:  C3                           ret     
