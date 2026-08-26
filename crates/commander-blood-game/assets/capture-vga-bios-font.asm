; Capture the two VGA BIOS 8x8 ROM-font halves used by BLOODPRG.EXE.
bits 16
org 0x100

start:
    push cs
    pop ds
    mov dx, output_name
    xor cx, cx
    mov ah, 0x3c
    int 0x21
    jc failure
    mov [handle], ax

    mov ax, 0x1130
    mov bh, 3
    int 0x10
    push es
    pop ds
    mov dx, bp
    mov cx, 1024
    mov bx, [cs:handle]
    mov ah, 0x40
    int 0x21
    jc failure
    cmp ax, cx
    jne failure

    mov ax, 0x1130
    mov bh, 4
    int 0x10
    push es
    pop ds
    mov dx, bp
    mov cx, 1024
    mov bx, [cs:handle]
    mov ah, 0x40
    int 0x21
    jc failure
    cmp ax, cx
    jne failure

    mov bx, [cs:handle]
    mov ah, 0x3e
    int 0x21
    mov ax, 0x4c00
    int 0x21

failure:
    mov ax, 0x4c01
    int 0x21

handle dw 0
output_name db 'FONT8X8.BIN', 0
