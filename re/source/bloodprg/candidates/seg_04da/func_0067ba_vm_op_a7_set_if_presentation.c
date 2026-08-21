#include "../include/bloodprg_vm.h"

bloodprg_vm_image_ptr CB_NEAR vm_op_a7_set_if_presentation(
    bloodprg_vm_image_ptr script_bytes)
{
    cb_u16 value;

    value = *(const volatile cb_u16 CB_FAR *)script_bytes;
    script_bytes += sizeof(cb_u16);

    if ((vm_presentation_active_gs & 1u) != 0) {
        vm_presentation_reg_6770_gs = value;
    }

    return script_bytes;
}
