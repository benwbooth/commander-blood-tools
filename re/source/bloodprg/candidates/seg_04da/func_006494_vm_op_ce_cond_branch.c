#include "../include/bloodprg_vm.h"

void CB_NEAR vm_op_ce_cond_branch(void)
{
    if ((vm_ui_state_gs.bytes.flags & 1u) == 0) {
        vm_branch_fail();
    }
}
