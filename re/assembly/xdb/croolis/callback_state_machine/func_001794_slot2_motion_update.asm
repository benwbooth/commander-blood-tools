; Commander Blood recovered routine assembly
; module: xdb_croolis
; artifact: output/_tmp_dat/croolis.xdb
; artifact_sha256: 13eba7c4e4f38662c44c849bd07be293c7583d2b2dc9ae273d7fe94746048c31
; overlay_offset: 0x001794
; byte_count: 80
; routine_bytes_sha256: 3f9771607db8b021979ce6d5a245793abd7ae12b76304810cebf22bf86dad7bf
; routine_entry: 0x001794
; group: callback_state_machine
; provenance: shared motion tail reached by callbacks 0x1727, 0x17F2, and 0x1960
; direct_callees: none
; raw stop: 0x0017E4


output/_tmp_dat/croolis.xdb:     file format binary


Disassembly of section .data:

00001794 <.data+0x1794>:
    1794:	8b 44 3c             	mov    0x3c(%si),%ax
    1797:	03 45 3c             	add    0x3c(%di),%ax
    179a:	2b 44 4e             	sub    0x4e(%si),%ax
    179d:	c1 f8 03             	sar    $0x3,%ax
    17a0:	03 44 4e             	add    0x4e(%si),%ax
    17a3:	3d 00 03             	cmp    $0x300,%ax
    17a6:	7c 03                	jl     0x17ab
    17a8:	b8 00 03             	mov    $0x300,%ax
    17ab:	3d 00 fd             	cmp    $0xfd00,%ax
    17ae:	7f 03                	jg     0x17b3
    17b0:	b8 00 fd             	mov    $0xfd00,%ax
    17b3:	89 44 4e             	mov    %ax,0x4e(%si)
    17b6:	8b 44 58             	mov    0x58(%si),%ax
    17b9:	2b 44 54             	sub    0x54(%si),%ax
    17bc:	c1 f8 03             	sar    $0x3,%ax
    17bf:	01 44 54             	add    %ax,0x54(%si)
    17c2:	8b 55 3a             	mov    0x3a(%di),%dx
    17c5:	03 54 52             	add    0x52(%si),%dx
    17c8:	89 54 52             	mov    %dx,0x52(%si)
    17cb:	8b c2                	mov    %dx,%ax
    17cd:	d1 fa                	sar    $1,%dx
    17cf:	89 94 0a 01          	mov    %dx,0x10a(%si)
    17d3:	f7 da                	neg    %dx
    17d5:	89 94 ae 00          	mov    %dx,0xae(%si)
    17d9:	89 94 68 01          	mov    %dx,0x168(%si)
    17dd:	c1 f8 04             	sar    $0x4,%ax
    17e0:	11 44 50             	adc    %ax,0x50(%si)
    17e3:	c3                   	ret
