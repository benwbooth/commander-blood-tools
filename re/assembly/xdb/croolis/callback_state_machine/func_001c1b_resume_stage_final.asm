; Commander Blood recovered routine assembly
; module: xdb_croolis
; artifact: output/_tmp_dat/croolis.xdb
; artifact_sha256: 13eba7c4e4f38662c44c849bd07be293c7583d2b2dc9ae273d7fe94746048c31
; overlay_offset: 0x001C1B
; byte_count: 43
; routine_bytes_sha256: 4c9003270e9f990c1f42e15caea4a6acf61098712ebd5ed5588d1c8db5eb77ef
; routine_entry: 0x001C1B
; group: callback_state_machine
; provenance: continuation stored at context +0x36 by 0x1C0B
; raw stop: 0x001C46


output/_tmp_dat/croolis.xdb:     file format binary


Disassembly of section .data:

00001c1b <.data+0x1c1b>:
    1c1b:	57                   	push   %di
    1c1c:	8b 75 16             	mov    0x16(%di),%si
    1c1f:	83 c6 5e             	add    $0x5e,%si
    1c22:	c7 44 54 64 00       	movw   $0x64,0x54(%si)
    1c27:	2e 8b 3e 2e 1b       	mov    %cs:0x1b2e,%di
    1c2c:	e8 17 00             	call   0x1c46
    1c2f:	5f                   	pop    %di
    1c30:	72 01                	jb     0x1c33
    1c32:	c3                   	ret
    1c33:	c7 45 36 85 1b       	movw   $0x1b85,0x36(%di)
    1c38:	8b 5d 3a             	mov    0x3a(%di),%bx
    1c3b:	c7 47 0e b0 15       	movw   $0x15b0,0xe(%bx)
    1c40:	c7 44 54 00 00       	movw   $0x0,0x54(%si)
    1c45:	c3                   	ret
