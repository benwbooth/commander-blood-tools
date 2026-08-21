#include "../include/bloodprg_vm.h"

bloodprg_vm_image_ptr CB_NEAR vm_op_a1_pop(
    bloodprg_vm_image_ptr script_bytes)
{
    vm_query_mode_gs = 0;
    if (vm_branch_stack_top_gs != 2u) {
        vm_branch_stack_top_gs -= 2u;
    }

    return script_bytes;
}
