#include "../include/bloodprg_vm.h"

void CB_NEAR vm_op_d2_script_profile_request(const cb_i8 **script_bytes)
{
    cb_i8 operand;

    operand = **script_bytes;
    ++*script_bytes;
    vm_script_profile_request = (int)operand - 1;
}
