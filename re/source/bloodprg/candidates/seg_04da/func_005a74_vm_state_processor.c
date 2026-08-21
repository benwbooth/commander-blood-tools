#include "../include/bloodprg_ship3d.h"

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#include <dos.h>
#define VM_STATE_RECORD_AT(base, offset) \
    ((volatile bloodprg_vm_state_record CB_FAR *)MK_FP( \
        FP_SEG(base), (offset)))
#else
#define VM_STATE_RECORD_AT(base, offset) \
    ((volatile bloodprg_vm_state_record *)((base) + (offset)))
#endif

void CB_NEAR vm_state_processor(void)
{
    volatile cb_u8 CB_FAR *record_base;
    bloodprg_vm_directory_ptr entry;
    volatile bloodprg_vm_state_record CB_FAR *record;
    volatile ship_3d_position_field CB_FAR *position;
    volatile ship_3d_position_field CB_FAR *comparison;
    cb_u16 record_offset;
    cb_u16 state;

    record_base = vm_record_base_gs;
    entry = vm_record_directory_gs;
    do {
        record_offset = entry->object_offset;
        record = VM_STATE_RECORD_AT(record_base, record_offset);
        if (record->kind == 2u) {
            state = record->state;
            if ((vm_presentation_request_flags_gs & 3u) == 0u &&
                    ((vm_text_display_active_gs & 1u) == 0u ||
                    (record_offset == vm_named_honk_object_gs &&
                    record_offset == vm_post_update_record_offset))) {
                state &= 0x7fefu;
            }

            position = ship_3d_position_field_resolve(
                    (volatile bloodprg_vm_object_header CB_FAR *)record,
                    state);
            comparison = ship_3d_position_field_resolve(
                    (volatile bloodprg_vm_object_header CB_FAR *)
                        VM_STATE_RECORD_AT(
                            record_base, vm_named_orxx_object_gs),
                    state);
            if (*(volatile cb_u32 CB_FAR *)position !=
                    *(volatile cb_u32 CB_FAR *)comparison) {
                comparison = ship_3d_position_field_resolve(
                        (volatile bloodprg_vm_object_header CB_FAR *)
                            VM_STATE_RECORD_AT(
                                record_base, vm_arche_record_offset_gs),
                        state);
                if (*(volatile cb_u32 CB_FAR *)position !=
                        *(volatile cb_u32 CB_FAR *)comparison) {
                    record->state = state;
                    ++entry;
                    continue;
                }
            }
            state |= 0x0010u;
            record->state = state;
        }
        ++entry;
    } while ((cb_u8)entry->entry_kind == 1u);
}
