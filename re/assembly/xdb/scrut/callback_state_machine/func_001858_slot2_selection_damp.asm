; Commander Blood recovered routine assembly
; module: xdb_scrut
; artifact: output/_tmp_dat/scrut.xdb
; artifact_sha256: 8522e77aad6639cf9bb06f148048fed3c6bf1cfbe6459c9a0be7b987c3c3ac77
; overlay_offset: 0x001858
; byte_count: 16
; routine_bytes_sha256: 572e53995576e48498edd2c441ddd5ae7e341ffd82b323df0b6f34844a4bd245
; routine_entry: 0x001858
; group: callback_state_machine
; provenance: callback published by selection callback 0x181B
; direct_callees: 0x001868, 0x0018D9
; raw stop: 0x001868


output/_tmp_dat/scrut.xdb:     file format binary


Disassembly of section .data:

00001858 <.data+0x1858>:
    1858:	d1 7c 54             	sarw   $1,0x54(%si)
    185b:	74 06                	je     0x1863
    185d:	b1 14                	mov    $0x14,%cl
    185f:	e8 77 00             	call   0x18d9
    1862:	c3                   	ret
    1863:	c7 44 0e 68 18       	movw   $0x1868,0xe(%si)
