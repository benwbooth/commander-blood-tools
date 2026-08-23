; Commander Blood recovered routine assembly
; module: xdb_croolis
; artifact: output/_tmp_dat/croolis.xdb
; artifact_sha256: 13eba7c4e4f38662c44c849bd07be293c7583d2b2dc9ae273d7fe94746048c31
; overlay_offset: 0x000CB5
; byte_count: 36
; routine_bytes_sha256: 35fa77194a66391aba092fdedb546ba97276c7e7389a4a5389ae6bbc9954809b
; routine_entry: 0x000CB5
; group: callback_state_machine
; provenance: shared camera-to-motion transition
; direct_callees: none
; raw stop: 0x000CD9


output/_tmp_dat/croolis.xdb:     file format binary


Disassembly of section .data:

00000cb5 <.data+0xcb5>:
     cb5:	a1 f8 22             	mov    0x22f8,%ax
     cb8:	8b 5c 50             	mov    0x50(%si),%bx
     cbb:	25 fc 0f             	and    $0xffc,%ax
     cbe:	81 e3 fc 0f          	and    $0xffc,%bx
     cc2:	2b c3                	sub    %bx,%ax
     cc4:	c1 f8 04             	sar    $0x4,%ax
     cc7:	89 44 56             	mov    %ax,0x56(%si)
     cca:	8b 44 52             	mov    0x52(%si),%ax
     ccd:	c1 f8 04             	sar    $0x4,%ax
     cd0:	89 44 10             	mov    %ax,0x10(%si)
     cd3:	c7 44 0e d9 0c       	movw   $0xcd9,0xe(%si)
     cd8:	c3                   	ret
