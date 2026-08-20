; Commander Blood recovered routine assembly
; module: xdb_croolis
; artifact: output/_tmp_dat/croolis.xdb
; artifact_sha256: 13eba7c4e4f38662c44c849bd07be293c7583d2b2dc9ae273d7fe94746048c31
; overlay_offset: 0x001B5F
; byte_count: 38
; routine_bytes_sha256: d0846d2653c7edb275038d45f3b0a9e3d2896a74492c0de63b33082175d4a552
; routine_entry: 0x001B5F
; group: callback_state_machine
; provenance: near helper called by resume pair and timeout stages
; raw stop: 0x001B85


output/_tmp_dat/croolis.xdb:     file format binary


Disassembly of section .data:

00001b5f <.data+0x1b5f>:
    1b5f:	8b 75 1c             	mov    0x1c(%di),%si
    1b62:	8e 06 02 00          	mov    0x2,%es
    1b66:	8b 45 38             	mov    0x38(%di),%ax
    1b69:	8b d0                	mov    %ax,%dx
    1b6b:	98                   	cbtw
    1b6c:	26 01 04             	add    %ax,%es:(%si)
    1b6f:	26 29 84 f4 01       	sub    %ax,%es:0x1f4(%si)
    1b74:	02 f2                	add    %dl,%dh
    1b76:	75 02                	jne    0x1b7a
    1b78:	b2 02                	mov    $0x2,%dl
    1b7a:	80 fe 16             	cmp    $0x16,%dh
    1b7d:	7c 02                	jl     0x1b81
    1b7f:	b2 fe                	mov    $0xfe,%dl
    1b81:	89 55 38             	mov    %dx,0x38(%di)
    1b84:	c3                   	ret
