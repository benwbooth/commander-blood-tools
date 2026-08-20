; Commander Blood recovered routine assembly
; module: xdb_amer
; artifact: output/_tmp_dat/amer.xdb
; artifact_sha256: 6fddeb5cc7c62fe5e638900746decc299f1637ea11612c0c51aced367dd12b31
; overlay_offset: 0x001A2B
; byte_count: 49
; routine_bytes_sha256: d7e35b8fcb41a11a654f391fead15a119688738b4cf48b5391bfb0461604c403
; routine_entry: 0x001A2B
; group: callback_state_machine
; provenance: shared reset tail reached by four AMER slot-2 callbacks
; direct_callees: none
; raw stop: 0x001A5C


output/_tmp_dat/amer.xdb:     file format binary


Disassembly of section .data:

00001a2b <.data+0x1a2b>:
    1a2b:	c7 44 52 00 00       	movw   $0x0,0x52(%si)
    1a30:	c7 45 3a 00 00       	movw   $0x0,0x3a(%di)
    1a35:	c7 44 5c 00 00       	movw   $0x0,0x5c(%si)
    1a3a:	c7 44 54 3c 00       	movw   $0x3c,0x54(%si)
    1a3f:	c7 44 0e 5c 1a       	movw   $0x1a5c,0xe(%si)
    1a44:	8b 45 40             	mov    0x40(%di),%ax
    1a47:	c1 c8 07             	ror    $0x7,%ax
    1a4a:	1d 00 00             	sbb    $0x0,%ax
    1a4d:	89 45 40             	mov    %ax,0x40(%di)
    1a50:	c1 f8 06             	sar    $0x6,%ax
    1a53:	89 44 4e             	mov    %ax,0x4e(%si)
    1a56:	c7 44 56 20 00       	movw   $0x20,0x56(%si)
    1a5b:	c3                   	ret
