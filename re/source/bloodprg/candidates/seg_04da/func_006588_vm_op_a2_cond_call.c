#include "../include/bloodprg_vm.h"

bloodprg_vm_image_ptr CB_NEAR vm_op_a2_cond_call(
    bloodprg_vm_image_ptr script_bytes)
{
    cb_u16 modulus;

    modulus = *(const volatile cb_u16 CB_FAR *)script_bytes;
    script_bytes += sizeof(cb_u16);

    if (blood_prng_next(modulus) != 0) {
        return BLOODPRG_VM_CURSOR_AT(script_bytes, vm_branch_fail());
    }

    return script_bytes;
}
