#include "../include/bloodprg_vm.h"

#define VM_TOKEN_END 0xffu
#define VM_BLOCK_END 0xaau
#define VM_TEXT_TOKEN 0xa6u
#define VM_TEXT_MARKED 0x80u

cb_u16 CB_NEAR vm_cod_scan(cb_u16 object_offset)
{
    bloodprg_vm_image_ptr token;
    bloodprg_vm_image_ptr record;
    cb_u16 saved_query_mode;
    cb_u16 kind;
    cb_u16 code_offset;
    cb_u8 opcode;

    saved_query_mode = vm_query_mode_word;
    token = vm_script_image;
    for (;;) {
        opcode = *token;
        if (opcode == VM_TOKEN_END) {
            break;
        }
        if (opcode == VM_TEXT_TOKEN &&
                *(volatile cb_u16 CB_FAR *)(token + 1u) ==
                    object_offset) {
            token[5] |= VM_TEXT_MARKED;
        }
        token = vm_token_advance(token);
    }

    vm_block_scan_flags = 1u;
    record = vm_record_base + object_offset;
    token = vm_code_image;
    kind = *(volatile cb_u16 CB_FAR *)record;
    record += vm_field_offset(2u, kind);
    code_offset = *(volatile cb_u16 CB_FAR *)record;

    if (code_offset != 0u) {
        token += code_offset;
        for (;;) {
            opcode = *token;
            if (opcode == VM_TOKEN_END || opcode == VM_BLOCK_END) {
                break;
            }
            if (opcode == VM_TEXT_TOKEN) {
                token[5] |= VM_TEXT_MARKED;
            }
            token = vm_token_advance(token);
        }
    }

    vm_query_mode_word = saved_query_mode;
    return kind;
}
