; Commander Blood recovered routine assembly
; module: xdb_scrut
; artifact: output/_tmp_dat/scrut.xdb
; artifact_sha256: 8522e77aad6639cf9bb06f148048fed3c6bf1cfbe6459c9a0be7b987c3c3ac77
; overlay_offset: 0x0018D9
; byte_count: 121
; routine_bytes_sha256: a20721a934c4580920004f000e3e848c125c790acaae44a380dcd98b9092e51b
; routine_entry: 0x0018D9
; group: callback_state_machine
; provenance: near carry-return helper called by callbacks 0x1858 and 0x1868
; direct_callees: none
; raw stop: 0x001952


output/_tmp_dat/scrut.xdb:     file format binary


Disassembly of section .data:

000018d9 <.data+0x18d9>:
    18d9:	66 0f bf 5c 40       	movswl 0x40(%si),%ebx
    18de:	66 0f bf 44 38       	movswl 0x38(%si),%eax
    18e3:	66 0f af 44 32       	imul   0x32(%si),%eax
    18e8:	66 0f af 5c 1a       	imul   0x1a(%si),%ebx
    18ed:	66 2b c3             	sub    %ebx,%eax
    18f0:	66 d3 f8             	sar    %cl,%eax
    18f3:	15 00 00             	adc    $0x0,%ax
    18f6:	74 58                	je     0x1950
    18f8:	66 f7 d8             	neg    %eax
    18fb:	3d 20 00             	cmp    $0x20,%ax
    18fe:	7c 03                	jl     0x1903
    1900:	b8 20 00             	mov    $0x20,%ax
    1903:	3d e0 ff             	cmp    $0xffe0,%ax
    1906:	7f 03                	jg     0x190b
    1908:	b8 e0 ff             	mov    $0xffe0,%ax
    190b:	03 44 52             	add    0x52(%si),%ax
    190e:	8b 5c 5a             	mov    0x5a(%si),%bx
    1911:	33 d8                	xor    %ax,%bx
    1913:	79 05                	jns    0x191a
    1915:	d1 f8                	sar    $1,%ax
    1917:	89 44 5a             	mov    %ax,0x5a(%si)
    191a:	3d 00 03             	cmp    $0x300,%ax
    191d:	7c 03                	jl     0x1922
    191f:	b8 00 03             	mov    $0x300,%ax
    1922:	3d 00 fd             	cmp    $0xfd00,%ax
    1925:	7d 03                	jge    0x192a
    1927:	b8 00 fd             	mov    $0xfd00,%ax
    192a:	89 44 52             	mov    %ax,0x52(%si)
    192d:	8b d0                	mov    %ax,%dx
    192f:	c1 fa 05             	sar    $0x5,%dx
    1932:	11 54 50             	adc    %dx,0x50(%si)
    1935:	56                   	push   %si
    1936:	b9 05 00             	mov    $0x5,%cx
    1939:	f7 d8                	neg    %ax
    193b:	8b d8                	mov    %ax,%bx
    193d:	d1 f8                	sar    $1,%ax
    193f:	c1 fb 02             	sar    $0x2,%bx
    1942:	83 c6 5e             	add    $0x5e,%si
    1945:	89 44 50             	mov    %ax,0x50(%si)
    1948:	89 5c 52             	mov    %bx,0x52(%si)
    194b:	e2 f5                	loop   0x1942
    194d:	5e                   	pop    %si
    194e:	f9                   	stc
    194f:	c3                   	ret
    1950:	f8                   	clc
    1951:	c3                   	ret
