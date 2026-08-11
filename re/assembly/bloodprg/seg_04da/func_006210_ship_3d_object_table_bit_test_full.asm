; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x006210
; seg_off: 05c1:0000
; group: seg_04da
; provenance: direct_call_target, label_csv_target, manual_binary_boundary
; label: ship_3d_object_table_bit_test_full
; label_comment: maps an object record to its 20-byte directory index and returns a high-bit-first bitset test in CF
; incoming: call@0x006c2f->0x006210
; byte_count: 59
; boundary: cfg_blocks_5_terminals_1
; terminal: ret:1
; direct_callees: 0x006023
; indirect_calls: 0
; routine_bytes_sha256: 80aa0602ad218707fe83895885339336850fcd57e89a02fd9c51c2dcb9b24481

006210:  50                           push     ax
006211:  53                           push     bx
006212:  51                           push     cx
006213:  06                           push     es
006214:  57                           push     di
006215:  56                           push     si
006216:  65 C4 3E 2C 67               les      di, ptr gs:[0x672c]
00621B:  33 C9                        xor      cx, cx
00621D:  26 3B 45 10                  cmp      ax, word ptr es:[di + 0x10]
006221:  74 06                        je       0x6229
006223:  83 C7 14                     add      di, 0x14
006226:  41                           inc      cx
006227:  EB F4                        jmp      0x621d
006229:  B8 05 00                     mov      ax, 5
00622C:  BB 02 00                     mov      bx, 2
00622F:  E8 F1 FD                     call     0x6023
006232:  03 F0                        add      si, ax
006234:  8B C1                        mov      ax, cx
006236:  80 E1 07                     and      cl, 7
006239:  FE C1                        inc      cl
00623B:  C1 E8 03                     shr      ax, 3
00623E:  03 F0                        add      si, ax
006240:  8A 04                        mov      al, byte ptr [si]
006242:  D2 E0                        shl      al, cl
006244:  5E                           pop      si
006245:  5F                           pop      di
006246:  07                           pop      es
006247:  59                           pop      cx
006248:  5B                           pop      bx
006249:  58                           pop      ax
00624A:  C3                           ret
