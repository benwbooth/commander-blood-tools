
export_check/_tmp_dat/croolis.xdb:     file format binary


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
    1633:	2e c7 06 b7 0d 12 00 	movw   $0x12,%cs:0xdb7
    163a:	2e 89 36 b9 0d       	mov    %si,%cs:0xdb9
    163f:	66 c7 44 42 00 00 00 	movl   $0x0,0x42(%si)
    1646:	00 
    1647:	66 c7 44 46 a4 06 00 	movl   $0x6a4,0x46(%si)
    164e:	00 
    164f:	66 c7 44 4a 00 00 00 	movl   $0x0,0x4a(%si)
    1656:	00 
    1657:	c7 44 4e 00 00       	movw   $0x0,0x4e(%si)
    165c:	c7 44 50 00 00       	movw   $0x0,0x50(%si)
    1661:	c7 44 52 00 00       	movw   $0x0,0x52(%si)
    1666:	c7 44 54 00 00       	movw   $0x0,0x54(%si)
    166b:	c3                   	ret
; Commander Blood raw routine disassembly
; module: xdb_croolis
; artifact: export_check/_tmp_dat/croolis.xdb
; routine_entry: 0x0015E2
; group: callback_state_machine
; provenance: internal callback installed by the resume state machine at 0x001BF8
; raw stop: 0x00166C
