#include "../include/bloodprg_vm.h"

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#include <dos.h>
#define VM_C3_RECORD_AT(base, offset) \
    ((volatile cb_u8 CB_FAR *)MK_FP(FP_SEG(base), (offset)))
#else
#define VM_C3_RECORD_AT(base, offset) ((base) + (offset))
#endif

bloodprg_vm_image_ptr CB_NEAR vm_op_c3_state_record(
    bloodprg_vm_image_ptr script_bytes)
{
    cb_u8 inverted;
    cb_u16 record_offset;
    cb_u16 owner_offset;
    cb_u16 related_offset;
    volatile cb_u8 CB_FAR *record_base;
    volatile bloodprg_vm_object_header CB_FAR *owner;
    volatile bloodprg_vm_object_header CB_FAR *related;
    volatile bloodprg_vm_record_triple CB_FAR *record;

    record_base = vm_record_base_gs;
    inverted = 0;
    if (*script_bytes == 0xa1u) {
        inverted = 1;
        ++script_bytes;
    }

    record_offset = *(const volatile cb_u16 CB_FAR *)script_bytes;
    script_bytes += sizeof(cb_u16);
    owner_offset = vm_record_lookup_by_threshold(record_offset);
    related_offset = *(const volatile cb_u16 CB_FAR *)script_bytes;
    script_bytes += sizeof(cb_u16);

    owner = (volatile bloodprg_vm_object_header CB_FAR *)VM_C3_RECORD_AT(
        record_base, owner_offset);
    record = (volatile bloodprg_vm_record_triple CB_FAR *)VM_C3_RECORD_AT(
        record_base, record_offset);

    if ((vm_query_mode_gs & 1u) != 0u) {
        if ((owner->flags & 1u) != 0u
                && record->related == related_offset
                && record->kind == 0x00c3u) {
            if (!inverted) {
                return script_bytes;
            }
        } else if (inverted) {
            return script_bytes;
        }
        return BLOODPRG_VM_CURSOR_AT(script_bytes, vm_branch_fail());
    }

    related = (volatile bloodprg_vm_object_header CB_FAR *)
        VM_C3_RECORD_AT(record_base, related_offset);
    if ((owner->flags & 1u) == 0u
            || (related->flags & 1u) == 0u
            || record->kind == BLOODPRG_VM_RECORD_C4) {
        return BLOODPRG_VM_CURSOR_AT(script_bytes, vm_branch_fail());
    }
    record->kind = 0x00c3u;
    record->related = related_offset;
    record->value = 1u;
    return script_bytes;
}
