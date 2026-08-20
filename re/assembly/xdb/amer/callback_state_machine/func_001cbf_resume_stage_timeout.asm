; Commander Blood recovered routine assembly
; module: xdb_amer
; artifact: output/_tmp_dat/amer.xdb
; artifact_sha256: 6fddeb5cc7c62fe5e638900746decc299f1637ea11612c0c51aced367dd12b31
; overlay_offset: 0x001CBF
; byte_count: 16
; routine_bytes_sha256: cd54330606123c32becba15d2246dce8146706109af6aad33cfff812f55daa8d
; routine_entry: 0x001CBF
; group: callback_state_machine
; provenance: continuation stored at context +0x36 by 0x1C7D
; raw stop: 0x001CCF


output/_tmp_dat/amer.xdb:     file format binary


Disassembly of section .data:

00001cbf <.data+0x1cbf>:
    1cbf:	e8 41 ff             	call   0x1c03
    1cc2:	2e ff 0e 5f 0d       	decw   %cs:0xd5f
    1cc7:	79 05                	jns    0x1cce
    1cc9:	c7 45 36 cf 1c       	movw   $0x1ccf,0x36(%di)
    1cce:	c3                   	ret
