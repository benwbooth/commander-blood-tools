; Commander Blood recovered routine assembly
; module: xdb_croolis
; artifact: output/_tmp_dat/croolis.xdb
; artifact_sha256: 13eba7c4e4f38662c44c849bd07be293c7583d2b2dc9ae273d7fe94746048c31
; overlay_offset: 0x0017F2
; byte_count: 35
; routine_bytes_sha256: 07e35aed737796c8e0bb09f0f36e5f7c2fdafadb8c952c2f2de59c3b8ff6eaf5
; routine_entry: 0x0017F2
; group: callback_state_machine
; provenance: callback published by internal transition 0x17E4
; direct_callees: 0x00171D, 0x001794
; raw stop: 0x001815


output/_tmp_dat/croolis.xdb:     file format binary


Disassembly of section .data:

000017f2 <.data+0x17f2>:
    17f2:	8b 45 38             	mov    0x38(%di),%ax
    17f5:	89 84 be 01          	mov    %ax,0x1be(%si)
    17f9:	2d 04 00             	sub    $0x4,%ax
    17fc:	3d 92 00             	cmp    $0x92,%ax
    17ff:	7c 05                	jl     0x1806
    1801:	89 45 38             	mov    %ax,0x38(%di)
    1804:	eb 8e                	jmp    0x1794
    1806:	c7 45 38 00 00       	movw   $0x0,0x38(%di)
    180b:	2e c7 06 a0 16 00 00 	movw   $0x0,%cs:0x16a0
    1812:	e9 08 ff             	jmp    0x171d
