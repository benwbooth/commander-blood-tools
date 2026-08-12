#include "../include/bloodprg_vm.h"

void CB_NEAR vm_op_shared_state_marker(const cb_u8 **script_bytes)
{
    cb_u16 offset;
    cb_u8 op;
    cb_u8 rhs_mode;
    cb_u16 rhs;
    cb_u16 current;
    const cb_u16 *script_words;
    volatile cb_u16 CB_FAR *field;
    int pass;

    script_words = (const cb_u16 *)*script_bytes;
    offset = *script_words;
    *script_bytes = (const cb_u8 *)(script_words + 1);

    field = (volatile cb_u16 CB_FAR *)(vm_record_base + offset);
    current = *field;

    op = **script_bytes;
    ++*script_bytes;
    rhs_mode = **script_bytes;
    ++*script_bytes;
    rhs = *(const cb_u16 *)*script_bytes;
    if (rhs_mode == 0xc0u || rhs_mode == 0xc2u) {
        rhs = *(volatile cb_u16 CB_FAR *)(vm_record_base + rhs);
    }
    *script_bytes += 2;

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
            vm_branch_fail();
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
}
