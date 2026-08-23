; Commander Blood recovered routine assembly
; module: xdb_scrut
; artifact: output/_tmp_dat/scrut.xdb
; artifact_sha256: 8522e77aad6639cf9bb06f148048fed3c6bf1cfbe6459c9a0be7b987c3c3ac77
; overlay_offset: 0x000CF2
; byte_count: 175
; routine_bytes_sha256: eaef6abb054b3214c5fc275271aedf0c4b0e82122b8622b0fd2409188f12bf33
; routine_entry: 0x000CF2
; group: callback_state_machine
; provenance: out-of-bounds slot-1 motion continuation
; direct_callees: none
; raw stop: 0x000DA1


output/_tmp_dat/scrut.xdb:     file format binary


Disassembly of section .data:

00000cf2 <.data+0xcf2>:
     cf2:	c7 44 54 0c 00       	movw   $0xc,0x54(%si)
     cf7:	ff 4c 10             	decw   0x10(%si)
     cfa:	79 79                	jns    0xd75
     cfc:	90                   	nop
     cfd:	90                   	nop
     cfe:	8b 6c 5c             	mov    0x5c(%si),%bp
     d01:	8b 54 4a             	mov    0x4a(%si),%dx
     d04:	03 16 f4 22          	add    0x22f4,%dx
     d08:	8b 44 50             	mov    0x50(%si),%ax
     d0b:	05 00 08             	add    $0x800,%ax
     d0e:	81 fa 18 fc          	cmp    $0xfc18,%dx
     d12:	7c 22                	jl     0xd36
     d14:	05 00 08             	add    $0x800,%ax
     d17:	81 fa e8 03          	cmp    $0x3e8,%dx
     d1b:	7f 19                	jg     0xd36
     d1d:	8b 54 42             	mov    0x42(%si),%dx
     d20:	03 16 ec 22          	add    0x22ec,%dx
     d24:	05 00 04             	add    $0x400,%ax
     d27:	81 fa 18 fc          	cmp    $0xfc18,%dx
     d2b:	7c 09                	jl     0xd36
     d2d:	05 00 08             	add    $0x800,%ax
     d30:	81 fa e8 03          	cmp    $0x3e8,%dx
     d34:	7c 15                	jl     0xd4b
     d36:	25 fc 0f             	and    $0xffc,%ax
     d39:	b9 20 00             	mov    $0x20,%cx
     d3c:	c7 44 10 10 00       	movw   $0x10,0x10(%si)
     d41:	2d 00 08             	sub    $0x800,%ax
     d44:	c1 f8 02             	sar    $0x2,%ax
     d47:	f7 d8                	neg    %ax
     d49:	eb 1e                	jmp    0xd69
     d4b:	c1 cd 03             	ror    $0x3,%bp
     d4e:	83 dd 00             	sbb    $0x0,%bp
     d51:	8b c5                	mov    %bp,%ax
     d53:	25 ff 07             	and    $0x7ff,%ax
     d56:	2d ff 03             	sub    $0x3ff,%ax
     d59:	8b c8                	mov    %ax,%cx
     d5b:	0b c9                	or     %cx,%cx
     d5d:	79 02                	jns    0xd61
     d5f:	f7 d9                	neg    %cx
     d61:	d1 e9                	shr    $1,%cx
     d63:	83 c1 10             	add    $0x10,%cx
     d66:	89 4c 10             	mov    %cx,0x10(%si)
     d69:	2b 44 5a             	sub    0x5a(%si),%ax
     d6c:	99                   	cwtd
     d6d:	f7 f9                	idiv   %cx
     d6f:	89 6c 5c             	mov    %bp,0x5c(%si)
     d72:	89 44 56             	mov    %ax,0x56(%si)
     d75:	8b 44 56             	mov    0x56(%si),%ax
     d78:	03 44 5a             	add    0x5a(%si),%ax
     d7b:	8b d0                	mov    %ax,%dx
     d7d:	89 54 5a             	mov    %dx,0x5a(%si)
     d80:	c1 f8 05             	sar    $0x5,%ax
     d83:	01 44 50             	add    %ax,0x50(%si)
     d86:	8b 5c 58             	mov    0x58(%si),%bx
     d89:	81 c3 80 00          	add    $0x80,%bx
     d8d:	81 e3 fc 0f          	and    $0xffc,%bx
     d91:	89 5c 58             	mov    %bx,0x58(%si)
     d94:	8b 87 36 00          	mov    0x36(%bx),%ax
     d98:	c1 f8 05             	sar    $0x5,%ax
     d9b:	03 c2                	add    %dx,%ax
     d9d:	89 44 52             	mov    %ax,0x52(%si)
     da0:	c3                   	ret
