; Commander Blood recovered routine assembly
; module: xdb_amer
; artifact: output/_tmp_dat/amer.xdb
; artifact_sha256: 6fddeb5cc7c62fe5e638900746decc299f1637ea11612c0c51aced367dd12b31
; overlay_offset: 0x001558
; byte_count: 50
; routine_bytes_sha256: 2a1e8de52d2f3196361f13995fa21caf300fe036a58630bfd9d085138f4c491d
; routine_entry: 0x001558
; group: callback_state_machine
; provenance: generic slot-3 fallthrough and callback installed by final resume stage
; raw stop: 0x00158A


output/_tmp_dat/amer.xdb:     file format binary


Disassembly of section .data:

00001558 <.data+0x1558>:
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
