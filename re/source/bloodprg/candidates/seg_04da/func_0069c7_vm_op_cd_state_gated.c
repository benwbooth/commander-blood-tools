#include "../include/bloodprg_vm.h"

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#include <dos.h>
#define VM_CD_RECORD_AT(base, offset) \
    ((volatile cb_u8 CB_FAR *)MK_FP(FP_SEG(base), (offset)))
#else
#define VM_CD_RECORD_AT(base, offset) ((base) + (offset))
#endif

const cb_u8 CB_NEAR *CB_NEAR vm_op_cd_state_gated(
    const cb_u8 CB_NEAR *script_bytes)
{
    int inverted;
    cb_u16 first_record;
    cb_u16 second_record;
    cb_u16 third_record;
    cb_u16 owner;
    cb_u16 kind;
    cb_i16 field_offset;
    cb_u16 value;
    volatile cb_u8 CB_FAR *record_base;
    volatile cb_u16 CB_FAR *triple;

    record_base = vm_record_base;
    if ((vm_query_mode & 1u) != 0) {
        inverted = 0;
        if (*script_bytes == 0xa1u) {
            inverted = 1;
            ++script_bytes;
        }
        first_record = *(const cb_u16 CB_NEAR *)script_bytes;
        script_bytes += sizeof(cb_u16);
        second_record = *(const cb_u16 CB_NEAR *)script_bytes;
        script_bytes += sizeof(cb_u16);
        third_record = *(const cb_u16 CB_NEAR *)script_bytes;
        script_bytes += sizeof(cb_u16);

        triple = (volatile cb_u16 CB_FAR *)VM_CD_RECORD_AT(
            record_base, first_record);
        if ((triple[0] == 0xcdu
                && triple[1] == second_record
                && triple[2] == third_record) == inverted) {
            return (const cb_u8 CB_NEAR *)vm_branch_fail();
        }
        return script_bytes;
    }

    first_record = *(const cb_u16 CB_NEAR *)script_bytes;
    script_bytes += sizeof(cb_u16);
    owner = vm_record_lookup_by_threshold(first_record);
    second_record = *(const cb_u16 CB_NEAR *)script_bytes;
    script_bytes += sizeof(cb_u16);
    third_record = *(const cb_u16 CB_NEAR *)script_bytes;
    script_bytes += sizeof(cb_u16);

    (void)(*(volatile cb_u8 CB_FAR *)VM_CD_RECORD_AT(
        record_base, owner + 2u) & 1u);
    (void)(*(volatile cb_u8 CB_FAR *)VM_CD_RECORD_AT(
        record_base, third_record + 2u) & 1u);
    (void)(*(volatile cb_u8 CB_FAR *)VM_CD_RECORD_AT(
        record_base, second_record + 2u) & 1u);

    kind = *(volatile cb_u16 CB_FAR *)VM_CD_RECORD_AT(
        record_base, second_record);
    (void)vm_field_offset(0x11u, kind);

    if (owner == vm_wildcard_ref_value) {
        vm_special_slot_remove(second_record);
    }

    kind = *(volatile cb_u16 CB_FAR *)VM_CD_RECORD_AT(
        record_base, second_record);
    field_offset = (cb_i16)vm_field_offset(0x11u, kind);
    value = third_record;
    if (third_record == vm_wildcard_ref_value) {
        if (!vm_special_slot_insert(second_record)) {
            return script_bytes;
        }
        value = 0xffffu;
    }

    *(volatile cb_u16 CB_FAR *)VM_CD_RECORD_AT(
        record_base, (cb_u16)(second_record + field_offset)) = value;

    if ((vm_ui_flags & 1u) != 0
            || (vm_presentation_request_flags & 2u) != 0
            || kind != 0x0400u) {
        return script_bytes;
    }

    if (vm_c2_descript_lookup(
            VM_CD_RECORD_AT(record_base, second_record + 4u)) != 0) {
        vm_c2_presentation_gate = 0;
        vm_presentation_request_flags |= 2u;
        vm_active_line = 0x2bu;
    }

    return script_bytes;
}
