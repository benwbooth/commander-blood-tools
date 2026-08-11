; Commander Blood recovered routine assembly
; module: xdb_manu3
; artifact: output/_tmp_dat/manu3.xdb
; artifact_sha256: d0f64e99a646197906e273edfa0124172307a5cd766c88591c12ebd9ea556d31
; overlay_offset: 0x0001df
; group: manu3_labeled
; provenance: label:manu3_tween_constructor, manu3 tween constructor
; label: manu3_tween_constructor
; label_comment: EXACT tween-group spec: 8-byte groups {word0: count.lo/phase.hi, word1: (unused/flags), word2: target cell, word3: END VALUE}; construction: step=(end - *target)<<16 / count (Q16/frame), accum=(*target<<16)+step, count-1 -> active record {counter, target, accum, step}; groups gate on phase == [0x102C] (sequences are PHASED, constructor advances the phase). Pose-player implementation fully specified
; byte_count: 145
; boundary: cfg_blocks_9_terminals_3
; terminal: ret:3
; direct_callees: none
; indirect_calls: 0
; cxx_source: re/borland/xdb/manu3/manu3_labeled/func_0001df_manu3_tween_constructor.cpp
; routine_bytes_sha256: bda522f4e9b3ec9663a2568a6a45ec969621599ed2546d6531fbf61fb495409d

0001DF:  8B 36 2E 10                  mov      si, word ptr [0x102e]
0001E3:  66 0F B7 0C                  movzx    ecx, word ptr [si]
0001E7:  0A C9                        or       cl, cl
0001E9:  74 53                        je       0x23e
0001EB:  3A 2E 2C 10                  cmp      ch, byte ptr [0x102c]
0001EF:  75 44                        jne      0x235
0001F1:  8B 3F                        mov      di, word ptr [bx]
0001F3:  83 C3 02                     add      bx, 2
0001F6:  32 ED                        xor      ch, ch
0001F8:  8B 6C 04                     mov      bp, word ptr [si + 4]
0001FB:  89 6D 04                     mov      word ptr [di + 4], bp
0001FE:  8B 44 06                     mov      ax, word ptr [si + 6]
000201:  3E 8B 6E 00                  mov      bp, word ptr ds:[bp]
000205:  2B C5                        sub      ax, bp
000207:  66 C1 E0 10                  shl      eax, 0x10
00020B:  66 C1 E5 10                  shl      ebp, 0x10
00020F:  66 99                        cdq     
000211:  66 F7 F9                     idiv     ecx
000214:  49                           dec      cx
000215:  66 89 45 0A                  mov      dword ptr [di + 0xa], eax
000219:  66 03 E8                     add      ebp, eax
00021C:  89 0D                        mov      word ptr [di], cx
00021E:  66 89 6D 06                  mov      dword ptr [di + 6], ebp
000222:  83 C6 08                     add      si, 8
000225:  8B 0C                        mov      cx, word ptr [si]
000227:  0A C9                        or       cl, cl
000229:  74 13                        je       0x23e
00022B:  3A 2E 2C 10                  cmp      ch, byte ptr [0x102c]
00022F:  74 C0                        je       0x1f1
000231:  89 36 2E 10                  mov      word ptr [0x102e], si
000235:  89 1E 30 10                  mov      word ptr [0x1030], bx
000239:  FF 06 2C 10                  inc      word ptr [0x102c]
00023D:  C3                           ret     
00023E:  89 36 2E 10                  mov      word ptr [0x102e], si
000242:  89 1E 30 10                  mov      word ptr [0x1030], bx
000246:  81 FB 32 10                  cmp      bx, 0x1032
00024A:  75 1F                        jne      0x26b
00024C:  A1 1A 00                     mov      ax, word ptr [0x1a]
00024F:  2D A0 00                     sub      ax, 0xa0
000252:  D1 E0                        shl      ax, 1
000254:  8B 0E E4 23                  mov      cx, word ptr [0x23e4]
000258:  2B C8                        sub      cx, ax
00025A:  89 0E 3C 22                  mov      word ptr [0x223c], cx
00025E:  A1 E2 23                     mov      ax, word ptr [0x23e2]
000261:  A3 3A 22                     mov      word ptr [0x223a], ax
000264:  C7 06 2C 10 00 01            mov      word ptr [0x102c], 0x100
00026A:  C3                           ret     
00026B:  FF 06 2C 10                  inc      word ptr [0x102c]
00026F:  C3                           ret     
