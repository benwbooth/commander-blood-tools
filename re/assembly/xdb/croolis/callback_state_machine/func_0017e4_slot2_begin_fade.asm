; Commander Blood recovered routine assembly
; module: xdb_croolis
; artifact: output/_tmp_dat/croolis.xdb
; artifact_sha256: 13eba7c4e4f38662c44c849bd07be293c7583d2b2dc9ae273d7fe94746048c31
; overlay_offset: 0x0017E4
; byte_count: 14
; routine_bytes_sha256: 24917f24c813b1498fc1a70c9607f56455a82f6e516b6d7b62a055086a4fb4c4
; routine_entry: 0x0017E4
; group: callback_state_machine
; provenance: internal control-latch transition reached by 0x178E
; direct_callees: 0x0017F2
; raw stop: 0x0017F2


output/_tmp_dat/croolis.xdb:     file format binary


Disassembly of section .data:

000017e4 <.data+0x17e4>:
    17e4:	83 6c 46 1e          	subw   $0x1e,0x46(%si)
    17e8:	c7 45 38 b2 00       	movw   $0xb2,0x38(%di)
    17ed:	c7 44 0e f2 17       	movw   $0x17f2,0xe(%si)
