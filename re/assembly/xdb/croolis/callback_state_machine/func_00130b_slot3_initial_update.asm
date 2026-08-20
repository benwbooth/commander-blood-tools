; Commander Blood recovered routine assembly
; module: xdb_croolis
; artifact: output/_tmp_dat/croolis.xdb
; artifact_sha256: 13eba7c4e4f38662c44c849bd07be293c7583d2b2dc9ae273d7fe94746048c31
; overlay_offset: 0x00130B
; byte_count: 353
; routine_bytes_sha256: cbc387f1acb3362bddf8707c8a72921f522b1bfead168a90025709f94d671c7e
; routine_entry: 0x00130B
; group: callback_state_machine
; provenance: initial callback reached by method-table slot 3
; raw stop: 0x00146C


output/_tmp_dat/croolis.xdb:     file format binary


Disassembly of section .data:

0000130b <.data+0x130b>:
    130b:	8d 7c 4e             	lea    0x4e(%si),%di
    130e:	8b 6c 5a             	mov    0x5a(%si),%bp
    1311:	2e 8b 86 bb 0d       	mov    %cs:0xdbb(%bp),%ax
    1316:	2e 8b 9e bd 0d       	mov    %cs:0xdbd(%bp),%bx
    131b:	2e 8b 96 bf 0d       	mov    %cs:0xdbf(%bp),%dx
    1320:	2e c7 86 c1 0d 00 00 	movw   $0x0,%cs:0xdc1(%bp)
    1327:	01 44 4e             	add    %ax,0x4e(%si)
    132a:	01 5c 50             	add    %bx,0x50(%si)
    132d:	2e f7 06 72 0b ff ff 	testw  $0xffff,%cs:0xb72
    1334:	89 54 54             	mov    %dx,0x54(%si)
    1337:	74 01                	je     0x133a
    1339:	c3                   	ret
    133a:	83 c5 08             	add    $0x8,%bp
    133d:	81 e5 fc 03          	and    $0x3fc,%bp
    1341:	89 6c 5a             	mov    %bp,0x5a(%si)
    1344:	ff 4c 56             	decw   0x56(%si)
    1347:	0f 88 b2 00          	js     0x13fd
    134b:	2e 89 86 bb 0d       	mov    %ax,%cs:0xdbb(%bp)
    1350:	2e 89 9e bd 0d       	mov    %bx,%cs:0xdbd(%bp)
    1355:	2e 89 96 bf 0d       	mov    %dx,%cs:0xdbf(%bp)
    135a:	8b 44 50             	mov    0x50(%si),%ax
    135d:	25 fc 0f             	and    $0xffc,%ax
    1360:	8b 5c 4a             	mov    0x4a(%si),%bx
    1363:	81 fb 28 23          	cmp    $0x2328,%bx
    1367:	7c 0a                	jl     0x1373
    1369:	ba 00 08             	mov    $0x800,%dx
    136c:	25 fc 0f             	and    $0xffc,%ax
    136f:	2b d0                	sub    %ax,%dx
    1371:	eb 38                	jmp    0x13ab
    1373:	83 fb 00             	cmp    $0x0,%bx
    1376:	7f 0c                	jg     0x1384
    1378:	ba 00 08             	mov    $0x800,%dx
    137b:	03 c2                	add    %dx,%ax
    137d:	25 fc 0f             	and    $0xffc,%ax
    1380:	2b d0                	sub    %ax,%dx
    1382:	eb 27                	jmp    0x13ab
    1384:	8b 5c 42             	mov    0x42(%si),%bx
    1387:	81 fb b8 0b          	cmp    $0xbb8,%bx
    138b:	7c 0d                	jl     0x139a
    138d:	ba 00 08             	mov    $0x800,%dx
    1390:	2d 00 04             	sub    $0x400,%ax
    1393:	25 fc 0f             	and    $0xffc,%ax
    1396:	2b d0                	sub    %ax,%dx
    1398:	eb 11                	jmp    0x13ab
    139a:	81 fb 48 f4          	cmp    $0xf448,%bx
    139e:	7f 13                	jg     0x13b3
    13a0:	ba 00 08             	mov    $0x800,%dx
    13a3:	05 00 04             	add    $0x400,%ax
    13a6:	25 fc 0f             	and    $0xffc,%ax
    13a9:	2b d0                	sub    %ax,%dx
    13ab:	c1 fa 04             	sar    $0x4,%dx
    13ae:	2e 89 96 bd 0d       	mov    %dx,%cs:0xdbd(%bp)
    13b3:	8b 44 4e             	mov    0x4e(%si),%ax
    13b6:	8b 54 46             	mov    0x46(%si),%dx
    13b9:	81 fa 18 fc          	cmp    $0xfc18,%dx
    13bd:	7f 1c                	jg     0x13db
    13bf:	bb 00 08             	mov    $0x800,%bx
    13c2:	03 c3                	add    %bx,%ax
    13c4:	25 fc 0f             	and    $0xffc,%ax
    13c7:	81 eb 00 02          	sub    $0x200,%bx
    13cb:	2b d8                	sub    %ax,%bx
    13cd:	c1 fb 03             	sar    $0x3,%bx
    13d0:	c7 44 56 00 00       	movw   $0x0,0x56(%si)
    13d5:	2e 89 9e bb 0d       	mov    %bx,%cs:0xdbb(%bp)
    13da:	c3                   	ret
    13db:	81 fa e8 03          	cmp    $0x3e8,%dx
    13df:	7c 1b                	jl     0x13fc
    13e1:	bb 00 08             	mov    $0x800,%bx
    13e4:	03 c3                	add    %bx,%ax
    13e6:	25 fc 0f             	and    $0xffc,%ax
    13e9:	81 c3 00 02          	add    $0x200,%bx
    13ed:	2b d8                	sub    %ax,%bx
    13ef:	c1 fb 03             	sar    $0x3,%bx
    13f2:	c7 44 56 00 00       	movw   $0x0,0x56(%si)
    13f7:	2e 89 9e bb 0d       	mov    %bx,%cs:0xdbb(%bp)
    13fc:	c3                   	ret
    13fd:	8b 44 5c             	mov    0x5c(%si),%ax
    1400:	c1 c8 03             	ror    $0x3,%ax
    1403:	1d 00 00             	sbb    $0x0,%ax
    1406:	8b c8                	mov    %ax,%cx
    1408:	c1 c8 03             	ror    $0x3,%ax
    140b:	1d 00 00             	sbb    $0x0,%ax
    140e:	83 e1 3f             	and    $0x3f,%cx
    1411:	8b d8                	mov    %ax,%bx
    1413:	83 c1 08             	add    $0x8,%cx
    1416:	c1 f8 09             	sar    $0x9,%ax
    1419:	2e 89 86 bd 0d       	mov    %ax,%cs:0xdbd(%bp)
    141e:	8b 54 4e             	mov    0x4e(%si),%dx
    1421:	81 c2 00 08          	add    $0x800,%dx
    1425:	81 e2 fc 0f          	and    $0xffc,%dx
    1429:	81 ea 00 08          	sub    $0x800,%dx
    142d:	89 54 4e             	mov    %dx,0x4e(%si)
    1430:	f7 da                	neg    %dx
    1432:	8b c3                	mov    %bx,%ax
    1434:	c1 c8 03             	ror    $0x3,%ax
    1437:	1d 00 00             	sbb    $0x0,%ax
    143a:	8b d8                	mov    %ax,%bx
    143c:	25 fc 0f             	and    $0xffc,%ax
    143f:	2d 00 08             	sub    $0x800,%ax
    1442:	c1 f8 02             	sar    $0x2,%ax
    1445:	13 c2                	adc    %dx,%ax
    1447:	99                   	cwtd
    1448:	f7 f9                	idiv   %cx
    144a:	2e 89 86 bb 0d       	mov    %ax,%cs:0xdbb(%bp)
    144f:	c1 f9 03             	sar    $0x3,%cx
    1452:	89 4c 56             	mov    %cx,0x56(%si)
    1455:	8b c3                	mov    %bx,%ax
    1457:	c1 c8 03             	ror    $0x3,%ax
    145a:	1d 00 00             	sbb    $0x0,%ax
    145d:	89 44 5c             	mov    %ax,0x5c(%si)
    1460:	25 3f 00             	and    $0x3f,%ax
    1463:	05 08 00             	add    $0x8,%ax
    1466:	2e 89 86 bf 0d       	mov    %ax,%cs:0xdbf(%bp)
    146b:	c3                   	ret
