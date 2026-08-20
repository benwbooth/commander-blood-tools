; Commander Blood recovered routine assembly
; module: xdb_croolis
; artifact: output/_tmp_dat/croolis.xdb
; artifact_sha256: 13eba7c4e4f38662c44c849bd07be293c7583d2b2dc9ae273d7fe94746048c31
; overlay_offset: 0x001B85
; byte_count: 68
; routine_bytes_sha256: c80321702e0c0ab6c36580c421b8185cbe6f1dea84515c44341e38a020c76e13
; routine_entry: 0x001B85
; group: callback_state_machine
; provenance: resume callback published by method-table slot 13
; raw stop: 0x001BC9


output/_tmp_dat/croolis.xdb:     file format binary


Disassembly of section .data:

00001b85 <.data+0x1b85>:
    1b85:	8b 75 16             	mov    0x16(%di),%si
    1b88:	2e 8b 1e 34 1b       	mov    %cs:0x1b34,%bx
    1b8d:	2e 8b 87 36 1b       	mov    %cs:0x1b36(%bx),%ax
    1b92:	0b c0                	or     %ax,%ax
    1b94:	75 1d                	jne    0x1bb3
    1b96:	83 c3 02             	add    $0x2,%bx
    1b99:	83 e3 0f             	and    $0xf,%bx
    1b9c:	2e 89 1e 34 1b       	mov    %bx,%cs:0x1b34
    1ba1:	8b 84 ac 00          	mov    0xac(%si),%ax
    1ba5:	2d e0 07             	sub    $0x7e0,%ax
    1ba8:	25 fc 0f             	and    $0xffc,%ax
    1bab:	2d 00 08             	sub    $0x800,%ax
    1bae:	89 84 ac 00          	mov    %ax,0xac(%si)
    1bb2:	c3                   	ret
    1bb3:	2e c7 06 30 1b 00 00 	movw   $0x0,%cs:0x1b30
    1bba:	2e c7 87 36 1b 00 00 	movw   $0x0,%cs:0x1b36(%bx)
    1bc1:	c7 45 36 c9 1b       	movw   $0x1bc9,0x36(%di)
    1bc6:	89 45 3a             	mov    %ax,0x3a(%di)
