; Commander Blood recovered routine assembly
; module: xdb_croolis
; artifact: output/_tmp_dat/croolis.xdb
; artifact_sha256: 13eba7c4e4f38662c44c849bd07be293c7583d2b2dc9ae273d7fe94746048c31
; overlay_offset: 0x00166C
; byte_count: 52
; routine_bytes_sha256: 4c3455654656d4a75e669da99de2f4d1522968313870307fd469726c4b100d05
; routine_entry: 0x00166C
; group: callback_state_machine
; provenance: callback installed by slot-3 resume callback
; raw stop: 0x0016A0


output/_tmp_dat/croolis.xdb:     file format binary


Disassembly of section .data:

0000166c <.data+0x166c>:
    166c:	90                   	nop
    166d:	2e f7 06 72 0b ff ff 	testw  $0xffff,%cs:0xb72
    1674:	75 29                	jne    0x169f
    1676:	8b 6c 5a             	mov    0x5a(%si),%bp
    1679:	83 c5 08             	add    $0x8,%bp
    167c:	81 e5 fc 03          	and    $0x3fc,%bp
    1680:	89 6c 5a             	mov    %bp,0x5a(%si)
    1683:	2e c7 86 bb 0d 00 00 	movw   $0x0,%cs:0xdbb(%bp)
    168a:	2e c7 86 bd 0d 00 00 	movw   $0x0,%cs:0xdbd(%bp)
    1691:	2e c7 86 bf 0d 00 00 	movw   $0x0,%cs:0xdbf(%bp)
    1698:	2e c7 86 c1 0d 00 00 	movw   $0x0,%cs:0xdc1(%bp)
    169f:	c3                   	ret
