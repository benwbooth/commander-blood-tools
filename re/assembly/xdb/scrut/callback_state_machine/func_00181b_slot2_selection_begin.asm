; Commander Blood recovered routine assembly
; module: xdb_scrut
; artifact: output/_tmp_dat/scrut.xdb
; artifact_sha256: 8522e77aad6639cf9bb06f148048fed3c6bf1cfbe6459c9a0be7b987c3c3ac77
; overlay_offset: 0x00181B
; byte_count: 61
; routine_bytes_sha256: 4a3f15007d8a7654fb57d11d8f34cf5935bd1a8f1a57cce61fec589cd864b435
; routine_entry: 0x00181B
; group: callback_state_machine
; provenance: callback published by internal transition 0x1810
; direct_callees: 0x001858, 0x0019CF, 0x001A11
; raw stop: 0x001858


output/_tmp_dat/scrut.xdb:     file format binary


Disassembly of section .data:

0000181b <.data+0x181b>:
    181b:	2e f7 06 70 0b 03 00 	testw  $0x3,%cs:0xb70
    1822:	0f 84 a9 01          	je     0x19cf
    1826:	8b 44 40             	mov    0x40(%si),%ax
    1829:	3d bc 02             	cmp    $0x2bc,%ax
    182c:	0f 8c e1 01          	jl     0x1a11
    1830:	8b 5c 38             	mov    0x38(%si),%bx
    1833:	81 fb f4 01          	cmp    $0x1f4,%bx
    1837:	0f 8f d6 01          	jg     0x1a11
    183b:	81 fb 0c fe          	cmp    $0xfe0c,%bx
    183f:	0f 8f ce 01          	jg     0x1a11
    1843:	c7 44 0e 58 18       	movw   $0x1858,0xe(%si)
    1848:	c7 44 56 c8 00       	movw   $0xc8,0x56(%si)
    184d:	8b 44 52             	mov    0x52(%si),%ax
    1850:	89 44 5a             	mov    %ax,0x5a(%si)
    1853:	c7 44 58 00 00       	movw   $0x0,0x58(%si)
