; Commander Blood recovered routine assembly
; module: xdb_croolis
; artifact: output/_tmp_dat/croolis.xdb
; artifact_sha256: 13eba7c4e4f38662c44c849bd07be293c7583d2b2dc9ae273d7fe94746048c31
; overlay_offset: 0x00178E
; byte_count: 6
; routine_bytes_sha256: 6461ac176d66dfb8ccd1ebf42c195d0ec0d81716b96773e6e3a762e7beaa33d0
; routine_entry: 0x00178E
; group: callback_state_machine
; provenance: shared control-latch dispatch reached by slot-2 callbacks
; direct_callees: 0x001794, 0x0017E4
; raw stop: 0x001794


output/_tmp_dat/croolis.xdb:     file format binary


Disassembly of section .data:

0000178e <.data+0x178e>:
    178e:	39 3e 82 22          	cmp    %di,0x2282
    1792:	74 50                	je     0x17e4
