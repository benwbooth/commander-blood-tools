#include "../include/bloodprg_vm.h"

const cb_u8 CB_NEAR *CB_NEAR vm_op_d0_cond_branch(
        const cb_u8 CB_NEAR *script_bytes)
{
    if ((vm_sequence_active_gs & 1u) == 0) {
        return (const cb_u8 CB_NEAR *)vm_branch_fail();
    }
    return script_bytes;
}
