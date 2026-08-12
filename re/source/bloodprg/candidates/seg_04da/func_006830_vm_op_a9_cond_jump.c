#include "../include/bloodprg_vm.h"

const cb_u8 CB_NEAR *CB_NEAR vm_op_a9_cond_jump(
    const cb_u8 CB_NEAR *script_bytes)
{
    cb_u8 flags;
    cb_u16 target;

    flags = *script_bytes++;

    if ((flags & 1u) == 0) {
        return (const cb_u8 CB_NEAR *)
            *(const cb_u16 CB_NEAR *)script_bytes;
    }

    vm_query_mode = 1;
    target = *(const cb_u16 CB_NEAR *)script_bytes;
    script_bytes += sizeof(cb_u16);
    vm_branch_stack[0] = target;
    vm_branch_stack_top = 2;
    return script_bytes;
}
