; Commander Blood recovered routine assembly
; module: xdb_croolis
; artifact: output/_tmp_dat/croolis.xdb
; artifact_sha256: 13eba7c4e4f38662c44c849bd07be293c7583d2b2dc9ae273d7fe94746048c31
; overlay_offset: 0x001C46
; byte_count: 129
; routine_bytes_sha256: 4040f1efbaa817b8e622ac90cfcd07d6ebc06d4aaa1d4cc58ae812f569f39149
; routine_entry: 0x001C46
; group: callback_state_machine
; provenance: near helper called by resume pair and final stages
; raw stop: 0x001CC7


output/_tmp_dat/croolis.xdb:     file format binary


Disassembly of section .data:

00001c46 <.data+0x1c46>:
    1c46:	8b 5c 50             	mov    0x50(%si),%bx
    1c49:	66 0f bf 4d 4a       	movswl 0x4a(%di),%ecx
    1c4e:	66 0f bf 44 4a       	movswl 0x4a(%si),%eax
    1c53:	66 2b c8             	sub    %eax,%ecx
    1c56:	66 0f bf 55 42       	movswl 0x42(%di),%edx
    1c5b:	66 0f bf 44 42       	movswl 0x42(%si),%eax
    1c60:	66 2b d0             	sub    %eax,%edx
    1c63:	8b 45 46             	mov    0x46(%di),%ax
    1c66:	2b 44 46             	sub    0x46(%si),%ax
    1c69:	81 f9 38 ff          	cmp    $0xff38,%cx
    1c6d:	7c 1c                	jl     0x1c8b
    1c6f:	81 f9 c8 00          	cmp    $0xc8,%cx
    1c73:	7f 16                	jg     0x1c8b
    1c75:	81 fa 38 ff          	cmp    $0xff38,%dx
    1c79:	7c 10                	jl     0x1c8b
    1c7b:	81 fa c8 00          	cmp    $0xc8,%dx
    1c7f:	7f 0a                	jg     0x1c8b
    1c81:	3d 38 ff             	cmp    $0xff38,%ax
    1c84:	7c 05                	jl     0x1c8b
    1c86:	3d c8 00             	cmp    $0xc8,%ax
    1c89:	7c 3a                	jl     0x1cc5
    1c8b:	c1 f8 03             	sar    $0x3,%ax
    1c8e:	f7 d8                	neg    %ax
    1c90:	03 44 4e             	add    0x4e(%si),%ax
    1c93:	d1 f8                	sar    $1,%ax
    1c95:	89 44 4e             	mov    %ax,0x4e(%si)
    1c98:	81 e3 fc 0f          	and    $0xffc,%bx
    1c9c:	66 0f bf 87 38 00    	movswl 0x38(%bx),%eax
    1ca2:	66 0f af c8          	imul   %eax,%ecx
    1ca6:	66 0f bf 87 36 00    	movswl 0x36(%bx),%eax
    1cac:	66 0f af c2          	imul   %edx,%eax
    1cb0:	66 2b c1             	sub    %ecx,%eax
    1cb3:	ba 10 00             	mov    $0x10,%dx
    1cb6:	79 06                	jns    0x1cbe
    1cb8:	66 f7 d8             	neg    %eax
    1cbb:	ba e0 ff             	mov    $0xffe0,%dx
    1cbe:	03 da                	add    %dx,%bx
    1cc0:	89 5c 50             	mov    %bx,0x50(%si)
    1cc3:	f8                   	clc
    1cc4:	c3                   	ret
    1cc5:	f9                   	stc
    1cc6:	c3                   	ret
