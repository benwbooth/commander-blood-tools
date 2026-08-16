
export_check/_tmp_dat/amer.xdb:     file format binary


Disassembly of section .data:

0000158a <.data+0x158a>:
    158a:	8b 6c 5a             	mov    0x5a(%si),%bp
    158d:	66 c7 44 42 00 00 00 	movl   $0x0,0x42(%si)
    1594:	00 
    1595:	66 c7 44 46 a4 06 00 	movl   $0x6a4,0x46(%si)
    159c:	00 
    159d:	66 c7 44 4a 00 00 00 	movl   $0x0,0x4a(%si)
    15a4:	00 
    15a5:	c7 44 4e 00 00       	movw   $0x0,0x4e(%si)
    15aa:	c7 44 50 00 00       	movw   $0x0,0x50(%si)
    15af:	c7 44 52 00 00       	movw   $0x0,0x52(%si)
    15b4:	c7 44 54 00 00       	movw   $0x0,0x54(%si)
    15b9:	c7 44 0e 14 16       	movw   $0x1614,0xe(%si)
    15be:	2e c7 86 63 0d 00 00 	movw   $0x0,%cs:0xd63(%bp)
    15c5:	2e c7 86 65 0d 00 00 	movw   $0x0,%cs:0xd65(%bp)
    15cc:	2e c7 86 67 0d 00 00 	movw   $0x0,%cs:0xd67(%bp)
    15d3:	2e c7 86 69 0d 02 00 	movw   $0x2,%cs:0xd69(%bp)
    15da:	c3                   	ret
    15db:	2e c7 06 5f 0d 12 00 	movw   $0x12,%cs:0xd5f
    15e2:	2e 89 36 61 0d       	mov    %si,%cs:0xd61
    15e7:	66 c7 44 42 00 00 00 	movl   $0x0,0x42(%si)
    15ee:	00 
    15ef:	66 c7 44 46 a4 06 00 	movl   $0x6a4,0x46(%si)
    15f6:	00 
    15f7:	66 c7 44 4a 00 00 00 	movl   $0x0,0x4a(%si)
    15fe:	00 
    15ff:	c7 44 4e 00 00       	movw   $0x0,0x4e(%si)
    1604:	c7 44 50 00 00       	movw   $0x0,0x50(%si)
    1609:	c7 44 52 00 00       	movw   $0x0,0x52(%si)
    160e:	c7 44 54 00 00       	movw   $0x0,0x54(%si)
    1613:	c3                   	ret
; Commander Blood raw routine disassembly
; module: xdb_amer
; artifact: export_check/_tmp_dat/amer.xdb
; routine_entry: 0x00158A
; group: callback_state_machine
; provenance: internal callback installed by the resume state machine at 0x001CAC
; raw stop: 0x001614
