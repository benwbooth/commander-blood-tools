; Commander Blood recovered routine assembly
; module: xdb_scrut
; artifact: output/_tmp_dat/scrut.xdb
; artifact_sha256: 8522e77aad6639cf9bb06f148048fed3c6bf1cfbe6459c9a0be7b987c3c3ac77
; overlay_offset: 0x000C18
; byte_count: 26
; routine_bytes_sha256: 4e3e30bf3b60505f70dbdeaec57c819a9f122404c375d3749c423d2aa451b888
; routine_entry: 0x000C18
; group: callback_state_machine
; provenance: callback published by slot-1 wave update
; direct_callees: none
; raw stop: 0x000C32


output/_tmp_dat/scrut.xdb:     file format binary


Disassembly of section .data:

00000c18 <.data+0xc18>:
     c18:	2e a1 76 0b          	mov    %cs:0xb76,%ax
     c1c:	2d b0 00             	sub    $0xb0,%ax
     c1f:	89 44 46             	mov    %ax,0x46(%si)
     c22:	81 44 4e a0 00       	addw   $0xa0,0x4e(%si)
     c27:	81 44 50 d0 00       	addw   $0xd0,0x50(%si)
     c2c:	81 44 52 e0 00       	addw   $0xe0,0x52(%si)
     c31:	c3                   	ret
