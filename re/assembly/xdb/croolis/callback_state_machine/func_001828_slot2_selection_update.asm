; Commander Blood recovered routine assembly
; module: xdb_croolis
; artifact: output/_tmp_dat/croolis.xdb
; artifact_sha256: 13eba7c4e4f38662c44c849bd07be293c7583d2b2dc9ae273d7fe94746048c31
; overlay_offset: 0x001828
; byte_count: 312
; routine_bytes_sha256: 4aceda013ad70ae65d3afd1810d1f4cab0fddb38775f775e36f4931ac7b63f3e
; routine_entry: 0x001828
; group: callback_state_machine
; provenance: callback published by internal transition 0x1815
; direct_callees: 0x001727, 0x001960
; raw stop: 0x001960


output/_tmp_dat/croolis.xdb:     file format binary


Disassembly of section .data:

00001828 <.data+0x1828>:
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
