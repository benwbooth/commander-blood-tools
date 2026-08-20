; Commander Blood recovered routine assembly
; module: xdb_amer
; artifact: output/_tmp_dat/amer.xdb
; artifact_sha256: 6fddeb5cc7c62fe5e638900746decc299f1637ea11612c0c51aced367dd12b31
; overlay_offset: 0x0019CB
; byte_count: 96
; routine_bytes_sha256: f66d3ed73fcbbcac9a3bd53a3f6299b5684cc80aa4985b09321150334c4ebf01
; routine_entry: 0x0019CB
; group: callback_state_machine
; provenance: callback published by selection callback 0x1948
; direct_callees: 0x00171D, 0x00193E, 0x001A2B
; raw stop: 0x001A2B


output/_tmp_dat/amer.xdb:     file format binary


Disassembly of section .data:

000019cb <.data+0x19cb>:
    19cb:	66 0f bf 44 40       	movswl 0x40(%si),%eax
    19d0:	66 0f bf 5c 38       	movswl 0x38(%si),%ebx
    19d5:	3d e8 03             	cmp    $0x3e8,%ax
    19d8:	0f 87 62 ff          	ja     0x193e
    19dc:	81 fb f4 01          	cmp    $0x1f4,%bx
    19e0:	7f 49                	jg     0x1a2b
    19e2:	81 fb 0c fe          	cmp    $0xfe0c,%bx
    19e6:	7c 43                	jl     0x1a2b
    19e8:	66 2d c8 00 00 00    	sub    $0xc8,%eax
    19ee:	66 f7 d8             	neg    %eax
    19f1:	66 0f af 5c 32       	imul   0x32(%si),%ebx
    19f6:	66 0f af 44 1a       	imul   0x1a(%si),%eax
    19fb:	66 03 c3             	add    %ebx,%eax
    19fe:	b8 d0 ff             	mov    $0xffd0,%ax
    1a01:	79 03                	jns    0x1a06
    1a03:	b8 30 00             	mov    $0x30,%ax
    1a06:	89 45 3a             	mov    %ax,0x3a(%di)
    1a09:	8b 44 46             	mov    0x46(%si),%ax
    1a0c:	03 06 f0 22          	add    0x22f0,%ax
    1a10:	03 44 4e             	add    0x4e(%si),%ax
    1a13:	d1 f8                	sar    $1,%ax
    1a15:	3d 00 03             	cmp    $0x300,%ax
    1a18:	7c 03                	jl     0x1a1d
    1a1a:	b8 00 03             	mov    $0x300,%ax
    1a1d:	3d 00 fd             	cmp    $0xfd00,%ax
    1a20:	7f 03                	jg     0x1a25
    1a22:	b8 00 fd             	mov    $0xfd00,%ax
    1a25:	89 44 4e             	mov    %ax,0x4e(%si)
    1a28:	e9 f2 fc             	jmp    0x171d
