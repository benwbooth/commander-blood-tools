; Commander Blood recovered routine assembly
; module: xdb_scrut
; artifact: output/_tmp_dat/scrut.xdb
; artifact_sha256: 8522e77aad6639cf9bb06f148048fed3c6bf1cfbe6459c9a0be7b987c3c3ac77
; overlay_offset: 0x001957
; byte_count: 120
; routine_bytes_sha256: 8be4c09bf2b8dd7ba844acf0172a131a0e932a77d01a3fe576059a6e37a8129a
; routine_entry: 0x001957
; group: callback_state_machine
; provenance: callback published by internal transition 0x1952
; direct_callees: 0x001802
; raw stop: 0x0019CF


output/_tmp_dat/scrut.xdb:     file format binary


Disassembly of section .data:

00001957 <.data+0x1957>:
    1957:	2e c7 06 90 16 e8 03 	movw   $0x3e8,%cs:0x1690
    195e:	83 84 3a 03 40       	addw   $0x40,0x33a(%si)
    1963:	83 84 98 03 50       	addw   $0x50,0x398(%si)
    1968:	83 84 32 03 04       	addw   $0x4,0x332(%si)
    196d:	83 ac 90 03 04       	subw   $0x4,0x390(%si)
    1972:	8b 44 40             	mov    0x40(%si),%ax
    1975:	3d f4 01             	cmp    $0x1f4,%ax
    1978:	7c 1b                	jl     0x1995
    197a:	b8 c8 00             	mov    $0xc8,%ax
    197d:	2b 44 54             	sub    0x54(%si),%ax
    1980:	c1 f8 04             	sar    $0x4,%ax
    1983:	01 44 54             	add    %ax,0x54(%si)
    1986:	64 39 3e 82 22       	cmp    %di,%fs:0x2282
    198b:	75 07                	jne    0x1994
    198d:	2e c7 06 8e 16 01 00 	movw   $0x1,%cs:0x168e
    1994:	c3                   	ret
    1995:	0b c0                	or     %ax,%ax
    1997:	78 05                	js     0x199e
    1999:	83 6c 4e 20          	subw   $0x20,0x4e(%si)
    199d:	c3                   	ret
    199e:	2e c7 06 90 16 00 00 	movw   $0x0,%cs:0x1690
    19a5:	8b 84 46 03          	mov    0x346(%si),%ax
    19a9:	8b 9c 4a 03          	mov    0x34a(%si),%bx
    19ad:	89 84 32 03          	mov    %ax,0x332(%si)
    19b1:	89 9c 3a 03          	mov    %bx,0x33a(%si)
    19b5:	8b 84 a4 03          	mov    0x3a4(%si),%ax
    19b9:	8b 9c a8 03          	mov    0x3a8(%si),%bx
    19bd:	89 84 90 03          	mov    %ax,0x390(%si)
    19c1:	89 9c 98 03          	mov    %bx,0x398(%si)
    19c5:	2e c7 06 8e 16 00 00 	movw   $0x0,%cs:0x168e
    19cc:	e9 33 fe             	jmp    0x1802
