#include "../include/bloodprg_vm.h"

const cb_u8 CB_NEAR *CB_NEAR vm_op_a1_pop(
    const cb_u8 CB_NEAR *script_bytes)
{
    vm_query_mode_gs = 0;
    if (vm_branch_stack_top_gs != 2u) {
        vm_branch_stack_top_gs -= 2u;
    }

    return script_bytes;
}
