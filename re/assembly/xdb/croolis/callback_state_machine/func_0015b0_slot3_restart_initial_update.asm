; Commander Blood recovered routine assembly
; module: xdb_croolis
; artifact: output/_tmp_dat/croolis.xdb
; artifact_sha256: 13eba7c4e4f38662c44c849bd07be293c7583d2b2dc9ae273d7fe94746048c31
; overlay_offset: 0x0015B0
; byte_count: 50
; routine_bytes_sha256: bb668de9705df4c3eb4586efee3a9c7924993d3c98a63311a6f6980df5460739
; routine_entry: 0x0015B0
; group: callback_state_machine
; provenance: generic slot-3 fallthrough and callback installed by final resume stage
; raw stop: 0x0015E2


output/_tmp_dat/croolis.xdb:     file format binary


Disassembly of section .data:

000015b0 <.data+0x15b0>:
    15b0:	2e c7 86 c1 0d 00 00 	movw   $0x0,%cs:0xdc1(%bp)
    15b7:	2e c7 86 bf 0d 08 00 	movw   $0x8,%cs:0xdbf(%bp)
    15be:	c7 44 0e 0b 13       	movw   $0x130b,0xe(%si)
    15c3:	c7 44 52 00 00       	movw   $0x0,0x52(%si)
    15c8:	c7 44 54 08 00       	movw   $0x8,0x54(%si)
    15cd:	c7 44 56 1e 00       	movw   $0x1e,0x56(%si)
    15d2:	a1 5c 10             	mov    0x105c,%ax
    15d5:	c1 c8 03             	ror    $0x3,%ax
    15d8:	1d 00 00             	sbb    $0x0,%ax
    15db:	89 44 5c             	mov    %ax,0x5c(%si)
    15de:	a3 5c 10             	mov    %ax,0x105c
    15e1:	c3                   	ret
