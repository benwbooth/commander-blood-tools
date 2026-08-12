#include "../include/bloodprg_vm.h"

void CB_NEAR vm_op_shared_ae_b0_state(const cb_u8 **script_bytes)
{
    int inverted;
    cb_u16 offset;
    cb_u16 mask;
    const cb_u16 *script_words;
    volatile cb_u16 CB_FAR *field;
    int has_bits;

    inverted = 0;
    if (**script_bytes == 0xa1u) {
        inverted = 1;
        ++*script_bytes;
    }

    script_words = (const cb_u16 *)*script_bytes;
    offset = *script_words++;
    mask = *script_words++;
    *script_bytes = (const cb_u8 *)script_words;

    field = (volatile cb_u16 CB_FAR *)(vm_record_base + offset);
    if ((vm_query_mode & 1u) != 0) {
        has_bits = ((*field & mask) != 0);
        if (has_bits == inverted) {
            vm_branch_fail();
        }
    } else if (!inverted) {
        *field |= mask;
    } else {
        *field &= (cb_u16)~mask;
    }
}
