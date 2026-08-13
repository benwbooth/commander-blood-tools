; Commander Blood recovered routine assembly
; module: xdb_amer
; artifact: output/_tmp_dat/amer.xdb
; artifact_sha256: 6fddeb5cc7c62fe5e638900746decc299f1637ea11612c0c51aced367dd12b31
; overlay_offset: 0x001a5c
; group: callback_state_machine
; provenance: state_callback_store@0x001a3f
; byte_count: 68
; boundary: cfg_blocks_5_terminals_1
; terminal: ret:1
; direct_callees: none
; indirect_calls: 0
; routine_bytes_sha256: 96f1c02162c947f4c90f79d909f4aa38ed61c9a0b29e8ff07677ed5d7ecde293

001A5C:  66 0F BF 44 40               movsx    eax, word ptr [si + 0x40]
001A61:  66 0F BF 5C 38               movsx    ebx, word ptr [si + 0x38]
001A66:  66 0F B7 16 FC 22            movzx    edx, word ptr [0x22fc]
001A6C:  66 2B C2                     sub      eax, edx
001A6F:  66 2D E8 03 00 00            sub      eax, 0x3e8
001A75:  66 F7 D8                     neg      eax
001A78:  66 0F AF 5C 32               imul     ebx, dword ptr [si + 0x32]
001A7D:  66 0F AF 44 1A               imul     eax, dword ptr [si + 0x1a]
001A82:  66 03 C3                     add      eax, ebx
001A85:  B8 E0 FF                     mov      ax, 0xffe0
001A88:  79 03                        jns      0x1a8d
001A8A:  B8 20 00                     mov      ax, 0x20
001A8D:  01 44 50                     add      word ptr [si + 0x50], ax
001A90:  FF 4C 56                     dec      word ptr [si + 0x56]
001A93:  79 0A                        jns      0x1a9f
001A95:  C7 44 0E A0 1A               mov      word ptr [si + 0xe], 0x1aa0
001A9A:  C7 44 56 40 00               mov      word ptr [si + 0x56], 0x40
001A9F:  C3                           ret
