; Commander Blood recovered routine assembly
; module: xdb_scrut
; artifact: output/_tmp_dat/scrut.xdb
; artifact_sha256: 8522e77aad6639cf9bb06f148048fed3c6bf1cfbe6459c9a0be7b987c3c3ac77
; overlay_offset: 0x000CC7
; byte_count: 32
; routine_bytes_sha256: b50a287b70398f27f31ada746b8b5b9ad06fe7f654bb15612041930c850b9856
; routine_entry: 0x000CC7
; group: callback_state_machine
; provenance: callback published by slot-1 camera update
; direct_callees: none
; raw stop: 0x000CE7


output/_tmp_dat/scrut.xdb:     file format binary


Disassembly of section .data:

00000cc7 <.data+0xcc7>:
     cc7:	8b 44 56             	mov    0x56(%si),%ax
     cca:	01 44 50             	add    %ax,0x50(%si)
     ccd:	8b 44 10             	mov    0x10(%si),%ax
     cd0:	29 44 52             	sub    %ax,0x52(%si)
     cd3:	ff 44 54             	incw   0x54(%si)
     cd6:	83 7c 54 0f          	cmpw   $0xf,0x54(%si)
     cda:	7e 0a                	jle    0xce6
     cdc:	c7 44 0e e7 0c       	movw   $0xce7,0xe(%si)
     ce1:	c7 44 54 40 00       	movw   $0x40,0x54(%si)
     ce6:	c3                   	ret
