#include "../include/bloodprg_vm.h"

cb_u16 CB_NEAR vm_branch_fail(void)
{
    cb_u16 target;

    vm_branch_stack_top_gs = (cb_u16)(vm_branch_stack_top_gs - 2u);
    target = *(volatile cb_u16 CB_GAME_DATA *)
        ((volatile cb_u8 CB_GAME_DATA *)vm_branch_stack_gs
            + vm_branch_stack_top_gs);
    vm_query_mode_gs = 0;
    return target;
}
