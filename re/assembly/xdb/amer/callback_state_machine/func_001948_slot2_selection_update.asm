; Commander Blood recovered routine assembly
; module: xdb_amer
; artifact: output/_tmp_dat/amer.xdb
; artifact_sha256: 6fddeb5cc7c62fe5e638900746decc299f1637ea11612c0c51aced367dd12b31
; overlay_offset: 0x001948
; byte_count: 131
; routine_bytes_sha256: 607c4b8b7c9d038246d66392924856b59b01b12ec35a5ae5f5dac4016066a9c7
; routine_entry: 0x001948
; group: callback_state_machine
; provenance: callback published by selection wait 0x193E
; direct_callees: 0x001688, 0x00171D, 0x001A2B
; raw stop: 0x0019CB


output/_tmp_dat/amer.xdb:     file format binary


Disassembly of section .data:

00001948 <.data+0x1948>:
    1948:	2e f7 06 2f 0b 01 00 	testw  $0x1,%cs:0xb2f
    194f:	0f 84 35 fd          	je     0x1688
    1953:	66 0f bf 44 40       	movswl 0x40(%si),%eax
    1958:	66 0f bf 5c 38       	movswl 0x38(%si),%ebx
    195d:	3d b8 0b             	cmp    $0xbb8,%ax
    1960:	0f 87 c7 00          	ja     0x1a2b
    1964:	81 fb e8 03          	cmp    $0x3e8,%bx
    1968:	0f 8f bf 00          	jg     0x1a2b
    196c:	81 fb 18 fc          	cmp    $0xfc18,%bx
    1970:	0f 8c b7 00          	jl     0x1a2b
    1974:	3d 20 03             	cmp    $0x320,%ax
    1977:	7c 47                	jl     0x19c0
    1979:	66 f7 d8             	neg    %eax
    197c:	66 0f af 5c 32       	imul   0x32(%si),%ebx
    1981:	66 0f af 44 1a       	imul   0x1a(%si),%eax
    1986:	66 03 c3             	add    %ebx,%eax
    1989:	b8 c0 ff             	mov    $0xffc0,%ax
    198c:	79 03                	jns    0x1991
    198e:	b8 40 00             	mov    $0x40,%ax
    1991:	c7 45 3a 00 00       	movw   $0x0,0x3a(%di)
    1996:	01 44 50             	add    %ax,0x50(%si)
    1999:	c7 44 52 00 00       	movw   $0x0,0x52(%si)
    199e:	8b 44 46             	mov    0x46(%si),%ax
    19a1:	03 06 f0 22          	add    0x22f0,%ax
    19a5:	03 44 4e             	add    0x4e(%si),%ax
    19a8:	d1 f8                	sar    $1,%ax
    19aa:	3d 00 03             	cmp    $0x300,%ax
    19ad:	7c 03                	jl     0x19b2
    19af:	b8 00 03             	mov    $0x300,%ax
    19b2:	3d 00 fd             	cmp    $0xfd00,%ax
    19b5:	7f 03                	jg     0x19ba
    19b7:	b8 00 fd             	mov    $0xfd00,%ax
    19ba:	89 44 4e             	mov    %ax,0x4e(%si)
    19bd:	e9 5d fd             	jmp    0x171d
    19c0:	c7 44 58 50 00       	movw   $0x50,0x58(%si)
    19c5:	c7 44 0e cb 19       	movw   $0x19cb,0xe(%si)
    19ca:	c3                   	ret
