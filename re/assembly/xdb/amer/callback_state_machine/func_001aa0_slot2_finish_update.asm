; Commander Blood raw routine disassembly
; module: xdb_amer
; artifact: output/_tmp_dat/amer.xdb
; routine_entry: 0x001AA0
; group: callback_state_machine
; provenance: callback installed by slot-2 steering
; raw stop: 0x001B1A

/home/ben/src/commander-blood-tools/output/_tmp_dat/amer.xdb:     file format binary


Disassembly of section .data:

00001aa0 <.data+0x1aa0>:
    1aa0:	ff 4c 56             	dec    WORD PTR [si+0x56]
    1aa3:	78 86                	js     0x1a2b
    1aa5:	8b 44 3c             	mov    ax,WORD PTR [si+0x3c]
    1aa8:	03 44 4e             	add    ax,WORD PTR [si+0x4e]
    1aab:	d1 f8                	sar    ax,1
    1aad:	3d 00 03             	cmp    ax,0x300
    1ab0:	7c 03                	jl     0x1ab5
    1ab2:	b8 00 03             	mov    ax,0x300
    1ab5:	3d 00 fd             	cmp    ax,0xfd00
    1ab8:	7f 03                	jg     0x1abd
    1aba:	b8 00 fd             	mov    ax,0xfd00
    1abd:	89 44 4e             	mov    WORD PTR [si+0x4e],ax
    1ac0:	66 0f bf 44 40       	movsx  eax,WORD PTR [si+0x40]
    1ac5:	66 0f bf 5c 38       	movsx  ebx,WORD PTR [si+0x38]
    1aca:	66 0f b7 16 fc 22    	movzx  edx,WORD PTR ds:0x22fc
    1ad0:	66 2b c2             	sub    eax,edx
    1ad3:	66 2d e8 03 00 00    	sub    eax,0x3e8
    1ad9:	78 1a                	js     0x1af5
    1adb:	3d e8 03             	cmp    ax,0x3e8
    1ade:	7f 15                	jg     0x1af5
    1ae0:	81 fb 18 fc          	cmp    bx,0xfc18
    1ae4:	7c 0f                	jl     0x1af5
    1ae6:	81 fb e8 03          	cmp    bx,0x3e8
    1aea:	7f 09                	jg     0x1af5
    1aec:	c7 44 0e 3e 19       	mov    WORD PTR [si+0xe],0x193e
    1af1:	d1 7c 54             	sar    WORD PTR [si+0x54],1
    1af4:	c3                   	ret
    1af5:	83 44 54 0a          	add    WORD PTR [si+0x54],0xa
    1af9:	c7 44 58 f4 01       	mov    WORD PTR [si+0x58],0x1f4
    1afe:	66 f7 d8             	neg    eax
    1b01:	66 0f af 5c 32       	imul   ebx,DWORD PTR [si+0x32]
    1b06:	66 0f af 44 1a       	imul   eax,DWORD PTR [si+0x1a]
    1b0b:	66 03 c3             	add    eax,ebx
    1b0e:	b8 e0 ff             	mov    ax,0xffe0
    1b11:	79 03                	jns    0x1b16
    1b13:	b8 20 00             	mov    ax,0x20
    1b16:	01 44 50             	add    WORD PTR [si+0x50],ax
    1b19:	c3                   	ret
