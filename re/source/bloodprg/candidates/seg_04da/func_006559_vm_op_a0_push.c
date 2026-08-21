#include "../include/bloodprg_vm.h"

const cb_u16 CB_NEAR *CB_NEAR vm_op_a0_push(
    const cb_u16 CB_NEAR *script_words)
{
    cb_u16 top;
    cb_u16 target;

    vm_query_mode_gs = 1;
    top = vm_branch_stack_top_gs;
    vm_branch_stack_top_gs += 2u;
    target = *script_words++;
    *(volatile cb_u16 CB_GAME_DATA *)
        ((volatile cb_u8 CB_GAME_DATA *)vm_branch_stack_gs + top) = target;

    return script_words;
}
