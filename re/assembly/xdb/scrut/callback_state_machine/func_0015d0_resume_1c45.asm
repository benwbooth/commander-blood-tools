
export_check/_tmp_dat/scrut.xdb:     file format binary


Disassembly of section .data:

000015d0 <.data+0x15d0>:
    15d0:	8b 6c 5a             	mov    0x5a(%si),%bp
    15d3:	66 c7 44 42 a4 06 00 	movl   $0x6a4,0x42(%si)
    15da:	00 
    15db:	66 c7 44 46 00 00 00 	movl   $0x0,0x46(%si)
    15e2:	00 
    15e3:	66 c7 44 4a 00 00 00 	movl   $0x0,0x4a(%si)
    15ea:	00 
    15eb:	c7 44 4e 00 00       	movw   $0x0,0x4e(%si)
    15f0:	c7 44 50 00 00       	movw   $0x0,0x50(%si)
    15f5:	c7 44 52 00 00       	movw   $0x0,0x52(%si)
    15fa:	c7 44 54 00 00       	movw   $0x0,0x54(%si)
    15ff:	c7 44 0e 5a 16       	movw   $0x165a,0xe(%si)
    1604:	2e c7 86 a9 0d 00 00 	movw   $0x0,%cs:0xda9(%bp)
    160b:	2e c7 86 ab 0d 00 00 	movw   $0x0,%cs:0xdab(%bp)
    1612:	2e c7 86 ad 0d 00 00 	movw   $0x0,%cs:0xdad(%bp)
    1619:	2e c7 86 af 0d 02 00 	movw   $0x2,%cs:0xdaf(%bp)
    1620:	c3                   	ret
    1621:	2e c7 06 a5 0d 12 00 	movw   $0x12,%cs:0xda5
    1628:	2e 89 36 a7 0d       	mov    %si,%cs:0xda7
    162d:	66 c7 44 42 a4 06 00 	movl   $0x6a4,0x42(%si)
    1634:	00 
    1635:	66 c7 44 46 00 00 00 	movl   $0x0,0x46(%si)
    163c:	00 
    163d:	66 c7 44 4a 00 00 00 	movl   $0x0,0x4a(%si)
    1644:	00 
    1645:	c7 44 4e 00 00       	movw   $0x0,0x4e(%si)
    164a:	c7 44 50 00 00       	movw   $0x0,0x50(%si)
    164f:	c7 44 52 00 00       	movw   $0x0,0x52(%si)
    1654:	c7 44 54 00 00       	movw   $0x0,0x54(%si)
    1659:	c3                   	ret
; Commander Blood raw routine disassembly
; module: xdb_scrut
; artifact: export_check/_tmp_dat/scrut.xdb
; routine_entry: 0x0015D0
; group: callback_state_machine
; provenance: callback continuation referenced by slot-13 resume/init
; raw stop: 0x00165A
