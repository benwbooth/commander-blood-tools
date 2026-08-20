; Commander Blood recovered routine assembly
; module: xdb_scrut
; artifact: output/_tmp_dat/scrut.xdb
; artifact_sha256: 8522e77aad6639cf9bb06f148048fed3c6bf1cfbe6459c9a0be7b987c3c3ac77
; overlay_offset: 0x00159E
; byte_count: 50
; routine_bytes_sha256: 09d3cf58a7fef37a34eb3f352a57cad79c16c452e044f2c53daee13c7a9ce39f
; routine_entry: 0x00159E
; group: callback_state_machine
; provenance: generic slot-3 fallthrough and callback installed by final resume stage
; raw stop: 0x0015D0


output/_tmp_dat/scrut.xdb:     file format binary


Disassembly of section .data:

0000159e <.data+0x159e>:
    159e:	2e c7 86 af 0d 00 00 	movw   $0x0,%cs:0xdaf(%bp)
    15a5:	2e c7 86 ad 0d 08 00 	movw   $0x8,%cs:0xdad(%bp)
    15ac:	c7 44 0e f9 12       	movw   $0x12f9,0xe(%si)
    15b1:	c7 44 52 00 00       	movw   $0x0,0x52(%si)
    15b6:	c7 44 54 08 00       	movw   $0x8,0x54(%si)
    15bb:	c7 44 56 1e 00       	movw   $0x1e,0x56(%si)
    15c0:	a1 5c 10             	mov    0x105c,%ax
    15c3:	c1 c8 03             	ror    $0x3,%ax
    15c6:	1d 00 00             	sbb    $0x0,%ax
    15c9:	89 44 5c             	mov    %ax,0x5c(%si)
    15cc:	a3 5c 10             	mov    %ax,0x105c
    15cf:	c3                   	ret
