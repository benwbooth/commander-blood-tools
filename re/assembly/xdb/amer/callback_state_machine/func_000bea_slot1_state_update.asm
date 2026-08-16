; Commander Blood recovered routine assembly
; module: xdb_amer
; artifact: export_check/_tmp_dat/amer.xdb
; routine_entry: 0x000BEA
; group: callback_state_machine
; provenance: AMER slot-1 state callback reached from the 0x0CA1 countdown and the method-table slot-1 path
; raw stop: 0x000D5B (0x171 bytes)

export_check/_tmp_dat/amer.xdb:     file format binary


Disassembly of section .data:

00000bea <.data+0xbea>:
     bea:	8b 54 40             	mov    0x40(%si),%dx
     bed:	81 fa 80 00          	cmp    $0x80,%dx
     bf1:	0f 87 b7 00          	ja     0xcac
     bf5:	8b 44 38             	mov    0x38(%si),%ax
     bf8:	3d 40 00             	cmp    $0x40,%ax
     bfb:	0f 8f ad 00          	jg     0xcac
     bff:	3d c0 ff             	cmp    $0xffc0,%ax
     c02:	0f 8c a6 00          	jl     0xcac
     c06:	8b 5c 3c             	mov    0x3c(%si),%bx
     c09:	83 fb 40             	cmp    $0x40,%bx
     c0c:	0f 8f 9c 00          	jg     0xcac
     c10:	83 fb c0             	cmp    $0xffc0,%bx
     c13:	0f 8c 95 00          	jl     0xcac
     c17:	c7 06 82 22 01 00    	movw   $0x1,0x2282
     c1d:	2e f7 06 2f 0b 03 00 	testw  $0x3,%cs:0xb2f
     c24:	75 37                	jne    0xc5d
     c26:	2e c7 06 2f 0b 01 00 	movw   $0x1,%cs:0xb2f
     c2d:	c7 04 a8 25          	movw   $0x25a8,(%si)
     c31:	c7 44 54 00 00       	movw   $0x0,0x54(%si)
     c36:	c7 44 42 00 00       	movw   $0x0,0x42(%si)
     c3b:	c7 44 46 00 00       	movw   $0x0,0x46(%si)
     c40:	c7 44 4a 20 00       	movw   $0x20,0x4a(%si)
     c45:	66 83 06 94 25 1e    	addl   $0x1e,0x2594
     c4b:	66 83 06 f2 25 23    	addl   $0x23,0x25f2
     c51:	c7 44 0e 37 0b       	movw   $0xb37,0xe(%si)
     c56:	c7 06 1e 00 05 00    	movw   $0x5,0x1e
     c5c:	c3                   	ret
     c5d:	a1 f8 22             	mov    0x22f8,%ax
     c60:	8b 5c 50             	mov    0x50(%si),%bx
     c63:	25 fc 0f             	and    $0xffc,%ax
     c66:	81 e3 fc 0f          	and    $0xffc,%bx
     c6a:	2b c3                	sub    %bx,%ax
     c6c:	c1 f8 04             	sar    $0x4,%ax
     c6f:	89 44 56             	mov    %ax,0x56(%si)
     c72:	8b 44 52             	mov    0x52(%si),%ax
     c75:	c1 f8 04             	sar    $0x4,%ax
     c78:	89 44 10             	mov    %ax,0x10(%si)
     c7b:	c7 44 0e 81 0c       	movw   $0xc81,0xe(%si)
     c80:	c3                   	ret
     c81:	8b 44 56             	mov    0x56(%si),%ax
     c84:	01 44 50             	add    %ax,0x50(%si)
     c87:	8b 44 10             	mov    0x10(%si),%ax
     c8a:	29 44 52             	sub    %ax,0x52(%si)
     c8d:	ff 44 54             	incw   0x54(%si)
     c90:	83 7c 54 0f          	cmpw   $0xf,0x54(%si)
     c94:	7e 0a                	jle    0xca0
     c96:	c7 44 0e a1 0c       	movw   $0xca1,0xe(%si)
     c9b:	c7 44 54 40 00       	movw   $0x40,0x54(%si)
     ca0:	c3                   	ret
     ca1:	ff 4c 54             	decw   0x54(%si)
     ca4:	75 05                	jne    0xcab
     ca6:	c7 44 0e ea 0b       	movw   $0xbea,0xe(%si)
     cab:	c3                   	ret
     cac:	c7 44 54 0c 00       	movw   $0xc,0x54(%si)
     cb1:	ff 4c 10             	decw   0x10(%si)
     cb4:	79 79                	jns    0xd2f
     cb6:	90                   	nop
     cb7:	90                   	nop
     cb8:	8b 6c 5c             	mov    0x5c(%si),%bp
     cbb:	8b 54 4a             	mov    0x4a(%si),%dx
     cbe:	03 16 f4 22          	add    0x22f4,%dx
     cc2:	8b 44 50             	mov    0x50(%si),%ax
     cc5:	05 00 08             	add    $0x800,%ax
     cc8:	81 fa 18 fc          	cmp    $0xfc18,%dx
     ccc:	7c 22                	jl     0xcf0
     cce:	05 00 08             	add    $0x800,%ax
     cd1:	81 fa e8 03          	cmp    $0x3e8,%dx
     cd5:	7f 19                	jg     0xcf0
     cd7:	8b 54 42             	mov    0x42(%si),%dx
     cda:	03 16 ec 22          	add    0x22ec,%dx
     cde:	05 00 04             	add    $0x400,%ax
     ce1:	81 fa 18 fc          	cmp    $0xfc18,%dx
     ce5:	7c 09                	jl     0xcf0
     ce7:	05 00 08             	add    $0x800,%ax
     cea:	81 fa e8 03          	cmp    $0x3e8,%dx
     cee:	7c 15                	jl     0xd05
     cf0:	25 fc 0f             	and    $0xffc,%ax
     cf3:	b9 20 00             	mov    $0x20,%cx
     cf6:	c7 44 10 10 00       	movw   $0x10,0x10(%si)
     cfb:	2d 00 08             	sub    $0x800,%ax
     cfe:	c1 f8 02             	sar    $0x2,%ax
     d01:	f7 d8                	neg    %ax
     d03:	eb 1e                	jmp    0xd23
     d05:	c1 cd 03             	ror    $0x3,%bp
     d08:	83 dd 00             	sbb    $0x0,%bp
     d0b:	8b c5                	mov    %bp,%ax
     d0d:	25 ff 07             	and    $0x7ff,%ax
     d10:	2d ff 03             	sub    $0x3ff,%ax
     d13:	8b c8                	mov    %ax,%cx
     d15:	0b c9                	or     %cx,%cx
     d17:	79 02                	jns    0xd1b
     d19:	f7 d9                	neg    %cx
     d1b:	d1 e9                	shr    $1,%cx
     d1d:	83 c1 10             	add    $0x10,%cx
     d20:	89 4c 10             	mov    %cx,0x10(%si)
     d23:	2b 44 5a             	sub    0x5a(%si),%ax
     d26:	99                   	cwtd
     d27:	f7 f9                	idiv   %cx
     d29:	89 6c 5c             	mov    %bp,0x5c(%si)
     d2c:	89 44 56             	mov    %ax,0x56(%si)
     d2f:	8b 44 56             	mov    0x56(%si),%ax
     d32:	03 44 5a             	add    0x5a(%si),%ax
     d35:	8b d0                	mov    %ax,%dx
     d37:	89 54 5a             	mov    %dx,0x5a(%si)
     d3a:	c1 f8 05             	sar    $0x5,%ax
     d3d:	01 44 50             	add    %ax,0x50(%si)
     d40:	8b 5c 58             	mov    0x58(%si),%bx
     d43:	81 c3 80 00          	add    $0x80,%bx
     d47:	81 e3 fc 0f          	and    $0xffc,%bx
     d4b:	89 5c 58             	mov    %bx,0x58(%si)
     d4e:	8b 87 36 00          	mov    0x36(%bx),%ax
     d52:	c1 f8 05             	sar    $0x5,%ax
     d55:	03 c2                	add    %dx,%ax
     d57:	89 44 52             	mov    %ax,0x52(%si)
     d5a:	c3                   	ret
