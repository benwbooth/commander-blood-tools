; Commander Blood recovered routine assembly
; module: xdb_scrut
; artifact: output/_tmp_dat/scrut.xdb
; artifact_sha256: 8522e77aad6639cf9bb06f148048fed3c6bf1cfbe6459c9a0be7b987c3c3ac77
; overlay_offset: 0x001C45
; byte_count: 68
; routine_bytes_sha256: 87ae27e0dddce719d73cc636e6b66a703b3bdf12cb0aceb7b8613db316dc8854
; routine_entry: 0x001C45
; group: callback_state_machine
; provenance: resume callback published by method-table slot 13
; raw stop: 0x001C89


output/_tmp_dat/scrut.xdb:     file format binary


Disassembly of section .data:

00001c45 <.data+0x1c45>:
    1c45:	8b 75 16             	mov    0x16(%di),%si
    1c48:	2e 8b 1e e9 1b       	mov    %cs:0x1be9,%bx
    1c4d:	2e 8b 87 eb 1b       	mov    %cs:0x1beb(%bx),%ax
    1c52:	0b c0                	or     %ax,%ax
    1c54:	75 1d                	jne    0x1c73
    1c56:	83 c3 02             	add    $0x2,%bx
    1c59:	83 e3 0f             	and    $0xf,%bx
    1c5c:	2e 89 1e e9 1b       	mov    %bx,%cs:0x1be9
    1c61:	8b 84 ac 00          	mov    0xac(%si),%ax
    1c65:	2d e0 07             	sub    $0x7e0,%ax
    1c68:	25 fc 0f             	and    $0xffc,%ax
    1c6b:	2d 00 08             	sub    $0x800,%ax
    1c6e:	89 84 ac 00          	mov    %ax,0xac(%si)
    1c72:	c3                   	ret
    1c73:	2e c7 06 e5 1b 00 00 	movw   $0x0,%cs:0x1be5
    1c7a:	2e c7 87 eb 1b 00 00 	movw   $0x0,%cs:0x1beb(%bx)
    1c81:	c7 45 36 89 1c       	movw   $0x1c89,0x36(%di)
    1c86:	89 45 3a             	mov    %ax,0x3a(%di)
