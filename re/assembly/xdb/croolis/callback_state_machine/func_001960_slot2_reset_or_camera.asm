; Commander Blood recovered routine assembly
; module: xdb_croolis
; artifact: output/_tmp_dat/croolis.xdb
; artifact_sha256: 13eba7c4e4f38662c44c849bd07be293c7583d2b2dc9ae273d7fe94746048c31
; overlay_offset: 0x001960
; byte_count: 294
; routine_bytes_sha256: f953b882ed544eca90e7cb4a23dc35ab074345d5b3f24831b086c79b8522c11e
; routine_entry: 0x001960
; group: callback_state_machine
; provenance: shared reset and camera tail reached by callbacks 0x1727 and 0x1828
; direct_callees: 0x00178E
; raw stop: 0x001A86


output/_tmp_dat/croolis.xdb:     file format binary


Disassembly of section .data:

00001960 <.data+0x1960>:
    1960:	66 ba d0 07 00 00    	mov    $0x7d0,%edx
    1966:	66 0f bf 44 40       	movswl 0x40(%si),%eax
    196b:	66 3d 0c fe ff ff    	cmp    $0xfffffe0c,%eax
    1971:	7c 6a                	jl     0x19dd
    1973:	66 2b c2             	sub    %edx,%eax
    1976:	66 0f bf 4c 38       	movswl 0x38(%si),%ecx
    197b:	66 2b 4d 3c          	sub    0x3c(%di),%ecx
    197f:	66 8b d8             	mov    %eax,%ebx
    1982:	66 8b d1             	mov    %ecx,%edx
    1985:	66 0f af 44 32       	imul   0x32(%si),%eax
    198a:	66 0f af 4c 1a       	imul   0x1a(%si),%ecx
    198f:	66 03 c8             	add    %eax,%ecx
    1992:	66 f7 d9             	neg    %ecx
    1995:	66 c1 f9 0f          	sar    $0xf,%ecx
    1999:	79 05                	jns    0x19a0
    199b:	8b 4c 58             	mov    0x58(%si),%cx
    199e:	d1 f9                	sar    $1,%cx
    19a0:	89 4c 58             	mov    %cx,0x58(%si)
    19a3:	66 f7 db             	neg    %ebx
    19a6:	66 0f af 54 32       	imul   0x32(%si),%edx
    19ab:	66 0f af 5c 1a       	imul   0x1a(%si),%ebx
    19b0:	66 03 d3             	add    %ebx,%edx
    19b3:	b8 f0 ff             	mov    $0xfff0,%ax
    19b6:	79 03                	jns    0x19bb
    19b8:	b8 10 00             	mov    $0x10,%ax
    19bb:	89 45 3a             	mov    %ax,0x3a(%di)
    19be:	8b 44 52             	mov    0x52(%si),%ax
    19c1:	3d 00 03             	cmp    $0x300,%ax
    19c4:	7c 08                	jl     0x19ce
    19c6:	c7 44 52 00 03       	movw   $0x300,0x52(%si)
    19cb:	e9 c0 fd             	jmp    0x178e
    19ce:	3d 00 fd             	cmp    $0xfd00,%ax
    19d1:	0f 8d b9 fd          	jge    0x178e
    19d5:	c7 44 52 00 fd       	movw   $0xfd00,0x52(%si)
    19da:	e9 b1 fd             	jmp    0x178e
    19dd:	8b 5d 42             	mov    0x42(%di),%bx
    19e0:	c1 cb 07             	ror    $0x7,%bx
    19e3:	83 db 00             	sbb    $0x0,%bx
    19e6:	89 5d 42             	mov    %bx,0x42(%di)
    19e9:	81 e3 fc 0f          	and    $0xffc,%bx
    19ed:	66 0f bf 8f 36 00    	movswl 0x36(%bx),%ecx
    19f3:	66 0f bf 9f 38 00    	movswl 0x38(%bx),%ebx
    19f9:	66 8b c1             	mov    %ecx,%eax
    19fc:	66 0f af 06 ba 22    	imul   0x22ba,%eax
    1a02:	66 8b e8             	mov    %eax,%ebp
    1a05:	66 8b c1             	mov    %ecx,%eax
    1a08:	66 0f af 06 be 22    	imul   0x22be,%eax
    1a0e:	66 03 c5             	add    %ebp,%eax
    1a11:	66 c1 f8 10          	sar    $0x10,%eax
    1a15:	2b 06 ec 22          	sub    0x22ec,%ax
    1a19:	89 44 42             	mov    %ax,0x42(%si)
    1a1c:	66 8b c1             	mov    %ecx,%eax
    1a1f:	66 0f af 06 c6 22    	imul   0x22c6,%eax
    1a25:	66 8b e8             	mov    %eax,%ebp
    1a28:	66 8b c1             	mov    %ecx,%eax
    1a2b:	66 0f af 06 ca 22    	imul   0x22ca,%eax
    1a31:	66 03 c5             	add    %ebp,%eax
    1a34:	66 c1 f8 10          	sar    $0x10,%eax
    1a38:	2b 06 f0 22          	sub    0x22f0,%ax
    1a3c:	89 44 46             	mov    %ax,0x46(%si)
    1a3f:	66 8b c1             	mov    %ecx,%eax
    1a42:	66 0f af 06 d2 22    	imul   0x22d2,%eax
    1a48:	66 8b e8             	mov    %eax,%ebp
    1a4b:	66 8b c1             	mov    %ecx,%eax
    1a4e:	66 0f af 06 d6 22    	imul   0x22d6,%eax
    1a54:	66 03 c5             	add    %ebp,%eax
    1a57:	66 c1 f8 10          	sar    $0x10,%eax
    1a5b:	2b 06 f4 22          	sub    0x22f4,%ax
    1a5f:	89 44 4a             	mov    %ax,0x4a(%si)
    1a62:	a1 f6 22             	mov    0x22f6,%ax
    1a65:	8b 1e f8 22          	mov    0x22f8,%bx
    1a69:	89 44 4e             	mov    %ax,0x4e(%si)
    1a6c:	89 5c 50             	mov    %bx,0x50(%si)
    1a6f:	c7 44 52 00 00       	movw   $0x0,0x52(%si)
    1a74:	a1 fc 22             	mov    0x22fc,%ax
    1a77:	05 2c 01             	add    $0x12c,%ax
    1a7a:	89 44 54             	mov    %ax,0x54(%si)
    1a7d:	89 44 58             	mov    %ax,0x58(%si)
    1a80:	c7 45 38 08 00       	movw   $0x8,0x38(%di)
    1a85:	c3                   	ret
