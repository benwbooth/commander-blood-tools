; Commander Blood recovered routine assembly
; module: xdb_amer
; artifact: output/_tmp_dat/amer.xdb
; artifact_sha256: 6fddeb5cc7c62fe5e638900746decc299f1637ea11612c0c51aced367dd12b31
; overlay_offset: 0x001614
; byte_count: 52
; routine_bytes_sha256: 0778ff3e9ee060dfd17aba0e58538e744f76ce3d6cf44419509d3e19616df88f
; routine_entry: 0x001614
; group: callback_state_machine
; provenance: callback installed by slot-3 resume callback
; raw stop: 0x001648


output/_tmp_dat/amer.xdb:     file format binary


Disassembly of section .data:

00001614 <.data+0x1614>:
    1614:	90                   	nop
    1615:	2e f7 06 31 0b ff ff 	testw  $0xffff,%cs:0xb31
    161c:	75 29                	jne    0x1647
    161e:	8b 6c 5a             	mov    0x5a(%si),%bp
    1621:	83 c5 08             	add    $0x8,%bp
    1624:	81 e5 fc 03          	and    $0x3fc,%bp
    1628:	89 6c 5a             	mov    %bp,0x5a(%si)
    162b:	2e c7 86 63 0d 00 00 	movw   $0x0,%cs:0xd63(%bp)
    1632:	2e c7 86 65 0d 00 00 	movw   $0x0,%cs:0xd65(%bp)
    1639:	2e c7 86 67 0d 00 00 	movw   $0x0,%cs:0xd67(%bp)
    1640:	2e c7 86 69 0d 00 00 	movw   $0x0,%cs:0xd69(%bp)
    1647:	c3                   	ret
