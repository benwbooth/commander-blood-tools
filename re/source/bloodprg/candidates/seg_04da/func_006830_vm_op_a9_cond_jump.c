#include "../include/bloodprg_vm.h"

void CB_NEAR vm_op_a9_cond_jump(const cb_u8 **script_bytes)
{
    cb_u8 flags;
    cb_u16 target;

    flags = **script_bytes;
    ++*script_bytes;
    target = *(const cb_u16 *)*script_bytes;

    if ((flags & 1u) == 0) {
        *script_bytes = (const cb_u8 *)(unsigned long)target;
    } else {
        vm_query_mode = 1;
        vm_branch_stack[0] = target;
        vm_branch_stack_top = 2;
        *script_bytes += 2;
    }
}
