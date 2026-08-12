#include "../include/bloodprg_vm.h"

cb_u16 CB_NEAR vm_branch_fail(void)
{
    cb_u16 target;

    vm_branch_stack_top = (cb_u16)(vm_branch_stack_top - 2u);
    target = *(volatile cb_u16 *)
        ((volatile cb_u8 *)vm_branch_stack + vm_branch_stack_top);
    vm_query_mode = 0;
    return target;
}
