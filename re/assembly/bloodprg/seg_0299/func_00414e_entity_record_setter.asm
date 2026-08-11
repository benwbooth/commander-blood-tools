; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x00414e
; seg_off: 0299:11be
; group: seg_0299
; provenance: recursive_graph, relocation_proven_far_transfer_target
; label: entity_record_setter
; label_comment: entity record write: ds=gs; si=0x6212 + id<<5 (entity_object_table[id]); writes a field into the 32-byte entity record. Part of the object-instance accessor set
; incoming: call@0x005990->0299:11be
; incoming: call@0x0070dd->0299:11be
; incoming: call@0x007e7d->0299:11be
; incoming: call@0x0090d4->0299:11be
; byte_count: 117
; boundary: cfg_blocks_7_terminals_1
; terminal: retf:1
; direct_callees: none
; indirect_calls: 0
; routine_bytes_sha256: d8a9e868f509555f30b4f5187fffab8b5f02990791a5d3dae58a39d2a564824c

00414E:  50                           push     ax
00414F:  52                           push     dx
004150:  66 55                        push     ebp
004152:  1E                           push     ds
004153:  56                           push     si
004154:  57                           push     di
004155:  06                           push     es
004156:  8C EA                        mov      dx, gs
004158:  8E DA                        mov      ds, dx
00415A:  BE 12 62                     mov      si, 0x6212
00415D:  C1 E0 05                     shl      ax, 5
004160:  03 F0                        add      si, ax
004162:  26 3B 6D 02                  cmp      bp, word ptr es:[di + 2]
004166:  7D 52                        jge      0x41ba
004168:  26 8B 05                     mov      ax, word ptr es:[di]
00416B:  83 E0 04                     and      ax, 4
00416E:  0C 83                        or       al, 0x83
004170:  89 04                        mov      word ptr [si], ax
004172:  83 C7 04                     add      di, 4
004175:  C1 E5 02                     shl      bp, 2
004178:  66 26 8B 2B                  mov      ebp, dword ptr es:[bp + di]
00417C:  8B C5                        mov      ax, bp
00417E:  83 E0 0F                     and      ax, 0xf
004181:  03 F8                        add      di, ax
004183:  66 C1 ED 04                  shr      ebp, 4
004187:  8C C0                        mov      ax, es
004189:  03 C5                        add      ax, bp
00418B:  8E C0                        mov      es, ax
00418D:  89 44 06                     mov      word ptr [si + 6], ax
004190:  89 7C 04                     mov      word ptr [si + 4], di
004193:  26 8B 05                     mov      ax, word ptr es:[di]
004196:  89 44 0C                     mov      word ptr [si + 0xc], ax
004199:  8B 54 14                     mov      dx, word ptr [si + 0x14]
00419C:  0B D2                        or       dx, dx
00419E:  75 03                        jne      0x41a3
0041A0:  89 44 14                     mov      word ptr [si + 0x14], ax
0041A3:  26 8B 45 02                  mov      ax, word ptr es:[di + 2]
0041A7:  89 44 0E                     mov      word ptr [si + 0xe], ax
0041AA:  8B 54 16                     mov      dx, word ptr [si + 0x16]
0041AD:  0B D2                        or       dx, dx
0041AF:  75 03                        jne      0x41b4
0041B1:  89 44 16                     mov      word ptr [si + 0x16], ax
0041B4:  89 5C 08                     mov      word ptr [si + 8], bx
0041B7:  89 4C 0A                     mov      word ptr [si + 0xa], cx
0041BA:  07                           pop      es
0041BB:  5F                           pop      di
0041BC:  5E                           pop      si
0041BD:  1F                           pop      ds
0041BE:  66 5D                        pop      ebp
0041C0:  5A                           pop      dx
0041C1:  58                           pop      ax
0041C2:  CB                           retf    
