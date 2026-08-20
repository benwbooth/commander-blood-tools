; Commander Blood recovered routine assembly
; module: xdb_scrut
; artifact: output/_tmp_dat/scrut.xdb
; artifact_sha256: 8522e77aad6639cf9bb06f148048fed3c6bf1cfbe6459c9a0be7b987c3c3ac77
; overlay_offset: 0x00145A
; byte_count: 324
; routine_bytes_sha256: 993f9c819402d1b81d3e335edc8bcc80a4763845c0aa2687cf7291ec63d1061a
; routine_entry: 0x00145A
; group: callback_state_machine
; provenance: generic callback published by slot-3 initializer
; raw stop: 0x00159E


output/_tmp_dat/scrut.xdb:     file format binary


Disassembly of section .data:

0000145a <.data+0x145a>:
    145a:	8b 6c 5a             	mov    0x5a(%si),%bp
    145d:	2e 8b 86 a9 0d       	mov    %cs:0xda9(%bp),%ax
    1462:	2e 8b 9e ab 0d       	mov    %cs:0xdab(%bp),%bx
    1467:	2e 8b 96 ad 0d       	mov    %cs:0xdad(%bp),%dx
    146c:	01 44 4e             	add    %ax,0x4e(%si)
    146f:	01 5c 50             	add    %bx,0x50(%si)
    1472:	2e f7 06 72 0b ff ff 	testw  $0xffff,%cs:0xb72
    1479:	89 54 54             	mov    %dx,0x54(%si)
    147c:	75 15                	jne    0x1493
    147e:	83 c5 08             	add    $0x8,%bp
    1481:	81 e5 fc 03          	and    $0x3fc,%bp
    1485:	89 6c 5a             	mov    %bp,0x5a(%si)
    1488:	2e f7 86 af 0d 03 00 	testw  $0x3,%cs:0xdaf(%bp)
    148f:	0f 85 c7 00          	jne    0x155a
    1493:	f7 44 5c ff ff       	testw  $0xffff,0x5c(%si)
    1498:	0f 85 a9 00          	jne    0x1545
    149c:	8b 54 40             	mov    0x40(%si),%dx
    149f:	83 fa 40             	cmp    $0x40,%dx
    14a2:	0f 87 9f 00          	ja     0x1545
    14a6:	8b 44 38             	mov    0x38(%si),%ax
    14a9:	3d 40 00             	cmp    $0x40,%ax
    14ac:	0f 8f 95 00          	jg     0x1545
    14b0:	3d c0 ff             	cmp    $0xffc0,%ax
    14b3:	0f 8c 8e 00          	jl     0x1545
    14b7:	8b 5c 3c             	mov    0x3c(%si),%bx
    14ba:	83 fb 40             	cmp    $0x40,%bx
    14bd:	0f 8f 84 00          	jg     0x1545
    14c1:	83 fb c0             	cmp    $0xffc0,%bx
    14c4:	7c 7f                	jl     0x1545
    14c6:	90                   	nop
    14c7:	90                   	nop
    14c8:	c7 06 82 22 01 00    	movw   $0x1,0x2282
    14ce:	f7 06 1e 00 ff ff    	testw  $0xffff,0x1e
    14d4:	75 06                	jne    0x14dc
    14d6:	c7 06 1e 00 02 00    	movw   $0x2,0x1e
    14dc:	2e c7 86 ad 0d 08 00 	movw   $0x8,%cs:0xdad(%bp)
    14e3:	2e f7 06 70 0b 03 00 	testw  $0x3,%cs:0xb70
    14ea:	75 59                	jne    0x1545
    14ec:	2e c7 86 af 0d 01 00 	movw   $0x1,%cs:0xdaf(%bp)
    14f3:	e9 69 f7             	jmp    0xc5f
    14f6:	a1 5c 10             	mov    0x105c,%ax
    14f9:	c1 c8 03             	ror    $0x3,%ax
    14fc:	1d 00 00             	sbb    $0x0,%ax
    14ff:	89 44 5c             	mov    %ax,0x5c(%si)
    1502:	c7 44 58 00 00       	movw   $0x0,0x58(%si)
    1507:	a3 5c 10             	mov    %ax,0x105c
    150a:	a1 f8 22             	mov    0x22f8,%ax
    150d:	8b 5c 50             	mov    0x50(%si),%bx
    1510:	25 fc 0f             	and    $0xffc,%ax
    1513:	81 e3 fc 0f          	and    $0xffc,%bx
    1517:	2b c3                	sub    %bx,%ax
    1519:	c1 f8 04             	sar    $0x4,%ax
    151c:	c7 44 54 00 00       	movw   $0x0,0x54(%si)
    1521:	c7 06 fc 22 c0 ff    	movw   $0xffc0,0x22fc
    1527:	89 44 56             	mov    %ax,0x56(%si)
    152a:	8b 44 52             	mov    0x52(%si),%ax
    152d:	c1 f8 04             	sar    $0x4,%ax
    1530:	89 44 10             	mov    %ax,0x10(%si)
    1533:	c7 44 0e c7 0c       	movw   $0xcc7,0xe(%si)
    1538:	c7 06 fc 22 9c ff    	movw   $0xff9c,0x22fc
    153e:	c7 06 1e 00 02 00    	movw   $0x2,0x1e
    1544:	c3                   	ret
    1545:	8b 5c 58             	mov    0x58(%si),%bx
    1548:	83 c3 28             	add    $0x28,%bx
    154b:	81 e3 fc 0f          	and    $0xffc,%bx
    154f:	89 5c 58             	mov    %bx,0x58(%si)
    1552:	8b 87 36 00          	mov    0x36(%bx),%ax
    1556:	c1 f8 05             	sar    $0x5,%ax
    1559:	c3                   	ret
    155a:	2e f7 86 af 0d 02 00 	testw  $0x2,%cs:0xdaf(%bp)
    1561:	0f 85 bc 00          	jne    0x1621
    1565:	2e 8b 1e e7 1b       	mov    %cs:0x1be7,%bx
    156a:	2e 89 b7 eb 1b       	mov    %si,%cs:0x1beb(%bx)
    156f:	83 c3 02             	add    $0x2,%bx
    1572:	83 e3 0f             	and    $0xf,%bx
    1575:	2e 8b 1e e7 1b       	mov    %cs:0x1be7,%bx
    157a:	2e 89 36 e5 1b       	mov    %si,%cs:0x1be5
    157f:	f7 44 5c ff ff       	testw  $0xffff,0x5c(%si)
    1584:	75 18                	jne    0x159e
    1586:	1e                   	push   %ds
    1587:	8b 7c 06             	mov    0x6(%si),%di
    158a:	8b 4c 02             	mov    0x2(%si),%cx
    158d:	8e 1e 02 00          	mov    0x2,%ds
    1591:	66 81 2d 80 00 80 00 	subl   $0x800080,(%di)
    1598:	83 c7 14             	add    $0x14,%di
    159b:	e2 f4                	loop   0x1591
    159d:	1f                   	pop    %ds
