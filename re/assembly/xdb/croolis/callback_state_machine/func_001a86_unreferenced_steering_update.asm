; Commander Blood recovered routine assembly
; module: xdb_croolis
; artifact: output/_tmp_dat/croolis.xdb
; artifact_sha256: 13eba7c4e4f38662c44c849bd07be293c7583d2b2dc9ae273d7fe94746048c31
; overlay_offset: 0x001A86
; byte_count: 69
; routine_bytes_sha256: 583efaabde835eef2a35421a23eeb3e1a2dc1acd68eb27d88d336ada0e24a892
; routine_entry: 0x001A86
; group: callback_state_machine
; provenance: compiled context-ABI sibling present in all three alien overlays; no table or in-overlay pointer reference
; direct_callees: none
; raw stop: 0x001ACB


output/_tmp_dat/croolis.xdb:     file format binary


Disassembly of section .data:

00001a86 <.data+0x1a86>:
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
