#include "../include/bloodprg_entity.h"

void CB_NEAR sprite_blit_rle_opaque(
        volatile bloodprg_entity_record *record)
{
    const volatile bloodprg_sprite_frame CB_FAR *frame;
    const volatile cb_u8 CB_FAR *source;
    const volatile cb_u8 CB_FAR *literal;
    volatile cb_u8 CB_FAR *destination;
    volatile cb_u8 CB_FAR *row_destination;
    cb_i16 sprite_left;
    cb_i16 sprite_top;
    cb_i16 sprite_right;
    cb_i16 sprite_bottom;
    cb_i16 destination_x;
    cb_i16 destination_y;
    cb_i16 destination_step;
    cb_i16 row_step;
    cb_i8 control;
    cb_u8 pixel;
    cb_u16 draw_width;
    cb_u16 draw_height;
    cb_u16 clipped;
    cb_u16 rows;
    cb_u16 skip_rows;
    cb_u16 decoded;
    cb_u16 run_start;
    cb_u16 run_end;
    cb_u16 run_length;
    cb_u16 visible_start;
    cb_u16 visible_end;
    cb_u16 copy_start;
    cb_u16 copy_end;
    cb_u16 columns;
    cb_u16 frame_stride;
    cb_u16 left_clip;
    cb_u16 right_clip;
    cb_u8 flip_x;
    cb_u8 flip_y;

    frame = record->frame;
    flip_x = (cb_u8)((record->flags & 0x0020u) != 0u);
    flip_y = (cb_u8)((record->flags & 0x0040u) != 0u);
    sprite_top = (cb_i16)(record->draw_y + (cb_u16)frame->y_offset);
    sprite_right = (cb_i16)(record->draw_x + record->extent_width +
            (cb_u16)frame->x_offset);
    sprite_bottom = (cb_i16)(record->draw_y + record->extent_height +
            (cb_u16)frame->y_offset);
    frame_stride = frame->stride;
    left_clip = 0u;
    right_clip = 0u;
    source = (const volatile cb_u8 CB_FAR *)frame + 8u;
    draw_width = record->extent_width;
    draw_height = record->extent_height;
    destination_y = sprite_top;

    if (sprite_top < (cb_i16)record->dirty_rect.top) {
        clipped = (cb_u16)((cb_i16)record->dirty_rect.top - sprite_top);
        draw_height = (cb_u16)(draw_height - clipped);
        if ((flip_y & 1u) == 0u) {
            skip_rows = clipped;
            do {
                decoded = frame_stride;
                do {
                    control = (cb_i8)*source++;
                    if (control < 0) {
                        run_length = (cb_u16)(-(cb_i16)control + 1);
                        ++source;
                    } else {
                        run_length = (cb_u16)((cb_u8)control + 1u);
                        source += run_length;
                    }
                    decoded = (cb_u16)(decoded - run_length);
                } while (decoded != 0u);
                --skip_rows;
            } while (skip_rows != 0u);
        }
        destination_y = (cb_i16)record->dirty_rect.top;
    }
    if (sprite_bottom > (cb_i16)record->dirty_rect.bottom) {
        clipped = (cb_u16)(sprite_bottom -
                (cb_i16)record->dirty_rect.bottom);
        draw_height = (cb_u16)(draw_height - clipped);
        if ((flip_y & 1u) != 0u) {
            skip_rows = clipped;
            do {
                decoded = frame_stride;
                do {
                    control = (cb_i8)*source++;
                    if (control < 0) {
                        run_length = (cb_u16)(-(cb_i16)control + 1);
                        ++source;
                    } else {
                        run_length = (cb_u16)((cb_u8)control + 1u);
                        source += run_length;
                    }
                    decoded = (cb_u16)(decoded - run_length);
                } while (decoded != 0u);
                --skip_rows;
            } while (skip_rows != 0u);
        }
    }

    /* Vertical row skipping makes this a literal read behind the RLE cursor. */
    sprite_left = (cb_i16)(record->draw_x + (cb_u16)
            *(const volatile cb_i16 CB_FAR *)(source - 4u));
    destination_x = sprite_left;
    if (sprite_left < (cb_i16)record->dirty_rect.left) {
        clipped = (cb_u16)((cb_i16)record->dirty_rect.left - sprite_left);
        draw_width = (cb_u16)(draw_width - clipped);
        left_clip = clipped;
        destination_x = (cb_i16)record->dirty_rect.left;
    }
    if (sprite_right >= (cb_i16)record->dirty_rect.right) {
        clipped = (cb_u16)(sprite_right -
                (cb_i16)record->dirty_rect.right);
        draw_width = (cb_u16)(draw_width - clipped);
        right_clip = clipped;
    }

    if ((flip_y & 1u) != 0u) {
        destination_y = (cb_i16)(destination_y + draw_height - 1u);
    }
    if (flip_y == 0u) {
        row_step = 320;
    } else {
        row_step = -320;
    }
    if (flip_x != 0u) {
        destination_x = (cb_i16)(destination_x + draw_width - 1u);
        destination_step = -1;
        visible_start = right_clip;
    } else {
        destination_step = 1;
        visible_start = left_clip;
    }
    visible_end = (cb_u16)(visible_start + draw_width);
    row_destination = bloodprg_display_buffer +
            (cb_u16)((cb_u16)destination_y * 320u +
                    (cb_u16)destination_x);

    rows = draw_height;
    do {
        destination = row_destination;
        decoded = 0u;
        do {
            control = (cb_i8)*source++;
            run_start = decoded;
            if (control < 0) {
                run_length = (cb_u16)(-(cb_i16)control + 1);
                pixel = *source++;
                literal = 0;
            } else {
                run_length = (cb_u16)((cb_u8)control + 1u);
                literal = source;
                source += run_length;
            }
            run_end = (cb_u16)(run_start + run_length);

            copy_start = run_start;
            if (copy_start < visible_start) {
                copy_start = visible_start;
            }
            copy_end = run_end;
            if (copy_end > visible_end) {
                copy_end = visible_end;
            }
            if (copy_start < copy_end) {
                columns = (cb_u16)(copy_end - copy_start);
                if (literal != 0) {
                    literal += (cb_u16)(copy_start - run_start);
                    do {
                        *destination = *literal++;
                        destination += destination_step;
                        --columns;
                    } while (columns != 0u);
                } else {
                    do {
                        *destination = pixel;
                        destination += destination_step;
                        --columns;
                    } while (columns != 0u);
                }
            }
            decoded = run_end;
        } while (decoded != frame_stride);

        row_destination += row_step;
        --rows;
    } while (rows != 0u);
}
