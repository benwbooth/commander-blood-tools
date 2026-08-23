; Commander Blood recovered routine assembly
; module: xdb_scrut
; artifact: output/_tmp_dat/scrut.xdb
; artifact_sha256: 8522e77aad6639cf9bb06f148048fed3c6bf1cfbe6459c9a0be7b987c3c3ac77
; overlay_offset: 0x000B78
; byte_count: 160
; routine_bytes_sha256: 866abe2a41ca054d817fe1087b3d101a04de02c0c324cb695badd2499d5bf815
; routine_entry: 0x000B78
; group: callback_state_machine
; provenance: state callback selected by method slot 1
; direct_callees: none
; raw stop: 0x000C18


output/_tmp_dat/scrut.xdb:     file format binary


Disassembly of section .data:

00000b78 <.data+0xb78>:
     b78:	2e f7 06 8e 16 01 00 	testw  $0x1,%cs:0x168e
     b7f:	75 56                	jne    0xbd7
     b81:	c7 44 4e 00 00       	movw   $0x0,0x4e(%si)
     b86:	c7 44 50 00 08       	movw   $0x800,0x50(%si)
     b8b:	83 44 52 35          	addw   $0x35,0x52(%si)
     b8f:	2e f7 06 70 0b 02 00 	testw  $0x2,%cs:0xb70
     b96:	74 3e                	je     0xbd6
     b98:	2e a1 99 00          	mov    %cs:0x99,%ax
     b9c:	05 08 00             	add    $0x8,%ax
     b9f:	3d 80 00             	cmp    $0x80,%ax
     ba2:	72 03                	jb     0xba7
     ba4:	b8 7f 00             	mov    $0x7f,%ax
     ba7:	2e a3 99 00          	mov    %ax,%cs:0x99
     bab:	2e a1 74 0b          	mov    %cs:0xb74,%ax
     baf:	89 04                	mov    %ax,(%si)
     bb1:	c7 44 0e 18 0c       	movw   $0xc18,0xe(%si)
     bb6:	2e c7 06 70 0b 00 00 	movw   $0x0,%cs:0xb70
     bbd:	66 83 2e 94 25 1e    	subl   $0x1e,0x2594
     bc3:	66 83 2e f2 25 23    	subl   $0x23,0x25f2
     bc9:	2e c7 06 8e 16 00 00 	movw   $0x0,%cs:0x168e
     bd0:	c7 06 1e 00 04 00    	movw   $0x4,0x1e
     bd6:	c3                   	ret
     bd7:	2e c7 06 70 0b 00 00 	movw   $0x0,%cs:0xb70
     bde:	66 83 2e 94 25 1e    	subl   $0x1e,0x2594
     be4:	66 83 2e f2 25 23    	subl   $0x23,0x25f2
     bea:	c7 04 a8 22          	movw   $0x22a8,(%si)
     bee:	66 0f bf 06 ec 22    	movswl 0x22ec,%eax
     bf4:	66 0f bf 1e f0 22    	movswl 0x22f0,%ebx
     bfa:	66 0f bf 0e f4 22    	movswl 0x22f4,%ecx
     c00:	66 f7 d8             	neg    %eax
     c03:	66 f7 db             	neg    %ebx
     c06:	66 f7 d9             	neg    %ecx
     c09:	66 89 44 42          	mov    %eax,0x42(%si)
     c0d:	66 89 5c 46          	mov    %ebx,0x46(%si)
     c11:	66 89 4c 4a          	mov    %ecx,0x4a(%si)
     c15:	e9 8b 00             	jmp    0xca3
