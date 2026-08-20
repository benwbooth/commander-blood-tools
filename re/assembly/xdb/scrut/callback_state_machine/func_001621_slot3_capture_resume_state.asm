; Commander Blood recovered routine assembly
; module: xdb_scrut
; artifact: output/_tmp_dat/scrut.xdb
; artifact_sha256: 8522e77aad6639cf9bb06f148048fed3c6bf1cfbe6459c9a0be7b987c3c3ac77
; overlay_offset: 0x001621
; byte_count: 57
; routine_bytes_sha256: b8da15e8b430e9e7a8a8995767caba0bf80dcc79bf4abde228023143ea31c966
; routine_entry: 0x001621
; group: callback_state_machine
; provenance: tail target of slot-3 update when ring flag bit 1 is set
; raw stop: 0x00165A


output/_tmp_dat/scrut.xdb:     file format binary


Disassembly of section .data:

00001621 <.data+0x1621>:
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
