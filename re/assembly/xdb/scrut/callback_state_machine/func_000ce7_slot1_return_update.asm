; Commander Blood recovered routine assembly
; module: xdb_scrut
; artifact: output/_tmp_dat/scrut.xdb
; artifact_sha256: 8522e77aad6639cf9bb06f148048fed3c6bf1cfbe6459c9a0be7b987c3c3ac77
; overlay_offset: 0x000CE7
; byte_count: 11
; routine_bytes_sha256: d5580cd2f2ac4ce275691820010419226fecba0aad027399eb80a0b62e8a4d61
; routine_entry: 0x000CE7
; group: callback_state_machine
; provenance: callback published by slot-1 motion update
; direct_callees: none
; raw stop: 0x000CF2


output/_tmp_dat/scrut.xdb:     file format binary


Disassembly of section .data:

00000ce7 <.data+0xce7>:
     ce7:	ff 4c 54             	decw   0x54(%si)
     cea:	75 05                	jne    0xcf1
     cec:	c7 44 0e 32 0c       	movw   $0xc32,0xe(%si)
     cf1:	c3                   	ret
