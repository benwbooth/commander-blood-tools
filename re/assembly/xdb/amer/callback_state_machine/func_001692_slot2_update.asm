; Commander Blood recovered routine assembly
; module: xdb_amer
; artifact: output/_tmp_dat/amer.xdb
; artifact_sha256: 6fddeb5cc7c62fe5e638900746decc299f1637ea11612c0c51aced367dd12b31
; overlay_offset: 0x001692
; byte_count: 139
; routine_bytes_sha256: 4c3ed275fcb6c9fc02a012635361ab7483f2b715637e5bd7ca0d4e6b1a18d280
; routine_entry: 0x001692
; group: callback_state_machine
; provenance: callback published by method-table slot 2
; direct_callees: 0x00171D, 0x00193E, 0x001A2B
; raw stop: 0x00171D


output/_tmp_dat/amer.xdb:     file format binary


Disassembly of section .data:

00001692 <.data+0x1692>:
    1692:	2e f7 06 99 00 ff ff 	testw  $0xffff,%cs:0x99
    1699:	78 0b                	js     0x16a6
    169b:	2e f7 06 2f 0b 01 00 	testw  $0x1,%cs:0xb2f
    16a2:	0f 85 98 02          	jne    0x193e
    16a6:	ff 4d 38             	decw   0x38(%di)
    16a9:	79 72                	jns    0x171d
    16ab:	8b 5c 38             	mov    0x38(%si),%bx
    16ae:	8b 44 40             	mov    0x40(%si),%ax
    16b1:	3d dc 05             	cmp    $0x5dc,%ax
    16b4:	0f 8f 73 03          	jg     0x1a2b
    16b8:	3d 18 fc             	cmp    $0xfc18,%ax
    16bb:	0f 8c 6c 03          	jl     0x1a2b
    16bf:	81 fb dc 05          	cmp    $0x5dc,%bx
    16c3:	0f 8f 64 03          	jg     0x1a2b
    16c7:	81 fb 24 fa          	cmp    $0xfa24,%bx
    16cb:	0f 8c 5c 03          	jl     0x1a2b
    16cf:	8b 6d 40             	mov    0x40(%di),%bp
    16d2:	c1 cd 03             	ror    $0x3,%bp
    16d5:	83 dd 00             	sbb    $0x0,%bp
    16d8:	8b c5                	mov    %bp,%ax
    16da:	25 ff 07             	and    $0x7ff,%ax
    16dd:	2d ff 03             	sub    $0x3ff,%ax
    16e0:	8b c8                	mov    %ax,%cx
    16e2:	0b c9                	or     %cx,%cx
    16e4:	79 02                	jns    0x16e8
    16e6:	f7 d9                	neg    %cx
    16e8:	c1 e9 02             	shr    $0x2,%cx
    16eb:	83 c1 10             	add    $0x10,%cx
    16ee:	89 4d 38             	mov    %cx,0x38(%di)
    16f1:	2b 44 52             	sub    0x52(%si),%ax
    16f4:	99                   	cwtd
    16f5:	f7 f9                	idiv   %cx
    16f7:	89 6d 40             	mov    %bp,0x40(%di)
    16fa:	89 45 3a             	mov    %ax,0x3a(%di)
    16fd:	c7 44 58 14 00       	movw   $0x14,0x58(%si)
    1702:	8b 44 3c             	mov    0x3c(%si),%ax
    1705:	03 44 4e             	add    0x4e(%si),%ax
    1708:	d1 f8                	sar    $1,%ax
    170a:	3d 00 03             	cmp    $0x300,%ax
    170d:	7c 03                	jl     0x1712
    170f:	b8 00 03             	mov    $0x300,%ax
    1712:	3d 00 fd             	cmp    $0xfd00,%ax
    1715:	7f 03                	jg     0x171a
    1717:	b8 00 fd             	mov    $0xfd00,%ax
    171a:	89 44 4e             	mov    %ax,0x4e(%si)
