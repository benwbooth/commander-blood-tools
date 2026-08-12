#include "../include/bloodprg_vm.h"

const cb_u8 CB_NEAR *CB_NEAR vm_op_a3_block(
    const cb_u8 CB_NEAR *script_bytes)
{
    cb_u8 inverted;
    cb_u16 target;
    cb_u16 match;

    if ((vm_block_scan_flags & 1u) != 0) {
        return vm_token_special(0, script_bytes);
    }

    inverted = 0;
    if (*script_bytes == 0xa1u) {
        inverted = 1;
        ++script_bytes;
    }

    target = *(const cb_u16 CB_NEAR *)script_bytes;
    script_bytes += 2;
    match = ((vm_resume_state & 2u) != 0) ? vm_resume_value : vm_block_match_value;

    if (match != 0) {
        if (inverted != 0) {
            if (target != match) {
                return script_bytes;
            }
        } else if (target == match) {
            return script_bytes;
        }
    }

    return (const cb_u8 CB_NEAR *)vm_branch_fail();
}
