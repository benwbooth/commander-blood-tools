; Commander Blood recovered routine assembly
; module: xdb_scrut
; artifact: output/_tmp_dat/scrut.xdb
; artifact_sha256: 8522e77aad6639cf9bb06f148048fed3c6bf1cfbe6459c9a0be7b987c3c3ac77
; overlay_offset: 0x001CDB
; byte_count: 43
; routine_bytes_sha256: 961e2bac48921f78dfbcfe18042de1e9f594574a4ce14c154b133fe4b84d291f
; routine_entry: 0x001CDB
; group: callback_state_machine
; provenance: continuation stored at context +0x36 by 0x1CCB
; raw stop: 0x001D06


output/_tmp_dat/scrut.xdb:     file format binary


Disassembly of section .data:

00001cdb <.data+0x1cdb>:
    1cdb:	57                   	push   %di
    1cdc:	8b 75 16             	mov    0x16(%di),%si
    1cdf:	83 c6 5e             	add    $0x5e,%si
    1ce2:	c7 44 54 64 00       	movw   $0x64,0x54(%si)
    1ce7:	2e 8b 3e e3 1b       	mov    %cs:0x1be3,%di
    1cec:	e8 17 00             	call   0x1d06
    1cef:	5f                   	pop    %di
    1cf0:	72 01                	jb     0x1cf3
    1cf2:	c3                   	ret
    1cf3:	c7 45 36 45 1c       	movw   $0x1c45,0x36(%di)
    1cf8:	8b 5d 3a             	mov    0x3a(%di),%bx
    1cfb:	c7 47 0e 9e 15       	movw   $0x159e,0xe(%bx)
    1d00:	c7 44 54 00 00       	movw   $0x0,0x54(%si)
    1d05:	c3                   	ret
