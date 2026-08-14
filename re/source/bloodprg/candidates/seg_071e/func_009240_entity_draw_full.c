#include "../include/bloodprg_nav.h"

#define LOCATION_PANEL_SCALE_NUMERATOR 3u
#define LOCATION_PANEL_SCALE_SHIFT 1
#define LOCATION_PANEL_EXTENT_SHIFT 4
#define LOCATION_PANEL_POSITION_DIVISOR 13
#define LOCATION_PANEL_Y_BIAS 10u

void CB_NEAR entity_draw_full(
        const volatile bloodprg_sprite_source_extent CB_FAR *comparison_extent)
{
    const volatile bloodprg_sprite_source_extent CB_FAR *source_extent;
    cb_u16 scaled_width;
    cb_u16 scaled_height;
    cb_u16 current_x;
    cb_u16 current_y;
    cb_u16 target_x;
    cb_u16 target_y;
    cb_u16 source_width;
    cb_u16 draw_x;
    cb_u16 draw_y;
    cb_i16 delta_x;
    cb_i16 delta_y;
    cb_i8 step_x;
    cb_i8 step_y;
    cb_i8 signed_scale;
    cb_u8 scale;

    source_extent =
            (const volatile bloodprg_sprite_source_extent CB_FAR *)
            bloodprg_entity_table_ds[0].frame;
    scale = (cb_u8)((
            (cb_u8)(nav_location_panel_scale_step *
                LOCATION_PANEL_SCALE_NUMERATOR) >>
            LOCATION_PANEL_SCALE_SHIFT) + 1u);
    scaled_width = (cb_u16)(
            ((cb_u16)(cb_u8)source_extent->width * (cb_u16)scale) >>
            LOCATION_PANEL_EXTENT_SHIFT);
    scaled_height = (cb_u16)(
            ((cb_u16)(cb_u8)source_extent->height * (cb_u16)scale) >>
            LOCATION_PANEL_EXTENT_SHIFT);

    sprite_slot_extent_update(
            0u, scaled_width, scaled_height, comparison_extent);

    signed_scale = (cb_i8)scale;
    current_x = (cb_u16)nav_location_panel_current_rect.x;
    target_x = (cb_u16)nav_location_panel_target_rect.x;
    source_width = nav_location_panel_source_width;
    delta_x = (cb_i16)(cb_u16)(target_x - source_width - current_x);
    step_x = (cb_i8)(delta_x / LOCATION_PANEL_POSITION_DIVISOR);
    draw_x = (cb_u16)(current_x +
            (cb_u16)((cb_i16)step_x * (cb_i16)signed_scale));

    current_y = (cb_u16)nav_location_panel_current_rect.y;
    target_y = (cb_u16)nav_location_panel_target_rect.y;
    delta_y = (cb_i16)(cb_u16)(
            target_y + LOCATION_PANEL_Y_BIAS - current_y);
    step_y = (cb_i8)(delta_y / LOCATION_PANEL_POSITION_DIVISOR);
    draw_y = (cb_u16)(current_y +
            (cb_u16)((cb_i16)step_y * (cb_i16)signed_scale));

    sprite_slot_position_update(0u, draw_x, draw_y);
}
