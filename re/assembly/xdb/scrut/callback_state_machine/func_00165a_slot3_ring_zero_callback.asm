; Commander Blood recovered routine assembly
; module: xdb_scrut
; artifact: output/_tmp_dat/scrut.xdb
; artifact_sha256: 8522e77aad6639cf9bb06f148048fed3c6bf1cfbe6459c9a0be7b987c3c3ac77
; overlay_offset: 0x00165A
; byte_count: 52
; routine_bytes_sha256: b7eef2effd0ae190399c62fabf7137013f83d44d492c9cdd152b03a9a9d5924d
; routine_entry: 0x00165A
; group: callback_state_machine
; provenance: callback installed by slot-3 resume callback
; raw stop: 0x00168E


output/_tmp_dat/scrut.xdb:     file format binary


Disassembly of section .data:

0000165a <.data+0x165a>:
    165a:	90                   	nop
    165b:	2e f7 06 72 0b ff ff 	testw  $0xffff,%cs:0xb72
    1662:	75 29                	jne    0x168d
    1664:	8b 6c 5a             	mov    0x5a(%si),%bp
    1667:	83 c5 08             	add    $0x8,%bp
    166a:	81 e5 fc 03          	and    $0x3fc,%bp
    166e:	89 6c 5a             	mov    %bp,0x5a(%si)
    1671:	2e c7 86 a9 0d 00 00 	movw   $0x0,%cs:0xda9(%bp)
    1678:	2e c7 86 ab 0d 00 00 	movw   $0x0,%cs:0xdab(%bp)
    167f:	2e c7 86 ad 0d 00 00 	movw   $0x0,%cs:0xdad(%bp)
    1686:	2e c7 86 af 0d 00 00 	movw   $0x0,%cs:0xdaf(%bp)
    168d:	c3                   	ret
