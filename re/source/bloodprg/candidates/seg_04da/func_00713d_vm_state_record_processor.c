#include "../include/bloodprg_ship3d.h"

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#include <dos.h>
#define VM_STATE_LIST_RECORD_AT(base, offset) \
    ((const volatile bloodprg_vm_object_header CB_FAR *)MK_FP( \
        FP_SEG(base), (offset)))
#define VM_STATE_LIST_WORD_AT(base, offset) \
    ((const volatile cb_u16 CB_FAR *)MK_FP(FP_SEG(base), (offset)))
#define VM_STATE_LIST_POSITION_AT(base, offset) \
    ((const volatile ship_3d_position_field CB_FAR *)MK_FP( \
        FP_SEG(base), (offset)))
#else
#define VM_STATE_LIST_RECORD_AT(base, offset) \
    ((const volatile bloodprg_vm_object_header *)((base) + (offset)))
#define VM_STATE_LIST_WORD_AT(base, offset) \
    ((const volatile cb_u16 *)((base) + (offset)))
#define VM_STATE_LIST_POSITION_AT(base, offset) \
    ((const volatile ship_3d_position_field *)((base) + (offset)))
#endif

void CB_SAVE_REGS CB_FAR vm_state_record_processor(void)
{
    volatile cb_u8 CB_FAR *record_base;
    bloodprg_vm_directory_ptr entry;
    const volatile bloodprg_vm_object_header CB_FAR *record;
    const volatile ship_3d_position_field CB_FAR *arche_position;
    const volatile ship_3d_position_field CB_FAR *position;
    volatile cb_u16 CB_GAME_DATA *output;
    cb_u16 arche_offset;
    cb_u16 candidate_offset;
    cb_u16 effective_offset;
    cb_u16 field_offset;
    cb_u16 kind;

    record_base = vm_record_base_gs;
    entry = vm_record_directory_gs;
    output = vm_arche_position_match_offsets;
    arche_offset = vm_arche_record_offset_gs;
    record = VM_STATE_LIST_RECORD_AT(record_base, arche_offset);
    field_offset = (cb_u16)vm_field_offset(
        SHIP_3D_FIELD_SELECTOR_POSITION, record->kind);
    arche_position = VM_STATE_LIST_POSITION_AT(
        record_base, (cb_u16)(arche_offset + field_offset));

    while ((entry->entry_kind & 0x00ffu) == 1u) {
        candidate_offset = entry->object_offset;
        record = VM_STATE_LIST_RECORD_AT(record_base, candidate_offset);
        kind = record->kind;
        if ((record->flags & 1u) == 0u ||
                (kind & SHIP_3D_PRESENTABLE_KIND_MASK) == 0u ||
                candidate_offset == arche_offset) {
            goto next_entry;
        }

        *output = candidate_offset;
        effective_offset = candidate_offset;
        if ((kind & 0x0080u) != 0u) {
            field_offset = (cb_u16)vm_field_offset(
                SHIP_3D_FIELD_SELECTOR_PARENT_LINK, 0x0080u);
            effective_offset = *VM_STATE_LIST_WORD_AT(
                record_base, (cb_u16)(candidate_offset + field_offset));
            record = VM_STATE_LIST_RECORD_AT(record_base, effective_offset);
            kind = record->kind;
            if ((record->flags & 1u) == 0u || (kind & 0x0018u) == 0u) {
                goto next_entry;
            }
        }

        field_offset = (cb_u16)vm_field_offset(
            SHIP_3D_FIELD_SELECTOR_POSITION, kind);
        position = VM_STATE_LIST_POSITION_AT(
            record_base, (cb_u16)(effective_offset + field_offset));
        if (*(const volatile cb_u32 CB_FAR *)position ==
                *(const volatile cb_u32 CB_FAR *)arche_position) {
            ++output;
        }

next_entry:
        ++entry;
    }

    *output = 0u;
}
