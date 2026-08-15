#include "../include/bloodprg_vm.h"

void CB_NEAR vm_op_cf_clear_state(void)
{
    vm_resume_state_gs = 0;
    vm_resume_value_gs = 0;
}
