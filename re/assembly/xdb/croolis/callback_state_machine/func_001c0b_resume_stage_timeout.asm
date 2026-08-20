; Commander Blood recovered routine assembly
; module: xdb_croolis
; artifact: output/_tmp_dat/croolis.xdb
; artifact_sha256: 13eba7c4e4f38662c44c849bd07be293c7583d2b2dc9ae273d7fe94746048c31
; overlay_offset: 0x001C0B
; byte_count: 16
; routine_bytes_sha256: 4b4f21975f3379fccef85ef31b92b2fb1dd162c59b994e107b76147fe7c13f15
; routine_entry: 0x001C0B
; group: callback_state_machine
; provenance: continuation stored at context +0x36 by 0x1BC9
; raw stop: 0x001C1B


output/_tmp_dat/croolis.xdb:     file format binary


Disassembly of section .data:

00001c0b <.data+0x1c0b>:
    1c0b:	e8 51 ff             	call   0x1b5f
    1c0e:	2e ff 0e b7 0d       	decw   %cs:0xdb7
    1c13:	79 05                	jns    0x1c1a
    1c15:	c7 45 36 1b 1c       	movw   $0x1c1b,0x36(%di)
    1c1a:	c3                   	ret
