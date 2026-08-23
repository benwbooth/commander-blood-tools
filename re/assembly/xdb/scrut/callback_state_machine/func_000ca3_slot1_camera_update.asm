; Commander Blood recovered routine assembly
; module: xdb_scrut
; artifact: output/_tmp_dat/scrut.xdb
; artifact_sha256: 8522e77aad6639cf9bb06f148048fed3c6bf1cfbe6459c9a0be7b987c3c3ac77
; overlay_offset: 0x000CA3
; byte_count: 36
; routine_bytes_sha256: 7504ddab9aed90b826bb2f6c350de5301ccd5d77aa94d0b826a8b05529b1a5ec
; routine_entry: 0x000CA3
; group: callback_state_machine
; provenance: shared camera-to-motion transition
; direct_callees: none
; raw stop: 0x000CC7


output/_tmp_dat/scrut.xdb:     file format binary


Disassembly of section .data:

00000ca3 <.data+0xca3>:
     ca3:	a1 f8 22             	mov    0x22f8,%ax
     ca6:	8b 5c 50             	mov    0x50(%si),%bx
     ca9:	25 fc 0f             	and    $0xffc,%ax
     cac:	81 e3 fc 0f          	and    $0xffc,%bx
     cb0:	2b c3                	sub    %bx,%ax
     cb2:	c1 f8 04             	sar    $0x4,%ax
     cb5:	89 44 56             	mov    %ax,0x56(%si)
     cb8:	8b 44 52             	mov    0x52(%si),%ax
     cbb:	c1 f8 04             	sar    $0x4,%ax
     cbe:	89 44 10             	mov    %ax,0x10(%si)
     cc1:	c7 44 0e c7 0c       	movw   $0xcc7,0xe(%si)
     cc6:	c3                   	ret
