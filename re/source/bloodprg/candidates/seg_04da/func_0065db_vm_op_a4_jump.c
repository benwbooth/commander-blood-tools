#include "../include/bloodprg_vm.h"

const cb_u8 CB_NEAR *CB_NEAR vm_op_a4_jump(
    const cb_u16 CB_NEAR *script_words)
{
    const cb_u8 CB_NEAR *target;

    target = (const cb_u8 CB_NEAR *)*script_words;
    vm_resume_state_gs = 0;
    vm_resume_value_gs = 0;
    return target;
}
