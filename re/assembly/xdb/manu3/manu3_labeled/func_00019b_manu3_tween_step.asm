; Commander Blood recovered routine assembly
; module: xdb_manu3
; artifact: output/_tmp_dat/manu3.xdb
; artifact_sha256: d0f64e99a646197906e273edfa0124172307a5cd766c88591c12ebd9ea556d31
; overlay_offset: 0x00019b
; group: manu3_labeled
; provenance: direct_call_from_0x0, direct_call_from_0x150, label:manu3_tween_step, manu3 tween stepper
; label: manu3_tween_step
; label_comment: per-frame tween processor: records {counter[di], target[di+4], value[di+8], accum dword [di+6] += step [di+0xA]} -> writes value to *target; expired tweens swap-removed. The hand poses + menu motions are DATA-DRIVEN tweens; targets ~0x2670-0x2870 region
; byte_count: 163
; boundary: cfg_blocks_8_terminals_2
; terminal: jmp 0x1df:1, ret:1
; direct_callees: none
; indirect_calls: 0
; routine_bytes_sha256: 9072490ce643cd0fb2f7f955bf33ac5ffd75d4d3be30942b725a43991855a42d

00019B:  0F A0                        push     fs
00019D:  1F                           pop      ds
00019E:  F7 06 2C 10 00 FF            test     word ptr [0x102c], 0xff00
0001A4:  0F 85 95 00                  jne      0x23d
0001A8:  BE 32 10                     mov      si, 0x1032
0001AB:  8B 1E 30 10                  mov      bx, word ptr [0x1030]
0001AF:  3B F3                        cmp      si, bx
0001B1:  74 2C                        je       0x1df
0001B3:  8B 3C                        mov      di, word ptr [si]
0001B5:  8B 6D 04                     mov      bp, word ptr [di + 4]
0001B8:  8B 45 08                     mov      ax, word ptr [di + 8]
0001BB:  3E 89 46 00                  mov      word ptr ds:[bp], ax
0001BF:  FF 0D                        dec      word ptr [di]
0001C1:  78 11                        js       0x1d4
0001C3:  66 8B 45 0A                  mov      eax, dword ptr [di + 0xa]
0001C7:  66 01 45 06                  add      dword ptr [di + 6], eax
0001CB:  83 C6 02                     add      si, 2
0001CE:  3B F3                        cmp      si, bx
0001D0:  75 E1                        jne      0x1b3
0001D2:  EB 0B                        jmp      0x1df
0001D4:  83 EB 02                     sub      bx, 2
0001D7:  87 3F                        xchg     word ptr [bx], di
0001D9:  89 3C                        mov      word ptr [si], di
0001DB:  3B F3                        cmp      si, bx
0001DD:  75 D6                        jne      0x1b5
; -- non-contiguous block: next 0x00023d --
00023D:  C3                           ret     
