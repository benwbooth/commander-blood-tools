; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x000bd7
; seg_off: 0000:05d7
; group: seg_0000
; provenance: relocation_proven_far_transfer_target
; label: counter_gate_b12
; label_comment: counter gate: al=gs:[0xb12]; if nonzero branch. A per-frame counter/state gate near the audio-timer state
; incoming: call@0x001792->0000:05d7
; incoming: call@0x001acb->0000:05d7
; incoming: call@0x008625->0000:05d7
; byte_count: 40
; boundary: cfg_blocks_7_terminals_2
; terminal: jmp 0xbfc:1, retf:1
; direct_callees: none
; indirect_calls: 0
; cxx_source: re/borland/bloodprg/seg_0000/func_000bd7_counter_gate_b12.cpp
; routine_bytes_sha256: a22e3eda60d05ab9a97801a19f060a212a2d5c4fd8f87d4318aff1bd504823c0

000BD7:  50                           push     ax
000BD8:  52                           push     dx
000BD9:  33 C0                        xor      ax, ax
000BDB:  65 A0 12 0B                  mov      al, byte ptr gs:[0xb12]
000BDF:  0A C0                        or       al, al
000BE1:  75 02                        jne      0xbe5
000BE3:  EB 17                        jmp      0xbfc
000BE5:  65 8B 16 9E 0A               mov      dx, word ptr gs:[0xa9e]
000BEA:  83 C2 06                     add      dx, 6
000BED:  FE C8                        dec      al
000BEF:  75 02                        jne      0xbf3
000BF1:  B4 08                        mov      ah, 8
000BF3:  EC                           in       al, dx
000BF4:  24 08                        and      al, 8
000BF6:  32 C4                        xor      al, ah
000BF8:  74 F9                        je       0xbf3
000BFA:  F6 D4                        not      ah
000BFC:  5A                           pop      dx
000BFD:  58                           pop      ax
000BFE:  CB                           retf    
