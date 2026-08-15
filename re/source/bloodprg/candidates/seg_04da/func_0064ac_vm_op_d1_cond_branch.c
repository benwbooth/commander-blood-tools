#include "../include/bloodprg_vm.h"

void CB_NEAR vm_op_d1_cond_branch(void)
{
    if ((vm_scene_gate_gs & 1u) == 0) {
        vm_branch_fail();
    }
}
