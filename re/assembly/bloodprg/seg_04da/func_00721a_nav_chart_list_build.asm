; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x00721a
; seg_off: 04da:1e7a
; group: seg_04da
; provenance: recursive_graph, relocation_proven_far_transfer_target
; label: nav_chart_list_build
; label_comment: NAV-CHART VISIBLE-OBJECT LIST builder (far 0x4DA:0x1E7A; output DS:0x2AD3, count returned in AX and stored at [0x27C1] by 0x8D2A). Calls 0x604E to build the active-object candidate list at DS:0x6A16, then walks it (terminator = a NEGATIVE word) and keeps every object whose KIND word has any of the bits 0x118 -- kind 0x08, kind 0x10 (SHIP), kind 0x100 (BLACK HOLE). Stores the terminator and returns the count. PORTED: vm.rs build_nav_chart_list || NARROWER EARLIER READING `vm_helper_604e`: VM helper: eax=0; cx=ax; call 0x604e (VM lookup-prep sibling of 0x71cf). Zeroes accumulators then runs the record-resolution prep || MERGED 2026-07-25 (audit-fixes #133): one address, two names, the shorter describing a prologue or a single facet. Kept because a narrow reading records a true observation; renamed away because it is not what the routine IS.
; incoming: call@0x008d1f->04da:1e7a
; byte_count: 63
; boundary: cfg_blocks_6_terminals_2
; terminal: jmp 0x7233:1, retf:1
; direct_callees: 0x00604e
; indirect_calls: 0
; routine_bytes_sha256: 70a7a352c1d0b1bd3e8f67b342868de9071477eff51077a55a820b50c5c10da1

00721A:  53                           push     bx
00721B:  52                           push     dx
00721C:  51                           push     cx
00721D:  06                           push     es
00721E:  57                           push     di
00721F:  56                           push     si
007220:  55                           push     bp
007221:  66 33 C0                     xor      eax, eax
007224:  8B C8                        mov      cx, ax
007226:  E8 25 EE                     call     0x604e
007229:  BE 16 6A                     mov      si, 0x6a16
00722C:  C4 3E 24 67                  les      di, ptr [0x6724]
007230:  BD D3 2A                     mov      bp, 0x2ad3
007233:  AD                           lodsw    ax, word ptr [si]
007234:  0B C0                        or       ax, ax
007236:  78 14                        js       0x724c
007238:  67 26 8B 1C 38               mov      bx, word ptr es:[eax + edi]
00723D:  F7 C3 18 01                  test     bx, 0x118
007241:  74 07                        je       0x724a
007243:  89 46 00                     mov      word ptr [bp], ax
007246:  83 C5 02                     add      bp, 2
007249:  41                           inc      cx
00724A:  EB E7                        jmp      0x7233
00724C:  89 46 00                     mov      word ptr [bp], ax
00724F:  8B C1                        mov      ax, cx
007251:  5D                           pop      bp
007252:  5E                           pop      si
007253:  5F                           pop      di
007254:  07                           pop      es
007255:  59                           pop      cx
007256:  5A                           pop      dx
007257:  5B                           pop      bx
007258:  CB                           retf    
