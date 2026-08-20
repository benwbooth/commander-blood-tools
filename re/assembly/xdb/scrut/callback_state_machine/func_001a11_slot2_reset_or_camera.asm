; Commander Blood recovered routine assembly
; module: xdb_scrut
; artifact: output/_tmp_dat/scrut.xdb
; artifact_sha256: 8522e77aad6639cf9bb06f148048fed3c6bf1cfbe6459c9a0be7b987c3c3ac77
; overlay_offset: 0x001A11
; byte_count: 298
; routine_bytes_sha256: dd1811c09b707e1e5c4fa0403468434b436f9045a16bbbf655cce3e785fae174
; routine_entry: 0x001A11
; group: callback_state_machine
; provenance: shared reset and camera tail reached by callbacks 0x171B and 0x181B
; direct_callees: 0x001781
; raw stop: 0x001B3B


output/_tmp_dat/scrut.xdb:     file format binary


Disassembly of section .data:

00001a11 <.data+0x1a11>:
    1a11:	66 ba d0 07 00 00    	mov    $0x7d0,%edx
    1a17:	66 0f bf 44 40       	movswl 0x40(%si),%eax
    1a1c:	66 3d 0c fe ff ff    	cmp    $0xfffffe0c,%eax
    1a22:	7c 6e                	jl     0x1a92
    1a24:	66 2b c2             	sub    %edx,%eax
    1a27:	66 0f bf 4c 38       	movswl 0x38(%si),%ecx
    1a2c:	66 2b 4d 3a          	sub    0x3a(%di),%ecx
    1a30:	66 8b d8             	mov    %eax,%ebx
    1a33:	66 8b d1             	mov    %ecx,%edx
    1a36:	66 0f af 44 32       	imul   0x32(%si),%eax
    1a3b:	66 0f af 4c 1a       	imul   0x1a(%si),%ecx
    1a40:	66 03 c8             	add    %eax,%ecx
    1a43:	66 f7 d9             	neg    %ecx
    1a46:	66 c1 f9 0f          	sar    $0xf,%ecx
    1a4a:	79 09                	jns    0x1a55
    1a4c:	8b 4c 58             	mov    0x58(%si),%cx
    1a4f:	c1 f9 02             	sar    $0x2,%cx
    1a52:	83 c1 10             	add    $0x10,%cx
    1a55:	89 4c 58             	mov    %cx,0x58(%si)
    1a58:	66 f7 db             	neg    %ebx
    1a5b:	66 0f af 54 32       	imul   0x32(%si),%edx
    1a60:	66 0f af 5c 1a       	imul   0x1a(%si),%ebx
    1a65:	66 03 d3             	add    %ebx,%edx
    1a68:	b8 f0 ff             	mov    $0xfff0,%ax
    1a6b:	79 03                	jns    0x1a70
    1a6d:	b8 10 00             	mov    $0x10,%ax
    1a70:	89 44 5a             	mov    %ax,0x5a(%si)
    1a73:	8b 44 52             	mov    0x52(%si),%ax
    1a76:	3d 00 03             	cmp    $0x300,%ax
    1a79:	7c 08                	jl     0x1a83
    1a7b:	c7 44 52 00 03       	movw   $0x300,0x52(%si)
    1a80:	e9 fe fc             	jmp    0x1781
    1a83:	3d 00 fd             	cmp    $0xfd00,%ax
    1a86:	0f 8d f7 fc          	jge    0x1781
    1a8a:	c7 44 52 00 fd       	movw   $0xfd00,0x52(%si)
    1a8f:	e9 ef fc             	jmp    0x1781
    1a92:	8b 5d 42             	mov    0x42(%di),%bx
    1a95:	c1 cb 07             	ror    $0x7,%bx
    1a98:	83 db 00             	sbb    $0x0,%bx
    1a9b:	89 5d 42             	mov    %bx,0x42(%di)
    1a9e:	81 e3 fc 0f          	and    $0xffc,%bx
    1aa2:	66 0f bf 8f 36 00    	movswl 0x36(%bx),%ecx
    1aa8:	66 0f bf 9f 38 00    	movswl 0x38(%bx),%ebx
    1aae:	66 8b c1             	mov    %ecx,%eax
    1ab1:	66 0f af 06 ba 22    	imul   0x22ba,%eax
    1ab7:	66 8b e8             	mov    %eax,%ebp
    1aba:	66 8b c1             	mov    %ecx,%eax
    1abd:	66 0f af 06 be 22    	imul   0x22be,%eax
    1ac3:	66 03 c5             	add    %ebp,%eax
    1ac6:	66 c1 f8 10          	sar    $0x10,%eax
    1aca:	2b 06 ec 22          	sub    0x22ec,%ax
    1ace:	89 44 42             	mov    %ax,0x42(%si)
    1ad1:	66 8b c1             	mov    %ecx,%eax
    1ad4:	66 0f af 06 c6 22    	imul   0x22c6,%eax
    1ada:	66 8b e8             	mov    %eax,%ebp
    1add:	66 8b c1             	mov    %ecx,%eax
    1ae0:	66 0f af 06 ca 22    	imul   0x22ca,%eax
    1ae6:	66 03 c5             	add    %ebp,%eax
    1ae9:	66 c1 f8 10          	sar    $0x10,%eax
    1aed:	2b 06 f0 22          	sub    0x22f0,%ax
    1af1:	89 44 46             	mov    %ax,0x46(%si)
    1af4:	66 8b c1             	mov    %ecx,%eax
    1af7:	66 0f af 06 d2 22    	imul   0x22d2,%eax
    1afd:	66 8b e8             	mov    %eax,%ebp
    1b00:	66 8b c1             	mov    %ecx,%eax
    1b03:	66 0f af 06 d6 22    	imul   0x22d6,%eax
    1b09:	66 03 c5             	add    %ebp,%eax
    1b0c:	66 c1 f8 10          	sar    $0x10,%eax
    1b10:	2b 06 f4 22          	sub    0x22f4,%ax
    1b14:	89 44 4a             	mov    %ax,0x4a(%si)
    1b17:	a1 f6 22             	mov    0x22f6,%ax
    1b1a:	8b 1e f8 22          	mov    0x22f8,%bx
    1b1e:	89 44 4e             	mov    %ax,0x4e(%si)
    1b21:	89 5c 50             	mov    %bx,0x50(%si)
    1b24:	c7 44 52 00 00       	movw   $0x0,0x52(%si)
    1b29:	a1 fc 22             	mov    0x22fc,%ax
    1b2c:	05 2c 01             	add    $0x12c,%ax
    1b2f:	89 44 54             	mov    %ax,0x54(%si)
    1b32:	89 44 58             	mov    %ax,0x58(%si)
    1b35:	c7 45 38 08 00       	movw   $0x8,0x38(%di)
    1b3a:	c3                   	ret
