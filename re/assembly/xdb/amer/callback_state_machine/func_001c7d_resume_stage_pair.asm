; Commander Blood recovered routine assembly
; module: xdb_amer
; artifact: output/_tmp_dat/amer.xdb
; artifact_sha256: 6fddeb5cc7c62fe5e638900746decc299f1637ea11612c0c51aced367dd12b31
; overlay_offset: 0x001C7D
; byte_count: 66
; routine_bytes_sha256: e01aa35182c73bb499ac76d22abf4e26a7da7a97c20dac1d75f57ee3b10e9974
; routine_entry: 0x001C7D
; group: callback_state_machine
; provenance: continuation stored at context +0x36 by 0x1C34
; raw stop: 0x001CBF


output/_tmp_dat/amer.xdb:     file format binary


Disassembly of section .data:

00001c7d <.data+0x1c7d>:
    1c7d:	e8 83 ff             	call   0x1c03
    1c80:	57                   	push   %di
    1c81:	8b 75 16             	mov    0x16(%di),%si
    1c84:	8b 7d 3a             	mov    0x3a(%di),%di
    1c87:	83 c6 5e             	add    $0x5e,%si
    1c8a:	8b 45 54             	mov    0x54(%di),%ax
    1c8d:	8b d8                	mov    %ax,%bx
    1c8f:	d1 fb                	sar    $1,%bx
    1c91:	03 44 54             	add    0x54(%si),%ax
    1c94:	03 c3                	add    %bx,%ax
    1c96:	d1 f8                	sar    $1,%ax
    1c98:	89 44 54             	mov    %ax,0x54(%si)
    1c9b:	e8 5c 00             	call   0x1cfa
    1c9e:	5f                   	pop    %di
    1c9f:	72 01                	jb     0x1ca2
    1ca1:	c3                   	ret
    1ca2:	c7 45 36 bf 1c       	movw   $0x1cbf,0x36(%di)
    1ca7:	c7 44 54 00 00       	movw   $0x0,0x54(%si)
    1cac:	8b 75 3a             	mov    0x3a(%di),%si
    1caf:	c7 44 0e 8a 15       	movw   $0x158a,0xe(%si)
    1cb4:	89 75 3c             	mov    %si,0x3c(%di)
    1cb7:	2e c7 06 5f 0d 18 00 	movw   $0x18,%cs:0xd5f
    1cbe:	c3                   	ret
