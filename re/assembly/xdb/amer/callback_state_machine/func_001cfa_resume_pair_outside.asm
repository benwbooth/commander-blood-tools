; Commander Blood recovered routine assembly
; module: xdb_amer
; artifact: output/_tmp_dat/amer.xdb
; artifact_sha256: 6fddeb5cc7c62fe5e638900746decc299f1637ea11612c0c51aced367dd12b31
; overlay_offset: 0x001CFA
; byte_count: 127
; routine_bytes_sha256: 357e06679a3b1192aed9232d67f7409e4f0f7cc6d250b14e6ddfe709f76c1b0c
; routine_entry: 0x001CFA
; group: callback_state_machine
; provenance: near helper called by resume pair and final stages
; raw stop: 0x001D79


output/_tmp_dat/amer.xdb:     file format binary


Disassembly of section .data:

00001cfa <.data+0x1cfa>:
    1cfa:	8b 5c 50             	mov    0x50(%si),%bx
    1cfd:	66 0f bf 4d 4a       	movswl 0x4a(%di),%ecx
    1d02:	66 0f bf 44 4a       	movswl 0x4a(%si),%eax
    1d07:	66 2b c8             	sub    %eax,%ecx
    1d0a:	66 0f bf 55 42       	movswl 0x42(%di),%edx
    1d0f:	66 0f bf 44 42       	movswl 0x42(%si),%eax
    1d14:	66 2b d0             	sub    %eax,%edx
    1d17:	8b 45 46             	mov    0x46(%di),%ax
    1d1a:	2b 44 46             	sub    0x46(%si),%ax
    1d1d:	83 f9 9c             	cmp    $0xff9c,%cx
    1d20:	7c 1b                	jl     0x1d3d
    1d22:	83 f9 64             	cmp    $0x64,%cx
    1d25:	7f 16                	jg     0x1d3d
    1d27:	81 fa 38 ff          	cmp    $0xff38,%dx
    1d2b:	7c 10                	jl     0x1d3d
    1d2d:	81 fa c8 00          	cmp    $0xc8,%dx
    1d31:	7f 0a                	jg     0x1d3d
    1d33:	3d 38 ff             	cmp    $0xff38,%ax
    1d36:	7c 05                	jl     0x1d3d
    1d38:	3d c8 00             	cmp    $0xc8,%ax
    1d3b:	7c 3a                	jl     0x1d77
    1d3d:	c1 f8 03             	sar    $0x3,%ax
    1d40:	f7 d8                	neg    %ax
    1d42:	03 44 4e             	add    0x4e(%si),%ax
    1d45:	d1 f8                	sar    $1,%ax
    1d47:	89 44 4e             	mov    %ax,0x4e(%si)
    1d4a:	81 e3 fc 0f          	and    $0xffc,%bx
    1d4e:	66 0f bf 87 38 00    	movswl 0x38(%bx),%eax
    1d54:	66 0f af c8          	imul   %eax,%ecx
    1d58:	66 0f bf 87 36 00    	movswl 0x36(%bx),%eax
    1d5e:	66 0f af c2          	imul   %edx,%eax
    1d62:	66 2b c1             	sub    %ecx,%eax
    1d65:	ba 10 00             	mov    $0x10,%dx
    1d68:	79 06                	jns    0x1d70
    1d6a:	66 f7 d8             	neg    %eax
    1d6d:	ba e0 ff             	mov    $0xffe0,%dx
    1d70:	03 da                	add    %dx,%bx
    1d72:	89 5c 50             	mov    %bx,0x50(%si)
    1d75:	f8                   	clc
    1d76:	c3                   	ret
    1d77:	f9                   	stc
    1d78:	c3                   	ret
