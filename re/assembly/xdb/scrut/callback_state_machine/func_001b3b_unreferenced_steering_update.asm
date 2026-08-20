; Commander Blood recovered routine assembly
; module: xdb_scrut
; artifact: output/_tmp_dat/scrut.xdb
; artifact_sha256: 8522e77aad6639cf9bb06f148048fed3c6bf1cfbe6459c9a0be7b987c3c3ac77
; overlay_offset: 0x001B3B
; byte_count: 69
; routine_bytes_sha256: 583efaabde835eef2a35421a23eeb3e1a2dc1acd68eb27d88d336ada0e24a892
; routine_entry: 0x001B3B
; group: callback_state_machine
; provenance: compiled context-ABI sibling present in all three alien overlays; no table or in-overlay pointer reference
; direct_callees: none
; raw stop: 0x001B80


output/_tmp_dat/scrut.xdb:     file format binary


Disassembly of section .data:

00001b3b <.data+0x1b3b>:
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
