; Commander Blood recovered routine assembly
; module: xdb_croolis
; artifact: output/_tmp_dat/croolis.xdb
; artifact_sha256: 13eba7c4e4f38662c44c849bd07be293c7583d2b2dc9ae273d7fe94746048c31
; overlay_offset: 0x000C24
; byte_count: 26
; routine_bytes_sha256: 4e3e30bf3b60505f70dbdeaec57c819a9f122404c375d3749c423d2aa451b888
; routine_entry: 0x000C24
; group: callback_state_machine
; provenance: callback published by slot-1 wave update
; direct_callees: none
; raw stop: 0x000C3E


output/_tmp_dat/croolis.xdb:     file format binary


Disassembly of section .data:

00000c24 <.data+0xc24>:
     c24:	2e a1 76 0b          	mov    %cs:0xb76,%ax
     c28:	2d b0 00             	sub    $0xb0,%ax
     c2b:	89 44 46             	mov    %ax,0x46(%si)
     c2e:	81 44 4e a0 00       	addw   $0xa0,0x4e(%si)
     c33:	81 44 50 d0 00       	addw   $0xd0,0x50(%si)
     c38:	81 44 52 e0 00       	addw   $0xe0,0x52(%si)
     c3d:	c3                   	ret
