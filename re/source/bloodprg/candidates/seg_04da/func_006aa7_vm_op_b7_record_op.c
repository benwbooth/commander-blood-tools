#include "../include/bloodprg_vm.h"

bloodprg_vm_image_ptr CB_NEAR vm_op_b7_record_op(
    bloodprg_vm_image_ptr script_bytes)
{
    cb_u8 inverted;
    cb_u16 offset;
    cb_u8 bit_index;
    cb_u8 bit_in_byte;
    cb_u8 mask;
    volatile cb_u8 CB_FAR *record_base;
    volatile cb_u8 CB_FAR *field;

    record_base = vm_record_base_gs;
    inverted = 0;
    if (*script_bytes == 0xa1u) {
        inverted = 1;
        ++script_bytes;
    }

    offset = *(const volatile cb_u16 CB_FAR *)script_bytes;
    script_bytes += sizeof(cb_u16);
    bit_index = *script_bytes++;
    bit_in_byte = (cb_u8)(bit_index & 7u);
    field = record_base + (cb_u16)(offset + (bit_index >> 3));

    if ((vm_query_mode_gs & 1u) != 0) {
        if (((cb_u8)(*field << bit_in_byte) & 0x80u) != 0) {
            if (!inverted) {
                return script_bytes;
            }
        } else if (inverted) {
            return script_bytes;
        }
        return BLOODPRG_VM_CURSOR_AT(script_bytes, vm_branch_fail());
    }

    mask = (cb_u8)(1u << (7u - bit_in_byte));
    if (!inverted) {
        *field |= mask;
    } else {
        *field &= (cb_u8)~mask;
    }
    return script_bytes;
}
