#include "../include/bloodprg_graphics.h"
#include "../include/bloodprg_vm.h"

#define VM_FIRST_OPCODE 0xA0u
#define VM_LAST_OPCODE 0xD2u
#define VM_BLOCK_END 0xFFu
#define VM_SKIP_COUNT_MASK 0x0Fu
#define VM_STOP_BLOCK 1u

cb_i16 CB_NEAR vm_script_block_scan(bloodprg_vm_image_ptr script_bytes)
{
    bloodprg_vm_image_ptr cursor;
    cb_u8 opcode;
    cb_u8 signal;

    cursor = script_bytes;
    for (;;) {
        opcode = *cursor++;
        if (opcode == VM_BLOCK_END) {
            return 0;
        }
        if (opcode < VM_FIRST_OPCODE || opcode > VM_LAST_OPCODE) {
            error_overlay_draw(0u, (const cb_u8 CB_FAR *)cursor);
            return -1;
        }

        vm_yield_flag = 0u;
        cursor = vm_opcode_handlers[opcode - VM_FIRST_OPCODE](cursor);
        signal = vm_yield_flag;
        if (signal != 0u) {
            if (signal == VM_STOP_BLOCK) {
                return 0;
            }
            vm_skip_count = 0u;
            continue;
        }

        if ((vm_skip_count & VM_SKIP_COUNT_MASK) != 0u) {
            do {
                cursor = vm_token_advance(cursor);
            } while (--vm_skip_count != 0u);
        }
    }
}
