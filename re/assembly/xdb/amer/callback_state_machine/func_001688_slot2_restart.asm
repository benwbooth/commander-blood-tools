; Commander Blood recovered routine assembly
; module: xdb_amer
; artifact: output/_tmp_dat/amer.xdb
; artifact_sha256: 6fddeb5cc7c62fe5e638900746decc299f1637ea11612c0c51aced367dd12b31
; overlay_offset: 0x001688
; byte_count: 10
; routine_bytes_sha256: 1040cb5e1877ffda8f47ee00354659d4e5dfb0841fe1fd83c7a34aa0388cc794
; routine_entry: 0x001688
; group: callback_state_machine
; provenance: internal transition reached by callback 0x1948
; direct_callees: 0x001692
; raw stop: 0x001692


output/_tmp_dat/amer.xdb:     file format binary


Disassembly of section .data:

00001688 <.data+0x1688>:
    1688:	c7 44 0e 92 16       	movw   $0x1692,0xe(%si)
    168d:	c7 44 58 14 00       	movw   $0x14,0x58(%si)
