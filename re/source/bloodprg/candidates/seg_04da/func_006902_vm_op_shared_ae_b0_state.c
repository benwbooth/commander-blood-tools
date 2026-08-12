#include "../include/bloodprg_vm.h"

const cb_u8 CB_NEAR *CB_NEAR vm_op_shared_ae_b0_state(
    const cb_u8 CB_NEAR *script_bytes)
{
    int inverted;
    cb_u16 offset;
    cb_u16 mask;
    volatile cb_u8 CB_FAR *record_base;
    volatile cb_u16 CB_FAR *field;
    int has_bits;

    record_base = vm_record_base;
    inverted = 0;
    if (*script_bytes == 0xa1u) {
        inverted = 1;
        ++script_bytes;
    }

    offset = *(const cb_u16 CB_NEAR *)script_bytes;
    script_bytes += sizeof(cb_u16);
    mask = *(const cb_u16 CB_NEAR *)script_bytes;
    script_bytes += sizeof(cb_u16);

    field = (volatile cb_u16 CB_FAR *)(record_base + offset);
    if ((vm_query_mode & 1u) != 0) {
        has_bits = ((*field & mask) != 0);
        if (has_bits == inverted) {
            return (const cb_u8 CB_NEAR *)vm_branch_fail();
        }
    } else if (!inverted) {
        *field |= mask;
    } else {
        *field &= (cb_u16)~mask;
    }

    return script_bytes;
}
