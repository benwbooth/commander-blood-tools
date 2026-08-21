#include "../include/bloodprg_vm.h"

bloodprg_vm_image_ptr CB_NEAR vm_op_a3_block(
    bloodprg_vm_image_ptr script_bytes)
{
    cb_u8 inverted;
    cb_u16 target;
    cb_u16 match;

    if ((vm_block_scan_flags_gs & 1u) != 0) {
        return vm_token_special(0, script_bytes);
    }

    inverted = 0;
    if (*script_bytes == 0xa1u) {
        inverted = 1;
        ++script_bytes;
    }

    target = *(const volatile cb_u16 CB_FAR *)script_bytes;
    script_bytes += 2;
    match = ((vm_resume_state_gs & 2u) != 0)
        ? vm_resume_value_gs
        : vm_block_match_value_gs;

    if (match != 0) {
        if (inverted != 0) {
            if (target != match) {
                return script_bytes;
            }
        } else if (target == match) {
            return script_bytes;
        }
    }

    return BLOODPRG_VM_CURSOR_AT(script_bytes, vm_branch_fail());
}
