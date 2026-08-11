#include "../include/bloodprg_vm.h"

void CB_NEAR vm_op_a5_cond_state_array(const cb_u8 **script_bytes)
{
    cb_u8 index;
    const cb_u16 *script_words;

    index = **script_bytes;
    ++*script_bytes;

    if ((vm_query_mode & 1u) != 0) {
        if (vm_state_words[index] != 0) {
            vm_branch_fail();
        }
    } else {
        script_words = (const cb_u16 *)*script_bytes;
        vm_state_words[index] = *script_words;
        *script_bytes += 2;
    }
}
