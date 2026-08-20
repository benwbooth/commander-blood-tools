; Commander Blood recovered routine assembly
; module: xdb_scrut
; artifact: output/_tmp_dat/scrut.xdb
; artifact_sha256: 8522e77aad6639cf9bb06f148048fed3c6bf1cfbe6459c9a0be7b987c3c3ac77
; overlay_offset: 0x00171B
; byte_count: 102
; routine_bytes_sha256: a98e94d4a10bd19f4190dc38329e16f1eed5b40ee80e45f360e9921300ff01f0
; routine_entry: 0x00171B
; group: callback_state_machine
; provenance: callback published by method-table slots 2 and 4
; direct_callees: 0x001781, 0x001802, 0x001A11
; raw stop: 0x001781


output/_tmp_dat/scrut.xdb:     file format binary


Disassembly of section .data:

0000171b <.data+0x171b>:
    171b:	2e f7 06 70 0b 03 00 	testw  $0x3,%cs:0xb70
    1722:	0f 85 dc 00          	jne    0x1802
    1726:	ff 4d 38             	decw   0x38(%di)
    1729:	79 56                	jns    0x1781
    172b:	8b 44 40             	mov    0x40(%si),%ax
    172e:	8b 5c 38             	mov    0x38(%si),%bx
    1731:	05 f4 01             	add    $0x1f4,%ax
    1734:	3d b8 0b             	cmp    $0xbb8,%ax
    1737:	0f 87 d6 02          	ja     0x1a11
    173b:	81 c3 e8 03          	add    $0x3e8,%bx
    173f:	81 fb d0 07          	cmp    $0x7d0,%bx
    1743:	0f 87 ca 02          	ja     0x1a11
    1747:	8b 6d 42             	mov    0x42(%di),%bp
    174a:	c1 cd 03             	ror    $0x3,%bp
    174d:	83 dd 00             	sbb    $0x0,%bp
    1750:	8b c5                	mov    %bp,%ax
    1752:	25 ff 07             	and    $0x7ff,%ax
    1755:	2d ff 03             	sub    $0x3ff,%ax
    1758:	8b d0                	mov    %ax,%dx
    175a:	0b c0                	or     %ax,%ax
    175c:	78 02                	js     0x1760
    175e:	f7 da                	neg    %dx
    1760:	81 c2 00 04          	add    $0x400,%dx
    1764:	8b ca                	mov    %dx,%cx
    1766:	c1 e9 03             	shr    $0x3,%cx
    1769:	83 c1 20             	add    $0x20,%cx
    176c:	c1 ea 04             	shr    $0x4,%dx
    176f:	89 4d 38             	mov    %cx,0x38(%di)
    1772:	89 54 58             	mov    %dx,0x58(%si)
    1775:	2b 44 52             	sub    0x52(%si),%ax
    1778:	99                   	cwtd
    1779:	f7 f9                	idiv   %cx
    177b:	89 6d 42             	mov    %bp,0x42(%di)
    177e:	89 44 5a             	mov    %ax,0x5a(%si)
