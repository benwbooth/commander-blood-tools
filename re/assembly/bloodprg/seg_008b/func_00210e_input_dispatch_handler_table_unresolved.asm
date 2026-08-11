; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x00210e
; seg_off: 008b:125e
; group: seg_008b
; provenance: recursive_graph
; label: input_dispatch_handler_table_UNRESOLVED
; label_comment: HONEST RETRACTION (2nd correction): sampling more jump-table entries (0x153e,0xc8f,0x5819,0x255e,0x5d06) shows MOST disassemble mid-instruction under the flat file=0x600+cs_offset mapping. So neither '16' nor the 'measured ~126' handler count is reliable, and the flat near-handler mapping is likely WRONG (the dispatcher's real CS base and/or near-vs-far call semantics are unverified). VERIFIED: only the dispatch MECHANISM at 0x210e (poll 0x1ce:0x39d -> xlatb via 0x113e -> call cs:[idx*2+0x123e]). NOT verified: the handler table's base/size/entries. The 3 'clean' handlers (0x652/0x6c0/0x899) may be coincidental alignment. Resolving needs the dispatcher's actual runtime CS + a jump-table entry whose target is cross-checked as a real routine entry || SUPERSEDED READING `input_action_dispatch`: main-loop per-frame input dispatcher: lcall 0x1ce:0x39d (poll event -> ax); form action byte (al, |0x80 for the high variant); al = xlat via table cs:0x113e (input->action index); if index>=0, call cs:[index*2 + 0x123e] = the action handler. Routes raw input to command handlers || SUPERSEDED READING `input_dispatch`: input command dispatch: lcall 0x1ce:0x39d (read input event -> ax); if AL==0 use AH; or al,0x80 for the alt path; bx=0x113e; xlatb (translate the code via DS:0x113e); if result negative (0x80 bit) ignore; else index*2 and call cs:[bx+0x123e] (handler table at file 0x183e). The central keyboard/command handler dispatcher || MERGED 2026-07-25 (audit-fixes #131): the superseded rows were left in the file beside their own correction, so a reader who found one had no pointer to the other.
; byte_count: 50
; boundary: cfg_blocks_6_terminals_1
; terminal: retf:1
; direct_callees: none
; indirect_calls: 2
; routine_bytes_sha256: 3c635d7aebb8a3fe33353a58934f677a59d093ec7906b1b57aa30bf0fe8cfd6d

00210E:  50                           push     ax
00210F:  53                           push     bx
002110:  52                           push     dx
002111:  C6 06 15 0B 00               mov      byte ptr [0xb15], 0
002116:  9A 9D 03 CE 01               lcall    0x1ce, 0x39d
00211B:  0B C0                        or       ax, ax
00211D:  74 1D                        je       0x213c
00211F:  0A C0                        or       al, al
002121:  8A D0                        mov      dl, al
002123:  75 04                        jne      0x2129
002125:  8A C4                        mov      al, ah
002127:  0C 80                        or       al, 0x80
002129:  BB 3E 11                     mov      bx, 0x113e
00212C:  2E D7                        xlatb   
00212E:  0A C0                        or       al, al
002130:  78 0A                        js       0x213c
002132:  98                           cwde    
002133:  03 C0                        add      ax, ax
002135:  8B D8                        mov      bx, ax
002137:  2E FF 97 3E 12               call     word ptr cs:[bx + 0x123e]
00213C:  5A                           pop      dx
00213D:  5B                           pop      bx
00213E:  58                           pop      ax
00213F:  CB                           retf    
