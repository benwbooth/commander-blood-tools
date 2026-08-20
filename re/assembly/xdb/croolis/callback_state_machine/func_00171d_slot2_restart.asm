; Commander Blood recovered routine assembly
; module: xdb_croolis
; artifact: output/_tmp_dat/croolis.xdb
; artifact_sha256: 13eba7c4e4f38662c44c849bd07be293c7583d2b2dc9ae273d7fe94746048c31
; overlay_offset: 0x00171D
; byte_count: 10
; routine_bytes_sha256: 982012e95ec718643146d8a1d4d73df51b479dedf25b0d884d56bc01fdea2234
; routine_entry: 0x00171D
; group: callback_state_machine
; provenance: internal restart tail reached by fade callback 0x17F2
; direct_callees: 0x001727
; raw stop: 0x001727


output/_tmp_dat/croolis.xdb:     file format binary


Disassembly of section .data:

0000171d <.data+0x171d>:
    171d:	c7 44 0e 27 17       	movw   $0x1727,0xe(%si)
    1722:	c7 44 58 64 00       	movw   $0x64,0x58(%si)
