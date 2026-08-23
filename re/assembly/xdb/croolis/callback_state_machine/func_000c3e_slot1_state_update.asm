; Commander Blood recovered routine assembly
; module: xdb_croolis
; artifact: output/_tmp_dat/croolis.xdb
; artifact_sha256: 13eba7c4e4f38662c44c849bd07be293c7583d2b2dc9ae273d7fe94746048c31
; overlay_offset: 0x000C3E
; byte_count: 119
; routine_bytes_sha256: 7c43e7c97392048a7cd8a9b40ebee22d51ce74cceb5a6630d34d5c52075e8de5
; routine_entry: 0x000C3E
; group: callback_state_machine
; provenance: slot-1 bounds and selection state callback
; direct_callees: none
; raw stop: 0x000CB5


output/_tmp_dat/croolis.xdb:     file format binary


Disassembly of section .data:

00000c3e <.data+0xc3e>:
     c3e:	8b 54 40             	mov    0x40(%si),%dx
     c41:	81 fa 80 00          	cmp    $0x80,%dx
     c45:	0f 87 bb 00          	ja     0xd04
     c49:	8b 44 38             	mov    0x38(%si),%ax
     c4c:	3d 40 00             	cmp    $0x40,%ax
     c4f:	0f 8f b1 00          	jg     0xd04
     c53:	3d c0 ff             	cmp    $0xffc0,%ax
     c56:	0f 8c aa 00          	jl     0xd04
     c5a:	8b 5c 3c             	mov    0x3c(%si),%bx
     c5d:	83 fb 40             	cmp    $0x40,%bx
     c60:	0f 8f a0 00          	jg     0xd04
     c64:	83 fb c0             	cmp    $0xffc0,%bx
     c67:	0f 8c 99 00          	jl     0xd04
     c6b:	89 3e 82 22          	mov    %di,0x2282
     c6f:	2e f7 06 70 0b 03 00 	testw  $0x3,%cs:0xb70
     c76:	75 3d                	jne    0xcb5
     c78:	2e c7 06 70 0b 01 00 	movw   $0x1,%cs:0xb70
     c7f:	c7 04 a8 25          	movw   $0x25a8,(%si)
     c83:	c7 44 54 00 00       	movw   $0x0,0x54(%si)
     c88:	c7 44 42 00 00       	movw   $0x0,0x42(%si)
     c8d:	c7 44 46 00 00       	movw   $0x0,0x46(%si)
     c92:	c7 44 4a 20 00       	movw   $0x20,0x4a(%si)
     c97:	66 83 06 36 25 19    	addl   $0x19,0x2536
     c9d:	66 83 06 94 25 1e    	addl   $0x1e,0x2594
     ca3:	66 83 06 f2 25 23    	addl   $0x23,0x25f2
     ca9:	c7 44 0e 78 0b       	movw   $0xb78,0xe(%si)
     cae:	c7 06 1e 00 05 00    	movw   $0x5,0x1e
     cb4:	c3                   	ret
