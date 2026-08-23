; Commander Blood recovered routine assembly
; module: xdb_scrut
; artifact: output/_tmp_dat/scrut.xdb
; artifact_sha256: 8522e77aad6639cf9bb06f148048fed3c6bf1cfbe6459c9a0be7b987c3c3ac77
; overlay_offset: 0x000C32
; byte_count: 113
; routine_bytes_sha256: 9a12f55ba875f386f42d1619af6ee241afc7e0957b4436506062d58d27572071
; routine_entry: 0x000C32
; group: callback_state_machine
; provenance: slot-1 bounds and selection state callback
; direct_callees: none
; raw stop: 0x000CA3


output/_tmp_dat/scrut.xdb:     file format binary


Disassembly of section .data:

00000c32 <.data+0xc32>:
     c32:	8b 54 40             	mov    0x40(%si),%dx
     c35:	81 fa 80 00          	cmp    $0x80,%dx
     c39:	0f 87 b5 00          	ja     0xcf2
     c3d:	8b 44 38             	mov    0x38(%si),%ax
     c40:	3d 40 00             	cmp    $0x40,%ax
     c43:	0f 8f ab 00          	jg     0xcf2
     c47:	3d c0 ff             	cmp    $0xffc0,%ax
     c4a:	0f 8c a4 00          	jl     0xcf2
     c4e:	8b 5c 3c             	mov    0x3c(%si),%bx
     c51:	83 fb 40             	cmp    $0x40,%bx
     c54:	0f 8f 9a 00          	jg     0xcf2
     c58:	83 fb c0             	cmp    $0xffc0,%bx
     c5b:	0f 8c 93 00          	jl     0xcf2
     c5f:	89 3e 82 22          	mov    %di,0x2282
     c63:	2e f7 06 70 0b 03 00 	testw  $0x3,%cs:0xb70
     c6a:	75 37                	jne    0xca3
     c6c:	2e c7 06 70 0b 01 00 	movw   $0x1,%cs:0xb70
     c73:	c7 04 a8 25          	movw   $0x25a8,(%si)
     c77:	c7 44 54 00 00       	movw   $0x0,0x54(%si)
     c7c:	c7 44 42 00 00       	movw   $0x0,0x42(%si)
     c81:	c7 44 46 00 00       	movw   $0x0,0x46(%si)
     c86:	c7 44 4a 20 00       	movw   $0x20,0x4a(%si)
     c8b:	66 83 06 94 25 1e    	addl   $0x1e,0x2594
     c91:	66 83 06 f2 25 23    	addl   $0x23,0x25f2
     c97:	c7 44 0e 78 0b       	movw   $0xb78,0xe(%si)
     c9c:	c7 06 1e 00 05 00    	movw   $0x5,0x1e
     ca2:	c3                   	ret
