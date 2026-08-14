#include <dos.h>
#include <stddef.h>

#include "../include/bloodprg_entity.h"
#include "../include/bloodprg_input.h"
#include "../include/bloodprg_nav.h"
#include "../include/bloodprg_ship3d.h"

#define NAV_STATUS_HOVER_ENTITY 31u
#define NAV_STATUS_MODE_PENDING 0x01u
#define NAV_STATUS_MODE_VISIBLE 0x02u
#define NAV_STATUS_MODE_MASK 0x03u
#define NAV_STATUS_KIND_SHIP 0x0010u
#define NAV_STATUS_KIND_BLACK_HOLE 0x0100u
#define NAV_STATUS_SOURCE_KIND 0x0002u
#define NAV_STATUS_SOURCE_ACTIVE 0x0001u
#define NAV_STATUS_ARCHETYPE_LOCATION_OFFSET 0x0016u
#define NAV_STATUS_END 0xffffu
#define NAV_STATUS_LINE_END '\r'

typedef struct nav_status_object_record {
    cb_u16 kind;
    cb_u16 state;
    char name[0x14];
    cb_u16 location;
    cb_u8 reserved_1a[0x1c];
    cb_u16 life_support_visits;
} nav_status_object_record;

typedef char nav_status_object_name_must_be_at_4[
        offsetof(nav_status_object_record, name) == 4 ? 1 : -1];
typedef char nav_status_object_location_must_be_at_18[
        offsetof(nav_status_object_record, location) == 0x18 ? 1 : -1];
typedef char nav_status_object_life_support_must_be_at_36[
        offsetof(nav_status_object_record, life_support_visits) == 0x36
            ? 1 : -1];

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#define NAV_STATUS_OBJECT_AT(offset) \
    ((volatile nav_status_object_record CB_FAR *) \
        MK_FP(FP_SEG(vm_record_base), (offset)))
#else
#define NAV_STATUS_OBJECT_AT(offset) \
    ((volatile nav_status_object_record CB_FAR *) \
        (vm_record_base + (offset)))
#endif

void CB_NEAR nav_state_gate(void)
{
    const cb_u8 CB_NEAR *title;
    const volatile char CB_FAR *name;
    const volatile cb_u16 CB_GAME_DATA *source_offset;
    volatile bloodprg_entity_record *hover_entity;
    volatile nav_status_object_record CB_FAR *archetype;
    volatile nav_status_object_record CB_FAR *location;
    volatile nav_status_object_record CB_FAR *source;
    volatile char CB_GAME_DATA *output;
    cb_u16 location_offset;
    cb_u16 object_offset;
    cb_u16 ark_object;
    cb_u16 mouse_word;

    if ((nav_transition_pending & 1u) != 0u
            || (nav_camera_view_active & 1u) != 0u
            || (vm_presentation_active & 1u) != 0u) {
        return;
    }

    hover_entity = &bloodprg_entity_table_ds[NAV_STATUS_HOVER_ENTITY];
    if ((hover_entity->flags & BLOODPRG_ENTITY_STATE0_FLAG) == 0u) {
        return;
    }

    mouse_word = (cb_u16)mouse_x;
    if (mouse_word < hover_entity->draw_x
            || mouse_word > (cb_u16)(hover_entity->draw_x
                + hover_entity->extent_width)) {
        vm_subtitle_display_mode_ds = 0u;
        return;
    }
    mouse_word = (cb_u16)mouse_y;
    if (mouse_word < hover_entity->draw_y
            || mouse_word > (cb_u16)(hover_entity->draw_y
                + hover_entity->extent_height)) {
        vm_subtitle_display_mode_ds = 0u;
        return;
    }

    vm_subtitle_display_mode_ds |= NAV_STATUS_MODE_PENDING;
    if ((vm_subtitle_display_mode_ds & NAV_STATUS_MODE_MASK) == 0u
            || (vm_subtitle_display_mode_ds & NAV_STATUS_MODE_VISIBLE) != 0u) {
        return;
    }

    archetype = NAV_STATUS_OBJECT_AT(vm_arche_record_offset);
    location_offset = *(volatile cb_u16 CB_FAR *)
            ((volatile cb_u8 CB_FAR *)archetype
                + NAV_STATUS_ARCHETYPE_LOCATION_OFFSET);
    location = NAV_STATUS_OBJECT_AT(location_offset);

    title = nav_location_panel_planet_label;
    if (location->kind == NAV_STATUS_KIND_SHIP) {
        title = nav_location_panel_ship_label;
    }
    if ((location->kind & NAV_STATUS_KIND_BLACK_HOLE) != 0u) {
        title = nav_location_panel_black_hole_label;
    }

    output = vm_text_buffer_gs;
    while (*title != '\0') {
        *output++ = (char)*title++;
    }
    name = location->name;
    while (*name != '\0') {
        *output++ = *name++;
    }
    *output++ = NAV_STATUS_LINE_END;

    name = (const volatile char CB_GAME_DATA *)
            nav_location_panel_life_support_label_gs;
    while (*name != '\0') {
        *output++ = *name++;
    }
    *output++ = NAV_STATUS_LINE_END;

    ship_3d_nav_source_list_build_full(
            (const volatile bloodprg_vm_object_header CB_FAR *)location,
            (cb_u16 CB_NEAR *)ship_3d_nav_source_offsets);
    source_offset = ship_3d_nav_source_offsets;
    ark_object = vm_named_ark_object_gs;
    for (;;) {
        object_offset = *source_offset++;
        if (object_offset == NAV_STATUS_END) {
            break;
        }

        source = NAV_STATUS_OBJECT_AT(object_offset);
        if (source->kind != NAV_STATUS_SOURCE_KIND
                || (source->state & NAV_STATUS_SOURCE_ACTIVE) == 0u
                || source->life_support_visits == 0u
                || source->location == ark_object) {
            continue;
        }

        name = source->name;
        while (*name != '\0') {
            *output++ = *name++;
        }
        *output++ = NAV_STATUS_LINE_END;
    }

    *output++ = NAV_STATUS_LINE_END;
    *output = '\0';
    ++vm_subtitle_display_mode;
    vm_text_reveal_cursor_gs = 0u;
}

#undef NAV_STATUS_OBJECT_AT
