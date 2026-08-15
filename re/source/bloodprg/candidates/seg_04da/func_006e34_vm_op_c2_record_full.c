#include "../include/bloodprg_vm.h"

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#include <dos.h>
#define VM_C2_RECORD_AT(base, offset) \
    ((volatile cb_u8 CB_FAR *)MK_FP(FP_SEG(base), (offset)))
#else
#define VM_C2_RECORD_AT(base, offset) ((base) + (offset))
#endif

#define VM_C2_OWNER_ACTIVE_FLAG 0x01u
#define VM_C2_RELATED_PRESENTABLE_FLAG 0x20u
#define VM_C2_UI_BLOCK_FLAG 0x01u
#define VM_C2_REQUEST_BLOCK_FLAG 0x02u
#define VM_C2_RECORD_KIND 0x00c2u
#define VM_C2_SIMPLE_PRESENTATION_KIND 0x0002u
#define VM_C2_DESCRIPT_PRESENTATION_KIND 0x0400u
#define VM_C2_PARENT_FIELD_SELECTOR 0x0011u
#define VM_C2_SIMPLE_LINE 0x0027u
#define VM_C2_DESCRIPT_LINE 0x002bu
#define VM_C2_NAME_OFFSET 4u

const cb_u8 CB_NEAR *CB_NEAR vm_op_c2_record_full(
    const cb_u8 CB_NEAR *script_bytes)
{
    cb_u8 inverted;
    cb_u16 record_offset;
    cb_u16 owner_offset;
    cb_u16 related_offset;
    cb_u16 related_kind;
    cb_i16 field_offset;
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

    record_offset = *(const cb_u16 CB_NEAR *)script_bytes;
    script_bytes += sizeof(cb_u16);
    owner_offset = vm_record_lookup_by_threshold(record_offset);
    related_offset = *(const cb_u16 CB_NEAR *)script_bytes;
    script_bytes += sizeof(cb_u16);

    owner = (volatile bloodprg_vm_object_header CB_FAR *)VM_C2_RECORD_AT(
        record_base, owner_offset);
    record = (volatile bloodprg_vm_record_triple CB_FAR *)VM_C2_RECORD_AT(
        record_base, record_offset);
    if ((vm_query_mode_gs & 1u) != 0u) {
        if ((owner->flags & VM_C2_OWNER_ACTIVE_FLAG) != 0u
                && record->related == related_offset
                && record->kind == VM_C2_RECORD_KIND) {
            if (!inverted) {
                return script_bytes;
            }
        } else if (inverted) {
            return script_bytes;
        }
        return (const cb_u8 CB_NEAR *)vm_branch_fail();
    }

    if ((owner->flags & VM_C2_OWNER_ACTIVE_FLAG) == 0u) {
        return script_bytes;
    }
    related = (volatile bloodprg_vm_object_header CB_FAR *)VM_C2_RECORD_AT(
        record_base, related_offset);
    if ((related->flags & VM_C2_RELATED_PRESENTABLE_FLAG) == 0u) {
        return script_bytes;
    }
    if (!vm_special_slot_insert(related_offset)) {
        return script_bytes;
    }

    related_kind = related->kind;
    field_offset = (cb_i16)vm_field_offset(
        VM_C2_PARENT_FIELD_SELECTOR, related_kind);
    *(volatile cb_u16 CB_FAR *)VM_C2_RECORD_AT(
        record_base, (cb_u16)(related_offset + field_offset)) = 0xffffu;

    if ((vm_ui_state_gs.bytes.flags & VM_C2_UI_BLOCK_FLAG) != 0u
            || (vm_presentation_request_flags_gs
                & VM_C2_REQUEST_BLOCK_FLAG) != 0u) {
        return script_bytes;
    }
    if (related_kind == VM_C2_SIMPLE_PRESENTATION_KIND) {
        vm_c2_presentation_gate_gs = 0u;
        vm_active_line_gs = VM_C2_SIMPLE_LINE;
    } else if (related_kind == VM_C2_DESCRIPT_PRESENTATION_KIND
            && vm_c2_descript_lookup(
                (const volatile cb_u8 CB_FAR *)related
                    + VM_C2_NAME_OFFSET)) {
        vm_c2_presentation_gate_gs = 0u;
        vm_presentation_request_flags_gs |= VM_C2_REQUEST_BLOCK_FLAG;
        vm_active_line_gs = VM_C2_DESCRIPT_LINE;
    }
    return script_bytes;
}

#undef VM_C2_RECORD_AT
