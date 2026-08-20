; Commander Blood recovered routine assembly
; module: xdb_croolis
; artifact: output/_tmp_dat/croolis.xdb
; artifact_sha256: 13eba7c4e4f38662c44c849bd07be293c7583d2b2dc9ae273d7fe94746048c31
; overlay_offset: 0x00146C
; byte_count: 324
; routine_bytes_sha256: 1123ca26e88f4b20189b4fda085212cd066b3b509cdc1d433114c30f6ac28fab
; routine_entry: 0x00146C
; group: callback_state_machine
; provenance: generic callback published by slot-3 initializer
; raw stop: 0x0015B0


output/_tmp_dat/croolis.xdb:     file format binary


Disassembly of section .data:

0000146c <.data+0x146c>:
    146c:	8b 6c 5a             	mov    0x5a(%si),%bp
    146f:	2e 8b 86 bb 0d       	mov    %cs:0xdbb(%bp),%ax
    1474:	2e 8b 9e bd 0d       	mov    %cs:0xdbd(%bp),%bx
    1479:	2e 8b 96 bf 0d       	mov    %cs:0xdbf(%bp),%dx
    147e:	01 44 4e             	add    %ax,0x4e(%si)
    1481:	01 5c 50             	add    %bx,0x50(%si)
    1484:	2e f7 06 72 0b ff ff 	testw  $0xffff,%cs:0xb72
    148b:	89 54 54             	mov    %dx,0x54(%si)
    148e:	75 15                	jne    0x14a5
    1490:	83 c5 08             	add    $0x8,%bp
    1493:	81 e5 fc 03          	and    $0x3fc,%bp
    1497:	89 6c 5a             	mov    %bp,0x5a(%si)
    149a:	2e f7 86 c1 0d 03 00 	testw  $0x3,%cs:0xdc1(%bp)
    14a1:	0f 85 c7 00          	jne    0x156c
    14a5:	f7 44 5c ff ff       	testw  $0xffff,0x5c(%si)
    14aa:	0f 85 a9 00          	jne    0x1557
    14ae:	8b 54 40             	mov    0x40(%si),%dx
    14b1:	83 fa 40             	cmp    $0x40,%dx
    14b4:	0f 87 9f 00          	ja     0x1557
    14b8:	8b 44 38             	mov    0x38(%si),%ax
    14bb:	3d 40 00             	cmp    $0x40,%ax
    14be:	0f 8f 95 00          	jg     0x1557
    14c2:	3d c0 ff             	cmp    $0xffc0,%ax
    14c5:	0f 8c 8e 00          	jl     0x1557
    14c9:	8b 5c 3c             	mov    0x3c(%si),%bx
    14cc:	83 fb 40             	cmp    $0x40,%bx
    14cf:	0f 8f 84 00          	jg     0x1557
    14d3:	83 fb c0             	cmp    $0xffc0,%bx
    14d6:	7c 7f                	jl     0x1557
    14d8:	90                   	nop
    14d9:	90                   	nop
    14da:	c7 06 82 22 01 00    	movw   $0x1,0x2282
    14e0:	f7 06 1e 00 ff ff    	testw  $0xffff,0x1e
    14e6:	75 06                	jne    0x14ee
    14e8:	c7 06 1e 00 02 00    	movw   $0x2,0x1e
    14ee:	2e c7 86 bf 0d 08 00 	movw   $0x8,%cs:0xdbf(%bp)
    14f5:	2e f7 06 70 0b 03 00 	testw  $0x3,%cs:0xb70
    14fc:	75 59                	jne    0x1557
    14fe:	2e c7 86 c1 0d 01 00 	movw   $0x1,%cs:0xdc1(%bp)
    1505:	e9 63 f7             	jmp    0xc6b
    1508:	a1 5c 10             	mov    0x105c,%ax
    150b:	c1 c8 03             	ror    $0x3,%ax
    150e:	1d 00 00             	sbb    $0x0,%ax
    1511:	89 44 5c             	mov    %ax,0x5c(%si)
    1514:	c7 44 58 00 00       	movw   $0x0,0x58(%si)
    1519:	a3 5c 10             	mov    %ax,0x105c
    151c:	a1 f8 22             	mov    0x22f8,%ax
    151f:	8b 5c 50             	mov    0x50(%si),%bx
    1522:	25 fc 0f             	and    $0xffc,%ax
    1525:	81 e3 fc 0f          	and    $0xffc,%bx
    1529:	2b c3                	sub    %bx,%ax
    152b:	c1 f8 04             	sar    $0x4,%ax
    152e:	c7 44 54 00 00       	movw   $0x0,0x54(%si)
    1533:	c7 06 fc 22 c0 ff    	movw   $0xffc0,0x22fc
    1539:	89 44 56             	mov    %ax,0x56(%si)
    153c:	8b 44 52             	mov    0x52(%si),%ax
    153f:	c1 f8 04             	sar    $0x4,%ax
    1542:	89 44 10             	mov    %ax,0x10(%si)
    1545:	c7 44 0e d9 0c       	movw   $0xcd9,0xe(%si)
    154a:	c7 06 fc 22 9c ff    	movw   $0xff9c,0x22fc
    1550:	c7 06 1e 00 02 00    	movw   $0x2,0x1e
    1556:	c3                   	ret
    1557:	8b 5c 58             	mov    0x58(%si),%bx
    155a:	83 c3 28             	add    $0x28,%bx
    155d:	81 e3 fc 0f          	and    $0xffc,%bx
    1561:	89 5c 58             	mov    %bx,0x58(%si)
    1564:	8b 87 36 00          	mov    0x36(%bx),%ax
    1568:	c1 f8 05             	sar    $0x5,%ax
    156b:	c3                   	ret
    156c:	2e f7 86 c1 0d 02 00 	testw  $0x2,%cs:0xdc1(%bp)
    1573:	0f 85 bc 00          	jne    0x1633
    1577:	2e 8b 1e 32 1b       	mov    %cs:0x1b32,%bx
    157c:	2e 89 b7 36 1b       	mov    %si,%cs:0x1b36(%bx)
    1581:	83 c3 02             	add    $0x2,%bx
    1584:	83 e3 0f             	and    $0xf,%bx
    1587:	2e 8b 1e 32 1b       	mov    %cs:0x1b32,%bx
    158c:	2e 89 36 30 1b       	mov    %si,%cs:0x1b30
    1591:	f7 44 5c ff ff       	testw  $0xffff,0x5c(%si)
    1596:	75 18                	jne    0x15b0
    1598:	1e                   	push   %ds
    1599:	8b 7c 06             	mov    0x6(%si),%di
    159c:	8b 4c 02             	mov    0x2(%si),%cx
    159f:	8e 1e 02 00          	mov    0x2,%ds
    15a3:	66 81 2d 80 00 80 00 	subl   $0x800080,(%di)
    15aa:	83 c7 14             	add    $0x14,%di
    15ad:	e2 f4                	loop   0x15a3
    15af:	1f                   	pop    %ds
