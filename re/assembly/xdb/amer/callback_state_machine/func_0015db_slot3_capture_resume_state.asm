; Commander Blood recovered routine assembly
; module: xdb_amer
; artifact: output/_tmp_dat/amer.xdb
; artifact_sha256: 6fddeb5cc7c62fe5e638900746decc299f1637ea11612c0c51aced367dd12b31
; overlay_offset: 0x0015DB
; byte_count: 57
; routine_bytes_sha256: 5203cdb5019a7ebb327a92120bf26347b26fef3da785235379f48f0b46135a40
; routine_entry: 0x0015DB
; group: callback_state_machine
; provenance: tail target of slot-3 update when ring flag bit 1 is set
; raw stop: 0x001614


output/_tmp_dat/amer.xdb:     file format binary


Disassembly of section .data:

000015db <.data+0x15db>:
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
