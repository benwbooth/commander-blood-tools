; Commander Blood recovered routine assembly
; module: xdb_scrut
; artifact: output/_tmp_dat/scrut.xdb
; artifact_sha256: 8522e77aad6639cf9bb06f148048fed3c6bf1cfbe6459c9a0be7b987c3c3ac77
; overlay_offset: 0x0017E1
; byte_count: 5
; routine_bytes_sha256: 392df0d852bc458b925b4ccc3cd38c8f14650207e6cd773d0142eca6b3ee11ad
; routine_entry: 0x0017E1
; group: callback_state_machine
; provenance: compiled callback setup with no table or in-overlay pointer reference
; direct_callees: 0x0017E6
; raw stop: 0x0017E6


output/_tmp_dat/scrut.xdb:     file format binary


Disassembly of section .data:

000017e1 <.data+0x17e1>:
    17e1:	c7 44 0e e6 17       	movw   $0x17e6,0xe(%si)
