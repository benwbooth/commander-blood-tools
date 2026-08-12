#include "../include/bloodprg_vm.h"

const cb_u8 CB_NEAR *CB_NEAR vm_op_shared_state_marker(
    const cb_u8 CB_NEAR *script_bytes)
{
    cb_u16 offset;
    cb_u8 op;
    cb_u8 rhs_mode;
    cb_u16 rhs;
    cb_u16 current;
    volatile cb_u16 CB_FAR *field;
    int pass;

    offset = *(const cb_u16 CB_NEAR *)script_bytes;
    field = (volatile cb_u16 CB_FAR *)(vm_record_base + offset);
    current = *field;
    script_bytes += sizeof(cb_u16);

    op = *script_bytes++;
    rhs_mode = *script_bytes++;
    rhs = *(const cb_u16 CB_NEAR *)script_bytes;
    if (rhs_mode == 0xc0u || rhs_mode == 0xc2u) {
        rhs = *(volatile cb_u16 CB_FAR *)(vm_record_base + rhs);
    }
    script_bytes += sizeof(cb_u16);

    if ((vm_query_mode & 1u) != 0) {
        pass = 0;
        if (op == 0xf0u) {
            pass = current != rhs;
        } else if (op == 0xf3u) {
            pass = (cb_i16)current <= (cb_i16)rhs;
        } else if (op == 0xf4u) {
            pass = (cb_i16)current >= (cb_i16)rhs;
        } else if (op == 0xf1u) {
            pass = (cb_i16)current < (cb_i16)rhs;
        } else if (op == 0xf2u) {
            pass = (cb_i16)current > (cb_i16)rhs;
        } else if (op == 0xf5u) {
            pass = current == rhs;
        }

        if (!pass) {
            return (const cb_u8 CB_NEAR *)vm_branch_fail();
        }
    } else {
        if (op == 0xf6u) {
            current += rhs;
        } else if (op == 0xf7u) {
            current -= rhs;
        } else if (op == 0xf5u) {
            current = rhs;
        }
        *field = current;
    }

    return script_bytes;
}
