; Commander Blood recovered routine assembly
; module: xdb_scrut
; artifact: output/_tmp_dat/scrut.xdb
; artifact_sha256: 8522e77aad6639cf9bb06f148048fed3c6bf1cfbe6459c9a0be7b987c3c3ac77
; overlay_offset: 0x001711
; byte_count: 10
; routine_bytes_sha256: 98267be9dfb4b3a616cbd7b02c9d4a872ec7e998cec75d1bc1f33faab055a204
; routine_entry: 0x001711
; group: callback_state_machine
; provenance: shared restart tail reached by callbacks 0x17E6 and 0x181B
; direct_callees: 0x00171B
; raw stop: 0x00171B


output/_tmp_dat/scrut.xdb:     file format binary


Disassembly of section .data:

00001711 <.data+0x1711>:
    1711:	c7 44 0e 1b 17       	movw   $0x171b,0xe(%si)
    1716:	c7 44 58 64 00       	movw   $0x64,0x58(%si)
