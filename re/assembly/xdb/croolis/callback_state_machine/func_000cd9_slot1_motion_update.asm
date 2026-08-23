; Commander Blood recovered routine assembly
; module: xdb_croolis
; artifact: output/_tmp_dat/croolis.xdb
; artifact_sha256: 13eba7c4e4f38662c44c849bd07be293c7583d2b2dc9ae273d7fe94746048c31
; overlay_offset: 0x000CD9
; byte_count: 32
; routine_bytes_sha256: 2be0d433e1cfc88eec92546572f709494b1c447bccf92f56814b981c3009a0a2
; routine_entry: 0x000CD9
; group: callback_state_machine
; provenance: callback published by slot-1 camera update
; direct_callees: none
; raw stop: 0x000CF9


output/_tmp_dat/croolis.xdb:     file format binary


Disassembly of section .data:

00000cd9 <.data+0xcd9>:
     cd9:	8b 44 56             	mov    0x56(%si),%ax
     cdc:	01 44 50             	add    %ax,0x50(%si)
     cdf:	8b 44 10             	mov    0x10(%si),%ax
     ce2:	29 44 52             	sub    %ax,0x52(%si)
     ce5:	ff 44 54             	incw   0x54(%si)
     ce8:	83 7c 54 0f          	cmpw   $0xf,0x54(%si)
     cec:	7e 0a                	jle    0xcf8
     cee:	c7 44 0e f9 0c       	movw   $0xcf9,0xe(%si)
     cf3:	c7 44 54 40 00       	movw   $0x40,0x54(%si)
     cf8:	c3                   	ret
