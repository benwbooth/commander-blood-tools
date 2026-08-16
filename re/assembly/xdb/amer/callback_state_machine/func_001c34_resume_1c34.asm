
export_check/_tmp_dat/amer.xdb:     file format binary


Disassembly of section .data:

00001c34 <.data+0x1c34>:
    1c34:	8b 75 16             	mov    0x16(%di),%si
    1c37:	2e 8b 1e c8 1b       	mov    %cs:0x1bc8,%bx
    1c3c:	2e 8b 87 ca 1b       	mov    %cs:0x1bca(%bx),%ax
    1c41:	0b c0                	or     %ax,%ax
    1c43:	75 22                	jne    0x1c67
    1c45:	83 c3 02             	add    $0x2,%bx
    1c48:	83 e3 0f             	and    $0xf,%bx
    1c4b:	2e 89 1e c8 1b       	mov    %bx,%cs:0x1bc8
    1c50:	8b 84 ac 00          	mov    0xac(%si),%ax
    1c54:	05 e0 07             	add    $0x7e0,%ax
    1c57:	25 fc 0f             	and    $0xffc,%ax
    1c5a:	2d 00 08             	sub    $0x800,%ax
    1c5d:	89 84 ac 00          	mov    %ax,0xac(%si)
    1c61:	83 84 ae 00 08       	addw   $0x8,0xae(%si)
    1c66:	c3                   	ret
    1c67:	2e c7 06 c4 1b 00 00 	movw   $0x0,%cs:0x1bc4
    1c6e:	2e c7 87 ca 1b 00 00 	movw   $0x0,%cs:0x1bca(%bx)
    1c75:	c7 45 36 7d 1c       	movw   $0x1c7d,0x36(%di)
    1c7a:	89 45 3a             	mov    %ax,0x3a(%di)
    1c7d:	e8 83 ff             	call   0x1c03
    1c80:	57                   	push   %di
    1c81:	8b 75 16             	mov    0x16(%di),%si
    1c84:	8b 7d 3a             	mov    0x3a(%di),%di
    1c87:	83 c6 5e             	add    $0x5e,%si
    1c8a:	8b 45 54             	mov    0x54(%di),%ax
    1c8d:	8b d8                	mov    %ax,%bx
    1c8f:	d1 fb                	sar    $1,%bx
    1c91:	03 44 54             	add    0x54(%si),%ax
    1c94:	03 c3                	add    %bx,%ax
    1c96:	d1 f8                	sar    $1,%ax
    1c98:	89 44 54             	mov    %ax,0x54(%si)
    1c9b:	e8 5c 00             	call   0x1cfa
    1c9e:	5f                   	pop    %di
    1c9f:	72 01                	jb     0x1ca2
    1ca1:	c3                   	ret
    1ca2:	c7 45 36 bf 1c       	movw   $0x1cbf,0x36(%di)
    1ca7:	c7 44 54 00 00       	movw   $0x0,0x54(%si)
    1cac:	8b 75 3a             	mov    0x3a(%di),%si
    1caf:	c7 44 0e 8a 15       	movw   $0x158a,0xe(%si)
    1cb4:	89 75 3c             	mov    %si,0x3c(%di)
    1cb7:	2e c7 06 5f 0d 18 00 	movw   $0x18,%cs:0xd5f
    1cbe:	c3                   	ret
    1cbf:	e8 41 ff             	call   0x1c03
    1cc2:	2e ff 0e 5f 0d       	decw   %cs:0xd5f
    1cc7:	79 05                	jns    0x1cce
    1cc9:	c7 45 36 cf 1c       	movw   $0x1ccf,0x36(%di)
    1cce:	c3                   	ret
    1ccf:	57                   	push   %di
    1cd0:	8b 75 16             	mov    0x16(%di),%si
    1cd3:	83 c6 5e             	add    $0x5e,%si
    1cd6:	c7 44 54 64 00       	movw   $0x64,0x54(%si)
    1cdb:	2e 8b 3e c2 1b       	mov    %cs:0x1bc2,%di
    1ce0:	e8 17 00             	call   0x1cfa
    1ce3:	5f                   	pop    %di
    1ce4:	72 01                	jb     0x1ce7
    1ce6:	c3                   	ret
    1ce7:	c7 45 36 34 1c       	movw   $0x1c34,0x36(%di)
    1cec:	8b 5d 3a             	mov    0x3a(%di),%bx
    1cef:	c7 47 0e 58 15       	movw   $0x1558,0xe(%bx)
    1cf4:	c7 44 54 00 00       	movw   $0x0,0x54(%si)
    1cf9:	c3                   	ret
; Commander Blood raw routine disassembly
; module: xdb_amer
; artifact: export_check/_tmp_dat/amer.xdb
; routine_entry: 0x001C34
; group: callback_state_machine
; provenance: resume callback published by method-table slot 13
; raw stop: 0x001CFA
