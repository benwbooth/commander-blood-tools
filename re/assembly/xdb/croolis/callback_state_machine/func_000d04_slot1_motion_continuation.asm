; Commander Blood recovered routine assembly
; module: xdb_croolis
; artifact: output/_tmp_dat/croolis.xdb
; artifact_sha256: 13eba7c4e4f38662c44c849bd07be293c7583d2b2dc9ae273d7fe94746048c31
; overlay_offset: 0x000D04
; byte_count: 175
; routine_bytes_sha256: eaef6abb054b3214c5fc275271aedf0c4b0e82122b8622b0fd2409188f12bf33
; routine_entry: 0x000D04
; group: callback_state_machine
; provenance: out-of-bounds slot-1 motion continuation
; direct_callees: none
; raw stop: 0x000DB3


output/_tmp_dat/croolis.xdb:     file format binary


Disassembly of section .data:

00000d04 <.data+0xd04>:
     d04:	c7 44 54 0c 00       	movw   $0xc,0x54(%si)
     d09:	ff 4c 10             	decw   0x10(%si)
     d0c:	79 79                	jns    0xd87
     d0e:	90                   	nop
     d0f:	90                   	nop
     d10:	8b 6c 5c             	mov    0x5c(%si),%bp
     d13:	8b 54 4a             	mov    0x4a(%si),%dx
     d16:	03 16 f4 22          	add    0x22f4,%dx
     d1a:	8b 44 50             	mov    0x50(%si),%ax
     d1d:	05 00 08             	add    $0x800,%ax
     d20:	81 fa 18 fc          	cmp    $0xfc18,%dx
     d24:	7c 22                	jl     0xd48
     d26:	05 00 08             	add    $0x800,%ax
     d29:	81 fa e8 03          	cmp    $0x3e8,%dx
     d2d:	7f 19                	jg     0xd48
     d2f:	8b 54 42             	mov    0x42(%si),%dx
     d32:	03 16 ec 22          	add    0x22ec,%dx
     d36:	05 00 04             	add    $0x400,%ax
     d39:	81 fa 18 fc          	cmp    $0xfc18,%dx
     d3d:	7c 09                	jl     0xd48
     d3f:	05 00 08             	add    $0x800,%ax
     d42:	81 fa e8 03          	cmp    $0x3e8,%dx
     d46:	7c 15                	jl     0xd5d
     d48:	25 fc 0f             	and    $0xffc,%ax
     d4b:	b9 20 00             	mov    $0x20,%cx
     d4e:	c7 44 10 10 00       	movw   $0x10,0x10(%si)
     d53:	2d 00 08             	sub    $0x800,%ax
     d56:	c1 f8 02             	sar    $0x2,%ax
     d59:	f7 d8                	neg    %ax
     d5b:	eb 1e                	jmp    0xd7b
     d5d:	c1 cd 03             	ror    $0x3,%bp
     d60:	83 dd 00             	sbb    $0x0,%bp
     d63:	8b c5                	mov    %bp,%ax
     d65:	25 ff 07             	and    $0x7ff,%ax
     d68:	2d ff 03             	sub    $0x3ff,%ax
     d6b:	8b c8                	mov    %ax,%cx
     d6d:	0b c9                	or     %cx,%cx
     d6f:	79 02                	jns    0xd73
     d71:	f7 d9                	neg    %cx
     d73:	d1 e9                	shr    $1,%cx
     d75:	83 c1 10             	add    $0x10,%cx
     d78:	89 4c 10             	mov    %cx,0x10(%si)
     d7b:	2b 44 5a             	sub    0x5a(%si),%ax
     d7e:	99                   	cwtd
     d7f:	f7 f9                	idiv   %cx
     d81:	89 6c 5c             	mov    %bp,0x5c(%si)
     d84:	89 44 56             	mov    %ax,0x56(%si)
     d87:	8b 44 56             	mov    0x56(%si),%ax
     d8a:	03 44 5a             	add    0x5a(%si),%ax
     d8d:	8b d0                	mov    %ax,%dx
     d8f:	89 54 5a             	mov    %dx,0x5a(%si)
     d92:	c1 f8 05             	sar    $0x5,%ax
     d95:	01 44 50             	add    %ax,0x50(%si)
     d98:	8b 5c 58             	mov    0x58(%si),%bx
     d9b:	81 c3 80 00          	add    $0x80,%bx
     d9f:	81 e3 fc 0f          	and    $0xffc,%bx
     da3:	89 5c 58             	mov    %bx,0x58(%si)
     da6:	8b 87 36 00          	mov    0x36(%bx),%ax
     daa:	c1 f8 05             	sar    $0x5,%ax
     dad:	03 c2                	add    %dx,%ax
     daf:	89 44 52             	mov    %ax,0x52(%si)
     db2:	c3                   	ret
