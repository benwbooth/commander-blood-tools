; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x0040d0
; seg_off: 0299:1140
; group: seg_0299
; provenance: recursive_graph, relocation_proven_far_transfer_target
; label: entity_object_populate
; label_comment: populate entity_object_table[AX] from resource handle DX: di=0x6212+AX<<5; lcall resource_handle_resolve 0x5320 (ds=resource seg,si); bound-check bp<[si+2]; flags=([si]&4)|0x83 -> +0x00; unpack packed dword ds:[bp<<2+si] into a far ptr (low nibble=extra si offset, >>4=segment delta added to ds) -> +0x04/+0x06; copy data words -> +0x0c/+0x0e, init +0x14/+0x16. First decode of the object-instance system || MERGED 2026-07-25 (audit-fixes #130), also recorded as: ACTIVATES an entity: AX=id DX=resource handle -> di=0x6212+id*32; resolve handle 0x4b9:0x190; flags=([res_hdr]&4)|0x83 (0x80 active bit) at +0; unpack object far ptr +4/+6; copy data +0xc/+0xe; init +0x14/+0x16. THE entity-activation primitive (nav destinations 0x15..0x1F, crew, location content). = 0x299:0x1140
; incoming: call@0x008d76->0299:1140
; incoming: call@0x008d96->0299:1140
; incoming: call@0x008df5->0299:1140
; incoming: call@0x0095e7->0299:1140
; byte_count: 126
; boundary: cfg_blocks_8_terminals_1
; terminal: retf:1
; direct_callees: none
; indirect_calls: 1
; routine_bytes_sha256: 21ebe98dec4b2b51201ea38666d0d4f1a2f4d198036bc892bad67e4a31ec2909

0040D0:  50                           push     ax
0040D1:  52                           push     dx
0040D2:  66 55                        push     ebp
0040D4:  1E                           push     ds
0040D5:  56                           push     si
0040D6:  57                           push     di
0040D7:  BF 12 62                     mov      di, 0x6212
0040DA:  C1 E0 05                     shl      ax, 5
0040DD:  03 F8                        add      di, ax
0040DF:  8B C2                        mov      ax, dx
0040E1:  9A 90 01 B9 04               lcall    0x4b9, 0x190
0040E6:  0B C0                        or       ax, ax
0040E8:  74 5C                        je       0x4146
0040EA:  3B 6C 02                     cmp      bp, word ptr [si + 2]
0040ED:  7D 57                        jge      0x4146
0040EF:  8B 04                        mov      ax, word ptr [si]
0040F1:  83 E0 04                     and      ax, 4
0040F4:  0C 83                        or       al, 0x83
0040F6:  65 89 05                     mov      word ptr gs:[di], ax
0040F9:  83 C6 04                     add      si, 4
0040FC:  C1 E5 02                     shl      bp, 2
0040FF:  66 3E 8B 2A                  mov      ebp, dword ptr ds:[bp + si]
004103:  8B C5                        mov      ax, bp
004105:  83 E0 0F                     and      ax, 0xf
004108:  03 F0                        add      si, ax
00410A:  66 C1 ED 04                  shr      ebp, 4
00410E:  8C D8                        mov      ax, ds
004110:  03 C5                        add      ax, bp
004112:  8E D8                        mov      ds, ax
004114:  65 89 45 06                  mov      word ptr gs:[di + 6], ax
004118:  65 89 75 04                  mov      word ptr gs:[di + 4], si
00411C:  AD                           lodsw    ax, word ptr [si]
00411D:  65 89 45 0C                  mov      word ptr gs:[di + 0xc], ax
004121:  65 8B 55 14                  mov      dx, word ptr gs:[di + 0x14]
004125:  0B D2                        or       dx, dx
004127:  75 04                        jne      0x412d
004129:  65 89 45 14                  mov      word ptr gs:[di + 0x14], ax
00412D:  AD                           lodsw    ax, word ptr [si]
00412E:  65 89 45 0E                  mov      word ptr gs:[di + 0xe], ax
004132:  65 8B 55 16                  mov      dx, word ptr gs:[di + 0x16]
004136:  0B D2                        or       dx, dx
004138:  75 04                        jne      0x413e
00413A:  65 89 45 16                  mov      word ptr gs:[di + 0x16], ax
00413E:  65 89 5D 08                  mov      word ptr gs:[di + 8], bx
004142:  65 89 4D 0A                  mov      word ptr gs:[di + 0xa], cx
004146:  5F                           pop      di
004147:  5E                           pop      si
004148:  1F                           pop      ds
004149:  66 5D                        pop      ebp
00414B:  5A                           pop      dx
00414C:  58                           pop      ax
00414D:  CB                           retf    
