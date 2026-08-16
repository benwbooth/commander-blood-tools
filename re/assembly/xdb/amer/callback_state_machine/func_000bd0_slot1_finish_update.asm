; Commander Blood recovered routine assembly
; module: xdb_amer
; artifact: export_check/_tmp_dat/amer.xdb
; routine_entry: 0x000BD0
; group: callback_state_machine
; provenance: callback installed by the AMER slot-1 wave update at 0x0B70
; raw stop: 0x000BEA (0x1A bytes)

export_check/_tmp_dat/amer.xdb:     file format binary


Disassembly of section .data:

00000bd0 <.data+0xbd0>:
     bd0:	2e a1 35 0b          	mov    %cs:0xb35,%ax
     bd4:	2d b0 00             	sub    $0xb0,%ax
     bd7:	89 44 46             	mov    %ax,0x46(%si)
     bda:	81 44 4e a0 00       	addw   $0xa0,0x4e(%si)
     bdf:	81 44 50 d0 00       	addw   $0xd0,0x50(%si)
     be4:	81 44 52 e0 00       	addw   $0xe0,0x52(%si)
     be9:	c3                   	ret
