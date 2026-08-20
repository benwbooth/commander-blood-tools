; Commander Blood recovered routine assembly
; module: xdb_amer
; artifact: output/_tmp_dat/amer.xdb
; artifact_sha256: 6fddeb5cc7c62fe5e638900746decc299f1637ea11612c0c51aced367dd12b31
; overlay_offset: 0x000CA1
; byte_count: 11
; routine_bytes_sha256: 3be9fc348ca22fcee496cfab1bcdef23da23f238d3df9f7436a4330bb7b20bea
; routine_entry: 0x000CA1
; group: callback_state_machine
; provenance: callback published by the AMER slot-1 motion callback at 0x0C96
; raw stop: 0x000CAC


output/_tmp_dat/amer.xdb:     file format binary


Disassembly of section .data:

00000ca1 <.data+0xca1>:
     ca1:	ff 4c 54             	decw   0x54(%si)
     ca4:	75 05                	jne    0xcab
     ca6:	c7 44 0e ea 0b       	movw   $0xbea,0xe(%si)
     cab:	c3                   	ret
