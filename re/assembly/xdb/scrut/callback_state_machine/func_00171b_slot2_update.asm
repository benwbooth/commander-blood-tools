
/home/ben/src/commander-blood-tools/output/_tmp_dat/scrut.xdb:     file format binary


Disassembly of section .data:

0000171b <.data+0x171b>:
    171b:	2e f7 06 70 0b 03 00 	testw  $0x3,%cs:0xb70
    1722:	0f 85 dc 00          	jne    0x1802
    1726:	ff 4d 38             	decw   0x38(%di)
    1729:	79 56                	jns    0x1781
    172b:	8b 44 40             	mov    0x40(%si),%ax
    172e:	8b 5c 38             	mov    0x38(%si),%bx
    1731:	05 f4 01             	add    $0x1f4,%ax
    1734:	3d b8 0b             	cmp    $0xbb8,%ax
    1737:	0f 87 d6 02          	ja     0x1a11
    173b:	81 c3 e8 03          	add    $0x3e8,%bx
    173f:	81 fb d0 07          	cmp    $0x7d0,%bx
    1743:	0f 87 ca 02          	ja     0x1a11
    1747:	8b 6d 42             	mov    0x42(%di),%bp
    174a:	c1 cd 03             	ror    $0x3,%bp
    174d:	83 dd 00             	sbb    $0x0,%bp
    1750:	8b c5                	mov    %bp,%ax
    1752:	25 ff 07             	and    $0x7ff,%ax
    1755:	2d ff 03             	sub    $0x3ff,%ax
    1758:	8b d0                	mov    %ax,%dx
    175a:	0b c0                	or     %ax,%ax
    175c:	78 02                	js     0x1760
    175e:	f7 da                	neg    %dx
    1760:	81 c2 00 04          	add    $0x400,%dx
    1764:	8b ca                	mov    %dx,%cx
    1766:	c1 e9 03             	shr    $0x3,%cx
    1769:	83 c1 20             	add    $0x20,%cx
    176c:	c1 ea 04             	shr    $0x4,%dx
    176f:	89 4d 38             	mov    %cx,0x38(%di)
    1772:	89 54 58             	mov    %dx,0x58(%si)
    1775:	2b 44 52             	sub    0x52(%si),%ax
    1778:	99                   	cwtd
    1779:	f7 f9                	idiv   %cx
    177b:	89 6d 42             	mov    %bp,0x42(%di)
    177e:	89 44 5a             	mov    %ax,0x5a(%si)
    1781:	39 3e 82 22          	cmp    %di,0x2282
    1785:	74 59                	je     0x17e0
    1787:	8b 44 3c             	mov    0x3c(%si),%ax
    178a:	03 45 3a             	add    0x3a(%di),%ax
    178d:	d1 f8                	sar    $1,%ax
    178f:	2b 44 4e             	sub    0x4e(%si),%ax
    1792:	c1 f8 03             	sar    $0x3,%ax
    1795:	03 44 4e             	add    0x4e(%si),%ax
    1798:	3d 00 03             	cmp    $0x300,%ax
    179b:	7c 03                	jl     0x17a0
    179d:	b8 00 03             	mov    $0x300,%ax
    17a0:	3d 00 fd             	cmp    $0xfd00,%ax
    17a3:	7f 03                	jg     0x17a8
    17a5:	b8 00 fd             	mov    $0xfd00,%ax
    17a8:	89 44 4e             	mov    %ax,0x4e(%si)
    17ab:	8b 44 58             	mov    0x58(%si),%ax
    17ae:	2b 44 54             	sub    0x54(%si),%ax
    17b1:	c1 f8 03             	sar    $0x3,%ax
    17b4:	01 44 54             	add    %ax,0x54(%si)
    17b7:	8b 44 5a             	mov    0x5a(%si),%ax
    17ba:	03 44 52             	add    0x52(%si),%ax
    17bd:	89 44 52             	mov    %ax,0x52(%si)
    17c0:	8b d0                	mov    %ax,%dx
    17c2:	c1 fa 05             	sar    $0x5,%dx
    17c5:	11 54 50             	adc    %dx,0x50(%si)
    17c8:	b9 05 00             	mov    $0x5,%cx
    17cb:	f7 d8                	neg    %ax
    17cd:	8b d8                	mov    %ax,%bx
    17cf:	d1 f8                	sar    $1,%ax
    17d1:	c1 fb 02             	sar    $0x2,%bx
    17d4:	83 c6 5e             	add    $0x5e,%si
    17d7:	89 44 50             	mov    %ax,0x50(%si)
    17da:	89 5c 52             	mov    %bx,0x52(%si)
    17dd:	e2 f5                	loop   0x17d4
    17df:	c3                   	ret
    17e0:	c3                   	ret
    17e1:	c7 44 0e e6 17       	movw   $0x17e6,0xe(%si)
    17e6:	8b 45 38             	mov    0x38(%di),%ax
    17e9:	2d 04 00             	sub    $0x4,%ax
    17ec:	7c 05                	jl     0x17f3
    17ee:	89 45 38             	mov    %ax,0x38(%di)
    17f1:	eb 94                	jmp    0x1787
    17f3:	c7 45 38 00 00       	movw   $0x0,0x38(%di)
    17f8:	2e c7 06 8e 16 00 00 	movw   $0x0,%cs:0x168e
    17ff:	e9 0f ff             	jmp    0x1711
    1802:	2e c7 06 90 16 ff ff 	movw   $0xffff,%cs:0x1690
    1809:	2e c7 06 8e 16 00 00 	movw   $0x0,%cs:0x168e
    1810:	c7 44 0e 1b 18       	movw   $0x181b,0xe(%si)
    1815:	8b 44 36             	mov    0x36(%si),%ax
    1818:	89 44 56             	mov    %ax,0x56(%si)
    181b:	2e f7 06 70 0b 03 00 	testw  $0x3,%cs:0xb70
    1822:	0f 84 a9 01          	je     0x19cf
    1826:	8b 44 40             	mov    0x40(%si),%ax
    1829:	3d bc 02             	cmp    $0x2bc,%ax
    182c:	0f 8c e1 01          	jl     0x1a11
    1830:	8b 5c 38             	mov    0x38(%si),%bx
    1833:	81 fb f4 01          	cmp    $0x1f4,%bx
    1837:	0f 8f d6 01          	jg     0x1a11
    183b:	81 fb 0c fe          	cmp    $0xfe0c,%bx
    183f:	0f 8f ce 01          	jg     0x1a11
    1843:	c7 44 0e 58 18       	movw   $0x1858,0xe(%si)
    1848:	c7 44 56 c8 00       	movw   $0xc8,0x56(%si)
    184d:	8b 44 52             	mov    0x52(%si),%ax
    1850:	89 44 5a             	mov    %ax,0x5a(%si)
    1853:	c7 44 58 00 00       	movw   $0x0,0x58(%si)
    1858:	d1 7c 54             	sarw   $1,0x54(%si)
    185b:	74 06                	je     0x1863
    185d:	b1 14                	mov    $0x14,%cl
    185f:	e8 77 00             	call   0x18d9
    1862:	c3                   	ret
    1863:	c7 44 0e 68 18       	movw   $0x1868,0xe(%si)
    1868:	2e f7 06 70 0b 03 00 	testw  $0x3,%cs:0xb70
    186f:	0f 84 5c 01          	je     0x19cf
    1873:	d1 7c 4e             	sarw   $1,0x4e(%si)
    1876:	66 a1 d2 22          	mov    0x22d2,%eax
    187a:	66 c1 f8 03          	sar    $0x3,%eax
    187e:	66 8b 0e da 22       	mov    0x22da,%ecx
    1883:	66 c1 f9 03          	sar    $0x3,%ecx
    1887:	8b 55 3a             	mov    0x3a(%di),%dx
    188a:	2b 06 ec 22          	sub    0x22ec,%ax
    188e:	8b 1e f0 22          	mov    0x22f0,%bx
    1892:	2b 0e f4 22          	sub    0x22f4,%cx
    1896:	03 c2                	add    %dx,%ax
    1898:	03 ca                	add    %dx,%cx
    189a:	2b 44 42             	sub    0x42(%si),%ax
    189d:	03 5c 46             	add    0x46(%si),%bx
    18a0:	2b 4c 4a             	sub    0x4a(%si),%cx
    18a3:	c1 f8 04             	sar    $0x4,%ax
    18a6:	01 44 42             	add    %ax,0x42(%si)
    18a9:	f7 db                	neg    %bx
    18ab:	c1 fb 05             	sar    $0x5,%bx
    18ae:	11 5c 46             	adc    %bx,0x46(%si)
    18b1:	c1 f9 04             	sar    $0x4,%cx
    18b4:	01 4c 4a             	add    %cx,0x4a(%si)
    18b7:	81 7c 40 2c 01       	cmpw   $0x12c,0x40(%si)
    18bc:	7c 15                	jl     0x18d3
    18be:	8b 44 38             	mov    0x38(%si),%ax
    18c1:	05 b8 0b             	add    $0xbb8,%ax
    18c4:	3d 70 17             	cmp    $0x1770,%ax
    18c7:	77 0a                	ja     0x18d3
    18c9:	b1 13                	mov    $0x13,%cl
    18cb:	e8 0b 00             	call   0x18d9
    18ce:	0f 83 80 00          	jae    0x1952
    18d2:	c3                   	ret
    18d3:	c7 44 0e 10 18       	movw   $0x1810,0xe(%si)
    18d8:	c3                   	ret
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
    1952:	c7 44 0e 57 19       	movw   $0x1957,0xe(%si)
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
    19cf:	8b 84 46 03          	mov    0x346(%si),%ax
    19d3:	8b 9c 4a 03          	mov    0x34a(%si),%bx
    19d7:	89 84 32 03          	mov    %ax,0x332(%si)
    19db:	89 9c 3a 03          	mov    %bx,0x33a(%si)
    19df:	8b 84 a4 03          	mov    0x3a4(%si),%ax
    19e3:	8b 9c a8 03          	mov    0x3a8(%si),%bx
    19e7:	89 84 90 03          	mov    %ax,0x390(%si)
    19eb:	89 9c 98 03          	mov    %bx,0x398(%si)
    19ef:	2e c7 06 8e 16 00 00 	movw   $0x0,%cs:0x168e
    19f6:	c7 44 58 00 00       	movw   $0x0,0x58(%si)
    19fb:	c7 44 5a 00 00       	movw   $0x0,0x5a(%si)
    1a00:	e9 0e fd             	jmp    0x1711
    1a03:	2e c7 06 8e 16 01 00 	movw   $0x1,%cs:0x168e
    1a0a:	2e c7 06 90 16 00 00 	movw   $0x0,%cs:0x1690
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
    1b3b:	8b 75 16             	mov    0x16(%di),%si
    1b3e:	83 c6 5e             	add    $0x5e,%si
    1b41:	c7 44 54 0a 00       	movw   $0xa,0x54(%si)
    1b46:	66 0f bf 44 40       	movswl 0x40(%si),%eax
    1b4b:	66 0f bf 5c 38       	movswl 0x38(%si),%ebx
    1b50:	8b 54 50             	mov    0x50(%si),%dx
    1b53:	66 f7 d8             	neg    %eax
    1b56:	66 0f af 5c 32       	imul   0x32(%si),%ebx
    1b5b:	66 0f af 44 1a       	imul   0x1a(%si),%eax
    1b60:	66 03 c3             	add    %ebx,%eax
    1b63:	b8 f0 ff             	mov    $0xfff0,%ax
    1b66:	79 03                	jns    0x1b6b
    1b68:	b8 10 00             	mov    $0x10,%ax
    1b6b:	03 d0                	add    %ax,%dx
    1b6d:	03 54 50             	add    0x50(%si),%dx
    1b70:	8b 5c 58             	mov    0x58(%si),%bx
    1b73:	33 d8                	xor    %ax,%bx
    1b75:	79 02                	jns    0x1b79
    1b77:	d1 f8                	sar    $1,%ax
    1b79:	89 44 58             	mov    %ax,0x58(%si)
    1b7c:	01 44 50             	add    %ax,0x50(%si)
    1b7f:	c3                   	ret
    1b80:	1e                   	push   %ds
    1b81:	8b 75 38             	mov    0x38(%di),%si
    1b84:	8b 5d 3a             	mov    0x3a(%di),%bx
    1b87:	8b 84 36 00          	mov    0x36(%si),%ax
    1b8b:	83 c6 04             	add    $0x4,%si
    1b8e:	81 e6 fc 0f          	and    $0xffc,%si
    1b92:	89 75 38             	mov    %si,0x38(%di)
    1b95:	89 45 3a             	mov    %ax,0x3a(%di)
    1b98:	2b c3                	sub    %bx,%ax
    1b9a:	64 8e 1e 02 00       	mov    %fs:0x2,%ds
    1b9f:	64 8b 75 1c          	mov    %fs:0x1c(%di),%si
    1ba3:	64 8b 4d 20          	mov    %fs:0x20(%di),%cx
    1ba7:	01 04                	add    %ax,(%si)
    1ba9:	83 c6 14             	add    $0x14,%si
    1bac:	e2 f9                	loop   0x1ba7
    1bae:	1f                   	pop    %ds
    1baf:	c3                   	ret
    1bb0:	1e                   	push   %ds
    1bb1:	8b 75 38             	mov    0x38(%di),%si
    1bb4:	8b 5d 3a             	mov    0x3a(%di),%bx
    1bb7:	8b 84 36 00          	mov    0x36(%si),%ax
    1bbb:	83 c6 04             	add    $0x4,%si
    1bbe:	81 e6 fc 0f          	and    $0xffc,%si
    1bc2:	89 75 38             	mov    %si,0x38(%di)
    1bc5:	c1 f8 04             	sar    $0x4,%ax
    1bc8:	89 45 3a             	mov    %ax,0x3a(%di)
    1bcb:	2b c3                	sub    %bx,%ax
    1bcd:	64 8e 1e 02 00       	mov    %fs:0x2,%ds
    1bd2:	64 8b 75 1c          	mov    %fs:0x1c(%di),%si
    1bd6:	64 8b 4d 20          	mov    %fs:0x20(%di),%cx
    1bda:	01 04                	add    %ax,(%si)
    1bdc:	83 c6 14             	add    $0x14,%si
    1bdf:	e2 f9                	loop   0x1bda
    1be1:	1f                   	pop    %ds
    1be2:	c3                   	ret
