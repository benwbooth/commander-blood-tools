; Commander Blood recovered routine assembly
; module: xdb_amer
; artifact: output/_tmp_dat/amer.xdb
; artifact_sha256: 6fddeb5cc7c62fe5e638900746decc299f1637ea11612c0c51aced367dd12b31
; overlay_offset: 0x001B1A
; byte_count: 69
; routine_bytes_sha256: 583efaabde835eef2a35421a23eeb3e1a2dc1acd68eb27d88d336ada0e24a892
; routine_entry: 0x001B1A
; group: callback_state_machine
; provenance: compiled context-ABI sibling present in all three alien overlays; no table or in-overlay pointer reference
; direct_callees: none
; raw stop: 0x001B5F


output/_tmp_dat/amer.xdb:     file format binary


Disassembly of section .data:

00001b1a <.data+0x1b1a>:
    1b1a:	8b 75 16             	mov    0x16(%di),%si
    1b1d:	83 c6 5e             	add    $0x5e,%si
    1b20:	c7 44 54 0a 00       	movw   $0xa,0x54(%si)
    1b25:	66 0f bf 44 40       	movswl 0x40(%si),%eax
    1b2a:	66 0f bf 5c 38       	movswl 0x38(%si),%ebx
    1b2f:	8b 54 50             	mov    0x50(%si),%dx
    1b32:	66 f7 d8             	neg    %eax
    1b35:	66 0f af 5c 32       	imul   0x32(%si),%ebx
    1b3a:	66 0f af 44 1a       	imul   0x1a(%si),%eax
    1b3f:	66 03 c3             	add    %ebx,%eax
    1b42:	b8 f0 ff             	mov    $0xfff0,%ax
    1b45:	79 03                	jns    0x1b4a
    1b47:	b8 10 00             	mov    $0x10,%ax
    1b4a:	03 d0                	add    %ax,%dx
    1b4c:	03 54 50             	add    0x50(%si),%dx
    1b4f:	8b 5c 58             	mov    0x58(%si),%bx
    1b52:	33 d8                	xor    %ax,%bx
    1b54:	79 02                	jns    0x1b58
    1b56:	d1 f8                	sar    $1,%ax
    1b58:	89 44 58             	mov    %ax,0x58(%si)
    1b5b:	01 44 50             	add    %ax,0x50(%si)
    1b5e:	c3                   	ret
