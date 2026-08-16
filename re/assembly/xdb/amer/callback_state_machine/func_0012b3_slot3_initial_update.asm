
export_check/_tmp_dat/amer.xdb:     file format binary


Disassembly of section .data:

000012b3 <.data+0x12b3>:
    12b3:	8d 7c 4e             	lea    0x4e(%si),%di
    12b6:	8b 6c 5a             	mov    0x5a(%si),%bp
    12b9:	2e 8b 86 63 0d       	mov    %cs:0xd63(%bp),%ax
    12be:	2e 8b 9e 65 0d       	mov    %cs:0xd65(%bp),%bx
    12c3:	2e 8b 96 67 0d       	mov    %cs:0xd67(%bp),%dx
    12c8:	2e c7 86 69 0d 00 00 	movw   $0x0,%cs:0xd69(%bp)
    12cf:	01 44 4e             	add    %ax,0x4e(%si)
    12d2:	01 5c 50             	add    %bx,0x50(%si)
    12d5:	2e f7 06 31 0b ff ff 	testw  $0xffff,%cs:0xb31
    12dc:	89 54 54             	mov    %dx,0x54(%si)
    12df:	74 01                	je     0x12e2
    12e1:	c3                   	ret
    12e2:	83 c5 08             	add    $0x8,%bp
    12e5:	81 e5 fc 03          	and    $0x3fc,%bp
    12e9:	89 6c 5a             	mov    %bp,0x5a(%si)
    12ec:	ff 4c 56             	decw   0x56(%si)
    12ef:	0f 88 b2 00          	js     0x13a5
    12f3:	2e 89 86 63 0d       	mov    %ax,%cs:0xd63(%bp)
    12f8:	2e 89 9e 65 0d       	mov    %bx,%cs:0xd65(%bp)
    12fd:	2e 89 96 67 0d       	mov    %dx,%cs:0xd67(%bp)
    1302:	8b 44 50             	mov    0x50(%si),%ax
    1305:	25 fc 0f             	and    $0xffc,%ax
    1308:	8b 5c 4a             	mov    0x4a(%si),%bx
    130b:	81 fb 00 30          	cmp    $0x3000,%bx
    130f:	7c 0a                	jl     0x131b
    1311:	ba 00 08             	mov    $0x800,%dx
    1314:	25 fc 0f             	and    $0xffc,%ax
    1317:	2b d0                	sub    %ax,%dx
    1319:	eb 38                	jmp    0x1353
    131b:	83 fb 00             	cmp    $0x0,%bx
    131e:	7f 0c                	jg     0x132c
    1320:	ba 00 08             	mov    $0x800,%dx
    1323:	03 c2                	add    %dx,%ax
    1325:	25 fc 0f             	and    $0xffc,%ax
    1328:	2b d0                	sub    %ax,%dx
    132a:	eb 27                	jmp    0x1353
    132c:	8b 5c 42             	mov    0x42(%si),%bx
    132f:	81 fb 00 15          	cmp    $0x1500,%bx
    1333:	7c 0d                	jl     0x1342
    1335:	ba 00 08             	mov    $0x800,%dx
    1338:	2d 00 04             	sub    $0x400,%ax
    133b:	25 fc 0f             	and    $0xffc,%ax
    133e:	2b d0                	sub    %ax,%dx
    1340:	eb 11                	jmp    0x1353
    1342:	81 fb 00 eb          	cmp    $0xeb00,%bx
    1346:	7f 13                	jg     0x135b
    1348:	ba 00 08             	mov    $0x800,%dx
    134b:	05 00 04             	add    $0x400,%ax
    134e:	25 fc 0f             	and    $0xffc,%ax
    1351:	2b d0                	sub    %ax,%dx
    1353:	c1 fa 04             	sar    $0x4,%dx
    1356:	2e 89 96 65 0d       	mov    %dx,%cs:0xd65(%bp)
    135b:	8b 44 4e             	mov    0x4e(%si),%ax
    135e:	8b 54 46             	mov    0x46(%si),%dx
    1361:	81 fa 18 fc          	cmp    $0xfc18,%dx
    1365:	7f 1c                	jg     0x1383
    1367:	bb 00 08             	mov    $0x800,%bx
    136a:	03 c3                	add    %bx,%ax
    136c:	25 fc 0f             	and    $0xffc,%ax
    136f:	81 eb 00 02          	sub    $0x200,%bx
    1373:	2b d8                	sub    %ax,%bx
    1375:	c1 fb 03             	sar    $0x3,%bx
    1378:	c7 44 56 00 00       	movw   $0x0,0x56(%si)
    137d:	2e 89 9e 63 0d       	mov    %bx,%cs:0xd63(%bp)
    1382:	c3                   	ret
    1383:	81 fa 08 07          	cmp    $0x708,%dx
    1387:	7c 1b                	jl     0x13a4
    1389:	bb 00 08             	mov    $0x800,%bx
    138c:	03 c3                	add    %bx,%ax
    138e:	25 fc 0f             	and    $0xffc,%ax
    1391:	81 c3 00 02          	add    $0x200,%bx
    1395:	2b d8                	sub    %ax,%bx
    1397:	c1 fb 03             	sar    $0x3,%bx
    139a:	c7 44 56 00 00       	movw   $0x0,0x56(%si)
    139f:	2e 89 9e 63 0d       	mov    %bx,%cs:0xd63(%bp)
    13a4:	c3                   	ret
    13a5:	8b 44 5c             	mov    0x5c(%si),%ax
    13a8:	c1 c8 03             	ror    $0x3,%ax
    13ab:	1d 00 00             	sbb    $0x0,%ax
    13ae:	8b c8                	mov    %ax,%cx
    13b0:	c1 c8 03             	ror    $0x3,%ax
    13b3:	1d 00 00             	sbb    $0x0,%ax
    13b6:	83 e1 3f             	and    $0x3f,%cx
    13b9:	8b d8                	mov    %ax,%bx
    13bb:	83 c1 08             	add    $0x8,%cx
    13be:	c1 f8 09             	sar    $0x9,%ax
    13c1:	2e 89 86 65 0d       	mov    %ax,%cs:0xd65(%bp)
    13c6:	8b 54 4e             	mov    0x4e(%si),%dx
    13c9:	81 c2 00 08          	add    $0x800,%dx
    13cd:	81 e2 fc 0f          	and    $0xffc,%dx
    13d1:	81 ea 00 08          	sub    $0x800,%dx
    13d5:	89 54 4e             	mov    %dx,0x4e(%si)
    13d8:	f7 da                	neg    %dx
    13da:	8b c3                	mov    %bx,%ax
    13dc:	c1 c8 03             	ror    $0x3,%ax
    13df:	1d 00 00             	sbb    $0x0,%ax
    13e2:	8b d8                	mov    %ax,%bx
    13e4:	25 fc 0f             	and    $0xffc,%ax
    13e7:	2d 00 08             	sub    $0x800,%ax
    13ea:	c1 f8 02             	sar    $0x2,%ax
    13ed:	13 c2                	adc    %dx,%ax
    13ef:	99                   	cwtd
    13f0:	f7 f9                	idiv   %cx
    13f2:	2e 89 86 63 0d       	mov    %ax,%cs:0xd63(%bp)
    13f7:	c1 f9 03             	sar    $0x3,%cx
    13fa:	89 4c 56             	mov    %cx,0x56(%si)
    13fd:	8b c3                	mov    %bx,%ax
    13ff:	c1 c8 03             	ror    $0x3,%ax
    1402:	1d 00 00             	sbb    $0x0,%ax
    1405:	89 44 5c             	mov    %ax,0x5c(%si)
    1408:	25 7f 00             	and    $0x7f,%ax
    140b:	05 08 00             	add    $0x8,%ax
    140e:	2e 89 86 67 0d       	mov    %ax,%cs:0xd67(%bp)
    1413:	c3                   	ret
; Commander Blood recovered routine assembly
; module: xdb_amer
; artifact: export_check/_tmp_dat/amer.xdb
; routine_entry: 0x0012B3
; group: callback_state_machine
; provenance: internal callback reached by method-table slot 3 at 0x001286
; raw stop: 0x001414 (0x161 bytes)
