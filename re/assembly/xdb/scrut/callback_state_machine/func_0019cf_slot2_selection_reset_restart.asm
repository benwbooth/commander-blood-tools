; Commander Blood recovered routine assembly
; module: xdb_scrut
; artifact: output/_tmp_dat/scrut.xdb
; artifact_sha256: 8522e77aad6639cf9bb06f148048fed3c6bf1cfbe6459c9a0be7b987c3c3ac77
; overlay_offset: 0x0019CF
; byte_count: 52
; routine_bytes_sha256: 65cda118a7dc1b124b49a78b8ddb0e6f152fc75f773a1718fe8902ab432c28e7
; routine_entry: 0x0019CF
; group: callback_state_machine
; provenance: shared selection-reset tail reached by callbacks 0x181B and 0x1868
; direct_callees: 0x001711
; raw stop: 0x001A03


output/_tmp_dat/scrut.xdb:     file format binary


Disassembly of section .data:

000019cf <.data+0x19cf>:
    19cf:	8b 84 46 03          	mov    0x346(%si),%ax
    19d3:	8b 9c 4a 03          	mov    0x34a(%si),%bx
    19d7:	89 84 32 03          	mov    %ax,0x332(%si)
    19db:	89 9c 3a 03          	mov    %bx,0x33a(%si)
    19df:	8b 84 a4 03          	mov    0x3a4(%si),%ax
    19e3:	8b 9c a8 03          	mov    0x3a8(%si),%bx
    19e7:	89 84 90 03          	mov    %ax,0x390(%si)
    19eb:	89 9c 98 03          	mov    %bx,0x398(%si)
    19ef:	2e c7 06 8e 16 00 00 	movw   $0x0,%cs:0x168e
    19f6:	c7 44 58 00 00       	movw   $0x0,0x58(%si)
    19fb:	c7 44 5a 00 00       	movw   $0x0,0x5a(%si)
    1a00:	e9 0e fd             	jmp    0x1711
