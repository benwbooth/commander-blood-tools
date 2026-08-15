#include "../include/bloodprg_vm.h"

cb_u16 CB_NEAR vm_op_a1_pop(void)
{
    cb_u16 top;

    vm_query_mode_gs = 0;
    top = vm_branch_stack_top_gs;
    if (top == 2u) {
        return top;
    }

    vm_branch_stack_top_gs -= 2u;
    return top;
}
