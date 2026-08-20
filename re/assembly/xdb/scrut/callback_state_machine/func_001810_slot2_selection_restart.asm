; Commander Blood recovered routine assembly
; module: xdb_scrut
; artifact: output/_tmp_dat/scrut.xdb
; artifact_sha256: 8522e77aad6639cf9bb06f148048fed3c6bf1cfbe6459c9a0be7b987c3c3ac77
; overlay_offset: 0x001810
; byte_count: 11
; routine_bytes_sha256: 4cd8e34c2659337fefadab58f32ffe4618a3a1025ecb22d9dcf4ea952bac2676
; routine_entry: 0x001810
; group: callback_state_machine
; provenance: callback published by 0x1868 and entered by 0x1802
; direct_callees: 0x00181B
; raw stop: 0x00181B


output/_tmp_dat/scrut.xdb:     file format binary


Disassembly of section .data:

00001810 <.data+0x1810>:
    1810:	c7 44 0e 1b 18       	movw   $0x181b,0xe(%si)
    1815:	8b 44 36             	mov    0x36(%si),%ax
    1818:	89 44 56             	mov    %ax,0x56(%si)
