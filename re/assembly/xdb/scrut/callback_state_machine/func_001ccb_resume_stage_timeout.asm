; Commander Blood recovered routine assembly
; module: xdb_scrut
; artifact: output/_tmp_dat/scrut.xdb
; artifact_sha256: 8522e77aad6639cf9bb06f148048fed3c6bf1cfbe6459c9a0be7b987c3c3ac77
; overlay_offset: 0x001CCB
; byte_count: 16
; routine_bytes_sha256: 5e665638886cc5812a253a978c8d3cbc212da776776ecfd1ae52fbe9dde74ac4
; routine_entry: 0x001CCB
; group: callback_state_machine
; provenance: continuation stored at context +0x36 by 0x1C89
; raw stop: 0x001CDB


output/_tmp_dat/scrut.xdb:     file format binary


Disassembly of section .data:

00001ccb <.data+0x1ccb>:
    1ccb:	e8 46 ff             	call   0x1c14
    1cce:	2e ff 0e a5 0d       	decw   %cs:0xda5
    1cd3:	79 05                	jns    0x1cda
    1cd5:	c7 45 36 db 1c       	movw   $0x1cdb,0x36(%di)
    1cda:	c3                   	ret
