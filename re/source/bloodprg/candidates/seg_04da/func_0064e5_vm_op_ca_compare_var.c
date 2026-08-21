#include "../include/bloodprg_vm.h"

bloodprg_vm_image_ptr CB_NEAR vm_op_ca_compare_var(
    bloodprg_vm_image_ptr script_bytes)
{
    cb_u8 operator;
    cb_i16 value;

    operator = (cb_u8)*(const volatile cb_u16 CB_FAR *)script_bytes;
    script_bytes += sizeof(cb_u16);
    value = (cb_i16)*(const volatile cb_u16 CB_FAR *)script_bytes;
    script_bytes += sizeof(cb_u16);

    if (operator == 0xf1u) {
        if (value > rtc_hour) {
            return script_bytes;
        }
    } else if (operator == 0xf2u) {
        if (value < rtc_hour) {
            return script_bytes;
        }
    } else if (value == rtc_hour) {
        return script_bytes;
    }

    return BLOODPRG_VM_CURSOR_AT(script_bytes, vm_branch_fail());
}
