#include "../include/bloodprg_vm.h"

bloodprg_vm_image_ptr CB_NEAR vm_op_cf_clear_state(
        bloodprg_vm_image_ptr script_bytes)
{
    vm_resume_state_gs = 0;
    vm_resume_value_gs = 0;
    return script_bytes;
}
