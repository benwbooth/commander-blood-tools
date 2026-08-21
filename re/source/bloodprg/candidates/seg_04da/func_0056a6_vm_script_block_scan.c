#include "../include/bloodprg_graphics.h"
#include "../include/bloodprg_vm.h"

cb_i16 CB_NEAR vm_script_block_scan(bloodprg_vm_image_ptr script_bytes)
{
    bloodprg_vm_image_ptr cursor;
    cb_u8 opcode;
    cb_u8 signal;

    cursor = script_bytes;
    for (;;) {
        opcode = *cursor++;
        if (opcode == BLOODPRG_VM_STREAM_END) {
            return 0;
        }
        if (opcode < BLOODPRG_VM_OPCODE_MIN
                || opcode > BLOODPRG_VM_OPCODE_MAX) {
            error_overlay_draw(0u, (const cb_u8 CB_FAR *)cursor);
            return -1;
        }

        vm_yield_flag_gs = 0u;
        cursor = vm_opcode_dispatch(opcode, cursor);
        signal = vm_yield_flag_gs;
        if (signal != 0u) {
            if (signal == BLOODPRG_VM_YIELD_STOP_BLOCK) {
                return 0;
            }
            vm_skip_count_gs = 0u;
            continue;
        }

        if ((vm_skip_count_gs & BLOODPRG_VM_SKIP_COUNT_MASK) != 0u) {
            do {
                cursor = vm_token_advance(cursor);
            } while (--vm_skip_count_gs != 0u);
        }
    }
}
