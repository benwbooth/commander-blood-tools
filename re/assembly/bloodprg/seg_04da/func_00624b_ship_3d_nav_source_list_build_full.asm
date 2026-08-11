; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x00624b
; seg_off: 04da:0eab
; group: seg_04da
; provenance: recursive_graph, relocation_proven_far_transfer_target
; label: ship_3d_nav_source_list_build_full
; label_comment: NAV SOURCE-LIST BUILDER (output DS:0x6886, -1 terminated) - FULL decode, PURE RECORD LOGIC (no frontend state, which is what makes the C1 nav-source path portable): lds si,gs:[0x672c] (the 20-byte directory = the DEB records, +0x10 = object offset, +0x12 = entry kind); bx=[si+0x10]; read the object's kind es:[bx]; ax=vm_field_offset(0x11,kind); skip when ax==0 (or ax,ax / je); if es:[bx+ax] == di (the current target) then append [si+0x10] at [bp], bp+=2, and RECURSE depth-first with di = that object. Advance si+=0x14 and continue only while the NEXT entry's +0x12 == 1, else store 0xFFFF and return. Ported: VmMachine::build_nav_source_list (src/vm.rs) || ALSO RECORDED as `ship_3d_navigation_source_list_build`: recursively fills DS:0x6886 with selector-0x11 children of the current target, depth-first, -1 terminated || MERGED 2026-07-25 (#186): one address, several names, folded by union. || DI LIFETIME SETTLED (audit-fixes #194): DI is PRESERVED across the recursion (0x6276 push di / mov di,ax / call 0x624b / 0x627D pop di), so the routine returns the caller's target in DI. This is what lets 0x7259's `mov ax,di` @0x726F be read as the target.
; incoming: call@0x0083b9->04da:0eab
; incoming: call@0x0091b3->04da:0eab
; byte_count: 72
; boundary: cfg_blocks_6_terminals_1
; terminal: retf:1
; direct_callees: 0x006023, 0x00624b
; indirect_calls: 0
; cxx_source: re/borland/bloodprg/seg_04da/func_00624b_ship_3d_nav_source_list_build_full.cpp
; routine_bytes_sha256: f268dc67f3d776dfbdfbd201f389565afd4bb8f818d1fa53d1343b16281d9c91

00624B:  1E                           push     ds
00624C:  56                           push     si
00624D:  53                           push     bx
00624E:  50                           push     ax
00624F:  65 C5 36 2C 67               lds      si, ptr gs:[0x672c]
006254:  8B 5C 10                     mov      bx, word ptr [si + 0x10]
006257:  53                           push     bx
006258:  26 8B 1F                     mov      bx, word ptr es:[bx]
00625B:  B8 11 00                     mov      ax, 0x11
00625E:  E8 C2 FD                     call     0x6023
006261:  5B                           pop      bx
006262:  0B C0                        or       ax, ax
006264:  74 18                        je       0x627e
006266:  03 D8                        add      bx, ax
006268:  26 3B 3F                     cmp      di, word ptr es:[bx]
00626B:  75 11                        jne      0x627e
00626D:  8B 44 10                     mov      ax, word ptr [si + 0x10]
006270:  89 46 00                     mov      word ptr [bp], ax
006273:  83 C5 02                     add      bp, 2
006276:  57                           push     di
006277:  8B F8                        mov      di, ax
006279:  0E                           push     cs
00627A:  E8 CE FF                     call     0x624b
00627D:  5F                           pop      di
00627E:  83 C6 14                     add      si, 0x14
006281:  8B 44 12                     mov      ax, word ptr [si + 0x12]
006284:  83 F8 01                     cmp      ax, 1
006287:  74 CB                        je       0x6254
006289:  C7 46 00 FF FF               mov      word ptr [bp], 0xffff
00628E:  58                           pop      ax
00628F:  5B                           pop      bx
006290:  5E                           pop      si
006291:  1F                           pop      ds
006292:  CB                           retf    
