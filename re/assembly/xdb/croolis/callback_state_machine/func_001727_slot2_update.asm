; Commander Blood recovered routine assembly
; module: xdb_croolis
; artifact: output/_tmp_dat/croolis.xdb
; artifact_sha256: 13eba7c4e4f38662c44c849bd07be293c7583d2b2dc9ae273d7fe94746048c31
; overlay_offset: 0x001727
; byte_count: 103
; routine_bytes_sha256: 170aadbb7b3f3e6c8b9b3375c3041a9ac959bf139ad461e449e75b34e41233db
; routine_entry: 0x001727
; group: callback_state_machine
; provenance: callback published by method-table slots 2 and 4
; direct_callees: 0x00178E, 0x001815, 0x001960
; raw stop: 0x00178E


output/_tmp_dat/croolis.xdb:     file format binary


Disassembly of section .data:

00001727 <.data+0x1727>:
    1727:	2e f7 06 70 0b 03 00 	testw  $0x3,%cs:0xb70
    172e:	0f 85 e3 00          	jne    0x1815
    1732:	ff 4d 38             	decw   0x38(%di)
    1735:	79 57                	jns    0x178e
    1737:	8b 44 40             	mov    0x40(%si),%ax
    173a:	8b 5c 38             	mov    0x38(%si),%bx
    173d:	05 f4 01             	add    $0x1f4,%ax
    1740:	3d b8 0b             	cmp    $0xbb8,%ax
    1743:	0f 87 19 02          	ja     0x1960
    1747:	81 c3 e8 03          	add    $0x3e8,%bx
    174b:	81 fb d0 07          	cmp    $0x7d0,%bx
    174f:	0f 87 0d 02          	ja     0x1960
    1753:	8b 6d 42             	mov    0x42(%di),%bp
    1756:	c1 cd 03             	ror    $0x3,%bp
    1759:	83 dd 00             	sbb    $0x0,%bp
    175c:	8b c5                	mov    %bp,%ax
    175e:	25 ff 03             	and    $0x3ff,%ax
    1761:	2d ff 01             	sub    $0x1ff,%ax
    1764:	8b d0                	mov    %ax,%dx
    1766:	0b d2                	or     %dx,%dx
    1768:	79 02                	jns    0x176c
    176a:	f7 da                	neg    %dx
    176c:	8b ca                	mov    %dx,%cx
    176e:	d1 e9                	shr    $1,%cx
    1770:	83 c1 10             	add    $0x10,%cx
    1773:	f7 da                	neg    %dx
    1775:	81 c2 00 03          	add    $0x300,%dx
    1779:	c1 ea 03             	shr    $0x3,%dx
    177c:	89 4d 38             	mov    %cx,0x38(%di)
    177f:	89 54 58             	mov    %dx,0x58(%si)
    1782:	2b 44 52             	sub    0x52(%si),%ax
    1785:	99                   	cwtd
    1786:	f7 f9                	idiv   %cx
    1788:	89 6d 42             	mov    %bp,0x42(%di)
    178b:	89 45 3a             	mov    %ax,0x3a(%di)
