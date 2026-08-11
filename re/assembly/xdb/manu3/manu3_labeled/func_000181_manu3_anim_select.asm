; Commander Blood recovered routine assembly
; module: xdb_manu3
; artifact: output/_tmp_dat/manu3.xdb
; artifact_sha256: d0f64e99a646197906e273edfa0124172307a5cd766c88591c12ebd9ea556d31
; overlay_offset: 0x000181
; group: manu3_labeled
; provenance: direct_call_from_0x0, direct_call_from_0x17c, label:manu3_anim_select, manu3 animation selector
; label: manu3_anim_select
; label_comment: function selector -> relative-offset SEQUENCE table at [0x2306] (=0x3E72): picks an animation/script sequence ([0x102E] cursor, 0x1032 active-tween list)
; byte_count: 26
; boundary: cfg_blocks_1_terminals_1
; terminal: jmp 0x1df:1
; direct_callees: none
; indirect_calls: 0
; cxx_source: re/borland/xdb/manu3/manu3_labeled/func_000181_manu3_anim_select.cpp
; routine_bytes_sha256: 2c95a4b6fd3aaae13c30793487b21286b267e557b3b8ab231caf4802fde54c61

000181:  83 E3 1F                     and      bx, 0x1f
000184:  03 DB                        add      bx, bx
000186:  8B 3E 06 23                  mov      di, word ptr [0x2306]
00018A:  C7 06 2C 10 00 00            mov      word ptr [0x102c], 0
000190:  03 39                        add      di, word ptr [bx + di]
000192:  89 3E 2E 10                  mov      word ptr [0x102e], di
000196:  BB 32 10                     mov      bx, 0x1032
000199:  EB 44                        jmp      0x1df
