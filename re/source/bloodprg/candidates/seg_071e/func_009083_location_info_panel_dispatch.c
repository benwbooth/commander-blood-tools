#include <dos.h>
#include <stddef.h>

#include "../include/bloodprg_graphics.h"
#include "../include/bloodprg_nav.h"
#include "../include/bloodprg_ship3d.h"

#define LOCATION_PANEL_OPENING 0x01u
#define LOCATION_PANEL_CLOSING 0x02u
#define LOCATION_PANEL_ENTITY 0u
#define LOCATION_PANEL_SPRITE_LAST 1u
#define LOCATION_PANEL_RESOURCE_FLAG 0x8000u
#define LOCATION_PANEL_SOURCE_WIDTH_NUMERATOR 14u
#define LOCATION_PANEL_SOURCE_WIDTH_SHIFT 5
#define LOCATION_PANEL_TEXT_X 110u
#define LOCATION_PANEL_TITLE_Y 25u
#define LOCATION_PANEL_TEXT_ROW_HEIGHT 10u
#define LOCATION_PANEL_NAME_GAP 6u
#define LOCATION_PANEL_TITLE_COLOR 0xeeu
#define LOCATION_PANEL_SOURCE_COLOR 0xfeu
#define LOCATION_PANEL_KIND_SHIP 0x0010u
#define LOCATION_PANEL_KIND_BLACK_HOLE 0x0100u
#define LOCATION_PANEL_SOURCE_KIND_BIT 0x0002u
#define LOCATION_PANEL_SOURCE_ACTIVE_BIT 0x0001u

typedef struct location_panel_object_record {
    cb_u16 kind;
    cb_u16 state;
    char name[0x32];
    cb_u16 life_support_visits;
} location_panel_object_record;

typedef char location_panel_object_name_must_be_at_4[
        offsetof(location_panel_object_record, name) == 4 ? 1 : -1];
typedef char location_panel_object_life_support_must_be_at_36[
        offsetof(location_panel_object_record, life_support_visits) == 0x36
            ? 1 : -1];

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#define LOCATION_PANEL_OBJECT_AT(offset) \
    ((volatile location_panel_object_record CB_FAR *) \
        MK_FP(FP_SEG(vm_record_base), (offset)))
#else
#define LOCATION_PANEL_OBJECT_AT(offset) \
    ((volatile location_panel_object_record CB_FAR *) \
        (vm_record_base + (offset)))
#endif

