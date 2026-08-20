; Commander Blood recovered routine assembly
; module: xdb_scrut
; artifact: output/_tmp_dat/scrut.xdb
; artifact_sha256: 8522e77aad6639cf9bb06f148048fed3c6bf1cfbe6459c9a0be7b987c3c3ac77
; overlay_offset: 0x001C14
; byte_count: 49
; routine_bytes_sha256: a4a99542e0dbb814845cf741cef70cd493f95ec4274a49401fc39ca31f048f4e
; routine_entry: 0x001C14
; group: callback_state_machine
; provenance: near helper called by resume pair and timeout stages
; raw stop: 0x001C45


output/_tmp_dat/scrut.xdb:     file format binary


Disassembly of section .data:

00001c14 <.data+0x1c14>:
    1c14:	8b 75 1c             	mov    0x1c(%di),%si
    1c17:	8e 06 02 00          	mov    0x2,%es
    1c1b:	8b 45 38             	mov    0x38(%di),%ax
    1c1e:	8b d0                	mov    %ax,%dx
    1c20:	98                   	cbtw
    1c21:	26 01 44 02          	add    %ax,%es:0x2(%si)
    1c25:	26 29 84 5e 03       	sub    %ax,%es:0x35e(%si)
    1c2a:	26 01 84 4a 03       	add    %ax,%es:0x34a(%si)
    1c2f:	26 29 84 f6 01       	sub    %ax,%es:0x1f6(%si)
    1c34:	02 f2                	add    %dl,%dh
    1c36:	75 02                	jne    0x1c3a
    1c38:	b2 02                	mov    $0x2,%dl
    1c3a:	80 fe 16             	cmp    $0x16,%dh
    1c3d:	7c 02                	jl     0x1c41
    1c3f:	b2 fe                	mov    $0xfe,%dl
    1c41:	89 55 38             	mov    %dx,0x38(%di)
    1c44:	c3                   	ret
