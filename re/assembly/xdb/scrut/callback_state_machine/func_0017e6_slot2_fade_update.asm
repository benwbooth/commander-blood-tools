; Commander Blood recovered routine assembly
; module: xdb_scrut
; artifact: output/_tmp_dat/scrut.xdb
; artifact_sha256: 8522e77aad6639cf9bb06f148048fed3c6bf1cfbe6459c9a0be7b987c3c3ac77
; overlay_offset: 0x0017E6
; byte_count: 28
; routine_bytes_sha256: 4af9369736fa34b6d76c8bc26dbcb0450073f82e97dcf3aca9bb431da8637507
; routine_entry: 0x0017E6
; group: callback_state_machine
; provenance: callback published by unreferenced setup 0x17E1
; direct_callees: 0x001711, 0x001787
; raw stop: 0x001802


output/_tmp_dat/scrut.xdb:     file format binary


Disassembly of section .data:

000017e6 <.data+0x17e6>:
    17e6:	8b 45 38             	mov    0x38(%di),%ax
    17e9:	2d 04 00             	sub    $0x4,%ax
    17ec:	7c 05                	jl     0x17f3
    17ee:	89 45 38             	mov    %ax,0x38(%di)
    17f1:	eb 94                	jmp    0x1787
    17f3:	c7 45 38 00 00       	movw   $0x0,0x38(%di)
    17f8:	2e c7 06 8e 16 00 00 	movw   $0x0,%cs:0x168e
    17ff:	e9 0f ff             	jmp    0x1711
