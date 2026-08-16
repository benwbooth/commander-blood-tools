
export_check/_tmp_dat/croolis.xdb:     file format binary


Disassembly of section .data:

00001b85 <.data+0x1b85>:
    1b85:	8b 75 16             	mov    0x16(%di),%si
    1b88:	2e 8b 1e 34 1b       	mov    %cs:0x1b34,%bx
    1b8d:	2e 8b 87 36 1b       	mov    %cs:0x1b36(%bx),%ax
    1b92:	0b c0                	or     %ax,%ax
    1b94:	75 1d                	jne    0x1bb3
    1b96:	83 c3 02             	add    $0x2,%bx
    1b99:	83 e3 0f             	and    $0xf,%bx
    1b9c:	2e 89 1e 34 1b       	mov    %bx,%cs:0x1b34
    1ba1:	8b 84 ac 00          	mov    0xac(%si),%ax
    1ba5:	2d e0 07             	sub    $0x7e0,%ax
    1ba8:	25 fc 0f             	and    $0xffc,%ax
    1bab:	2d 00 08             	sub    $0x800,%ax
    1bae:	89 84 ac 00          	mov    %ax,0xac(%si)
    1bb2:	c3                   	ret
    1bb3:	2e c7 06 30 1b 00 00 	movw   $0x0,%cs:0x1b30
    1bba:	2e c7 87 36 1b 00 00 	movw   $0x0,%cs:0x1b36(%bx)
    1bc1:	c7 45 36 c9 1b       	movw   $0x1bc9,0x36(%di)
    1bc6:	89 45 3a             	mov    %ax,0x3a(%di)
    1bc9:	e8 93 ff             	call   0x1b5f
    1bcc:	57                   	push   %di
    1bcd:	8b 75 16             	mov    0x16(%di),%si
    1bd0:	8b 7d 3a             	mov    0x3a(%di),%di
    1bd3:	83 c6 5e             	add    $0x5e,%si
    1bd6:	8b 45 54             	mov    0x54(%di),%ax
    1bd9:	8b d8                	mov    %ax,%bx
    1bdb:	d1 fb                	sar    $1,%bx
    1bdd:	03 44 54             	add    0x54(%si),%ax
    1be0:	03 c3                	add    %bx,%ax
    1be2:	d1 f8                	sar    $1,%ax
    1be4:	89 44 54             	mov    %ax,0x54(%si)
    1be7:	e8 5c 00             	call   0x1c46
    1bea:	5f                   	pop    %di
    1beb:	72 01                	jb     0x1bee
    1bed:	c3                   	ret
    1bee:	c7 45 36 0b 1c       	movw   $0x1c0b,0x36(%di)
    1bf3:	c7 44 54 00 00       	movw   $0x0,0x54(%si)
    1bf8:	8b 75 3a             	mov    0x3a(%di),%si
    1bfb:	c7 44 0e e2 15       	movw   $0x15e2,0xe(%si)
    1c00:	89 75 3c             	mov    %si,0x3c(%di)
    1c03:	2e c7 06 b7 0d 18 00 	movw   $0x18,%cs:0xdb7
    1c0a:	c3                   	ret
    1c0b:	e8 51 ff             	call   0x1b5f
    1c0e:	2e ff 0e b7 0d       	decw   %cs:0xdb7
    1c13:	79 05                	jns    0x1c1a
    1c15:	c7 45 36 1b 1c       	movw   $0x1c1b,0x36(%di)
    1c1a:	c3                   	ret
    1c1b:	57                   	push   %di
    1c1c:	8b 75 16             	mov    0x16(%di),%si
    1c1f:	83 c6 5e             	add    $0x5e,%si
    1c22:	c7 44 54 64 00       	movw   $0x64,0x54(%si)
    1c27:	2e 8b 3e 2e 1b       	mov    %cs:0x1b2e,%di
    1c2c:	e8 17 00             	call   0x1c46
    1c2f:	5f                   	pop    %di
    1c30:	72 01                	jb     0x1c33
    1c32:	c3                   	ret
    1c33:	c7 45 36 85 1b       	movw   $0x1b85,0x36(%di)
    1c38:	8b 5d 3a             	mov    0x3a(%di),%bx
    1c3b:	c7 47 0e b0 15       	movw   $0x15b0,0xe(%bx)
    1c40:	c7 44 54 00 00       	movw   $0x0,0x54(%si)
    1c45:	c3                   	ret
; Commander Blood raw routine disassembly
; module: xdb_croolis
; artifact: export_check/_tmp_dat/croolis.xdb
; routine_entry: 0x001B85
; group: callback_state_machine
; provenance: resume callback published by method-table slot 13
; raw stop: 0x001C46
