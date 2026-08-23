; Commander Blood recovered routine assembly
; module: xdb_croolis
; artifact: output/_tmp_dat/croolis.xdb
; artifact_sha256: 13eba7c4e4f38662c44c849bd07be293c7583d2b2dc9ae273d7fe94746048c31
; overlay_offset: 0x000CF9
; byte_count: 11
; routine_bytes_sha256: bbf6b1d62aeca626500417b8c6af31c9eda0c270c47d9e7d3ab188952d56deb9
; routine_entry: 0x000CF9
; group: callback_state_machine
; provenance: callback published by slot-1 motion update
; direct_callees: none
; raw stop: 0x000D04


output/_tmp_dat/croolis.xdb:     file format binary


Disassembly of section .data:

00000cf9 <.data+0xcf9>:
     cf9:	ff 4c 54             	decw   0x54(%si)
     cfc:	75 05                	jne    0xd03
     cfe:	c7 44 0e 3e 0c       	movw   $0xc3e,0xe(%si)
     d03:	c3                   	ret
