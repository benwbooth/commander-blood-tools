; Commander Blood recovered routine assembly
; module: xdb_croolis
; artifact: output/_tmp_dat/croolis.xdb
; artifact_sha256: 13eba7c4e4f38662c44c849bd07be293c7583d2b2dc9ae273d7fe94746048c31
; overlay_offset: 0x0015E2
; byte_count: 81
; routine_bytes_sha256: a177b430a47f03da75c8706811233e23fb0c92511a0fd89fef2fa966246199be
; routine_entry: 0x0015E2
; group: callback_state_machine
; provenance: callback installed by resume pair stage
; raw stop: 0x001633


output/_tmp_dat/croolis.xdb:     file format binary


Disassembly of section .data:

000015e2 <.data+0x15e2>:
    15e2:	8b 6c 5a             	mov    0x5a(%si),%bp
    15e5:	66 c7 44 42 00 00 00 	movl   $0x0,0x42(%si)
    15ec:	00
    15ed:	66 c7 44 46 a4 06 00 	movl   $0x6a4,0x46(%si)
    15f4:	00
    15f5:	66 c7 44 4a 00 00 00 	movl   $0x0,0x4a(%si)
    15fc:	00
    15fd:	c7 44 4e 00 00       	movw   $0x0,0x4e(%si)
    1602:	c7 44 50 00 00       	movw   $0x0,0x50(%si)
    1607:	c7 44 52 00 00       	movw   $0x0,0x52(%si)
    160c:	c7 44 54 00 00       	movw   $0x0,0x54(%si)
    1611:	c7 44 0e 6c 16       	movw   $0x166c,0xe(%si)
    1616:	2e c7 86 bb 0d 00 00 	movw   $0x0,%cs:0xdbb(%bp)
    161d:	2e c7 86 bd 0d 00 00 	movw   $0x0,%cs:0xdbd(%bp)
    1624:	2e c7 86 bf 0d 00 00 	movw   $0x0,%cs:0xdbf(%bp)
    162b:	2e c7 86 c1 0d 02 00 	movw   $0x2,%cs:0xdc1(%bp)
    1632:	c3                   	ret
