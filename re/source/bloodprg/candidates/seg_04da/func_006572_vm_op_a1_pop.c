#include "../include/bloodprg_vm.h"

void CB_NEAR vm_op_a1_pop(void)
{
    vm_query_mode = 0;
    if (vm_branch_stack_top != 2u) {
        vm_branch_stack_top -= 2u;
    }
}
