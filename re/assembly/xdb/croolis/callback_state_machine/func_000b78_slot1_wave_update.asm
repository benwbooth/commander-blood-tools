; Commander Blood recovered routine assembly
; module: xdb_croolis
; artifact: output/_tmp_dat/croolis.xdb
; artifact_sha256: 13eba7c4e4f38662c44c849bd07be293c7583d2b2dc9ae273d7fe94746048c31
; overlay_offset: 0x000B78
; byte_count: 172
; routine_bytes_sha256: b31f5bd9edba9484f09f381b769138430c73c30375fdf4a09a000a6d39be513c
; routine_entry: 0x000B78
; group: callback_state_machine
; provenance: state callback selected by method slot 1
; direct_callees: none
; raw stop: 0x000C24


output/_tmp_dat/croolis.xdb:     file format binary


Disassembly of section .data:

00000b78 <.data+0xb78>:
     b78:	2e f7 06 a0 16 01 00 	testw  $0x1,%cs:0x16a0
     b7f:	75 5c                	jne    0xbdd
     b81:	c7 44 4e 00 00       	movw   $0x0,0x4e(%si)
     b86:	c7 44 50 00 08       	movw   $0x800,0x50(%si)
     b8b:	83 44 52 35          	addw   $0x35,0x52(%si)
     b8f:	2e f7 06 70 0b 02 00 	testw  $0x2,%cs:0xb70
     b96:	74 44                	je     0xbdc
     b98:	2e a1 99 00          	mov    %cs:0x99,%ax
     b9c:	05 08 00             	add    $0x8,%ax
     b9f:	3d 80 00             	cmp    $0x80,%ax
     ba2:	72 03                	jb     0xba7
     ba4:	b8 7f 00             	mov    $0x7f,%ax
     ba7:	2e a3 99 00          	mov    %ax,%cs:0x99
     bab:	2e a1 74 0b          	mov    %cs:0xb74,%ax
     baf:	89 04                	mov    %ax,(%si)
     bb1:	c7 44 0e 24 0c       	movw   $0xc24,0xe(%si)
     bb6:	2e c7 06 70 0b 00 00 	movw   $0x0,%cs:0xb70
     bbd:	66 83 2e 36 25 19    	subl   $0x19,0x2536
     bc3:	66 83 2e 94 25 1e    	subl   $0x1e,0x2594
     bc9:	66 83 2e f2 25 23    	subl   $0x23,0x25f2
     bcf:	2e c7 06 a0 16 00 00 	movw   $0x0,%cs:0x16a0
     bd6:	c7 06 1e 00 04 00    	movw   $0x4,0x1e
     bdc:	c3                   	ret
     bdd:	2e c7 06 70 0b 00 00 	movw   $0x0,%cs:0xb70
     be4:	66 83 2e 36 25 19    	subl   $0x19,0x2536
     bea:	66 83 2e 94 25 1e    	subl   $0x1e,0x2594
     bf0:	66 83 2e f2 25 23    	subl   $0x23,0x25f2
     bf6:	c7 04 a8 22          	movw   $0x22a8,(%si)
     bfa:	66 0f bf 06 ec 22    	movswl 0x22ec,%eax
     c00:	66 0f bf 1e f0 22    	movswl 0x22f0,%ebx
     c06:	66 0f bf 0e f4 22    	movswl 0x22f4,%ecx
     c0c:	66 f7 d8             	neg    %eax
     c0f:	66 f7 db             	neg    %ebx
     c12:	66 f7 d9             	neg    %ecx
     c15:	66 89 44 42          	mov    %eax,0x42(%si)
     c19:	66 89 5c 46          	mov    %ebx,0x46(%si)
     c1d:	66 89 4c 4a          	mov    %ecx,0x4a(%si)
     c21:	e9 91 00             	jmp    0xcb5
