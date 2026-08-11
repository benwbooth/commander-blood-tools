; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x004f62
; seg_off: 0299:1fd2
; group: seg_0299
; provenance: static_dispatch_table_target
; label: frame_func_locals18
; label_comment: framed function: push bp; sub sp,0x12; bp=sp. C-style stack frame with 18 bytes of locals (a compound routine with local working storage)
; incoming: sprite_blitter_candidates:blit_4
; byte_count: 312
; boundary: cfg_blocks_22_terminals_3
; terminal: jmp 0x508a:2, ret:1
; direct_callees: none
; indirect_calls: 0
; routine_bytes_sha256: edfe8ea346c15b34b87fa3432eea992f12c95c435118f8bec8283084c42fe6c6

004F62:  66 50                        push     eax
004F64:  53                           push     bx
004F65:  66 51                        push     ecx
004F67:  66 52                        push     edx
004F69:  06                           push     es
004F6A:  57                           push     di
004F6B:  1E                           push     ds
004F6C:  56                           push     si
004F6D:  8B C5                        mov      ax, bp
004F6F:  55                           push     bp
004F70:  83 EC 12                     sub      sp, 0x12
004F73:  8B EC                        mov      bp, sp
004F75:  C5 75 04                     lds      si, ptr [di + 4]
004F78:  52                           push     dx
004F79:  50                           push     ax
004F7A:  66 33 D2                     xor      edx, edx
004F7D:  66 89 56 0C                  mov      dword ptr [bp + 0xc], edx
004F81:  AD                           lodsw    ax, word ptr [si]
004F82:  89 46 0A                     mov      word ptr [bp + 0xa], ax
004F85:  66 C1 E0 10                  shl      eax, 0x10
004F89:  66 26 0F B7 4D 0C            movzx    ecx, word ptr es:[di + 0xc]
004F8F:  0B C9                        or       cx, cx
004F91:  75 06                        jne      0x4f99
004F93:  83 C4 04                     add      sp, 4
004F96:  E9 F1 00                     jmp      0x508a
004F99:  66 F7 F1                     div      ecx
004F9C:  66 89 46 00                  mov      dword ptr [bp], eax
004FA0:  AD                           lodsw    ax, word ptr [si]
004FA1:  66 C1 E0 10                  shl      eax, 0x10
004FA5:  26 8B 4D 0E                  mov      cx, word ptr es:[di + 0xe]
004FA9:  0B C9                        or       cx, cx
004FAB:  75 06                        jne      0x4fb3
004FAD:  83 C4 04                     add      sp, 4
004FB0:  E9 D7 00                     jmp      0x508a
004FB3:  66 33 D2                     xor      edx, edx
004FB6:  66 F7 F1                     div      ecx
004FB9:  66 89 46 04                  mov      dword ptr [bp + 4], eax
004FBD:  8B C3                        mov      ax, bx
004FBF:  26 2B 45 1C                  sub      ax, word ptr es:[di + 0x1c]
004FC3:  79 1A                        jns      0x4fdf
004FC5:  F7 D8                        neg      ax
004FC7:  2B C8                        sub      cx, ax
004FC9:  66 98                        cwde    
004FCB:  66 F7 66 04                  mul      dword ptr [bp + 4]
004FCF:  89 46 0E                     mov      word ptr [bp + 0xe], ax
004FD2:  66 C1 E8 10                  shr      eax, 0x10
004FD6:  F7 66 0A                     mul      word ptr [bp + 0xa]
004FD9:  03 F0                        add      si, ax
004FDB:  26 8B 5D 1C                  mov      bx, word ptr es:[di + 0x1c]
004FDF:  58                           pop      ax
004FE0:  26 2B 45 1E                  sub      ax, word ptr es:[di + 0x1e]
004FE4:  78 02                        js       0x4fe8
004FE6:  2B C8                        sub      cx, ax
004FE8:  26 8B 45 0C                  mov      ax, word ptr es:[di + 0xc]
004FEC:  89 46 08                     mov      word ptr [bp + 8], ax
004FEF:  26 8B 55 08                  mov      dx, word ptr es:[di + 8]
004FF3:  8B C2                        mov      ax, dx
004FF5:  26 2B 45 18                  sub      ax, word ptr es:[di + 0x18]
004FF9:  79 18                        jns      0x5013
004FFB:  F7 D8                        neg      ax
004FFD:  29 46 08                     sub      word ptr [bp + 8], ax
005000:  66 98                        cwde    
005002:  66 F7 66 00                  mul      dword ptr [bp]
005006:  89 46 0C                     mov      word ptr [bp + 0xc], ax
005009:  66 C1 E8 10                  shr      eax, 0x10
00500D:  03 F0                        add      si, ax
00500F:  26 8B 55 18                  mov      dx, word ptr es:[di + 0x18]
005013:  58                           pop      ax
005014:  26 2B 45 1A                  sub      ax, word ptr es:[di + 0x1a]
005018:  78 03                        js       0x501d
00501A:  29 46 08                     sub      word ptr [bp + 8], ax
00501D:  0B C9                        or       cx, cx
00501F:  74 69                        je       0x508a
005021:  78 67                        js       0x508a
005023:  8B 46 08                     mov      ax, word ptr [bp + 8]
005026:  0B C0                        or       ax, ax
005028:  74 60                        je       0x508a
00502A:  78 5E                        js       0x508a
00502C:  65 C4 3E 21 52               les      di, ptr gs:[0x5221]
005031:  8B C3                        mov      ax, bx
005033:  86 C4                        xchg     ah, al
005035:  C1 E3 06                     shl      bx, 6
005038:  03 C3                        add      ax, bx
00503A:  03 C2                        add      ax, dx
00503C:  03 F8                        add      di, ax
00503E:  83 C6 04                     add      si, 4
005041:  32 ED                        xor      ch, ch
005043:  8B 46 06                     mov      ax, word ptr [bp + 6]
005046:  F7 66 0A                     mul      word ptr [bp + 0xa]
005049:  89 46 06                     mov      word ptr [bp + 6], ax
00504C:  BA 40 01                     mov      dx, 0x140
00504F:  2B 56 08                     sub      dx, word ptr [bp + 8]
005052:  89 56 10                     mov      word ptr [bp + 0x10], dx
005055:  8B 5E 0E                     mov      bx, word ptr [bp + 0xe]
005058:  8A E1                        mov      ah, cl
00505A:  56                           push     si
00505B:  53                           push     bx
00505C:  8B 5E 00                     mov      bx, word ptr [bp]
00505F:  8B 56 0C                     mov      dx, word ptr [bp + 0xc]
005062:  8B 4E 08                     mov      cx, word ptr [bp + 8]
005065:  8A 04                        mov      al, byte ptr [si]
005067:  0A C0                        or       al, al
005069:  74 03                        je       0x506e
00506B:  26 88 05                     mov      byte ptr es:[di], al
00506E:  47                           inc      di
00506F:  03 D3                        add      dx, bx
005071:  13 76 02                     adc      si, word ptr [bp + 2]
005074:  E2 EF                        loop     0x5065
005076:  8A CC                        mov      cl, ah
005078:  5B                           pop      bx
005079:  5E                           pop      si
00507A:  03 7E 10                     add      di, word ptr [bp + 0x10]
00507D:  03 76 06                     add      si, word ptr [bp + 6]
005080:  03 5E 04                     add      bx, word ptr [bp + 4]
005083:  73 03                        jae      0x5088
005085:  03 76 0A                     add      si, word ptr [bp + 0xa]
005088:  E2 CE                        loop     0x5058
00508A:  83 C4 12                     add      sp, 0x12
00508D:  5D                           pop      bp
00508E:  5E                           pop      si
00508F:  1F                           pop      ds
005090:  5F                           pop      di
005091:  07                           pop      es
005092:  66 5A                        pop      edx
005094:  66 59                        pop      ecx
005096:  5B                           pop      bx
005097:  66 58                        pop      eax
005099:  C3                           ret     
