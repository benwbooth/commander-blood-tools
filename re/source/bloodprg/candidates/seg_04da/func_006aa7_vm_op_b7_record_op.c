#include "../include/bloodprg_vm.h"

void CB_NEAR vm_op_b7_record_op(const cb_u8 **script_bytes)
{
    int inverted;
    cb_u16 offset;
    cb_u8 bit_index;
    cb_u8 mask;
    const cb_u16 *script_words;
    volatile cb_u8 CB_FAR *field;
    int is_set;

    inverted = 0;
    if (**script_bytes == 0xa1u) {
        inverted = 1;
        ++*script_bytes;
    }

    script_words = (const cb_u16 *)*script_bytes;
    offset = *script_words;
    *script_bytes = (const cb_u8 *)(script_words + 1);
    bit_index = **script_bytes;
    ++*script_bytes;

    field = vm_record_base + offset + (bit_index >> 3);
    mask = (cb_u8)(0x80u >> (bit_index & 7u));

    if ((vm_query_mode & 1u) != 0) {
        is_set = ((*field & mask) != 0);
        if (is_set == inverted) {
            vm_branch_fail();
        }
    } else if (!inverted) {
        *field |= mask;
    } else {
        *field &= (cb_u8)~mask;
    }
}
