; Commander Blood recovered routine assembly
; module: xdb_scrut
; artifact: output/_tmp_dat/scrut.xdb
; artifact_sha256: 8522e77aad6639cf9bb06f148048fed3c6bf1cfbe6459c9a0be7b987c3c3ac77
; overlay_offset: 0x0012F9
; byte_count: 353
; routine_bytes_sha256: 08cba9d7d3807f6bcc0f527f6278a633f99b6697fb42110ad9cf04a75a697c4d
; routine_entry: 0x0012F9
; group: callback_state_machine
; provenance: initial callback reached by method-table slot 3
; raw stop: 0x00145A


output/_tmp_dat/scrut.xdb:     file format binary


Disassembly of section .data:

000012f9 <.data+0x12f9>:
    12f9:	8d 7c 4e             	lea    0x4e(%si),%di
    12fc:	8b 6c 5a             	mov    0x5a(%si),%bp
    12ff:	2e 8b 86 a9 0d       	mov    %cs:0xda9(%bp),%ax
    1304:	2e 8b 9e ab 0d       	mov    %cs:0xdab(%bp),%bx
    1309:	2e 8b 96 ad 0d       	mov    %cs:0xdad(%bp),%dx
    130e:	2e c7 86 af 0d 00 00 	movw   $0x0,%cs:0xdaf(%bp)
    1315:	01 44 4e             	add    %ax,0x4e(%si)
    1318:	01 5c 50             	add    %bx,0x50(%si)
    131b:	2e f7 06 72 0b ff ff 	testw  $0xffff,%cs:0xb72
    1322:	89 54 54             	mov    %dx,0x54(%si)
    1325:	74 01                	je     0x1328
    1327:	c3                   	ret
    1328:	83 c5 08             	add    $0x8,%bp
    132b:	81 e5 fc 03          	and    $0x3fc,%bp
    132f:	89 6c 5a             	mov    %bp,0x5a(%si)
    1332:	ff 4c 56             	decw   0x56(%si)
    1335:	0f 88 b2 00          	js     0x13eb
    1339:	2e 89 86 a9 0d       	mov    %ax,%cs:0xda9(%bp)
    133e:	2e 89 9e ab 0d       	mov    %bx,%cs:0xdab(%bp)
    1343:	2e 89 96 ad 0d       	mov    %dx,%cs:0xdad(%bp)
    1348:	8b 44 50             	mov    0x50(%si),%ax
    134b:	25 fc 0f             	and    $0xffc,%ax
    134e:	8b 5c 4a             	mov    0x4a(%si),%bx
    1351:	81 fb 28 23          	cmp    $0x2328,%bx
    1355:	7c 0a                	jl     0x1361
    1357:	ba 00 08             	mov    $0x800,%dx
    135a:	25 fc 0f             	and    $0xffc,%ax
    135d:	2b d0                	sub    %ax,%dx
    135f:	eb 38                	jmp    0x1399
    1361:	83 fb 00             	cmp    $0x0,%bx
    1364:	7f 0c                	jg     0x1372
    1366:	ba 00 08             	mov    $0x800,%dx
    1369:	03 c2                	add    %dx,%ax
    136b:	25 fc 0f             	and    $0xffc,%ax
    136e:	2b d0                	sub    %ax,%dx
    1370:	eb 27                	jmp    0x1399
    1372:	8b 5c 42             	mov    0x42(%si),%bx
    1375:	81 fb b8 0b          	cmp    $0xbb8,%bx
    1379:	7c 0d                	jl     0x1388
    137b:	ba 00 08             	mov    $0x800,%dx
    137e:	2d 00 04             	sub    $0x400,%ax
    1381:	25 fc 0f             	and    $0xffc,%ax
    1384:	2b d0                	sub    %ax,%dx
    1386:	eb 11                	jmp    0x1399
    1388:	81 fb 48 f4          	cmp    $0xf448,%bx
    138c:	7f 13                	jg     0x13a1
    138e:	ba 00 08             	mov    $0x800,%dx
    1391:	05 00 04             	add    $0x400,%ax
    1394:	25 fc 0f             	and    $0xffc,%ax
    1397:	2b d0                	sub    %ax,%dx
    1399:	c1 fa 04             	sar    $0x4,%dx
    139c:	2e 89 96 ab 0d       	mov    %dx,%cs:0xdab(%bp)
    13a1:	8b 44 4e             	mov    0x4e(%si),%ax
    13a4:	8b 54 46             	mov    0x46(%si),%dx
    13a7:	81 fa 18 fc          	cmp    $0xfc18,%dx
    13ab:	7f 1c                	jg     0x13c9
    13ad:	bb 00 08             	mov    $0x800,%bx
    13b0:	03 c3                	add    %bx,%ax
    13b2:	25 fc 0f             	and    $0xffc,%ax
    13b5:	81 eb 00 02          	sub    $0x200,%bx
    13b9:	2b d8                	sub    %ax,%bx
    13bb:	c1 fb 03             	sar    $0x3,%bx
    13be:	c7 44 56 00 00       	movw   $0x0,0x56(%si)
    13c3:	2e 89 9e a9 0d       	mov    %bx,%cs:0xda9(%bp)
    13c8:	c3                   	ret
    13c9:	81 fa e8 03          	cmp    $0x3e8,%dx
    13cd:	7c 1b                	jl     0x13ea
    13cf:	bb 00 08             	mov    $0x800,%bx
    13d2:	03 c3                	add    %bx,%ax
    13d4:	25 fc 0f             	and    $0xffc,%ax
    13d7:	81 c3 00 02          	add    $0x200,%bx
    13db:	2b d8                	sub    %ax,%bx
    13dd:	c1 fb 03             	sar    $0x3,%bx
    13e0:	c7 44 56 00 00       	movw   $0x0,0x56(%si)
    13e5:	2e 89 9e a9 0d       	mov    %bx,%cs:0xda9(%bp)
    13ea:	c3                   	ret
    13eb:	8b 44 5c             	mov    0x5c(%si),%ax
    13ee:	c1 c8 03             	ror    $0x3,%ax
    13f1:	1d 00 00             	sbb    $0x0,%ax
    13f4:	8b c8                	mov    %ax,%cx
    13f6:	c1 c8 03             	ror    $0x3,%ax
    13f9:	1d 00 00             	sbb    $0x0,%ax
    13fc:	83 e1 3f             	and    $0x3f,%cx
    13ff:	8b d8                	mov    %ax,%bx
    1401:	83 c1 08             	add    $0x8,%cx
    1404:	c1 f8 09             	sar    $0x9,%ax
    1407:	2e 89 86 ab 0d       	mov    %ax,%cs:0xdab(%bp)
    140c:	8b 54 4e             	mov    0x4e(%si),%dx
    140f:	81 c2 00 08          	add    $0x800,%dx
    1413:	81 e2 fc 0f          	and    $0xffc,%dx
    1417:	81 ea 00 08          	sub    $0x800,%dx
    141b:	89 54 4e             	mov    %dx,0x4e(%si)
    141e:	f7 da                	neg    %dx
    1420:	8b c3                	mov    %bx,%ax
    1422:	c1 c8 03             	ror    $0x3,%ax
    1425:	1d 00 00             	sbb    $0x0,%ax
    1428:	8b d8                	mov    %ax,%bx
    142a:	25 fc 0f             	and    $0xffc,%ax
    142d:	2d 00 08             	sub    $0x800,%ax
    1430:	c1 f8 02             	sar    $0x2,%ax
    1433:	13 c2                	adc    %dx,%ax
    1435:	99                   	cwtd
    1436:	f7 f9                	idiv   %cx
    1438:	2e 89 86 a9 0d       	mov    %ax,%cs:0xda9(%bp)
    143d:	c1 f9 03             	sar    $0x3,%cx
    1440:	89 4c 56             	mov    %cx,0x56(%si)
    1443:	8b c3                	mov    %bx,%ax
    1445:	c1 c8 03             	ror    $0x3,%ax
    1448:	1d 00 00             	sbb    $0x0,%ax
    144b:	89 44 5c             	mov    %ax,0x5c(%si)
    144e:	25 3f 00             	and    $0x3f,%ax
    1451:	05 08 00             	add    $0x8,%ax
    1454:	2e 89 86 ad 0d       	mov    %ax,%cs:0xdad(%bp)
    1459:	c3                   	ret
