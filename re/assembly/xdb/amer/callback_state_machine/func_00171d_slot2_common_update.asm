; Commander Blood recovered routine assembly
; module: xdb_amer
; artifact: output/_tmp_dat/amer.xdb
; artifact_sha256: 6fddeb5cc7c62fe5e638900746decc299f1637ea11612c0c51aced367dd12b31
; overlay_offset: 0x00171D
; byte_count: 438
; routine_bytes_sha256: e56a182c0ba482e63a1b97cf0b19ba137f36df4c06c0aeea54ac64984c2fa3f1
; routine_entry: 0x00171D
; group: callback_state_machine
; provenance: shared update tail reached by callbacks 0x1692, 0x1948, and 0x19CB
; direct_callees: none
; raw stop: 0x0018D3


output/_tmp_dat/amer.xdb:     file format binary


Disassembly of section .data:

0000171d <.data+0x171d>:
    171d:	8b 44 58             	mov    0x58(%si),%ax
    1720:	2b 44 54             	sub    0x54(%si),%ax
    1723:	c1 f8 03             	sar    $0x3,%ax
    1726:	01 44 54             	add    %ax,0x54(%si)
    1729:	8b 44 38             	mov    0x38(%si),%ax
    172c:	3d d8 ff             	cmp    $0xffd8,%ax
    172f:	7c 21                	jl     0x1752
    1731:	3d 28 00             	cmp    $0x28,%ax
    1734:	7d 1c                	jge    0x1752
    1736:	8b 44 3c             	mov    0x3c(%si),%ax
    1739:	3d d8 ff             	cmp    $0xffd8,%ax
    173c:	7c 14                	jl     0x1752
    173e:	3d 28 00             	cmp    $0x28,%ax
    1741:	7f 0f                	jg     0x1752
    1743:	8b 44 40             	mov    0x40(%si),%ax
    1746:	3d 28 00             	cmp    $0x28,%ax
    1749:	7f 07                	jg     0x1752
    174b:	3d b0 ff             	cmp    $0xffb0,%ax
    174e:	0f 8f c4 00          	jg     0x1816
    1752:	8b 55 3a             	mov    0x3a(%di),%dx
    1755:	03 54 52             	add    0x52(%si),%dx
    1758:	89 54 52             	mov    %dx,0x52(%si)
    175b:	8b ca                	mov    %dx,%cx
    175d:	c1 fa 03             	sar    $0x3,%dx
    1760:	01 54 50             	add    %dx,0x50(%si)
    1763:	8b 4c 52             	mov    0x52(%si),%cx
    1766:	8b 45 42             	mov    0x42(%di),%ax
    1769:	05 84 00             	add    $0x84,%ax
    176c:	25 ff 03             	and    $0x3ff,%ax
    176f:	89 45 42             	mov    %ax,0x42(%di)
    1772:	83 c1 20             	add    $0x20,%cx
    1775:	78 63                	js     0x17da
    1777:	83 e9 40             	sub    $0x40,%cx
    177a:	79 2d                	jns    0x17a9
    177c:	c7 84 ac 00 00 ff    	movw   $0xff00,0xac(%si)
    1782:	89 84 b0 00          	mov    %ax,0xb0(%si)
    1786:	f7 d8                	neg    %ax
    1788:	c7 84 0a 01 00 01    	movw   $0x100,0x10a(%si)
    178e:	89 84 0e 01          	mov    %ax,0x10e(%si)
    1792:	c7 84 68 01 00 ff    	movw   $0xff00,0x168(%si)
    1798:	89 84 6c 01          	mov    %ax,0x16c(%si)
    179c:	f7 d8                	neg    %ax
    179e:	c7 84 c6 01 00 01    	movw   $0x100,0x1c6(%si)
    17a4:	89 84 ca 01          	mov    %ax,0x1ca(%si)
    17a8:	c3                   	ret
    17a9:	f7 d8                	neg    %ax
    17ab:	c7 84 ac 00 00 fe    	movw   $0xfe00,0xac(%si)
    17b1:	c7 84 b0 00 00 00    	movw   $0x0,0xb0(%si)
    17b7:	c7 84 0a 01 00 02    	movw   $0x200,0x10a(%si)
    17bd:	c7 84 0e 01 00 00    	movw   $0x0,0x10e(%si)
    17c3:	c7 84 68 01 00 ff    	movw   $0xff00,0x168(%si)
    17c9:	89 84 6c 01          	mov    %ax,0x16c(%si)
    17cd:	f7 d8                	neg    %ax
    17cf:	c7 84 c6 01 00 01    	movw   $0x100,0x1c6(%si)
    17d5:	89 84 ca 01          	mov    %ax,0x1ca(%si)
    17d9:	c3                   	ret
    17da:	c7 84 ac 00 00 ff    	movw   $0xff00,0xac(%si)
    17e0:	89 84 b0 00          	mov    %ax,0xb0(%si)
    17e4:	f7 d8                	neg    %ax
    17e6:	c7 84 0a 01 00 01    	movw   $0x100,0x10a(%si)
    17ec:	89 84 0e 01          	mov    %ax,0x10e(%si)
    17f0:	c7 84 68 01 00 fe    	movw   $0xfe00,0x168(%si)
    17f6:	c7 84 6c 01 00 00    	movw   $0x0,0x16c(%si)
    17fc:	c7 84 c6 01 00 02    	movw   $0x200,0x1c6(%si)
    1802:	c7 84 ca 01 00 00    	movw   $0x0,0x1ca(%si)
    1808:	c3                   	ret
    1809:	a1 f8 22             	mov    0x22f8,%ax
    180c:	05 00 08             	add    $0x800,%ax
    180f:	25 fc 0f             	and    $0xffc,%ax
    1812:	89 44 50             	mov    %ax,0x50(%si)
    1815:	c3                   	ret
    1816:	0b c0                	or     %ax,%ax
    1818:	78 ef                	js     0x1809
    181a:	c7 44 54 00 00       	movw   $0x0,0x54(%si)
    181f:	66 bb a0 00 00 00    	mov    $0xa0,%ebx
    1825:	66 f7 db             	neg    %ebx
    1828:	66 a1 d2 22          	mov    0x22d2,%eax
    182c:	66 f7 eb             	imul   %ebx
    182f:	66 03 06 ea 22       	add    0x22ea,%eax
    1834:	66 c1 f8 10          	sar    $0x10,%eax
    1838:	66 f7 d8             	neg    %eax
    183b:	66 89 44 42          	mov    %eax,0x42(%si)
    183f:	66 a1 d6 22          	mov    0x22d6,%eax
    1843:	66 f7 eb             	imul   %ebx
    1846:	66 03 06 ee 22       	add    0x22ee,%eax
    184b:	66 c1 f8 10          	sar    $0x10,%eax
    184f:	66 f7 d8             	neg    %eax
    1852:	66 89 44 46          	mov    %eax,0x46(%si)
    1856:	66 a1 da 22          	mov    0x22da,%eax
    185a:	66 f7 eb             	imul   %ebx
    185d:	66 03 06 f2 22       	add    0x22f2,%eax
    1862:	66 c1 f8 10          	sar    $0x10,%eax
    1866:	66 f7 d8             	neg    %eax
    1869:	66 89 44 4a          	mov    %eax,0x4a(%si)
    186d:	66 0f bf 0e fc 22    	movswl 0x22fc,%ecx
    1873:	d1 f9                	sar    $1,%cx
    1875:	83 f9 28             	cmp    $0x28,%cx
    1878:	7f 03                	jg     0x187d
    187a:	b9 28 00             	mov    $0x28,%cx
    187d:	66 8b c1             	mov    %ecx,%eax
    1880:	66 8b d8             	mov    %eax,%ebx
    1883:	8b d1                	mov    %cx,%dx
    1885:	c1 ea 02             	shr    $0x2,%dx
    1888:	83 c2 14             	add    $0x14,%dx
    188b:	66 0f af 06 d2 22    	imul   0x22d2,%eax
    1891:	66 0f af 1e d6 22    	imul   0x22d6,%ebx
    1897:	66 0f af 0e da 22    	imul   0x22da,%ecx
    189d:	66 c1 f8 12          	sar    $0x12,%eax
    18a1:	66 c1 fb 12          	sar    $0x12,%ebx
    18a5:	66 c1 f9 12          	sar    $0x12,%ecx
    18a9:	89 55 38             	mov    %dx,0x38(%di)
    18ac:	89 45 3a             	mov    %ax,0x3a(%di)
    18af:	89 5d 3c             	mov    %bx,0x3c(%di)
    18b2:	89 4d 3e             	mov    %cx,0x3e(%di)
    18b5:	c7 06 fc 22 c0 ff    	movw   $0xffc0,0x22fc
    18bb:	c7 45 36 01 80       	movw   $0x8001,0x36(%di)
    18c0:	c7 44 0e d3 18       	movw   $0x18d3,0xe(%si)
    18c5:	2e c7 06 48 16 01 00 	movw   $0x1,%cs:0x1648
    18cc:	c7 06 1e 00 01 00    	movw   $0x1,0x1e
    18d2:	c3                   	ret
