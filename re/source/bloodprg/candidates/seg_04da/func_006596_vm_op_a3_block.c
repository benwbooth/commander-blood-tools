#include "../include/bloodprg_vm.h"

void CB_NEAR vm_op_a3_block(const cb_u8 **script_bytes)
{
    int inverted;
    cb_u16 target;
    cb_u16 match;

    if ((vm_block_scan_flags & 1u) != 0) {
        *script_bytes = vm_token_special(0, *script_bytes);
        return;
    }

    inverted = 0;
    if (**script_bytes == 0xa1u) {
        inverted = 1;
        ++*script_bytes;
    }

    target = *(const cb_u16 *)*script_bytes;
    *script_bytes += 2;
    match = ((vm_resume_state & 2u) != 0) ? vm_resume_value : vm_block_match_value;

    if (match == 0 || ((target == match) == inverted)) {
        vm_branch_fail();
    }
}
