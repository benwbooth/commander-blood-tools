; Commander Blood recovered routine assembly
; module: xdb_croolis
; artifact: output/_tmp_dat/croolis.xdb
; artifact_sha256: 13eba7c4e4f38662c44c849bd07be293c7583d2b2dc9ae273d7fe94746048c31
; overlay_offset: 0x001BC9
; byte_count: 66
; routine_bytes_sha256: 381c9a47aa1e32faa0f27d3eed8263318501027c41701e876611d2bdb301dd21
; routine_entry: 0x001BC9
; group: callback_state_machine
; provenance: continuation stored at context +0x36 by 0x1B85
; raw stop: 0x001C0B


output/_tmp_dat/croolis.xdb:     file format binary


Disassembly of section .data:

00001bc9 <.data+0x1bc9>:
    1bc9:	e8 93 ff             	call   0x1b5f
    1bcc:	57                   	push   %di
    1bcd:	8b 75 16             	mov    0x16(%di),%si
    1bd0:	8b 7d 3a             	mov    0x3a(%di),%di
    1bd3:	83 c6 5e             	add    $0x5e,%si
    1bd6:	8b 45 54             	mov    0x54(%di),%ax
    1bd9:	8b d8                	mov    %ax,%bx
    1bdb:	d1 fb                	sar    $1,%bx
    1bdd:	03 44 54             	add    0x54(%si),%ax
    1be0:	03 c3                	add    %bx,%ax
    1be2:	d1 f8                	sar    $1,%ax
    1be4:	89 44 54             	mov    %ax,0x54(%si)
    1be7:	e8 5c 00             	call   0x1c46
    1bea:	5f                   	pop    %di
    1beb:	72 01                	jb     0x1bee
    1bed:	c3                   	ret
    1bee:	c7 45 36 0b 1c       	movw   $0x1c0b,0x36(%di)
    1bf3:	c7 44 54 00 00       	movw   $0x0,0x54(%si)
    1bf8:	8b 75 3a             	mov    0x3a(%di),%si
    1bfb:	c7 44 0e e2 15       	movw   $0x15e2,0xe(%si)
    1c00:	89 75 3c             	mov    %si,0x3c(%di)
    1c03:	2e c7 06 b7 0d 18 00 	movw   $0x18,%cs:0xdb7
    1c0a:	c3                   	ret
