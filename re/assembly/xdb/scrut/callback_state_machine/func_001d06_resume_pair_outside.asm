; Commander Blood recovered routine assembly
; module: xdb_scrut
; artifact: output/_tmp_dat/scrut.xdb
; artifact_sha256: 8522e77aad6639cf9bb06f148048fed3c6bf1cfbe6459c9a0be7b987c3c3ac77
; overlay_offset: 0x001D06
; byte_count: 129
; routine_bytes_sha256: 4040f1efbaa817b8e622ac90cfcd07d6ebc06d4aaa1d4cc58ae812f569f39149
; routine_entry: 0x001D06
; group: callback_state_machine
; provenance: near helper called by resume pair and final stages
; raw stop: 0x001D87


output/_tmp_dat/scrut.xdb:     file format binary


Disassembly of section .data:

00001d06 <.data+0x1d06>:
    1d06:	8b 5c 50             	mov    0x50(%si),%bx
    1d09:	66 0f bf 4d 4a       	movswl 0x4a(%di),%ecx
    1d0e:	66 0f bf 44 4a       	movswl 0x4a(%si),%eax
    1d13:	66 2b c8             	sub    %eax,%ecx
    1d16:	66 0f bf 55 42       	movswl 0x42(%di),%edx
    1d1b:	66 0f bf 44 42       	movswl 0x42(%si),%eax
    1d20:	66 2b d0             	sub    %eax,%edx
    1d23:	8b 45 46             	mov    0x46(%di),%ax
    1d26:	2b 44 46             	sub    0x46(%si),%ax
    1d29:	81 f9 38 ff          	cmp    $0xff38,%cx
    1d2d:	7c 1c                	jl     0x1d4b
    1d2f:	81 f9 c8 00          	cmp    $0xc8,%cx
    1d33:	7f 16                	jg     0x1d4b
    1d35:	81 fa 38 ff          	cmp    $0xff38,%dx
    1d39:	7c 10                	jl     0x1d4b
    1d3b:	81 fa c8 00          	cmp    $0xc8,%dx
    1d3f:	7f 0a                	jg     0x1d4b
    1d41:	3d 38 ff             	cmp    $0xff38,%ax
    1d44:	7c 05                	jl     0x1d4b
    1d46:	3d c8 00             	cmp    $0xc8,%ax
    1d49:	7c 3a                	jl     0x1d85
    1d4b:	c1 f8 03             	sar    $0x3,%ax
    1d4e:	f7 d8                	neg    %ax
    1d50:	03 44 4e             	add    0x4e(%si),%ax
    1d53:	d1 f8                	sar    $1,%ax
    1d55:	89 44 4e             	mov    %ax,0x4e(%si)
    1d58:	81 e3 fc 0f          	and    $0xffc,%bx
    1d5c:	66 0f bf 87 38 00    	movswl 0x38(%bx),%eax
    1d62:	66 0f af c8          	imul   %eax,%ecx
    1d66:	66 0f bf 87 36 00    	movswl 0x36(%bx),%eax
    1d6c:	66 0f af c2          	imul   %edx,%eax
    1d70:	66 2b c1             	sub    %ecx,%eax
    1d73:	ba 10 00             	mov    $0x10,%dx
    1d76:	79 06                	jns    0x1d7e
    1d78:	66 f7 d8             	neg    %eax
    1d7b:	ba e0 ff             	mov    $0xffe0,%dx
    1d7e:	03 da                	add    %dx,%bx
    1d80:	89 5c 50             	mov    %bx,0x50(%si)
    1d83:	f8                   	clc
    1d84:	c3                   	ret
    1d85:	f9                   	stc
    1d86:	c3                   	ret
