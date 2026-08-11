; Commander Blood recovered routine assembly
; module: xdb_manu3
; artifact: output/_tmp_dat/manu3.xdb
; artifact_sha256: d0f64e99a646197906e273edfa0124172307a5cd766c88591c12ebd9ea556d31
; overlay_offset: 0x000121
; group: manu3_labeled
; provenance: label:manu3_init_protocol, manu3 self-relocation/init entry
; label: manu3_init_protocol
; label_comment: croolis's decoded overlay protocol generalizes: overlays self-relocate (segment table at file 0x0C..0x10) and their INIT fills the fs:0x22F0 pointer block with load-segment-relocated values (croolis: fs:0x22F0=0xFF11, 0x22F4=0xD9C2...). manu3's stale file values = build-time layout; init rewrites. Packed sources = the 0x3DFC.. descriptor blocks; the init body (head code past the 0x121 skip target) is the next disassembly slice to settle descriptor {src,len,stride,dst} semantics
; byte_count: 47
; boundary: cfg_blocks_1_terminals_1
; terminal: jmp 0x11f:1
; direct_callees: none
; indirect_calls: 0
; cxx_source: re/borland/xdb/manu3/manu3_labeled/func_000121_manu3_init_protocol.cpp
; routine_bytes_sha256: 53ee04799c1a04e8fa75a5da3c3003e16a4c3de7a8b51ec0ab5c519b9363a0f7

000121:  8C C8                        mov      ax, cs
000123:  2E 03 06 68 13               add      ax, word ptr cs:[0x1368]
000128:  8E D8                        mov      ds, ax
00012A:  8E E0                        mov      fs, ax
00012C:  2E A3 6A 13                  mov      word ptr cs:[0x136a], ax
000130:  03 06 0C 00                  add      ax, word ptr [0xc]
000134:  A3 02 00                     mov      word ptr [2], ax
000137:  03 06 0E 00                  add      ax, word ptr [0xe]
00013B:  A3 04 00                     mov      word ptr [4], ax
00013E:  03 06 10 00                  add      ax, word ptr [0x10]
000142:  A3 06 00                     mov      word ptr [6], ax
000145:  8E C0                        mov      es, ax
000147:  26 C7 06 7E 06 E0 0A         mov      word ptr es:[0x67e], 0xae0
00014E:  EB CF                        jmp      0x11f
