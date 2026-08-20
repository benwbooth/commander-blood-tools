; Commander Blood recovered routine assembly
; module: xdb_scrut
; artifact: output/_tmp_dat/scrut.xdb
; artifact_sha256: 8522e77aad6639cf9bb06f148048fed3c6bf1cfbe6459c9a0be7b987c3c3ac77
; overlay_offset: 0x001781
; byte_count: 6
; routine_bytes_sha256: da9d5fac31cb86554361661a2a31f22ed4fb8d69af3baa3ec24823a7e71cd3fe
; routine_entry: 0x001781
; group: callback_state_machine
; provenance: shared control-latch dispatch reached by slot-2 callbacks
; direct_callees: 0x001787
; raw stop: 0x001787


output/_tmp_dat/scrut.xdb:     file format binary


Disassembly of section .data:

00001781 <.data+0x1781>:
    1781:	39 3e 82 22          	cmp    %di,0x2282
    1785:	74 59                	je     0x17e0
