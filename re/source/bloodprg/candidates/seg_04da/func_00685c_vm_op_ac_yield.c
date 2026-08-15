#include "../include/bloodprg_vm.h"

void CB_NEAR vm_op_ac_yield(void)
{
    vm_yield_flag_gs = 1;
}
