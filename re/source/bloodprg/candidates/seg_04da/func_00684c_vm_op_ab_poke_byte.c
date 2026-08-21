#include "../include/bloodprg_vm.h"

bloodprg_vm_image_ptr CB_NEAR vm_op_ab_poke_byte(
    bloodprg_vm_image_ptr script_bytes)
{
    cb_u8 value;
    cb_u16 target_offset;
    volatile cb_u8 CB_FAR *target;

    value = *script_bytes++;
    target_offset = *(const volatile cb_u16 CB_FAR *)script_bytes;
    target = BLOODPRG_VM_CURSOR_AT(script_bytes, target_offset);
    *target = value;
    script_bytes += sizeof(target_offset);
    return script_bytes;
}
