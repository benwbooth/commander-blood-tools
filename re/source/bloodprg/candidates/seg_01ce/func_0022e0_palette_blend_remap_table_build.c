#include "../include/bloodprg_graphics.h"

#define BLOODPRG_PALETTE_COLORS 256u
#define BLOODPRG_PALETTE_COMPONENTS 3u
#define BLOODPRG_PALETTE_BLEND_LIMIT 0x0bb8u

cb_i16 CB_FAR palette_blend_remap_table_build(
        cb_i16 negative_percent,
        cb_u16 target_red,
        cb_u16 target_green,
        cb_u16 target_blue,
        volatile cb_u8 CB_GAME_DATA *table)
{
    cb_u16 best_distance;
    cb_i16 best_index;
    cb_u16 blended_blue;
    cb_u16 blended_green;
    cb_u16 blended_red;
    cb_u16 candidate;
    const volatile cb_u8 CB_GAME_DATA *candidate_color;
    cb_u16 delta;
    cb_u16 distance;
    cb_u16 percent;
    cb_u16 source;
    const volatile cb_u8 CB_GAME_DATA *source_color;
    cb_u16 source_weight;
    cb_u16 target_blue_scaled;
    cb_u16 target_green_scaled;
    cb_u16 target_red_scaled;

    percent = (cb_u16)(0u - (cb_u16)negative_percent);
    target_red_scaled = (cb_u16)(percent * target_red / 100u);
    target_green_scaled = (cb_u16)(percent * target_green / 100u);
    target_blue_scaled = (cb_u16)(percent * target_blue / 100u);
    source_weight = (cb_u16)(100u - percent);
    source_color = pbm_live_palette;

    for (source = 0; source < BLOODPRG_PALETTE_COLORS; ++source) {
        blended_red = (cb_u16)(
                (cb_u16)*source_color++ * source_weight / 100u
                + target_red_scaled);
        blended_green = (cb_u16)(
                (cb_u16)*source_color++ * source_weight / 100u
                + target_green_scaled);
        blended_blue = (cb_u16)(
                (cb_u16)*source_color++ * source_weight / 100u
                + target_blue_scaled);

        best_index = -1;
        best_distance = BLOODPRG_PALETTE_BLEND_LIMIT;
        candidate_color = pbm_live_palette;
        for (candidate = 0; candidate < BLOODPRG_PALETTE_COLORS;
                ++candidate) {
            delta = (cb_u16)(blended_red - *candidate_color++);
            if ((cb_i16)delta < 0) {
                delta = (cb_u16)(0u - delta);
            }
            distance = (cb_u16)(delta * delta);

            delta = (cb_u16)(blended_green - *candidate_color++);
            if ((cb_i16)delta < 0) {
                delta = (cb_u16)(0u - delta);
            }
            distance = (cb_u16)(distance + delta * delta);

            delta = (cb_u16)(blended_blue - *candidate_color++);
            if ((cb_i16)delta < 0) {
                delta = (cb_u16)(0u - delta);
            }
            distance = (cb_u16)(distance + delta * delta);

            if (distance <= best_distance) {
                best_distance = distance;
                best_index = (cb_i16)candidate;
            }
        }
        if (best_index >= 0) {
            *table = (cb_u8)best_index;
        }
        ++table;
    }
    return negative_percent;
}
