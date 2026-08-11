#include "../include/bloodprg_vm.h"

void CB_NEAR vm_op_aa_yield(void)
{
    vm_yield_flag = 1;
}
