#include "../include/bloodprg_vm.h"

const cb_u8 CB_NEAR *CB_NEAR vm_op_shared_record_wildcard(
    const cb_u8 CB_NEAR *script_bytes)
{
    int inverted;
    cb_u8 opcode;
    cb_u16 offset;
    cb_u16 value;
    cb_u16 owner;
    volatile cb_u8 CB_FAR *record_base;
    volatile cb_u16 CB_FAR *field;

    record_base = vm_record_base;

    if ((vm_query_mode & 1u) != 0) {
        inverted = 0;
        if (*script_bytes == 0xa1u) {
            inverted = 1;
            ++script_bytes;
        }

        offset = *(const cb_u16 CB_NEAR *)script_bytes;
        script_bytes += sizeof(cb_u16);
        value = *(const cb_u16 CB_NEAR *)script_bytes;
        script_bytes += sizeof(cb_u16);
        if (value == vm_wildcard_ref_value) {
            value = 0xffffu;
        }

        field = (volatile cb_u16 CB_FAR *)(record_base + offset);
        if ((*field == value) == inverted) {
            return (const cb_u8 CB_NEAR *)vm_branch_fail();
        }
        return script_bytes;
    }

    offset = *(const cb_u16 CB_NEAR *)script_bytes;
    script_bytes += sizeof(cb_u16);
    value = *(const cb_u16 CB_NEAR *)script_bytes;
    script_bytes += sizeof(cb_u16);
    opcode = script_bytes[-5];
    if (opcode == 0xbcu) {
        vm_branch_a = value;
    }

    field = (volatile cb_u16 CB_FAR *)(record_base + offset);
    if (*field == 0xffffu) {
        owner = vm_record_lookup_by_threshold(offset);
        vm_special_slot_remove(owner);
    } else if (value == vm_wildcard_ref_value || value == 0xffffu) {
        owner = vm_record_lookup_by_threshold(offset);
        if (!vm_special_slot_insert(owner)) {
            return script_bytes;
        }
        value = 0xffffu;
    }

    *field = value;
    return script_bytes;
}
