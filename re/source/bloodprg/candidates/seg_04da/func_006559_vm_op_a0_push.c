#include "../include/bloodprg_vm.h"

void CB_NEAR vm_op_a0_push(const cb_u16 **script_words)
{
    cb_u16 top;

    vm_query_mode = 1;
    top = vm_branch_stack_top;
    vm_branch_stack_top = top + 2u;
    vm_branch_stack[top >> 1] = **script_words;
    ++*script_words;
}
