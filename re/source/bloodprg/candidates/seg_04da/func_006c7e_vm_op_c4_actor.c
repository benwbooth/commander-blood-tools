#include "../include/bloodprg_vm.h"

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#include <dos.h>
#define VM_C4_RECORD_AT(base, offset) \
    ((volatile cb_u8 CB_FAR *)MK_FP(FP_SEG(base), (offset)))
#else
#define VM_C4_RECORD_AT(base, offset) ((base) + (offset))
#endif

const cb_u8 CB_NEAR *CB_NEAR vm_op_c4_actor(
    const cb_u8 CB_NEAR *script_bytes)
{
    cb_u8 inverted;
    cb_u8 matches;
    cb_u16 record_offset;
    cb_u16 owner_offset;
    cb_u16 related_offset;
    cb_u16 owner_kind;
    cb_u16 related_kind;
    cb_u16 record_kind;
    cb_u16 reciprocal_offset;
    volatile cb_u8 CB_FAR *record_base;
    volatile bloodprg_vm_object_header CB_FAR *owner;
    volatile bloodprg_vm_object_header CB_FAR *related;
    volatile bloodprg_vm_record_triple CB_FAR *record;
    volatile cb_u16 CB_FAR *reciprocal;

    record_base = vm_record_base_gs;
    inverted = 0;
    if (*script_bytes == 0xa1u) {
        inverted = 1;
        ++script_bytes;
    }

    record_offset = *(const cb_u16 CB_NEAR *)script_bytes;
    script_bytes += sizeof(cb_u16);
    owner_offset = vm_record_lookup_by_threshold(record_offset);
    related_offset = *(const cb_u16 CB_NEAR *)script_bytes;
    script_bytes += sizeof(cb_u16);

    owner = (volatile bloodprg_vm_object_header CB_FAR *)VM_C4_RECORD_AT(
        record_base, owner_offset);
    related = (volatile bloodprg_vm_object_header CB_FAR *)VM_C4_RECORD_AT(
        record_base, related_offset);
    record = (volatile bloodprg_vm_record_triple CB_FAR *)VM_C4_RECORD_AT(
        record_base, record_offset);
    record_kind = record->kind;

    if ((vm_query_mode_gs & 1u) != 0u) {
        matches = (owner->flags & 1u) != 0u
            && record_kind == BLOODPRG_VM_RECORD_C4
            && record->related == related_offset;
        if (matches == inverted) {
            return (const cb_u8 CB_NEAR *)vm_branch_fail();
        }
        return script_bytes;
    }

    if ((owner->flags & 1u) == 0u || (related->flags & 1u) == 0u) {
        return (const cb_u8 CB_NEAR *)vm_branch_fail();
    }

    owner_kind = owner->kind;
    if (owner_kind != 1u) {
        related_kind = related->kind;
        if (related_kind != 1u) {
            if (record_kind == BLOODPRG_VM_RECORD_C4) {
                return (const cb_u8 CB_NEAR *)vm_branch_fail();
            }
            reciprocal_offset = (cb_u16)(related_offset
                + (cb_i16)vm_field_offset(
                    BLOODPRG_VM_RECIPROCAL_SELECTOR, related_kind));
            reciprocal = (volatile cb_u16 CB_FAR *)VM_C4_RECORD_AT(
                record_base, reciprocal_offset);
            if (*reciprocal == BLOODPRG_VM_RECORD_C4) {
                return (const cb_u8 CB_NEAR *)vm_branch_fail();
            }
        }
    }

    record->kind = BLOODPRG_VM_RECORD_C4;
    record->related = related_offset;
    record->value = 0u;
    return script_bytes;
}

#undef VM_C4_RECORD_AT
