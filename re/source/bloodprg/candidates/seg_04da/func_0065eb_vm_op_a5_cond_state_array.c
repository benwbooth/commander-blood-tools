#include "../include/bloodprg_vm.h"

const cb_u8 CB_NEAR *CB_NEAR vm_op_a5_cond_state_array(
    const cb_u8 CB_NEAR *script_bytes)
{
    cb_i16 index;

    index = (cb_i8)*script_bytes++;

    if ((vm_query_mode_gs & 1u) != 0) {
        if (vm_state_words_gs[index] != 0) {
            return (const cb_u8 CB_NEAR *)vm_branch_fail();
        }
    } else {
        vm_state_words_gs[index] = *(const cb_u16 CB_NEAR *)script_bytes;
        script_bytes += sizeof(cb_u16);
    }

    return script_bytes;
}
