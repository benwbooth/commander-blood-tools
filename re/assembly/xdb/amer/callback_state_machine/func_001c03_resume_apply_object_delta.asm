; Commander Blood recovered routine assembly
; module: xdb_amer
; artifact: output/_tmp_dat/amer.xdb
; artifact_sha256: 6fddeb5cc7c62fe5e638900746decc299f1637ea11612c0c51aced367dd12b31
; overlay_offset: 0x001C03
; byte_count: 49
; routine_bytes_sha256: 5a55516b0dc5aa9000623a38a700941b02546abda10af5a7615608b77516d16a
; routine_entry: 0x001C03
; group: callback_state_machine
; provenance: near helper called by resume pair and timeout stages
; raw stop: 0x001C34


output/_tmp_dat/amer.xdb:     file format binary


Disassembly of section .data:

00001c03 <.data+0x1c03>:
    1c03:	8b 75 1c             	mov    0x1c(%di),%si
    1c06:	8e 06 02 00          	mov    0x2,%es
    1c0a:	8b 45 38             	mov    0x38(%di),%ax
    1c0d:	8b d0                	mov    %ax,%dx
    1c0f:	98                   	cbtw
    1c10:	26 01 44 02          	add    %ax,%es:0x2(%si)
    1c14:	26 29 84 26 04       	sub    %ax,%es:0x426(%si)
    1c19:	26 01 84 be 02       	add    %ax,%es:0x2be(%si)
    1c1e:	26 29 84 f6 01       	sub    %ax,%es:0x1f6(%si)
    1c23:	02 f2                	add    %dl,%dh
    1c25:	75 02                	jne    0x1c29
    1c27:	b2 02                	mov    $0x2,%dl
    1c29:	80 fe 16             	cmp    $0x16,%dh
    1c2c:	7c 02                	jl     0x1c30
    1c2e:	b2 fe                	mov    $0xfe,%dl
    1c30:	89 55 38             	mov    %dx,0x38(%di)
    1c33:	c3                   	ret
