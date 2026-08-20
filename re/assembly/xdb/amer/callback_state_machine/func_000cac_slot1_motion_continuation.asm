; Commander Blood recovered routine assembly
; module: xdb_amer
; artifact: output/_tmp_dat/amer.xdb
; artifact_sha256: 6fddeb5cc7c62fe5e638900746decc299f1637ea11612c0c51aced367dd12b31
; overlay_offset: 0x000CAC
; byte_count: 175
; routine_bytes_sha256: eaef6abb054b3214c5fc275271aedf0c4b0e82122b8622b0fd2409188f12bf33
; routine_entry: 0x000CAC
; group: callback_state_machine
; provenance: motion continuation reached by the AMER slot-1 state callback at 0x0BEA
; raw stop: 0x000D5B


output/_tmp_dat/amer.xdb:     file format binary


Disassembly of section .data:

00000cac <.data+0xcac>:
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
