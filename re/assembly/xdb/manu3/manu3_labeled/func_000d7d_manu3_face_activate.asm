; Commander Blood recovered routine assembly
; module: xdb_manu3
; artifact: output/_tmp_dat/manu3.xdb
; artifact_sha256: d0f64e99a646197906e273edfa0124172307a5cd766c88591c12ebd9ea556d31
; overlay_offset: 0x000d7d
; group: manu3_labeled
; provenance: direct_call_from_0x6f6, direct_call_from_0x700, direct_call_from_0x775, direct_call_from_0x849, label:manu3_face_activate, manu3 face activation routine
; label: manu3_face_activate
; label_comment: per-face activation (called per bucket face from 0x8C6): converts a face into EDGE records with linear interpolators {value = base(+0x20) + coord(+0xA)*step(+0x28)} — the u/v/depth gradient setup lives here (next slice). Edge flags +0x1A bit15; per-edge eval at 0x90C (eax = -coord*step+base)
; byte_count: 22
; boundary: cfg_blocks_1_terminals_0
; terminal: none
; direct_callees: none
; indirect_calls: 0
; routine_bytes_sha256: 2b309ac3e61e3280e4874c293d95ac9706bd0aa1151450d61ef8c32fef31287b

000D7D:  26 8B 5C 02                  mov      bx, word ptr es:[si + 2]
000D81:  26 8B 7C 04                  mov      di, word ptr es:[si + 4]
000D85:  26 8B 6C 06                  mov      bp, word ptr es:[si + 6]
000D89:  8B 36 08 09                  mov      si, word ptr [0x908]
000D8D:  0B F6                        or       si, si
000D8F:  0F 84 B5 FA                  je       0x848
