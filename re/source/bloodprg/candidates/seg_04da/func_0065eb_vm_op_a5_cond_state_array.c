#include "../include/bloodprg_vm.h"

bloodprg_vm_image_ptr CB_NEAR vm_op_a5_cond_state_array(
    bloodprg_vm_image_ptr script_bytes)
{
    cb_i16 index;

    index = (cb_i8)*script_bytes++;

    if ((vm_query_mode_gs & 1u) != 0) {
        if (vm_state_words_gs[index] != 0) {
            return BLOODPRG_VM_CURSOR_AT(script_bytes, vm_branch_fail());
        }
    } else {
        vm_state_words_gs[index] =
            *(const volatile cb_u16 CB_FAR *)script_bytes;
        script_bytes += sizeof(cb_u16);
    }

    return script_bytes;
}
