; Commander Blood recovered routine assembly
; module: xdb_scrut
; artifact: output/_tmp_dat/scrut.xdb
; artifact_sha256: 8522e77aad6639cf9bb06f148048fed3c6bf1cfbe6459c9a0be7b987c3c3ac77
; overlay_offset: 0x001787
; byte_count: 90
; routine_bytes_sha256: 19d6191609c7e9a140ed67c32825e248adc09169edee215942dd3286d68cce95
; routine_entry: 0x001787
; group: callback_state_machine
; provenance: shared motion tail reached by callbacks 0x171B, 0x17E6, and 0x1A11
; direct_callees: none
; raw stop: 0x0017E1


output/_tmp_dat/scrut.xdb:     file format binary


Disassembly of section .data:

00001787 <.data+0x1787>:
    1787:	8b 44 3c             	mov    0x3c(%si),%ax
    178a:	03 45 3a             	add    0x3a(%di),%ax
    178d:	d1 f8                	sar    $1,%ax
    178f:	2b 44 4e             	sub    0x4e(%si),%ax
    1792:	c1 f8 03             	sar    $0x3,%ax
    1795:	03 44 4e             	add    0x4e(%si),%ax
    1798:	3d 00 03             	cmp    $0x300,%ax
    179b:	7c 03                	jl     0x17a0
    179d:	b8 00 03             	mov    $0x300,%ax
    17a0:	3d 00 fd             	cmp    $0xfd00,%ax
    17a3:	7f 03                	jg     0x17a8
    17a5:	b8 00 fd             	mov    $0xfd00,%ax
    17a8:	89 44 4e             	mov    %ax,0x4e(%si)
    17ab:	8b 44 58             	mov    0x58(%si),%ax
    17ae:	2b 44 54             	sub    0x54(%si),%ax
    17b1:	c1 f8 03             	sar    $0x3,%ax
    17b4:	01 44 54             	add    %ax,0x54(%si)
    17b7:	8b 44 5a             	mov    0x5a(%si),%ax
    17ba:	03 44 52             	add    0x52(%si),%ax
    17bd:	89 44 52             	mov    %ax,0x52(%si)
    17c0:	8b d0                	mov    %ax,%dx
    17c2:	c1 fa 05             	sar    $0x5,%dx
    17c5:	11 54 50             	adc    %dx,0x50(%si)
    17c8:	b9 05 00             	mov    $0x5,%cx
    17cb:	f7 d8                	neg    %ax
    17cd:	8b d8                	mov    %ax,%bx
    17cf:	d1 f8                	sar    $1,%ax
    17d1:	c1 fb 02             	sar    $0x2,%bx
    17d4:	83 c6 5e             	add    $0x5e,%si
    17d7:	89 44 50             	mov    %ax,0x50(%si)
    17da:	89 5c 52             	mov    %bx,0x52(%si)
    17dd:	e2 f5                	loop   0x17d4
    17df:	c3                   	ret
    17e0:	c3                   	ret
