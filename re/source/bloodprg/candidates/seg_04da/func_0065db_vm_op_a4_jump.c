#include "../include/bloodprg_vm.h"

cb_u16 CB_NEAR vm_op_a4_jump(const cb_u16 *script_words)
{
    cb_u16 target;

    target = *script_words;
    vm_resume_state = 0;
    vm_resume_value = 0;
    return target;
}
