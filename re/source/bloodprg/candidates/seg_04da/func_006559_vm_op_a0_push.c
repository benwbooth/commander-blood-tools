#include "../include/bloodprg_vm.h"

bloodprg_vm_image_ptr CB_NEAR vm_op_a0_push(
    bloodprg_vm_image_ptr script_bytes)
{
    cb_u16 top;
    cb_u16 target;

    vm_query_mode_gs = 1;
    top = vm_branch_stack_top_gs;
    vm_branch_stack_top_gs += 2u;
    target = *(const volatile cb_u16 CB_FAR *)script_bytes;
    script_bytes += sizeof(cb_u16);
    *(volatile cb_u16 CB_GAME_DATA *)
        ((volatile cb_u8 CB_GAME_DATA *)vm_branch_stack_gs + top) = target;

    return script_bytes;
}
