; Commander Blood recovered routine assembly
; module: xdb_scrut
; artifact: output/_tmp_dat/scrut.xdb
; artifact_sha256: 8522e77aad6639cf9bb06f148048fed3c6bf1cfbe6459c9a0be7b987c3c3ac77
; overlay_offset: 0x001C89
; byte_count: 66
; routine_bytes_sha256: 4bacd769ed768410774f2f15ecf53ff07d317e521f3f412d8e19854f0d5cf6f3
; routine_entry: 0x001C89
; group: callback_state_machine
; provenance: continuation stored at context +0x36 by 0x1C45
; raw stop: 0x001CCB


output/_tmp_dat/scrut.xdb:     file format binary


Disassembly of section .data:

00001c89 <.data+0x1c89>:
    1c89:	e8 88 ff             	call   0x1c14
    1c8c:	57                   	push   %di
    1c8d:	8b 75 16             	mov    0x16(%di),%si
    1c90:	8b 7d 3a             	mov    0x3a(%di),%di
    1c93:	83 c6 5e             	add    $0x5e,%si
    1c96:	8b 45 54             	mov    0x54(%di),%ax
    1c99:	8b d8                	mov    %ax,%bx
    1c9b:	d1 fb                	sar    $1,%bx
    1c9d:	03 44 54             	add    0x54(%si),%ax
    1ca0:	03 c3                	add    %bx,%ax
    1ca2:	d1 f8                	sar    $1,%ax
    1ca4:	89 44 54             	mov    %ax,0x54(%si)
    1ca7:	e8 5c 00             	call   0x1d06
    1caa:	5f                   	pop    %di
    1cab:	72 01                	jb     0x1cae
    1cad:	c3                   	ret
    1cae:	c7 45 36 cb 1c       	movw   $0x1ccb,0x36(%di)
    1cb3:	c7 44 54 00 00       	movw   $0x0,0x54(%si)
    1cb8:	8b 75 3a             	mov    0x3a(%di),%si
    1cbb:	c7 44 0e d0 15       	movw   $0x15d0,0xe(%si)
    1cc0:	89 75 3c             	mov    %si,0x3c(%di)
    1cc3:	2e c7 06 a5 0d 18 00 	movw   $0x18,%cs:0xda5
    1cca:	c3                   	ret
