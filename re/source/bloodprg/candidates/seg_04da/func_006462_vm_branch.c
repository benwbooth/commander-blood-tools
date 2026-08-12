#include "../include/bloodprg_vm.h"

cb_u16 CB_NEAR vm_branch_fail(void)
{
    vm_branch_stack_top = (cb_u16)(vm_branch_stack_top - 2u);
    vm_query_mode = 0;
    return vm_branch_stack[vm_branch_stack_top >> 1];
}
