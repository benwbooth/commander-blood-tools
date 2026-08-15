#include "../include/bloodprg_vm.h"

void CB_NEAR vm_op_d0_cond_branch(void)
{
    if ((vm_sequence_active_gs & 1u) == 0) {
        vm_branch_fail();
    }
}
