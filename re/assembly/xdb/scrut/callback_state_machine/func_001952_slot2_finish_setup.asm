; Commander Blood recovered routine assembly
; module: xdb_scrut
; artifact: output/_tmp_dat/scrut.xdb
; artifact_sha256: 8522e77aad6639cf9bb06f148048fed3c6bf1cfbe6459c9a0be7b987c3c3ac77
; overlay_offset: 0x001952
; byte_count: 5
; routine_bytes_sha256: de90e82d83a006fce7dde05484a2dc1e184cf3d07a0785f09b4cd03cfcfde814
; routine_entry: 0x001952
; group: callback_state_machine
; provenance: internal callback transition reached by 0x1868
; direct_callees: 0x001957
; raw stop: 0x001957


output/_tmp_dat/scrut.xdb:     file format binary


Disassembly of section .data:

00001952 <.data+0x1952>:
    1952:	c7 44 0e 57 19       	movw   $0x1957,0xe(%si)
