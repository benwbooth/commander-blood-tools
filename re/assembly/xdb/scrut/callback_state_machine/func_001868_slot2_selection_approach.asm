; Commander Blood recovered routine assembly
; module: xdb_scrut
; artifact: output/_tmp_dat/scrut.xdb
; artifact_sha256: 8522e77aad6639cf9bb06f148048fed3c6bf1cfbe6459c9a0be7b987c3c3ac77
; overlay_offset: 0x001868
; byte_count: 113
; routine_bytes_sha256: 048eaf40cdc6f19b4c39d216d27cc4f028905ac6bf25716fc4808fa1fc6249dc
; routine_entry: 0x001868
; group: callback_state_machine
; provenance: callback published by damping callback 0x1858
; direct_callees: 0x001810, 0x0018D9, 0x001952, 0x0019CF
; raw stop: 0x0018D9


output/_tmp_dat/scrut.xdb:     file format binary


Disassembly of section .data:

00001868 <.data+0x1868>:
    1868:	2e f7 06 70 0b 03 00 	testw  $0x3,%cs:0xb70
    186f:	0f 84 5c 01          	je     0x19cf
    1873:	d1 7c 4e             	sarw   $1,0x4e(%si)
    1876:	66 a1 d2 22          	mov    0x22d2,%eax
    187a:	66 c1 f8 03          	sar    $0x3,%eax
    187e:	66 8b 0e da 22       	mov    0x22da,%ecx
    1883:	66 c1 f9 03          	sar    $0x3,%ecx
    1887:	8b 55 3a             	mov    0x3a(%di),%dx
    188a:	2b 06 ec 22          	sub    0x22ec,%ax
    188e:	8b 1e f0 22          	mov    0x22f0,%bx
    1892:	2b 0e f4 22          	sub    0x22f4,%cx
    1896:	03 c2                	add    %dx,%ax
    1898:	03 ca                	add    %dx,%cx
    189a:	2b 44 42             	sub    0x42(%si),%ax
    189d:	03 5c 46             	add    0x46(%si),%bx
    18a0:	2b 4c 4a             	sub    0x4a(%si),%cx
    18a3:	c1 f8 04             	sar    $0x4,%ax
    18a6:	01 44 42             	add    %ax,0x42(%si)
    18a9:	f7 db                	neg    %bx
    18ab:	c1 fb 05             	sar    $0x5,%bx
    18ae:	11 5c 46             	adc    %bx,0x46(%si)
    18b1:	c1 f9 04             	sar    $0x4,%cx
    18b4:	01 4c 4a             	add    %cx,0x4a(%si)
    18b7:	81 7c 40 2c 01       	cmpw   $0x12c,0x40(%si)
    18bc:	7c 15                	jl     0x18d3
    18be:	8b 44 38             	mov    0x38(%si),%ax
    18c1:	05 b8 0b             	add    $0xbb8,%ax
    18c4:	3d 70 17             	cmp    $0x1770,%ax
    18c7:	77 0a                	ja     0x18d3
    18c9:	b1 13                	mov    $0x13,%cl
    18cb:	e8 0b 00             	call   0x18d9
    18ce:	0f 83 80 00          	jae    0x1952
    18d2:	c3                   	ret
    18d3:	c7 44 0e 10 18       	movw   $0x1810,0xe(%si)
    18d8:	c3                   	ret