void CB_NEAR location_info_panel_dispatch(
        const volatile bloodprg_sprite_source_extent CB_FAR *comparison_extent)
{
    const bloodprg_location_panel_art_entry CB_NEAR *art;
    volatile location_panel_object_record CB_FAR *selected;
    volatile location_panel_object_record CB_FAR *source;
    const volatile cb_u16 CB_GAME_DATA *source_offset;
    const cb_u8 CB_FAR *title;
    cb_u16 text_x;
    cb_u16 text_y;
    int interpolation_complete;

    if ((nav_location_panel_transition_state & LOCATION_PANEL_OPENING) != 0u) {
        if (nav_location_panel_scale_step == 0u) {
            selected = LOCATION_PANEL_OBJECT_AT(nav_selected_location_record);
            art = nav_location_panel_art_table;
            while (art->name[0] != '\0') {
                if (string_compare(
                        (const volatile char CB_FAR *)art->name,
                        (const volatile char CB_FAR *)selected->name)) {
                    (void)resource_named_file_load(
                            (cb_u16)(art->resource_id
                                | LOCATION_PANEL_RESOURCE_FLAG),
                            resource_copy_buffer);
                    entity_record_setter(
                            LOCATION_PANEL_ENTITY,
                            resource_copy_buffer,
                            (cb_u16)mouse_x,
                            (cb_u16)mouse_y,
                            0u);
                    nav_location_panel_source_width = (cb_u16)(
                            ((cb_u16)(cb_u8)
                                bloodprg_entity_table_ds[0].frame->stride
                                * LOCATION_PANEL_SOURCE_WIDTH_NUMERATOR)
                            >> LOCATION_PANEL_SOURCE_WIDTH_SHIFT);
                    (void)palette_blend_remap_table_build(
                            -50, 0u, 0u, 0u, graphics_span_remap_table);
                    break;
                }
                ++art;
            }
        }

        ++nav_location_panel_scale_step;
        entity_draw_full(comparison_extent);
        sprite_slot_dirty_range_render(
                LOCATION_PANEL_ENTITY, LOCATION_PANEL_SPRITE_LAST);
        interpolation_complete = framebuffer_transition_current_step
                == framebuffer_transition_total_steps;
        framebuffer_rect_interpolate_and_remap_step(
                (const bloodprg_rect_i16 CB_NEAR *)
                    &nav_location_panel_target_rect,
                (const bloodprg_rect_i16 CB_NEAR *)
                    &nav_location_panel_current_rect);
        if (!interpolation_complete) {
            return;
        }
        nav_location_panel_transition_state = 0u;
    }

    if ((nav_location_panel_transition_state & LOCATION_PANEL_CLOSING) == 0u) {
        if ((mouse_primary_pressed & 1u) != 0u) {
            nav_location_panel_active = 0u;
            nav_location_panel_transition_state = LOCATION_PANEL_CLOSING;
            framebuffer_transition_current_step = 0u;
            ++nav_location_panel_scale_step;
            goto close_panel;
        }

        sprite_slot_dirty_range_render(
                LOCATION_PANEL_ENTITY, LOCATION_PANEL_ENTITY);
#if defined(__WATCOMC__)
        framebuffer_rect_palette_remap_ds_bp(
                framebuffer_transition_remap_table,
                (cb_u16)nav_location_panel_target_rect.x,
                (cb_u16)nav_location_panel_target_rect.y,
                (cb_u16)nav_location_panel_target_rect.width,
                (cb_u16)nav_location_panel_target_rect.height);
#else
        framebuffer_rect_palette_remap(
                framebuffer_transition_remap_table,
                (cb_u16)nav_location_panel_target_rect.x,
                (cb_u16)nav_location_panel_target_rect.y,
                (cb_u16)nav_location_panel_target_rect.width,
                (cb_u16)nav_location_panel_target_rect.height);
#endif

        selected = LOCATION_PANEL_OBJECT_AT(nav_selected_location_record);
        title = nav_location_panel_planet_label;
        if ((selected->kind & LOCATION_PANEL_KIND_SHIP) != 0u) {
            title = nav_location_panel_ship_label;
        }
        if ((selected->kind & LOCATION_PANEL_KIND_BLACK_HOLE) != 0u) {
            title = nav_location_panel_black_hole_label;
        }

        text_x = LOCATION_PANEL_TEXT_X;
        text_y = LOCATION_PANEL_TITLE_Y;
        main_font_text_draw_display(
                title, text_x, text_y, LOCATION_PANEL_TITLE_COLOR);
        text_x = (cb_u16)(text_x + main_font_draw_width
                + LOCATION_PANEL_NAME_GAP);
        main_font_text_draw_display(
                (const cb_u8 CB_FAR *)selected->name,
                text_x,
                text_y,
                LOCATION_PANEL_TITLE_COLOR);

        text_x = LOCATION_PANEL_TEXT_X;
        text_y = (cb_u16)(text_y + LOCATION_PANEL_TEXT_ROW_HEIGHT);
        main_font_text_draw_display(
                nav_location_panel_life_support_label,
                text_x,
                text_y,
                LOCATION_PANEL_TITLE_COLOR);
        text_y = (cb_u16)(text_y + LOCATION_PANEL_TEXT_ROW_HEIGHT);

        ship_3d_nav_source_list_build_full(
                (const volatile bloodprg_vm_object_header CB_FAR *)selected,
                (cb_u16 CB_NEAR *)ship_3d_nav_source_offsets);
        source_offset = ship_3d_nav_source_offsets;
        for (;;) {
            cb_u16 offset;

            offset = *source_offset++;
            if (offset == 0xffffu) {
                break;
            }
            source = LOCATION_PANEL_OBJECT_AT(offset);
            if ((source->kind & LOCATION_PANEL_SOURCE_KIND_BIT) == 0u
                    || (source->state
                        & LOCATION_PANEL_SOURCE_ACTIVE_BIT) == 0u
                    || source->life_support_visits == 0u) {
                continue;
            }
            main_font_text_draw_display(
                    (const cb_u8 CB_FAR *)source->name,
                    text_x,
                    text_y,
                    LOCATION_PANEL_SOURCE_COLOR);
            text_y = (cb_u16)(text_y + LOCATION_PANEL_TEXT_ROW_HEIGHT);
        }
        return;
    }

close_panel:
    --nav_location_panel_scale_step;
    entity_draw_full(comparison_extent);
    sprite_slot_dirty_range_render(
            LOCATION_PANEL_ENTITY, LOCATION_PANEL_SPRITE_LAST);
    interpolation_complete = framebuffer_transition_current_step
            == framebuffer_transition_total_steps;
    framebuffer_rect_interpolate_and_remap_step(
            (const bloodprg_rect_i16 CB_NEAR *)
                &nav_location_panel_current_rect,
            (const bloodprg_rect_i16 CB_NEAR *)
                &nav_location_panel_target_rect);
    if (interpolation_complete) {
        entity_flag_state_transition(LOCATION_PANEL_ENTITY);
        nav_location_panel_transition_state = 0u;
        nav_selected_location_record = 0u;
        nav_deferred_record_link = 0u;
    }
}

#undef LOCATION_PANEL_OBJECT_AT
