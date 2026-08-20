; Commander Blood recovered routine assembly
; module: xdb_amer
; artifact: output/_tmp_dat/amer.xdb
; artifact_sha256: 6fddeb5cc7c62fe5e638900746decc299f1637ea11612c0c51aced367dd12b31
; overlay_offset: 0x001CCF
; byte_count: 43
; routine_bytes_sha256: 19ce08c00751bd9513c21baefb62c36357c87571ffca044098f3de8ecd2679cb
; routine_entry: 0x001CCF
; group: callback_state_machine
; provenance: continuation stored at context +0x36 by 0x1CBF
; raw stop: 0x001CFA


output/_tmp_dat/amer.xdb:     file format binary


Disassembly of section .data:

00001ccf <.data+0x1ccf>:
    1ccf:	57                   	push   %di
    1cd0:	8b 75 16             	mov    0x16(%di),%si
    1cd3:	83 c6 5e             	add    $0x5e,%si
    1cd6:	c7 44 54 64 00       	movw   $0x64,0x54(%si)
    1cdb:	2e 8b 3e c2 1b       	mov    %cs:0x1bc2,%di
    1ce0:	e8 17 00             	call   0x1cfa
    1ce3:	5f                   	pop    %di
    1ce4:	72 01                	jb     0x1ce7
    1ce6:	c3                   	ret
    1ce7:	c7 45 36 34 1c       	movw   $0x1c34,0x36(%di)
    1cec:	8b 5d 3a             	mov    0x3a(%di),%bx
    1cef:	c7 47 0e 58 15       	movw   $0x1558,0xe(%bx)
    1cf4:	c7 44 54 00 00       	movw   $0x0,0x54(%si)
    1cf9:	c3                   	ret
