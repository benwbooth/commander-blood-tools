; Commander Blood recovered routine assembly
; module: xdb_croolis
; artifact: output/_tmp_dat/croolis.xdb
; artifact_sha256: 13eba7c4e4f38662c44c849bd07be293c7583d2b2dc9ae273d7fe94746048c31
; overlay_offset: 0x001815
; byte_count: 19
; routine_bytes_sha256: 6d49a291f0541c75cbf5f96ab342d65931ad93872ac6796d20f05892f7d88b24
; routine_entry: 0x001815
; group: callback_state_machine
; provenance: internal selection transition reached by callback 0x1727
; direct_callees: 0x001828
; raw stop: 0x001828


output/_tmp_dat/croolis.xdb:     file format binary


Disassembly of section .data:

00001815 <.data+0x1815>:
    1815:	c7 44 0e 28 18       	movw   $0x1828,0xe(%si)
    181a:	2e c7 06 a2 16 00 00 	movw   $0x0,%cs:0x16a2
    1821:	2e c7 06 a0 16 00 00 	movw   $0x0,%cs:0x16a0
