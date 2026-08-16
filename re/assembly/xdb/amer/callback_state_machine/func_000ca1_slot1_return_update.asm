; Commander Blood recovered routine assembly
; module: xdb_amer
; artifact: export_check/_tmp_dat/amer.xdb
; routine_entry: 0x000CA1
; group: callback_state_machine
; provenance: callback published by the AMER slot-1 motion update at 0x0C96; terminal callback is the existing 0x0BEA body
; raw stop: 0x000CAB (0x0A bytes)

export_check/_tmp_dat/amer.xdb:     file format binary


Disassembly of section .data:

00000ca1 <.data+0xca1>:
     ca1:	ff 4c 54             	decw   0x54(%si)
     ca4:	75 05                	jne    0xcab
     ca6:	c7 44 0e ea 0b       	movw   $0xbea,0xe(%si)
