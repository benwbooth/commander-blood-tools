#include "../include/bloodprg_vm.h"

bloodprg_vm_image_ptr CB_NEAR vm_op_a4_jump(
    bloodprg_vm_image_ptr script_bytes)
{
    cb_u16 target;

    target = *(const volatile cb_u16 CB_FAR *)script_bytes;
    vm_resume_state_gs = 0;
    vm_resume_value_gs = 0;
    return BLOODPRG_VM_CURSOR_AT(script_bytes, target);
}
