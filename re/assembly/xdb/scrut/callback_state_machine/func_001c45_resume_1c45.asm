
export_check/_tmp_dat/scrut.xdb:     file format binary


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
    1ccb:	e8 46 ff             	call   0x1c14
    1cce:	2e ff 0e a5 0d       	decw   %cs:0xda5
    1cd3:	79 05                	jns    0x1cda
    1cd5:	c7 45 36 db 1c       	movw   $0x1cdb,0x36(%di)
    1cda:	c3                   	ret
    1cdb:	57                   	push   %di
    1cdc:	8b 75 16             	mov    0x16(%di),%si
    1cdf:	83 c6 5e             	add    $0x5e,%si
    1ce2:	c7 44 54 64 00       	movw   $0x64,0x54(%si)
    1ce7:	2e 8b 3e e3 1b       	mov    %cs:0x1be3,%di
    1cec:	e8 17 00             	call   0x1d06
    1cef:	5f                   	pop    %di
    1cf0:	72 01                	jb     0x1cf3
    1cf2:	c3                   	ret
    1cf3:	c7 45 36 45 1c       	movw   $0x1c45,0x36(%di)
    1cf8:	8b 5d 3a             	mov    0x3a(%di),%bx
    1cfb:	c7 47 0e 9e 15       	movw   $0x159e,0xe(%bx)
    1d00:	c7 44 54 00 00       	movw   $0x0,0x54(%si)
    1d05:	c3                   	ret
; Commander Blood raw routine disassembly
; module: xdb_scrut
; artifact: export_check/_tmp_dat/scrut.xdb
; routine_entry: 0x001C45
; group: callback_state_machine
; provenance: resume callback published by method-table slot 13
; raw stop: 0x001D06
