; Commander Blood recovered routine assembly
; module: xdb_amer
; artifact: output/_tmp_dat/amer.xdb
; artifact_sha256: 6fddeb5cc7c62fe5e638900746decc299f1637ea11612c0c51aced367dd12b31
; overlay_offset: 0x00158A
; byte_count: 81
; routine_bytes_sha256: 023c0d65c25447bdd89901d8a22fe08e5b4bc885f025bbb330d0f810ae96c332
; routine_entry: 0x00158A
; group: callback_state_machine
; provenance: callback installed by resume pair stage
; raw stop: 0x0015DB


output/_tmp_dat/amer.xdb:     file format binary


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
