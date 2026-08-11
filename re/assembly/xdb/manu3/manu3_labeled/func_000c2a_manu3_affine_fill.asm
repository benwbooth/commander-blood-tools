; Commander Blood recovered routine assembly
; module: xdb_manu3
; artifact: output/_tmp_dat/manu3.xdb
; artifact_sha256: d0f64e99a646197906e273edfa0124172307a5cd766c88591c12ebd9ea556d31
; overlay_offset: 0x000c2a
; group: manu3_labeled
; provenance: label:manu3_affine_fill, manu3 affine fill routine
; label: manu3_affine_fill
; label_comment: THE textured inner loop (column-oriented): per row {u(ax) += du(bp); v(dx) += dv(si); texel = ds:[bh:bl] = tex[v.hi*256+u.hi] (256x256 texture, ds = texture seg from lds [rec+0x54]); es:[di]=texel; di += 0x140 (next row)}. Span/edge record fields: +2 flags (0x8001 end/skip), +4/+6 links, +0xA coord, +0x42 u, +0x44 v, +0x52 du, +0x54 dv + texture far ptr. Column S-buffer fill. With this, EVERY manu3 mechanism is decoded — the Rust transcription is fully specified
; byte_count: 160
; boundary: cfg_blocks_7_terminals_2
; terminal: jmp 0xc91:1, jmp word ptr [si + 0x2c]:1
; direct_callees: none
; indirect_calls: 0
; cxx_source: re/borland/xdb/manu3/manu3_labeled/func_000c2a_manu3_affine_fill.cpp
; routine_bytes_sha256: 40ac85af84980d617d9a167077ca2bf9ceb70516a39bf9e4d491010277862df2

000C2A:  03 C5                        add      ax, bp
000C2C:  03 D6                        add      dx, si
000C2E:  8A 2F                        mov      ch, byte ptr [bx]
000C30:  8A DC                        mov      bl, ah
000C32:  26 88 2D                     mov      byte ptr es:[di], ch
000C35:  81 C7 40 01                  add      di, 0x140
000C39:  FE C9                        dec      cl
000C3B:  8A FE                        mov      bh, dh
000C3D:  75 EB                        jne      0xc2a
000C3F:  64 8E 1E 06 00               mov      ds, word ptr fs:[6]
000C44:  5B                           pop      bx
000C45:  F7 47 02 01 80               test     word ptr [bx + 2], 0x8001
000C4A:  74 B8                        je       0xc04
000C4C:  79 98                        jns      0xbe6
000C4E:  EB 41                        jmp      0xc91
; -- non-contiguous block: next 0x000c91 --
000C91:  8B 36 64 09                  mov      si, word ptr [0x964]
000C95:  FF 4C 2E                     dec      word ptr [si + 0x2e]
000C98:  78 2D                        js       0xcc7
000C9A:  8B 44 4A                     mov      ax, word ptr [si + 0x4a]
000C9D:  8B 5C 4C                     mov      bx, word ptr [si + 0x4c]
000CA0:  66 8B 4C 0C                  mov      ecx, dword ptr [si + 0xc]
000CA4:  66 8B 54 24                  mov      edx, dword ptr [si + 0x24]
000CA8:  01 44 42                     add      word ptr [si + 0x42], ax
000CAB:  01 5C 44                     add      word ptr [si + 0x44], bx
000CAE:  66 01 4C 08                  add      dword ptr [si + 8], ecx
000CB2:  66 01 54 20                  add      dword ptr [si + 0x20], edx
000CB6:  8B DE                        mov      bx, si
000CB8:  66 8B 4C 1C                  mov      ecx, dword ptr [si + 0x1c]
000CBC:  8B 37                        mov      si, word ptr [bx]
000CBE:  66 01 4F 18                  add      dword ptr [bx + 0x18], ecx
000CC2:  FF 4C 2E                     dec      word ptr [si + 0x2e]
000CC5:  79 D3                        jns      0xc9a
000CC7:  FF 64 2C                     jmp      word ptr [si + 0x2c]
