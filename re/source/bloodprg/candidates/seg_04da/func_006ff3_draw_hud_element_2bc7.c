#include <dos.h>

#include "../include/bloodprg_entity.h"
#include "../include/bloodprg_input.h"
#include "../include/bloodprg_resource.h"
#include "../include/bloodprg_ship3d.h"

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#define SHIP_3D_HUD_RECORD_AT(offset) \
    ((volatile cb_u8 CB_FAR *)MK_FP(FP_SEG(vm_record_base_gs), (offset)))
#else
#define SHIP_3D_HUD_RECORD_AT(offset) (vm_record_base_gs + (offset))
#endif

void CB_FAR draw_hud_element_2bc7(void)
{
    const volatile bloodprg_vm_directory_entry CB_FAR *directory;
    volatile ship_3d_hud_layout_entry CB_GAME_DATA *layout;
    volatile cb_u16 CB_GAME_DATA *match_output;
    const volatile cb_u32 CB_FAR *position;
    const volatile char CB_FAR *selected_name;
    cb_u32 current_position;
    cb_u16 current_offset;
    cb_u16 object_offset;
    cb_u16 object_kind;
    cb_u16 field_offset;
    cb_u16 match_count;
    int matched;

    entity_flag_state_transition(31u);

    layout = ship_3d_hud_layout;
    while (layout->name.first_word != 0u) {
        layout->active = 0u;
        ++layout;
    }

    current_offset = vm_arche_record_offset_gs;
    object_kind = *(const volatile cb_u16 CB_FAR *)
        SHIP_3D_HUD_RECORD_AT(current_offset);
    field_offset = (cb_u16)vm_field_offset(
        SHIP_3D_FIELD_SELECTOR_POSITION, object_kind);
    position = (const volatile cb_u32 CB_FAR *)SHIP_3D_HUD_RECORD_AT(
        (cb_u16)(current_offset + field_offset));
    current_position = *position;

    directory = vm_record_directory_gs;
    match_output = ship_3d_nav_source_offsets;
    match_count = 0u;
    do {
        object_offset = directory->object_offset;
        matched = 0;
        if (object_offset != current_offset) {
            object_kind = *(const volatile cb_u16 CB_FAR *)
                SHIP_3D_HUD_RECORD_AT(object_offset);
            if (object_kind == SHIP_3D_OBJECT_KIND_POSITION_KIND100) {
                field_offset = (cb_u16)vm_field_offset(
                    SHIP_3D_KIND100_POS_MATCH_FIELD,
                    object_kind);
                position = (const volatile cb_u32 CB_FAR *)
                    SHIP_3D_HUD_RECORD_AT(
                        (cb_u16)(object_offset + field_offset));
                matched = position[0] == current_position
                    || position[1] == current_position;
            } else {
                field_offset = (cb_u16)vm_field_offset(
                    SHIP_3D_FIELD_SELECTOR_POSITION, object_kind);
                if (field_offset != 0u) {
                    position = (const volatile cb_u32 CB_FAR *)
                        SHIP_3D_HUD_RECORD_AT(
                            (cb_u16)(object_offset + field_offset));
                    matched = *position == current_position;
                }
            }
        }
        if (matched) {
            *match_output++ = object_offset;
            ++match_count;
        }
        ++directory;
    } while (directory->entry_kind == BLOODPRG_VM_DIRECTORY_ACTIVE_KIND);
    *match_output = 0u;

    if (match_count == 0u) {
        return;
    }
    selected_name = (const volatile char CB_FAR *)
        SHIP_3D_HUD_RECORD_AT(
            (cb_u16)(ship_3d_nav_source_offsets[0] +
                SHIP_3D_OBJECT_NAME_OFFSET));

    layout = ship_3d_hud_layout;
    while (layout->name.first_word != 0u) {
        if (string_compare(
                (const volatile char CB_FAR *)layout->name.text,
                selected_name)) {
            layout->active = 1u;
            (void)resource_named_file_load(
                (cb_u16)(layout->resource_id | 0x8000u),
                resource_copy_buffer);
            entity_record_setter(
                layout->entity_id,
                resource_copy_buffer,
                (cb_u16)SHIP_3D_HUD_OFFSCREEN_COORD,
                (cb_u16)SHIP_3D_HUD_OFFSCREEN_COORD,
                0u);
            return;
        }
        ++layout;
    }
}

#undef SHIP_3D_HUD_RECORD_AT
