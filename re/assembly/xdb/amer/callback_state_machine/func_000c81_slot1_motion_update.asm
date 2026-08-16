; Commander Blood recovered routine assembly
; module: xdb_amer
; artifact: export_check/_tmp_dat/amer.xdb
; routine_entry: 0x000C81
; group: callback_state_machine
; provenance: callback published by the AMER slot-1 camera continuation at 0x0C7B
; raw stop: 0x000CA1 (0x20 bytes)

export_check/_tmp_dat/amer.xdb:     file format binary


Disassembly of section .data:

00000c81 <.data+0xc81>:
     c81:	8b 44 56             	mov    0x56(%si),%ax
     c84:	01 44 50             	add    %ax,0x50(%si)
     c87:	8b 44 10             	mov    0x10(%si),%ax
     c8a:	29 44 52             	sub    %ax,0x52(%si)
     c8d:	ff 44 54             	incw   0x54(%si)
     c90:	83 7c 54 0f          	cmpw   $0xf,0x54(%si)
     c94:	7e 0a                	jle    0xca0
     c96:	c7 44 0e a1 0c       	movw   $0xca1,0xe(%si)
     c9b:	c7 44 54 40 00       	movw   $0x40,0x54(%si)
     ca0:	c3                   	ret
