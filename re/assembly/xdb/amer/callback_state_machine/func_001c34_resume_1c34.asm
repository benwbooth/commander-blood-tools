; Commander Blood recovered routine assembly
; module: xdb_amer
; artifact: output/_tmp_dat/amer.xdb
; artifact_sha256: 6fddeb5cc7c62fe5e638900746decc299f1637ea11612c0c51aced367dd12b31
; overlay_offset: 0x001C34
; byte_count: 73
; routine_bytes_sha256: 60374253ea4ef91b0d6d368d979149fc12d5d62d6c114adc50238467d340058e
; routine_entry: 0x001C34
; group: callback_state_machine
; provenance: resume callback published by method-table slot 13
; raw stop: 0x001C7D


output/_tmp_dat/amer.xdb:     file format binary


Disassembly of section .data:

00001c34 <.data+0x1c34>:
    1c34:	8b 75 16             	mov    0x16(%di),%si
    1c37:	2e 8b 1e c8 1b       	mov    %cs:0x1bc8,%bx
    1c3c:	2e 8b 87 ca 1b       	mov    %cs:0x1bca(%bx),%ax
    1c41:	0b c0                	or     %ax,%ax
    1c43:	75 22                	jne    0x1c67
    1c45:	83 c3 02             	add    $0x2,%bx
    1c48:	83 e3 0f             	and    $0xf,%bx
    1c4b:	2e 89 1e c8 1b       	mov    %bx,%cs:0x1bc8
    1c50:	8b 84 ac 00          	mov    0xac(%si),%ax
    1c54:	05 e0 07             	add    $0x7e0,%ax
    1c57:	25 fc 0f             	and    $0xffc,%ax
    1c5a:	2d 00 08             	sub    $0x800,%ax
    1c5d:	89 84 ac 00          	mov    %ax,0xac(%si)
    1c61:	83 84 ae 00 08       	addw   $0x8,0xae(%si)
    1c66:	c3                   	ret
    1c67:	2e c7 06 c4 1b 00 00 	movw   $0x0,%cs:0x1bc4
    1c6e:	2e c7 87 ca 1b 00 00 	movw   $0x0,%cs:0x1bca(%bx)
    1c75:	c7 45 36 7d 1c       	movw   $0x1c7d,0x36(%di)
    1c7a:	89 45 3a             	mov    %ax,0x3a(%di)
