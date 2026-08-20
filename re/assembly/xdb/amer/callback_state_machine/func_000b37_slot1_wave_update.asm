; Commander Blood recovered routine assembly
; module: xdb_amer
; artifact: output/_tmp_dat/amer.xdb
; artifact_sha256: 6fddeb5cc7c62fe5e638900746decc299f1637ea11612c0c51aced367dd12b31
; overlay_offset: 0x000B37
; byte_count: 153
; routine_bytes_sha256: 99050300f8178ba956418cd222a091f5e9f5e2857f374d4a0ea0927aea9c42ff
; routine_entry: 0x000B37
; group: callback_state_machine
; provenance: internal callback selected by the AMER slot-1 wave state; alternate path tail-jumps to callback 0x0C5D
; raw stop: 0x000BD0


output/_tmp_dat/amer.xdb:     file format binary


Disassembly of section .data:

00000b37 <.data+0xb37>:
     b37:	2e f7 06 48 16 01 00 	testw  $0x1,%cs:0x1648
     b3e:	75 4f                	jne    0xb8f
     b40:	c7 44 4e 00 00       	movw   $0x0,0x4e(%si)
     b45:	c7 44 50 00 08       	movw   $0x800,0x50(%si)
     b4a:	83 44 52 35          	addw   $0x35,0x52(%si)
     b4e:	2e f7 06 2f 0b 02 00 	testw  $0x2,%cs:0xb2f
     b55:	74 37                	je     0xb8e
     b57:	2e a1 99 00          	mov    %cs:0x99,%ax
     b5b:	05 08 00             	add    $0x8,%ax
     b5e:	3d 80 00             	cmp    $0x80,%ax
     b61:	72 03                	jb     0xb66
     b63:	b8 7f 00             	mov    $0x7f,%ax
     b66:	2e a3 99 00          	mov    %ax,%cs:0x99
     b6a:	2e a1 33 0b          	mov    %cs:0xb33,%ax
     b6e:	89 04                	mov    %ax,(%si)
     b70:	c7 44 0e d0 0b       	movw   $0xbd0,0xe(%si)
     b75:	2e c7 06 2f 0b 00 00 	movw   $0x0,%cs:0xb2f
     b7c:	66 83 2e 94 25 1e    	subl   $0x1e,0x2594
     b82:	66 83 2e f2 25 23    	subl   $0x23,0x25f2
     b88:	c7 06 1e 00 04 00    	movw   $0x4,0x1e
     b8e:	c3                   	ret
     b8f:	2e c7 06 2f 0b 00 00 	movw   $0x0,%cs:0xb2f
     b96:	66 83 2e 94 25 1e    	subl   $0x1e,0x2594
     b9c:	66 83 2e f2 25 23    	subl   $0x23,0x25f2
     ba2:	c7 04 a8 22          	movw   $0x22a8,(%si)
     ba6:	66 0f bf 06 ec 22    	movswl 0x22ec,%eax
     bac:	66 0f bf 1e f0 22    	movswl 0x22f0,%ebx
     bb2:	66 0f bf 0e f4 22    	movswl 0x22f4,%ecx
     bb8:	66 f7 d8             	neg    %eax
     bbb:	66 f7 db             	neg    %ebx
     bbe:	66 f7 d9             	neg    %ecx
     bc1:	66 89 44 42          	mov    %eax,0x42(%si)
     bc5:	66 89 5c 46          	mov    %ebx,0x46(%si)
     bc9:	66 89 4c 4a          	mov    %ecx,0x4a(%si)
     bcd:	e9 8d 00             	jmp    0xc5d
