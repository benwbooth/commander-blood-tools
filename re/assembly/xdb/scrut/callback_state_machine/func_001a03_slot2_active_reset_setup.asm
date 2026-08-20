; Commander Blood recovered routine assembly
; module: xdb_scrut
; artifact: output/_tmp_dat/scrut.xdb
; artifact_sha256: 8522e77aad6639cf9bb06f148048fed3c6bf1cfbe6459c9a0be7b987c3c3ac77
; overlay_offset: 0x001A03
; byte_count: 14
; routine_bytes_sha256: 2026ba4087d663f2609e9e523cc5f79238cfb095158e2b47c5f7027aff2173ea
; routine_entry: 0x001A03
; group: callback_state_machine
; provenance: compiled active/reset setup with no table or in-overlay pointer reference
; direct_callees: 0x001A11
; raw stop: 0x001A11


output/_tmp_dat/scrut.xdb:     file format binary


Disassembly of section .data:

00001a03 <.data+0x1a03>:
    1a03:	2e c7 06 8e 16 01 00 	movw   $0x1,%cs:0x168e
    1a0a:	2e c7 06 90 16 00 00 	movw   $0x0,%cs:0x1690
