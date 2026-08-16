
export_check/_tmp_dat/amer.xdb:     file format binary


Disassembly of section .data:

00001414 <.data+0x1414>:
    1414:	8b 6c 5a             	mov    0x5a(%si),%bp
    1417:	2e 8b 86 63 0d       	mov    %cs:0xd63(%bp),%ax
    141c:	2e 8b 9e 65 0d       	mov    %cs:0xd65(%bp),%bx
    1421:	2e 8b 96 67 0d       	mov    %cs:0xd67(%bp),%dx
    1426:	01 44 4e             	add    %ax,0x4e(%si)
    1429:	01 5c 50             	add    %bx,0x50(%si)
    142c:	2e f7 06 31 0b ff ff 	testw  $0xffff,%cs:0xb31
    1433:	89 54 54             	mov    %dx,0x54(%si)
    1436:	75 15                	jne    0x144d
    1438:	83 c5 08             	add    $0x8,%bp
    143b:	81 e5 fc 03          	and    $0x3fc,%bp
    143f:	89 6c 5a             	mov    %bp,0x5a(%si)
    1442:	2e f7 86 69 0d 03 00 	testw  $0x3,%cs:0xd69(%bp)
    1449:	0f 85 c7 00          	jne    0x1514
    144d:	f7 44 5c ff ff       	testw  $0xffff,0x5c(%si)
    1452:	0f 85 a9 00          	jne    0x14ff
    1456:	8b 54 40             	mov    0x40(%si),%dx
    1459:	83 fa 40             	cmp    $0x40,%dx
    145c:	0f 87 9f 00          	ja     0x14ff
    1460:	8b 44 38             	mov    0x38(%si),%ax
    1463:	3d 40 00             	cmp    $0x40,%ax
    1466:	0f 8f 95 00          	jg     0x14ff
    146a:	3d c0 ff             	cmp    $0xffc0,%ax
    146d:	0f 8c 8e 00          	jl     0x14ff
    1471:	8b 5c 3c             	mov    0x3c(%si),%bx
    1474:	83 fb 40             	cmp    $0x40,%bx
    1477:	0f 8f 84 00          	jg     0x14ff
    147b:	83 fb c0             	cmp    $0xffc0,%bx
    147e:	7c 7f                	jl     0x14ff
    1480:	90                   	nop
    1481:	90                   	nop
    1482:	c7 06 82 22 01 00    	movw   $0x1,0x2282
    1488:	f7 06 1e 00 ff ff    	testw  $0xffff,0x1e
    148e:	75 06                	jne    0x1496
    1490:	c7 06 1e 00 02 00    	movw   $0x2,0x1e
    1496:	2e c7 86 67 0d 08 00 	movw   $0x8,%cs:0xd67(%bp)
    149d:	2e f7 06 2f 0b 03 00 	testw  $0x3,%cs:0xb2f
    14a4:	75 59                	jne    0x14ff
    14a6:	2e c7 86 69 0d 01 00 	movw   $0x1,%cs:0xd69(%bp)
    14ad:	e9 67 f7             	jmp    0xc17
    14b0:	a1 5c 10             	mov    0x105c,%ax
    14b3:	c1 c8 03             	ror    $0x3,%ax
    14b6:	1d 00 00             	sbb    $0x0,%ax
    14b9:	89 44 5c             	mov    %ax,0x5c(%si)
    14bc:	c7 44 58 00 00       	movw   $0x0,0x58(%si)
    14c1:	a3 5c 10             	mov    %ax,0x105c
    14c4:	a1 f8 22             	mov    0x22f8,%ax
    14c7:	8b 5c 50             	mov    0x50(%si),%bx
    14ca:	25 fc 0f             	and    $0xffc,%ax
    14cd:	81 e3 fc 0f          	and    $0xffc,%bx
    14d1:	2b c3                	sub    %bx,%ax
    14d3:	c1 f8 04             	sar    $0x4,%ax
    14d6:	c7 44 54 00 00       	movw   $0x0,0x54(%si)
    14db:	c7 06 fc 22 c0 ff    	movw   $0xffc0,0x22fc
    14e1:	89 44 56             	mov    %ax,0x56(%si)
    14e4:	8b 44 52             	mov    0x52(%si),%ax
    14e7:	c1 f8 04             	sar    $0x4,%ax
    14ea:	89 44 10             	mov    %ax,0x10(%si)
    14ed:	c7 44 0e 81 0c       	movw   $0xc81,0xe(%si)
    14f2:	c7 06 fc 22 9c ff    	movw   $0xff9c,0x22fc
    14f8:	c7 06 1e 00 02 00    	movw   $0x2,0x1e
    14fe:	c3                   	ret
    14ff:	8b 5c 58             	mov    0x58(%si),%bx
    1502:	83 c3 28             	add    $0x28,%bx
    1505:	81 e3 fc 0f          	and    $0xffc,%bx
    1509:	89 5c 58             	mov    %bx,0x58(%si)
    150c:	8b 87 36 00          	mov    0x36(%bx),%ax
    1510:	c1 f8 05             	sar    $0x5,%ax
    1513:	c3                   	ret
    1514:	2e f7 86 69 0d 02 00 	testw  $0x2,%cs:0xd69(%bp)
    151b:	0f 85 bc 00          	jne    0x15db
    151f:	2e 8b 1e c6 1b       	mov    %cs:0x1bc6,%bx
    1524:	2e 89 b7 ca 1b       	mov    %si,%cs:0x1bca(%bx)
    1529:	83 c3 02             	add    $0x2,%bx
    152c:	83 e3 0f             	and    $0xf,%bx
    152f:	2e 8b 1e c6 1b       	mov    %cs:0x1bc6,%bx
    1534:	2e 89 36 c4 1b       	mov    %si,%cs:0x1bc4
    1539:	f7 44 5c ff ff       	testw  $0xffff,0x5c(%si)
    153e:	75 18                	jne    0x1558
    1540:	1e                   	push   %ds
    1541:	8b 7c 06             	mov    0x6(%si),%di
    1544:	8b 4c 02             	mov    0x2(%si),%cx
    1547:	8e 1e 02 00          	mov    0x2,%ds
    154b:	66 81 2d 80 00 80 00 	subl   $0x800080,(%di)
    1552:	83 c7 14             	add    $0x14,%di
    1555:	e2 f4                	loop   0x154b
    1557:	1f                   	pop    %ds
    1558:	2e c7 86 69 0d 00 00 	movw   $0x0,%cs:0xd69(%bp)
    155f:	2e c7 86 67 0d 08 00 	movw   $0x8,%cs:0xd67(%bp)
    1566:	c7 44 0e b3 12       	movw   $0x12b3,0xe(%si)
    156b:	c7 44 52 00 00       	movw   $0x0,0x52(%si)
    1570:	c7 44 54 08 00       	movw   $0x8,0x54(%si)
    1575:	c7 44 56 1e 00       	movw   $0x1e,0x56(%si)
    157a:	a1 5c 10             	mov    0x105c,%ax
    157d:	c1 c8 03             	ror    $0x3,%ax
    1580:	1d 00 00             	sbb    $0x0,%ax
    1583:	89 44 5c             	mov    %ax,0x5c(%si)
    1586:	a3 5c 10             	mov    %ax,0x105c
    1589:	c3                   	ret
; Commander Blood raw routine disassembly
; module: xdb_amer
; artifact: export_check/_tmp_dat/amer.xdb
; routine_entry: 0x001414
; group: callback_state_machine
; provenance: generic callback published by slot-3 initializer
; raw stop: 0x00158A
