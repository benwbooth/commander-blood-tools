; Commander Blood recovered routine assembly
; module: xdb_manu3
; artifact: output/_tmp_dat/manu3.xdb
; artifact_sha256: d0f64e99a646197906e273edfa0124172307a5cd766c88591c12ebd9ea556d31
; overlay_offset: 0x0006f6
; group: manu3_labeled
; provenance: direct_call_from_0x0, direct_call_from_0x150, label:manu3_face_builder_next, manu3 face builder
; label: manu3_face_builder_next
; label_comment: ACTIVE-RENDER dumps banked (accuracy/manu3/, mouse-driven state; span free-list churned = renderer ran; face links converged steady-state). Face-record pointer decode needs 0x6F6 (the face builder) disassembled to settle which of the 216 records are live and the vertex-record segment/stride (seg2 candidate: rec ~{...,x,y,z,...} i16; pointers overlap => sub-lists per the 0x30/0x88/0x27C descriptor groups). NEXT SLICE: disassemble 0x6F6..0x700
; byte_count: 10
; boundary: cfg_blocks_1_terminals_0
; terminal: none
; direct_callees: none
; indirect_calls: 0
; cxx_source: re/borland/xdb/manu3/manu3_labeled/func_0006f6_manu3_face_builder_next.cpp
; routine_bytes_sha256: ca58b810bd257232784dea4ec18b70600fd2d014ef7fea532fbf7c810973ac29

0006F6:  64 8E 06 06 00               mov      es, word ptr fs:[6]
0006FB:  64 8E 1E 02 00               mov      ds, word ptr fs:[2]
