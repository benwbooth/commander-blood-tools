; Commander Blood raw routine disassembly
; module: xdb_amer
; artifact: output/_tmp_dat/amer.xdb
; routine_entry: 0x001692
; group: callback_state_machine
; provenance: slot-2 dispatch callback
; raw stop: 0x001A5C

/home/ben/src/commander-blood-tools/output/_tmp_dat/amer.xdb:     file format binary


Disassembly of section .data:

00001692 <.data+0x1692>:
    1692:	2e f7 06 99 00 ff ff 	test   WORD PTR cs:0x99,0xffff
    1699:	78 0b                	js     0x16a6
    169b:	2e f7 06 2f 0b 01 00 	test   WORD PTR cs:0xb2f,0x1
    16a2:	0f 85 98 02          	jne    0x193e
    16a6:	ff 4d 38             	dec    WORD PTR [di+0x38]
    16a9:	79 72                	jns    0x171d
    16ab:	8b 5c 38             	mov    bx,WORD PTR [si+0x38]
    16ae:	8b 44 40             	mov    ax,WORD PTR [si+0x40]
    16b1:	3d dc 05             	cmp    ax,0x5dc
    16b4:	0f 8f 73 03          	jg     0x1a2b
    16b8:	3d 18 fc             	cmp    ax,0xfc18
    16bb:	0f 8c 6c 03          	jl     0x1a2b
    16bf:	81 fb dc 05          	cmp    bx,0x5dc
    16c3:	0f 8f 64 03          	jg     0x1a2b
    16c7:	81 fb 24 fa          	cmp    bx,0xfa24
    16cb:	0f 8c 5c 03          	jl     0x1a2b
    16cf:	8b 6d 40             	mov    bp,WORD PTR [di+0x40]
    16d2:	c1 cd 03             	ror    bp,0x3
    16d5:	83 dd 00             	sbb    bp,0x0
    16d8:	8b c5                	mov    ax,bp
    16da:	25 ff 07             	and    ax,0x7ff
    16dd:	2d ff 03             	sub    ax,0x3ff
    16e0:	8b c8                	mov    cx,ax
    16e2:	0b c9                	or     cx,cx
    16e4:	79 02                	jns    0x16e8
    16e6:	f7 d9                	neg    cx
    16e8:	c1 e9 02             	shr    cx,0x2
    16eb:	83 c1 10             	add    cx,0x10
    16ee:	89 4d 38             	mov    WORD PTR [di+0x38],cx
    16f1:	2b 44 52             	sub    ax,WORD PTR [si+0x52]
    16f4:	99                   	cwd
    16f5:	f7 f9                	idiv   cx
    16f7:	89 6d 40             	mov    WORD PTR [di+0x40],bp
    16fa:	89 45 3a             	mov    WORD PTR [di+0x3a],ax
    16fd:	c7 44 58 14 00       	mov    WORD PTR [si+0x58],0x14
    1702:	8b 44 3c             	mov    ax,WORD PTR [si+0x3c]
    1705:	03 44 4e             	add    ax,WORD PTR [si+0x4e]
    1708:	d1 f8                	sar    ax,1
    170a:	3d 00 03             	cmp    ax,0x300
    170d:	7c 03                	jl     0x1712
    170f:	b8 00 03             	mov    ax,0x300
    1712:	3d 00 fd             	cmp    ax,0xfd00
    1715:	7f 03                	jg     0x171a
    1717:	b8 00 fd             	mov    ax,0xfd00
    171a:	89 44 4e             	mov    WORD PTR [si+0x4e],ax
    171d:	8b 44 58             	mov    ax,WORD PTR [si+0x58]
    1720:	2b 44 54             	sub    ax,WORD PTR [si+0x54]
    1723:	c1 f8 03             	sar    ax,0x3
    1726:	01 44 54             	add    WORD PTR [si+0x54],ax
    1729:	8b 44 38             	mov    ax,WORD PTR [si+0x38]
    172c:	3d d8 ff             	cmp    ax,0xffd8
    172f:	7c 21                	jl     0x1752
    1731:	3d 28 00             	cmp    ax,0x28
    1734:	7d 1c                	jge    0x1752
    1736:	8b 44 3c             	mov    ax,WORD PTR [si+0x3c]
    1739:	3d d8 ff             	cmp    ax,0xffd8
    173c:	7c 14                	jl     0x1752
    173e:	3d 28 00             	cmp    ax,0x28
    1741:	7f 0f                	jg     0x1752
    1743:	8b 44 40             	mov    ax,WORD PTR [si+0x40]
    1746:	3d 28 00             	cmp    ax,0x28
    1749:	7f 07                	jg     0x1752
    174b:	3d b0 ff             	cmp    ax,0xffb0
    174e:	0f 8f c4 00          	jg     0x1816
    1752:	8b 55 3a             	mov    dx,WORD PTR [di+0x3a]
    1755:	03 54 52             	add    dx,WORD PTR [si+0x52]
    1758:	89 54 52             	mov    WORD PTR [si+0x52],dx
    175b:	8b ca                	mov    cx,dx
    175d:	c1 fa 03             	sar    dx,0x3
    1760:	01 54 50             	add    WORD PTR [si+0x50],dx
    1763:	8b 4c 52             	mov    cx,WORD PTR [si+0x52]
    1766:	8b 45 42             	mov    ax,WORD PTR [di+0x42]
    1769:	05 84 00             	add    ax,0x84
    176c:	25 ff 03             	and    ax,0x3ff
    176f:	89 45 42             	mov    WORD PTR [di+0x42],ax
    1772:	83 c1 20             	add    cx,0x20
    1775:	78 63                	js     0x17da
    1777:	83 e9 40             	sub    cx,0x40
    177a:	79 2d                	jns    0x17a9
    177c:	c7 84 ac 00 00 ff    	mov    WORD PTR [si+0xac],0xff00
    1782:	89 84 b0 00          	mov    WORD PTR [si+0xb0],ax
    1786:	f7 d8                	neg    ax
    1788:	c7 84 0a 01 00 01    	mov    WORD PTR [si+0x10a],0x100
    178e:	89 84 0e 01          	mov    WORD PTR [si+0x10e],ax
    1792:	c7 84 68 01 00 ff    	mov    WORD PTR [si+0x168],0xff00
    1798:	89 84 6c 01          	mov    WORD PTR [si+0x16c],ax
    179c:	f7 d8                	neg    ax
    179e:	c7 84 c6 01 00 01    	mov    WORD PTR [si+0x1c6],0x100
    17a4:	89 84 ca 01          	mov    WORD PTR [si+0x1ca],ax
    17a8:	c3                   	ret
    17a9:	f7 d8                	neg    ax
    17ab:	c7 84 ac 00 00 fe    	mov    WORD PTR [si+0xac],0xfe00
    17b1:	c7 84 b0 00 00 00    	mov    WORD PTR [si+0xb0],0x0
    17b7:	c7 84 0a 01 00 02    	mov    WORD PTR [si+0x10a],0x200
    17bd:	c7 84 0e 01 00 00    	mov    WORD PTR [si+0x10e],0x0
    17c3:	c7 84 68 01 00 ff    	mov    WORD PTR [si+0x168],0xff00
    17c9:	89 84 6c 01          	mov    WORD PTR [si+0x16c],ax
    17cd:	f7 d8                	neg    ax
    17cf:	c7 84 c6 01 00 01    	mov    WORD PTR [si+0x1c6],0x100
    17d5:	89 84 ca 01          	mov    WORD PTR [si+0x1ca],ax
    17d9:	c3                   	ret
    17da:	c7 84 ac 00 00 ff    	mov    WORD PTR [si+0xac],0xff00
    17e0:	89 84 b0 00          	mov    WORD PTR [si+0xb0],ax
    17e4:	f7 d8                	neg    ax
    17e6:	c7 84 0a 01 00 01    	mov    WORD PTR [si+0x10a],0x100
    17ec:	89 84 0e 01          	mov    WORD PTR [si+0x10e],ax
    17f0:	c7 84 68 01 00 fe    	mov    WORD PTR [si+0x168],0xfe00
    17f6:	c7 84 6c 01 00 00    	mov    WORD PTR [si+0x16c],0x0
    17fc:	c7 84 c6 01 00 02    	mov    WORD PTR [si+0x1c6],0x200
    1802:	c7 84 ca 01 00 00    	mov    WORD PTR [si+0x1ca],0x0
    1808:	c3                   	ret
    1809:	a1 f8 22             	mov    ax,ds:0x22f8
    180c:	05 00 08             	add    ax,0x800
    180f:	25 fc 0f             	and    ax,0xffc
    1812:	89 44 50             	mov    WORD PTR [si+0x50],ax
    1815:	c3                   	ret
    1816:	0b c0                	or     ax,ax
    1818:	78 ef                	js     0x1809
    181a:	c7 44 54 00 00       	mov    WORD PTR [si+0x54],0x0
    181f:	66 bb a0 00 00 00    	mov    ebx,0xa0
    1825:	66 f7 db             	neg    ebx
    1828:	66 a1 d2 22          	mov    eax,ds:0x22d2
    182c:	66 f7 eb             	imul   ebx
    182f:	66 03 06 ea 22       	add    eax,DWORD PTR ds:0x22ea
    1834:	66 c1 f8 10          	sar    eax,0x10
    1838:	66 f7 d8             	neg    eax
    183b:	66 89 44 42          	mov    DWORD PTR [si+0x42],eax
    183f:	66 a1 d6 22          	mov    eax,ds:0x22d6
    1843:	66 f7 eb             	imul   ebx
    1846:	66 03 06 ee 22       	add    eax,DWORD PTR ds:0x22ee
    184b:	66 c1 f8 10          	sar    eax,0x10
    184f:	66 f7 d8             	neg    eax
    1852:	66 89 44 46          	mov    DWORD PTR [si+0x46],eax
    1856:	66 a1 da 22          	mov    eax,ds:0x22da
    185a:	66 f7 eb             	imul   ebx
    185d:	66 03 06 f2 22       	add    eax,DWORD PTR ds:0x22f2
    1862:	66 c1 f8 10          	sar    eax,0x10
    1866:	66 f7 d8             	neg    eax
    1869:	66 89 44 4a          	mov    DWORD PTR [si+0x4a],eax
    186d:	66 0f bf 0e fc 22    	movsx  ecx,WORD PTR ds:0x22fc
    1873:	d1 f9                	sar    cx,1
    1875:	83 f9 28             	cmp    cx,0x28
    1878:	7f 03                	jg     0x187d
    187a:	b9 28 00             	mov    cx,0x28
    187d:	66 8b c1             	mov    eax,ecx
    1880:	66 8b d8             	mov    ebx,eax
    1883:	8b d1                	mov    dx,cx
    1885:	c1 ea 02             	shr    dx,0x2
    1888:	83 c2 14             	add    dx,0x14
    188b:	66 0f af 06 d2 22    	imul   eax,DWORD PTR ds:0x22d2
    1891:	66 0f af 1e d6 22    	imul   ebx,DWORD PTR ds:0x22d6
    1897:	66 0f af 0e da 22    	imul   ecx,DWORD PTR ds:0x22da
    189d:	66 c1 f8 12          	sar    eax,0x12
    18a1:	66 c1 fb 12          	sar    ebx,0x12
    18a5:	66 c1 f9 12          	sar    ecx,0x12
    18a9:	89 55 38             	mov    WORD PTR [di+0x38],dx
    18ac:	89 45 3a             	mov    WORD PTR [di+0x3a],ax
    18af:	89 5d 3c             	mov    WORD PTR [di+0x3c],bx
    18b2:	89 4d 3e             	mov    WORD PTR [di+0x3e],cx
    18b5:	c7 06 fc 22 c0 ff    	mov    WORD PTR ds:0x22fc,0xffc0
    18bb:	c7 45 36 01 80       	mov    WORD PTR [di+0x36],0x8001
    18c0:	c7 44 0e d3 18       	mov    WORD PTR [si+0xe],0x18d3
    18c5:	2e c7 06 48 16 01 00 	mov    WORD PTR cs:0x1648,0x1
    18cc:	c7 06 1e 00 01 00    	mov    WORD PTR ds:0x1e,0x1
    18d2:	c3                   	ret
    18d3:	ff 4d 38             	dec    WORD PTR [di+0x38]
    18d6:	78 2a                	js     0x1902
    18d8:	c7 44 54 00 00       	mov    WORD PTR [si+0x54],0x0
    18dd:	81 44 50 80 00       	add    WORD PTR [si+0x50],0x80
    18e2:	83 6c 52 75          	sub    WORD PTR [si+0x52],0x75
    18e6:	66 0f bf 45 3a       	movsx  eax,WORD PTR [di+0x3a]
    18eb:	66 0f bf 5d 3c       	movsx  ebx,WORD PTR [di+0x3c]
    18f0:	66 0f bf 4d 3e       	movsx  ecx,WORD PTR [di+0x3e]
    18f5:	66 01 44 42          	add    DWORD PTR [si+0x42],eax
    18f9:	66 01 5c 46          	add    DWORD PTR [si+0x46],ebx
    18fd:	66 01 4c 4a          	add    DWORD PTR [si+0x4a],ecx
    1901:	c3                   	ret
    1902:	c7 45 36 01 00       	mov    WORD PTR [di+0x36],0x1
    1907:	c7 45 38 20 00       	mov    WORD PTR [di+0x38],0x20
    190c:	c7 44 54 00 00       	mov    WORD PTR [si+0x54],0x0
    1911:	8b 5c 50             	mov    bx,WORD PTR [si+0x50]
    1914:	8b 4c 52             	mov    cx,WORD PTR [si+0x52]
    1917:	c1 e3 04             	shl    bx,0x4
    191a:	c1 e1 04             	shl    cx,0x4
    191d:	c1 fb 04             	sar    bx,0x4
    1920:	c1 f9 04             	sar    cx,0x4
    1923:	89 5c 50             	mov    WORD PTR [si+0x50],bx
    1926:	89 4c 52             	mov    WORD PTR [si+0x52],cx
    1929:	f7 d9                	neg    cx
    192b:	c1 f9 05             	sar    cx,0x5
    192e:	89 4d 3a             	mov    WORD PTR [di+0x3a],cx
    1931:	c7 44 0e 92 16       	mov    WORD PTR [si+0xe],0x1692
    1936:	2e c7 06 48 16 00 00 	mov    WORD PTR cs:0x1648,0x0
    193d:	c3                   	ret
    193e:	c7 44 58 28 00       	mov    WORD PTR [si+0x58],0x28
    1943:	c7 44 0e 48 19       	mov    WORD PTR [si+0xe],0x1948
    1948:	2e f7 06 2f 0b 01 00 	test   WORD PTR cs:0xb2f,0x1
    194f:	0f 84 35 fd          	je     0x1688
    1953:	66 0f bf 44 40       	movsx  eax,WORD PTR [si+0x40]
    1958:	66 0f bf 5c 38       	movsx  ebx,WORD PTR [si+0x38]
    195d:	3d b8 0b             	cmp    ax,0xbb8
    1960:	0f 87 c7 00          	ja     0x1a2b
    1964:	81 fb e8 03          	cmp    bx,0x3e8
    1968:	0f 8f bf 00          	jg     0x1a2b
    196c:	81 fb 18 fc          	cmp    bx,0xfc18
    1970:	0f 8c b7 00          	jl     0x1a2b
    1974:	3d 20 03             	cmp    ax,0x320
    1977:	7c 47                	jl     0x19c0
    1979:	66 f7 d8             	neg    eax
    197c:	66 0f af 5c 32       	imul   ebx,DWORD PTR [si+0x32]
    1981:	66 0f af 44 1a       	imul   eax,DWORD PTR [si+0x1a]
    1986:	66 03 c3             	add    eax,ebx
    1989:	b8 c0 ff             	mov    ax,0xffc0
    198c:	79 03                	jns    0x1991
    198e:	b8 40 00             	mov    ax,0x40
    1991:	c7 45 3a 00 00       	mov    WORD PTR [di+0x3a],0x0
    1996:	01 44 50             	add    WORD PTR [si+0x50],ax
    1999:	c7 44 52 00 00       	mov    WORD PTR [si+0x52],0x0
    199e:	8b 44 46             	mov    ax,WORD PTR [si+0x46]
    19a1:	03 06 f0 22          	add    ax,WORD PTR ds:0x22f0
    19a5:	03 44 4e             	add    ax,WORD PTR [si+0x4e]
    19a8:	d1 f8                	sar    ax,1
    19aa:	3d 00 03             	cmp    ax,0x300
    19ad:	7c 03                	jl     0x19b2
    19af:	b8 00 03             	mov    ax,0x300
    19b2:	3d 00 fd             	cmp    ax,0xfd00
    19b5:	7f 03                	jg     0x19ba
    19b7:	b8 00 fd             	mov    ax,0xfd00
    19ba:	89 44 4e             	mov    WORD PTR [si+0x4e],ax
    19bd:	e9 5d fd             	jmp    0x171d
    19c0:	c7 44 58 50 00       	mov    WORD PTR [si+0x58],0x50
    19c5:	c7 44 0e cb 19       	mov    WORD PTR [si+0xe],0x19cb
    19ca:	c3                   	ret
    19cb:	66 0f bf 44 40       	movsx  eax,WORD PTR [si+0x40]
    19d0:	66 0f bf 5c 38       	movsx  ebx,WORD PTR [si+0x38]
    19d5:	3d e8 03             	cmp    ax,0x3e8
    19d8:	0f 87 62 ff          	ja     0x193e
    19dc:	81 fb f4 01          	cmp    bx,0x1f4
    19e0:	7f 49                	jg     0x1a2b
    19e2:	81 fb 0c fe          	cmp    bx,0xfe0c
    19e6:	7c 43                	jl     0x1a2b
    19e8:	66 2d c8 00 00 00    	sub    eax,0xc8
    19ee:	66 f7 d8             	neg    eax
    19f1:	66 0f af 5c 32       	imul   ebx,DWORD PTR [si+0x32]
    19f6:	66 0f af 44 1a       	imul   eax,DWORD PTR [si+0x1a]
    19fb:	66 03 c3             	add    eax,ebx
    19fe:	b8 d0 ff             	mov    ax,0xffd0
    1a01:	79 03                	jns    0x1a06
    1a03:	b8 30 00             	mov    ax,0x30
    1a06:	89 45 3a             	mov    WORD PTR [di+0x3a],ax
    1a09:	8b 44 46             	mov    ax,WORD PTR [si+0x46]
    1a0c:	03 06 f0 22          	add    ax,WORD PTR ds:0x22f0
    1a10:	03 44 4e             	add    ax,WORD PTR [si+0x4e]
    1a13:	d1 f8                	sar    ax,1
    1a15:	3d 00 03             	cmp    ax,0x300
    1a18:	7c 03                	jl     0x1a1d
    1a1a:	b8 00 03             	mov    ax,0x300
    1a1d:	3d 00 fd             	cmp    ax,0xfd00
    1a20:	7f 03                	jg     0x1a25
    1a22:	b8 00 fd             	mov    ax,0xfd00
    1a25:	89 44 4e             	mov    WORD PTR [si+0x4e],ax
    1a28:	e9 f2 fc             	jmp    0x171d
    1a2b:	c7 44 52 00 00       	mov    WORD PTR [si+0x52],0x0
    1a30:	c7 45 3a 00 00       	mov    WORD PTR [di+0x3a],0x0
    1a35:	c7 44 5c 00 00       	mov    WORD PTR [si+0x5c],0x0
    1a3a:	c7 44 54 3c 00       	mov    WORD PTR [si+0x54],0x3c
    1a3f:	c7 44 0e 5c 1a       	mov    WORD PTR [si+0xe],0x1a5c
    1a44:	8b 45 40             	mov    ax,WORD PTR [di+0x40]
    1a47:	c1 c8 07             	ror    ax,0x7
    1a4a:	1d 00 00             	sbb    ax,0x0
    1a4d:	89 45 40             	mov    WORD PTR [di+0x40],ax
    1a50:	c1 f8 06             	sar    ax,0x6
    1a53:	89 44 4e             	mov    WORD PTR [si+0x4e],ax
    1a56:	c7 44 56 20 00       	mov    WORD PTR [si+0x56],0x20
    1a5b:	c3                   	ret
