; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x002de2
; seg_off: 01ce:0b02
; group: seg_01ce
; provenance: recursive_graph, relocation_proven_far_transfer_target
; label: blood_prng_next
; label_comment: THE PRNG -- this IS 0x01CE:0x0B02 (file = 0x600 + 0x1CE*16 + 0xB02). Hedge removed 2026-07-24: it was labelled a hash/checksum that might be an auxiliary PRNG. AX IN is the MODULUS, AX OUT the value; the 8-round rcr/rcl loop threads a carry through the mixing bytes cs:[0xAF0]/[0xAF1] beside the seed word cs:[0xAEE]. Confirmed three ways: every call site passes a modulus in AX (0x6339 with 5, 0xB8AB with 10, VM 0xA2 at 0x6588), the port ships a faithful implementation of 0x01CE:0x0B02 as ship3d::BloodPrng::next, and 0x2DD3 seeds it from the CMOS RTC seconds
; incoming: call@0x003c0e->01ce:0b02
; incoming: call@0x006343->01ce:0b02
; incoming: call@0x006589->01ce:0b02
; incoming: call@0x008be2->01ce:0b02
; incoming: call@0x009b77->01ce:0b02
; incoming: call@0x009b80->01ce:0b02
; incoming: call@0x009b89->01ce:0b02
; incoming: call@0x00b6d3->01ce:0b02
; incoming: call@0x00b8ae->01ce:0b02
; byte_count: 81
; boundary: cfg_blocks_5_terminals_1
; terminal: retf:1
; direct_callees: none
; indirect_calls: 0
; routine_bytes_sha256: 556f10ea452fbf38cec2b971d2d5ce61a6a6da621427034ebb4f1a3f8249cd51

002DE2:  53                           push     bx
002DE3:  51                           push     cx
002DE4:  52                           push     dx
002DE5:  8B D0                        mov      dx, ax
002DE7:  2E 8A 1E F0 0A               mov      bl, byte ptr cs:[0xaf0]
002DEC:  2E 8A 3E F1 0A               mov      bh, byte ptr cs:[0xaf1]
002DF1:  B9 08 00                     mov      cx, 8
002DF4:  33 C0                        xor      ax, ax
002DF6:  D0 DB                        rcr      bl, 1
002DF8:  D1 D0                        rcl      ax, 1
002DFA:  D0 D7                        rcl      bh, 1
002DFC:  D1 D0                        rcl      ax, 1
002DFE:  E2 F6                        loop     0x2df6
002E00:  2E 8B 1E EE 0A               mov      bx, word ptr cs:[0xaee]
002E05:  C1 EB 03                     shr      bx, 3
002E08:  2E 33 06 EE 0A               xor      ax, word ptr cs:[0xaee]
002E0D:  2E FE 06 F2 0A               inc      byte ptr cs:[0xaf2]
002E12:  2E 8A 1E F2 0A               mov      bl, byte ptr cs:[0xaf2]
002E17:  2E 28 1E F1 0A               sub      byte ptr cs:[0xaf1], bl
002E1C:  D0 C3                        rol      bl, 1
002E1E:  2E 30 1E F0 0A               xor      byte ptr cs:[0xaf0], bl
002E23:  0B D2                        or       dx, dx
002E25:  75 04                        jne      0x2e2b
002E27:  74 06                        je       0x2e2f
002E29:  2B C2                        sub      ax, dx
002E2B:  3B C2                        cmp      ax, dx
002E2D:  73 FA                        jae      0x2e29
002E2F:  5A                           pop      dx
002E30:  59                           pop      cx
002E31:  5B                           pop      bx
002E32:  CB                           retf    
