; Commander Blood recovered routine assembly
; module: xdb_amer
; artifact: output/_tmp_dat/amer.xdb
; artifact_sha256: 6fddeb5cc7c62fe5e638900746decc299f1637ea11612c0c51aced367dd12b31
; overlay_offset: 0x001AA0
; byte_count: 122
; routine_bytes_sha256: d5a077cf66bdbc97d7b74b3a14387a786c324c2768881def25b6fd4c910de6dc
; routine_entry: 0x001AA0
; group: callback_state_machine
; provenance: callback installed by the AMER slot-2 steering callback at 0x1A95
; raw stop: 0x001B1A


output/_tmp_dat/amer.xdb:     file format binary


Disassembly of section .data:

00001aa0 <.data+0x1aa0>:
    1aa0:	ff 4c 56             	decw   0x56(%si)
    1aa3:	78 86                	js     0x1a2b
    1aa5:	8b 44 3c             	mov    0x3c(%si),%ax
    1aa8:	03 44 4e             	add    0x4e(%si),%ax
    1aab:	d1 f8                	sar    $1,%ax
    1aad:	3d 00 03             	cmp    $0x300,%ax
    1ab0:	7c 03                	jl     0x1ab5
    1ab2:	b8 00 03             	mov    $0x300,%ax
    1ab5:	3d 00 fd             	cmp    $0xfd00,%ax
    1ab8:	7f 03                	jg     0x1abd
    1aba:	b8 00 fd             	mov    $0xfd00,%ax
    1abd:	89 44 4e             	mov    %ax,0x4e(%si)
    1ac0:	66 0f bf 44 40       	movswl 0x40(%si),%eax
    1ac5:	66 0f bf 5c 38       	movswl 0x38(%si),%ebx
    1aca:	66 0f b7 16 fc 22    	movzwl 0x22fc,%edx
    1ad0:	66 2b c2             	sub    %edx,%eax
    1ad3:	66 2d e8 03 00 00    	sub    $0x3e8,%eax
    1ad9:	78 1a                	js     0x1af5
    1adb:	3d e8 03             	cmp    $0x3e8,%ax
    1ade:	7f 15                	jg     0x1af5
    1ae0:	81 fb 18 fc          	cmp    $0xfc18,%bx
    1ae4:	7c 0f                	jl     0x1af5
    1ae6:	81 fb e8 03          	cmp    $0x3e8,%bx
    1aea:	7f 09                	jg     0x1af5
    1aec:	c7 44 0e 3e 19       	movw   $0x193e,0xe(%si)
    1af1:	d1 7c 54             	sarw   $1,0x54(%si)
    1af4:	c3                   	ret
    1af5:	83 44 54 0a          	addw   $0xa,0x54(%si)
    1af9:	c7 44 58 f4 01       	movw   $0x1f4,0x58(%si)
    1afe:	66 f7 d8             	neg    %eax
    1b01:	66 0f af 5c 32       	imul   0x32(%si),%ebx
    1b06:	66 0f af 44 1a       	imul   0x1a(%si),%eax
    1b0b:	66 03 c3             	add    %ebx,%eax
    1b0e:	b8 e0 ff             	mov    $0xffe0,%ax
    1b11:	79 03                	jns    0x1b16
    1b13:	b8 20 00             	mov    $0x20,%ax
    1b16:	01 44 50             	add    %ax,0x50(%si)
    1b19:	c3                   	ret
