#include "../include/bloodprg_vm.h"

const cb_u16 CB_NEAR *CB_NEAR vm_op_a0_push(
    const cb_u16 CB_NEAR *script_words)
{
    cb_u16 top;
    cb_u16 target;

    vm_query_mode = 1;
    top = vm_branch_stack_top;
    vm_branch_stack_top += 2u;
    target = *script_words++;
    *(volatile cb_u16 CB_NEAR *)
        ((volatile cb_u8 CB_NEAR *)vm_branch_stack + top) = target;

    return script_words;
}
