; Commander Blood recovered routine assembly
; module: xdb_scrut
; artifact: output/_tmp_dat/scrut.xdb
; artifact_sha256: 8522e77aad6639cf9bb06f148048fed3c6bf1cfbe6459c9a0be7b987c3c3ac77
; overlay_offset: 0x0015D0
; byte_count: 81
; routine_bytes_sha256: f7f05bbfd9b9ef9344a73b91f483df4cc9927b6bfd2f5aea3ac2093dd03c57f3
; routine_entry: 0x0015D0
; group: callback_state_machine
; provenance: callback installed by resume pair stage
; raw stop: 0x001621


output/_tmp_dat/scrut.xdb:     file format binary


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
