; Commander Blood recovered routine assembly
; module: xdb_amer
; artifact: output/_tmp_dat/amer.xdb
; artifact_sha256: 6fddeb5cc7c62fe5e638900746decc299f1637ea11612c0c51aced367dd12b31
; overlay_offset: 0x000BEA
; byte_count: 115
; routine_bytes_sha256: 88a0152008a8470b6d96b9870b07602907490baa31488c22e7de1ba833e2cb57
; routine_entry: 0x000BEA
; group: callback_state_machine
; provenance: slot-1 state callback head; tail-transfers to callbacks 0x0C5D or continuation 0x0CAC
; raw stop: 0x000C5D


output/_tmp_dat/amer.xdb:     file format binary


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
