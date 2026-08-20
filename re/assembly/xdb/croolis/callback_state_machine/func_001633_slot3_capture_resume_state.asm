; Commander Blood recovered routine assembly
; module: xdb_croolis
; artifact: output/_tmp_dat/croolis.xdb
; artifact_sha256: 13eba7c4e4f38662c44c849bd07be293c7583d2b2dc9ae273d7fe94746048c31
; overlay_offset: 0x001633
; byte_count: 57
; routine_bytes_sha256: 592a5c7d078bee64223c401b2fdc131862c057151c9de27f6f425981a21a7de2
; routine_entry: 0x001633
; group: callback_state_machine
; provenance: tail target of slot-3 update when ring flag bit 1 is set
; raw stop: 0x00166C


output/_tmp_dat/croolis.xdb:     file format binary


Disassembly of section .data:

00001633 <.data+0x1633>:
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
