#include "../include/bloodprg_vm.h"

bloodprg_vm_image_ptr CB_NEAR vm_op_ac_yield(
        bloodprg_vm_image_ptr script_bytes)
{
    vm_yield_flag_gs = 1;
    return script_bytes;
}
