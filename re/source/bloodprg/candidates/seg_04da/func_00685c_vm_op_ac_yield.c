#include "../include/bloodprg_vm.h"

const cb_u8 CB_NEAR *CB_NEAR vm_op_ac_yield(
        const cb_u8 CB_NEAR *script_bytes)
{
    vm_yield_flag_gs = 1;
    return script_bytes;
}
