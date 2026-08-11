; Commander Blood recovered routine assembly
; module: xdb_scrut
; artifact: output/_tmp_dat/scrut.xdb
; artifact_sha256: 8522e77aad6639cf9bb06f148048fed3c6bf1cfbe6459c9a0be7b987c3c3ac77
; overlay_offset: 0x000000
; group: entry
; provenance: overlay_entry_0
; byte_count: 149
; boundary: cfg_blocks_5_terminals_1
; terminal: retf:1
; direct_callees: 0x0000a3
; indirect_calls: 0
; cxx_source: re/borland/xdb/scrut/entry/func_000000_routine.cpp
; routine_bytes_sha256: 7d04c0a35e82fbf020b08f8d7278e90f2f552717cc658572fab77cb889b307e7

000000:  66 50                        push     eax
000002:  66 53                        push     ebx
000004:  66 51                        push     ecx
000006:  66 52                        push     edx
000008:  66 56                        push     esi
00000A:  66 57                        push     edi
00000C:  1E                           push     ds
00000D:  06                           push     es
00000E:  0F A0                        push     fs
000010:  0F A8                        push     gs
000012:  66 55                        push     ebp
000014:  8C C8                        mov      ax, cs
000016:  2E 03 06 A5 33               add      ax, word ptr cs:[0x33a5]
00001B:  8E D8                        mov      ds, ax
00001D:  8E E0                        mov      fs, ax
00001F:  2E A3 A7 33                  mov      word ptr cs:[0x33a7], ax
000023:  03 06 0C 00                  add      ax, word ptr [0xc]
000027:  A3 02 00                     mov      word ptr [2], ax
00002A:  03 06 0E 00                  add      ax, word ptr [0xe]
00002E:  A3 04 00                     mov      word ptr [4], ax
000031:  03 06 10 00                  add      ax, word ptr [0x10]
000035:  A3 06 00                     mov      word ptr [6], ax
000038:  8E C0                        mov      es, ax
00003A:  26 C7 06 46 09 00 2A         mov      word ptr es:[0x946], 0x2a00
000041:  C4 7E 00                     les      di, ptr [bp]
000044:  26 8B 05                     mov      ax, word ptr es:[di]
000047:  C1 E0 03                     shl      ax, 3
00004A:  79 02                        jns      0x4e
00004C:  33 C0                        xor      ax, ax
00004E:  3D 80 00                     cmp      ax, 0x80
000051:  72 03                        jb       0x56
000053:  B8 7F 00                     mov      ax, 0x7f
000056:  2D 04 00                     sub      ax, 4
000059:  2E A3 99 00                  mov      word ptr cs:[0x99], ax
00005D:  2E C7 06 9B 00 00 00         mov      word ptr cs:[0x9b], 0
000064:  66 8B 46 04                  mov      eax, dword ptr [bp + 4]
000068:  66 A3 20 00                  mov      dword ptr [0x20], eax
00006C:  0E                           push     cs
00006D:  E8 33 00                     call     0xa3
000070:  66 5D                        pop      ebp
000072:  C4 7E 00                     les      di, ptr [bp]
000075:  2E A1 99 00                  mov      ax, word ptr cs:[0x99]
000079:  05 04 00                     add      ax, 4
00007C:  C1 E8 03                     shr      ax, 3
00007F:  26 89 05                     mov      word ptr es:[di], ax
000082:  0F A9                        pop      gs
000084:  0F A1                        pop      fs
000086:  07                           pop      es
000087:  1F                           pop      ds
000088:  66 5F                        pop      edi
00008A:  66 5E                        pop      esi
00008C:  66 5A                        pop      edx
00008E:  66 59                        pop      ecx
000090:  66 5B                        pop      ebx
000092:  66 58                        pop      eax
000094:  CB                           retf    
