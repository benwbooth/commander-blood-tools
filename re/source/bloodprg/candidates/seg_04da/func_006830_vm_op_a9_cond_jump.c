#include "../include/bloodprg_vm.h"

bloodprg_vm_image_ptr CB_NEAR vm_op_a9_cond_jump(
    bloodprg_vm_image_ptr script_bytes)
{
    cb_u8 flags;
    cb_u16 target;

    flags = *script_bytes++;

    if ((flags & 1u) == 0) {
        return BLOODPRG_VM_CURSOR_AT(script_bytes,
            *(const volatile cb_u16 CB_FAR *)script_bytes);
    }

    vm_query_mode_gs = 1;
    target = *(const volatile cb_u16 CB_FAR *)script_bytes;
    script_bytes += sizeof(cb_u16);
    vm_branch_stack_gs[0] = target;
    vm_branch_stack_top_gs = 2;
    return script_bytes;
}
