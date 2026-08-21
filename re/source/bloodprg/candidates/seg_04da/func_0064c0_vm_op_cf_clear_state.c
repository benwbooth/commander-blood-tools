#include "../include/bloodprg_vm.h"

const cb_u8 CB_NEAR *CB_NEAR vm_op_cf_clear_state(
        const cb_u8 CB_NEAR *script_bytes)
{
    vm_resume_state_gs = 0;
    vm_resume_value_gs = 0;
    return script_bytes;
}
