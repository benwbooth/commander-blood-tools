
/home/ben/src/commander-blood-tools/output/_tmp_dat/croolis.xdb:     file format binary


Disassembly of section .data:

00001727 <.data+0x1727>:
    1727:	2e f7 06 70 0b 03 00 	testw  $0x3,%cs:0xb70
    172e:	0f 85 e3 00          	jne    0x1815
    1732:	ff 4d 38             	decw   0x38(%di)
    1735:	79 57                	jns    0x178e
    1737:	8b 44 40             	mov    0x40(%si),%ax
    173a:	8b 5c 38             	mov    0x38(%si),%bx
    173d:	05 f4 01             	add    $0x1f4,%ax
    1740:	3d b8 0b             	cmp    $0xbb8,%ax
    1743:	0f 87 19 02          	ja     0x1960
    1747:	81 c3 e8 03          	add    $0x3e8,%bx
    174b:	81 fb d0 07          	cmp    $0x7d0,%bx
    174f:	0f 87 0d 02          	ja     0x1960
    1753:	8b 6d 42             	mov    0x42(%di),%bp
    1756:	c1 cd 03             	ror    $0x3,%bp
    1759:	83 dd 00             	sbb    $0x0,%bp
    175c:	8b c5                	mov    %bp,%ax
    175e:	25 ff 03             	and    $0x3ff,%ax
    1761:	2d ff 01             	sub    $0x1ff,%ax
    1764:	8b d0                	mov    %ax,%dx
    1766:	0b d2                	or     %dx,%dx
    1768:	79 02                	jns    0x176c
    176a:	f7 da                	neg    %dx
    176c:	8b ca                	mov    %dx,%cx
    176e:	d1 e9                	shr    $1,%cx
    1770:	83 c1 10             	add    $0x10,%cx
    1773:	f7 da                	neg    %dx
    1775:	81 c2 00 03          	add    $0x300,%dx
    1779:	c1 ea 03             	shr    $0x3,%dx
    177c:	89 4d 38             	mov    %cx,0x38(%di)
    177f:	89 54 58             	mov    %dx,0x58(%si)
    1782:	2b 44 52             	sub    0x52(%si),%ax
    1785:	99                   	cwtd
    1786:	f7 f9                	idiv   %cx
    1788:	89 6d 42             	mov    %bp,0x42(%di)
    178b:	89 45 3a             	mov    %ax,0x3a(%di)
    178e:	39 3e 82 22          	cmp    %di,0x2282
    1792:	74 50                	je     0x17e4
    1794:	8b 44 3c             	mov    0x3c(%si),%ax
    1797:	03 45 3c             	add    0x3c(%di),%ax
    179a:	2b 44 4e             	sub    0x4e(%si),%ax
    179d:	c1 f8 03             	sar    $0x3,%ax
    17a0:	03 44 4e             	add    0x4e(%si),%ax
    17a3:	3d 00 03             	cmp    $0x300,%ax
    17a6:	7c 03                	jl     0x17ab
    17a8:	b8 00 03             	mov    $0x300,%ax
    17ab:	3d 00 fd             	cmp    $0xfd00,%ax
    17ae:	7f 03                	jg     0x17b3
    17b0:	b8 00 fd             	mov    $0xfd00,%ax
    17b3:	89 44 4e             	mov    %ax,0x4e(%si)
    17b6:	8b 44 58             	mov    0x58(%si),%ax
    17b9:	2b 44 54             	sub    0x54(%si),%ax
    17bc:	c1 f8 03             	sar    $0x3,%ax
    17bf:	01 44 54             	add    %ax,0x54(%si)
    17c2:	8b 55 3a             	mov    0x3a(%di),%dx
    17c5:	03 54 52             	add    0x52(%si),%dx
    17c8:	89 54 52             	mov    %dx,0x52(%si)
    17cb:	8b c2                	mov    %dx,%ax
    17cd:	d1 fa                	sar    $1,%dx
    17cf:	89 94 0a 01          	mov    %dx,0x10a(%si)
    17d3:	f7 da                	neg    %dx
    17d5:	89 94 ae 00          	mov    %dx,0xae(%si)
    17d9:	89 94 68 01          	mov    %dx,0x168(%si)
    17dd:	c1 f8 04             	sar    $0x4,%ax
    17e0:	11 44 50             	adc    %ax,0x50(%si)
    17e3:	c3                   	ret
    17e4:	83 6c 46 1e          	subw   $0x1e,0x46(%si)
    17e8:	c7 45 38 b2 00       	movw   $0xb2,0x38(%di)
    17ed:	c7 44 0e f2 17       	movw   $0x17f2,0xe(%si)
    17f2:	8b 45 38             	mov    0x38(%di),%ax
    17f5:	89 84 be 01          	mov    %ax,0x1be(%si)
    17f9:	2d 04 00             	sub    $0x4,%ax
    17fc:	3d 92 00             	cmp    $0x92,%ax
    17ff:	7c 05                	jl     0x1806
    1801:	89 45 38             	mov    %ax,0x38(%di)
    1804:	eb 8e                	jmp    0x1794
    1806:	c7 45 38 00 00       	movw   $0x0,0x38(%di)
    180b:	2e c7 06 a0 16 00 00 	movw   $0x0,%cs:0x16a0
    1812:	e9 08 ff             	jmp    0x171d
    1815:	c7 44 0e 28 18       	movw   $0x1828,0xe(%si)
    181a:	2e c7 06 a2 16 00 00 	movw   $0x0,%cs:0x16a2
    1821:	2e c7 06 a0 16 00 00 	movw   $0x0,%cs:0x16a0
    1828:	2e f7 06 70 0b 03 00 	testw  $0x3,%cs:0xb70
    182f:	0f 84 ec 00          	je     0x191f
    1833:	2e 8b 16 a2 16       	mov    %cs:0x16a2,%dx
    1838:	0b d2                	or     %dx,%dx
    183a:	8b 44 32             	mov    0x32(%si),%ax
    183d:	74 06                	je     0x1845
    183f:	3b d6                	cmp    %si,%dx
    1841:	0f 85 fb 00          	jne    0x1940
    1845:	66 0f bf 4c 40       	movswl 0x40(%si),%ecx
    184a:	81 f9 dc 05          	cmp    $0x5dc,%cx
    184e:	0f 87 e7 00          	ja     0x1939
    1852:	3d 00 b0             	cmp    $0xb000,%ax
    1855:	0f 8f e0 00          	jg     0x1939
    1859:	66 0f bf 5c 38       	movswl 0x38(%si),%ebx
    185e:	81 fb f4 01          	cmp    $0x1f4,%bx
    1862:	0f 8f d3 00          	jg     0x1939
    1866:	81 fb 0c fe          	cmp    $0xfe0c,%bx
    186a:	0f 8c cb 00          	jl     0x1939
    186e:	39 3e 82 22          	cmp    %di,0x2282
    1872:	0f 84 bc 00          	je     0x1932
    1876:	2e 89 36 a2 16       	mov    %si,%cs:0x16a2
    187b:	2e 89 36 a2 16       	mov    %si,%cs:0x16a2
    1880:	66 83 c1 64          	add    $0x64,%ecx
    1884:	66 0f af 5c 32       	imul   0x32(%si),%ebx
    1889:	66 0f af 4c 1a       	imul   0x1a(%si),%ecx
    188e:	66 2b d9             	sub    %ecx,%ebx
    1891:	b8 d0 ff             	mov    $0xffd0,%ax
    1894:	79 03                	jns    0x1899
    1896:	b8 30 00             	mov    $0x30,%ax
    1899:	03 44 52             	add    0x52(%si),%ax
    189c:	3d 00 03             	cmp    $0x300,%ax
    189f:	7e 03                	jle    0x18a4
    18a1:	b8 00 03             	mov    $0x300,%ax
    18a4:	3d 00 fd             	cmp    $0xfd00,%ax
    18a7:	7d 03                	jge    0x18ac
    18a9:	b8 00 fd             	mov    $0xfd00,%ax
    18ac:	89 44 52             	mov    %ax,0x52(%si)
    18af:	c1 f8 04             	sar    $0x4,%ax
    18b2:	01 44 50             	add    %ax,0x50(%si)
    18b5:	8b 5c 4e             	mov    0x4e(%si),%bx
    18b8:	8b 44 46             	mov    0x46(%si),%ax
    18bb:	03 06 f0 22          	add    0x22f0,%ax
    18bf:	d1 f8                	sar    $1,%ax
    18c1:	2b c3                	sub    %bx,%ax
    18c3:	c1 f8 02             	sar    $0x2,%ax
    18c6:	03 44 4e             	add    0x4e(%si),%ax
    18c9:	3d 00 03             	cmp    $0x300,%ax
    18cc:	7c 03                	jl     0x18d1
    18ce:	b8 00 03             	mov    $0x300,%ax
    18d1:	3d 00 fd             	cmp    $0xfd00,%ax
    18d4:	7f 03                	jg     0x18d9
    18d6:	b8 00 fd             	mov    $0xfd00,%ax
    18d9:	89 44 4e             	mov    %ax,0x4e(%si)
    18dc:	b8 c8 00             	mov    $0xc8,%ax
    18df:	2b 44 54             	sub    0x54(%si),%ax
    18e2:	c1 f8 02             	sar    $0x2,%ax
    18e5:	01 44 54             	add    %ax,0x54(%si)
    18e8:	8b 44 56             	mov    0x56(%si),%ax
    18eb:	2d 10 00             	sub    $0x10,%ax
    18ee:	79 07                	jns    0x18f7
    18f0:	64 c7 06 1e 00 01 00 	movw   $0x1,%fs:0x1e
    18f7:	25 7f 00             	and    $0x7f,%ax
    18fa:	89 44 56             	mov    %ax,0x56(%si)
    18fd:	8b 9c 2c 02          	mov    0x22c(%si),%bx
    1901:	03 d8                	add    %ax,%bx
    1903:	89 9c 20 02          	mov    %bx,0x220(%si)
    1907:	8b 9c 8a 02          	mov    0x28a(%si),%bx
    190b:	03 d8                	add    %ax,%bx
    190d:	89 9c 7e 02          	mov    %bx,0x27e(%si)
    1911:	8b 9c e8 02          	mov    0x2e8(%si),%bx
    1915:	03 d8                	add    %ax,%bx
    1917:	89 9c dc 02          	mov    %bx,0x2dc(%si)
    191b:	89 44 56             	mov    %ax,0x56(%si)
    191e:	c3                   	ret
    191f:	c7 44 0e 27 17       	movw   $0x1727,0xe(%si)
    1924:	c7 45 38 00 00       	movw   $0x0,0x38(%di)
    1929:	2e c7 06 a0 16 00 00 	movw   $0x0,%cs:0x16a0
    1930:	eb 07                	jmp    0x1939
    1932:	2e c7 06 a0 16 01 00 	movw   $0x1,%cs:0x16a0
    1939:	2e c7 06 a2 16 00 00 	movw   $0x0,%cs:0x16a2
    1940:	8b 9c 2c 02          	mov    0x22c(%si),%bx
    1944:	89 9c 20 02          	mov    %bx,0x220(%si)
    1948:	8b 9c 8a 02          	mov    0x28a(%si),%bx
    194c:	89 9c 7e 02          	mov    %bx,0x27e(%si)
    1950:	8b 9c e8 02          	mov    0x2e8(%si),%bx
    1954:	89 9c dc 02          	mov    %bx,0x2dc(%si)
    1958:	66 ba e8 03 00 00    	mov    $0x3e8,%edx
    195e:	eb 06                	jmp    0x1966
    1960:	66 ba d0 07 00 00    	mov    $0x7d0,%edx
    1966:	66 0f bf 44 40       	movswl 0x40(%si),%eax
    196b:	66 3d 0c fe ff ff    	cmp    $0xfffffe0c,%eax
    1971:	7c 6a                	jl     0x19dd
    1973:	66 2b c2             	sub    %edx,%eax
    1976:	66 0f bf 4c 38       	movswl 0x38(%si),%ecx
    197b:	66 2b 4d 3c          	sub    0x3c(%di),%ecx
    197f:	66 8b d8             	mov    %eax,%ebx
    1982:	66 8b d1             	mov    %ecx,%edx
    1985:	66 0f af 44 32       	imul   0x32(%si),%eax
    198a:	66 0f af 4c 1a       	imul   0x1a(%si),%ecx
    198f:	66 03 c8             	add    %eax,%ecx
    1992:	66 f7 d9             	neg    %ecx
    1995:	66 c1 f9 0f          	sar    $0xf,%ecx
    1999:	79 05                	jns    0x19a0
    199b:	8b 4c 58             	mov    0x58(%si),%cx
    199e:	d1 f9                	sar    $1,%cx
    19a0:	89 4c 58             	mov    %cx,0x58(%si)
    19a3:	66 f7 db             	neg    %ebx
    19a6:	66 0f af 54 32       	imul   0x32(%si),%edx
    19ab:	66 0f af 5c 1a       	imul   0x1a(%si),%ebx
    19b0:	66 03 d3             	add    %ebx,%edx
    19b3:	b8 f0 ff             	mov    $0xfff0,%ax
    19b6:	79 03                	jns    0x19bb
    19b8:	b8 10 00             	mov    $0x10,%ax
    19bb:	89 45 3a             	mov    %ax,0x3a(%di)
    19be:	8b 44 52             	mov    0x52(%si),%ax
    19c1:	3d 00 03             	cmp    $0x300,%ax
    19c4:	7c 08                	jl     0x19ce
    19c6:	c7 44 52 00 03       	movw   $0x300,0x52(%si)
    19cb:	e9 c0 fd             	jmp    0x178e
    19ce:	3d 00 fd             	cmp    $0xfd00,%ax
    19d1:	0f 8d b9 fd          	jge    0x178e
    19d5:	c7 44 52 00 fd       	movw   $0xfd00,0x52(%si)
    19da:	e9 b1 fd             	jmp    0x178e
    19dd:	8b 5d 42             	mov    0x42(%di),%bx
    19e0:	c1 cb 07             	ror    $0x7,%bx
    19e3:	83 db 00             	sbb    $0x0,%bx
    19e6:	89 5d 42             	mov    %bx,0x42(%di)
    19e9:	81 e3 fc 0f          	and    $0xffc,%bx
    19ed:	66 0f bf 8f 36 00    	movswl 0x36(%bx),%ecx
    19f3:	66 0f bf 9f 38 00    	movswl 0x38(%bx),%ebx
    19f9:	66 8b c1             	mov    %ecx,%eax
    19fc:	66 0f af 06 ba 22    	imul   0x22ba,%eax
    1a02:	66 8b e8             	mov    %eax,%ebp
    1a05:	66 8b c1             	mov    %ecx,%eax
    1a08:	66 0f af 06 be 22    	imul   0x22be,%eax
    1a0e:	66 03 c5             	add    %ebp,%eax
    1a11:	66 c1 f8 10          	sar    $0x10,%eax
    1a15:	2b 06 ec 22          	sub    0x22ec,%ax
    1a19:	89 44 42             	mov    %ax,0x42(%si)
    1a1c:	66 8b c1             	mov    %ecx,%eax
    1a1f:	66 0f af 06 c6 22    	imul   0x22c6,%eax
    1a25:	66 8b e8             	mov    %eax,%ebp
    1a28:	66 8b c1             	mov    %ecx,%eax
    1a2b:	66 0f af 06 ca 22    	imul   0x22ca,%eax
    1a31:	66 03 c5             	add    %ebp,%eax
    1a34:	66 c1 f8 10          	sar    $0x10,%eax
    1a38:	2b 06 f0 22          	sub    0x22f0,%ax
    1a3c:	89 44 46             	mov    %ax,0x46(%si)
    1a3f:	66 8b c1             	mov    %ecx,%eax
    1a42:	66 0f af 06 d2 22    	imul   0x22d2,%eax
    1a48:	66 8b e8             	mov    %eax,%ebp
    1a4b:	66 8b c1             	mov    %ecx,%eax
    1a4e:	66 0f af 06 d6 22    	imul   0x22d6,%eax
    1a54:	66 03 c5             	add    %ebp,%eax
    1a57:	66 c1 f8 10          	sar    $0x10,%eax
    1a5b:	2b 06 f4 22          	sub    0x22f4,%ax
    1a5f:	89 44 4a             	mov    %ax,0x4a(%si)
    1a62:	a1 f6 22             	mov    0x22f6,%ax
    1a65:	8b 1e f8 22          	mov    0x22f8,%bx
    1a69:	89 44 4e             	mov    %ax,0x4e(%si)
    1a6c:	89 5c 50             	mov    %bx,0x50(%si)
    1a6f:	c7 44 52 00 00       	movw   $0x0,0x52(%si)
    1a74:	a1 fc 22             	mov    0x22fc,%ax
    1a77:	05 2c 01             	add    $0x12c,%ax
    1a7a:	89 44 54             	mov    %ax,0x54(%si)
    1a7d:	89 44 58             	mov    %ax,0x58(%si)
    1a80:	c7 45 38 08 00       	movw   $0x8,0x38(%di)
    1a85:	c3                   	ret
    1a86:	8b 75 16             	mov    0x16(%di),%si
    1a89:	83 c6 5e             	add    $0x5e,%si
    1a8c:	c7 44 54 0a 00       	movw   $0xa,0x54(%si)
    1a91:	66 0f bf 44 40       	movswl 0x40(%si),%eax
    1a96:	66 0f bf 5c 38       	movswl 0x38(%si),%ebx
    1a9b:	8b 54 50             	mov    0x50(%si),%dx
    1a9e:	66 f7 d8             	neg    %eax
    1aa1:	66 0f af 5c 32       	imul   0x32(%si),%ebx
    1aa6:	66 0f af 44 1a       	imul   0x1a(%si),%eax
    1aab:	66 03 c3             	add    %ebx,%eax
    1aae:	b8 f0 ff             	mov    $0xfff0,%ax
    1ab1:	79 03                	jns    0x1ab6
    1ab3:	b8 10 00             	mov    $0x10,%ax
    1ab6:	03 d0                	add    %ax,%dx
    1ab8:	03 54 50             	add    0x50(%si),%dx
    1abb:	8b 5c 58             	mov    0x58(%si),%bx
    1abe:	33 d8                	xor    %ax,%bx
    1ac0:	79 02                	jns    0x1ac4
    1ac2:	d1 f8                	sar    $1,%ax
    1ac4:	89 44 58             	mov    %ax,0x58(%si)
    1ac7:	01 44 50             	add    %ax,0x50(%si)
    1aca:	c3                   	ret
    1acb:	1e                   	push   %ds
    1acc:	8b 75 38             	mov    0x38(%di),%si
    1acf:	8b 5d 3a             	mov    0x3a(%di),%bx
    1ad2:	8b 84 36 00          	mov    0x36(%si),%ax
    1ad6:	83 c6 04             	add    $0x4,%si
    1ad9:	81 e6 fc 0f          	and    $0xffc,%si
    1add:	89 75 38             	mov    %si,0x38(%di)
    1ae0:	89 45 3a             	mov    %ax,0x3a(%di)
    1ae3:	2b c3                	sub    %bx,%ax
    1ae5:	64 8e 1e 02 00       	mov    %fs:0x2,%ds
    1aea:	64 8b 75 1c          	mov    %fs:0x1c(%di),%si
    1aee:	64 8b 4d 20          	mov    %fs:0x20(%di),%cx
    1af2:	01 04                	add    %ax,(%si)
    1af4:	83 c6 14             	add    $0x14,%si
    1af7:	e2 f9                	loop   0x1af2
    1af9:	1f                   	pop    %ds
    1afa:	c3                   	ret
    1afb:	1e                   	push   %ds
    1afc:	8b 75 38             	mov    0x38(%di),%si
    1aff:	8b 5d 3a             	mov    0x3a(%di),%bx
    1b02:	8b 84 36 00          	mov    0x36(%si),%ax
    1b06:	83 c6 04             	add    $0x4,%si
    1b09:	81 e6 fc 0f          	and    $0xffc,%si
    1b0d:	89 75 38             	mov    %si,0x38(%di)
    1b10:	c1 f8 04             	sar    $0x4,%ax
    1b13:	89 45 3a             	mov    %ax,0x3a(%di)
    1b16:	2b c3                	sub    %bx,%ax
    1b18:	64 8e 1e 02 00       	mov    %fs:0x2,%ds
    1b1d:	64 8b 75 1c          	mov    %fs:0x1c(%di),%si
    1b21:	64 8b 4d 20          	mov    %fs:0x20(%di),%cx
    1b25:	01 04                	add    %ax,(%si)
    1b27:	83 c6 14             	add    $0x14,%si
    1b2a:	e2 f9                	loop   0x1b25
    1b2c:	1f                   	pop    %ds
    1b2d:	c3                   	ret
