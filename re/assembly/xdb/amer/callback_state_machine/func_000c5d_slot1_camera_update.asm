; Commander Blood recovered routine assembly
; module: xdb_amer
; artifact: output/_tmp_dat/amer.xdb
; artifact_sha256: 6fddeb5cc7c62fe5e638900746decc299f1637ea11612c0c51aced367dd12b31
; overlay_offset: 0x000C5D
; byte_count: 36
; routine_bytes_sha256: 6d8a0445c1cb14b7ddccadf17ed7b1e28dc9abc0d717ac93809d13a459a64862
; routine_entry: 0x000C5D
; group: callback_state_machine
; provenance: callback reached from the slot-1 wave path at 0x0BCD and movement path at 0x0C24
; raw stop: 0x000C81


output/_tmp_dat/amer.xdb:     file format binary


Disassembly of section .data:

00000c5d <.data+0xc5d>:
     c5d:	a1 f8 22             	mov    0x22f8,%ax
     c60:	8b 5c 50             	mov    0x50(%si),%bx
     c63:	25 fc 0f             	and    $0xffc,%ax
     c66:	81 e3 fc 0f          	and    $0xffc,%bx
     c6a:	2b c3                	sub    %bx,%ax
     c6c:	c1 f8 04             	sar    $0x4,%ax
     c6f:	89 44 56             	mov    %ax,0x56(%si)
     c72:	8b 44 52             	mov    0x52(%si),%ax
     c75:	c1 f8 04             	sar    $0x4,%ax
     c78:	89 44 10             	mov    %ax,0x10(%si)
     c7b:	c7 44 0e 81 0c       	movw   $0xc81,0xe(%si)
     c80:	c3                   	ret
