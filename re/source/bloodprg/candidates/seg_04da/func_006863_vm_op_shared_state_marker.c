#include "../include/bloodprg_vm.h"

bloodprg_vm_image_ptr CB_NEAR vm_op_shared_state_marker(
    bloodprg_vm_image_ptr script_bytes)
{
    cb_u16 offset;
    cb_u8 op;
    cb_u8 rhs_mode;
    cb_u16 rhs;
    cb_u16 current;
    volatile cb_u8 CB_FAR *record_base;
    volatile cb_u16 CB_FAR *field;
    cb_u8 pass;

    record_base = vm_record_base_gs;
    offset = *(const volatile cb_u16 CB_FAR *)script_bytes;
    field = (volatile cb_u16 CB_FAR *)(record_base + offset);
    current = *field;
    script_bytes += sizeof(cb_u16);

    op = *script_bytes++;
    rhs_mode = *script_bytes++;
    rhs = *(const volatile cb_u16 CB_FAR *)script_bytes;
    if (rhs_mode == 0xc0u || rhs_mode == 0xc2u) {
        rhs = *(volatile cb_u16 CB_FAR *)(record_base + rhs);
    }
    script_bytes += sizeof(cb_u16);

    if ((vm_query_mode_gs & 1u) != 0) {
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
            return BLOODPRG_VM_CURSOR_AT(script_bytes, vm_branch_fail());
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
