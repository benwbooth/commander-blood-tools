; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x006588
; seg_off: 04da:11e8
; group: seg_04da
; provenance: static_dispatch_table_target
; label: vm_op_a2_cond_call
; label_comment: VM opcode 0xA2 RANDOM BRANCH: lodsw operand, lcall 0x1ce:0xb02 -- which is blood_prng_next (file 0x2DE2), NOT a condition test as this row said until audit-fixes #593. The operand is the MODULUS; `or ax,ax / je 0x6595` takes the branch (call vm_branch 0x6462) when the draw is NON-ZERO, so with modulus N the branch is taken with probability (N-1)/N and modulus 1 never branches. PORTED: vm.rs 0xA2 arm, which calls the faithful port of the PRNG in VmMachine::rand
; incoming: vm_opcode_handlers:opcode_0xa2
; byte_count: 14
; boundary: cfg_blocks_3_terminals_1
; terminal: ret:1
; direct_callees: 0x006462
; indirect_calls: 1
; routine_bytes_sha256: 00a5d4a09f44da47917b30e0a841fe07da3b35b9d97dd1cc89c041968c43142b

006588:  AD                           lodsw    ax, word ptr [si]
006589:  9A 02 0B CE 01               lcall    0x1ce, 0xb02
00658E:  0B C0                        or       ax, ax
006590:  74 03                        je       0x6595
006592:  E8 CD FE                     call     0x6462
006595:  C3                           ret     
