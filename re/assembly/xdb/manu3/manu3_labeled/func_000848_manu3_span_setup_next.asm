; Commander Blood recovered routine assembly
; module: xdb_manu3
; artifact: output/_tmp_dat/manu3.xdb
; artifact_sha256: d0f64e99a646197906e273edfa0124172307a5cd766c88591c12ebd9ea556d31
; overlay_offset: 0x000848
; group: manu3_labeled
; provenance: label:manu3_span_setup_next, manu3 span setup region
; label: manu3_span_setup_next
; label_comment: NEGATIVE banked: vertex +0xA/+0xC are PROJECTED coords (u 212..294 = screen-ish), not UVs — per-vertex UVs falsified. The u/v gradient source must come from the span-setup region (0x848..0xBE6: face -> span records with +0x42 u/+0x44 v/+0x52 du/+0x54 dv+texseg) — that disassembly is the single remaining decode for textured transcription
; byte_count: 1
; boundary: cfg_blocks_1_terminals_1
; terminal: ret:1
; direct_callees: none
; indirect_calls: 0
; routine_bytes_sha256: ae3f4619b0413d70d3004b9131c3752153074e45725be13b9a148978895e359e

000848:  C3                           ret     
