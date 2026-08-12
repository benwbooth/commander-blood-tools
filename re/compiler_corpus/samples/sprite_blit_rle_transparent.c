/* Codegen probe for BLOODPRG 0x0046BC. */
typedef unsigned char u8;
typedef signed char i8;
typedef unsigned int u16;
typedef signed int i16;

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#define FAR far
#define NEAR near
#else
#define FAR
#define NEAR
#endif

typedef struct dirty_rect_probe {
    u16 left;
    u16 right;
    u16 top;
    u16 bottom;
} dirty_rect_probe;

typedef struct sprite_frame_probe {
    u16 stride;
    u16 height;
    i16 x_offset;
    i16 y_offset;
    u8 pixels[1];
} sprite_frame_probe;

typedef struct sprite_slot_probe {
    u16 flags;
    u16 field_02;
    const volatile sprite_frame_probe FAR *frame;
    u16 draw_x;
    u16 draw_y;
    u16 extent_width;
    u16 extent_height;
    u16 committed_draw_x;
    u16 committed_draw_y;
    u16 committed_extent_width;
    u16 committed_extent_height;
    dirty_rect_probe dirty_rect;
} sprite_slot_probe;

extern volatile u8 sprite_flip_x_probe;
extern volatile u8 sprite_flip_y_probe;
extern volatile u8 FAR *display_buffer_probe;
extern volatile u8 remap_5f11_probe[256];
extern volatile u8 remap_6011_probe[256];
extern volatile u8 NEAR *selected_remap_probe;
extern volatile u16 rle_stride_probe;
extern volatile u16 rle_left_clip_probe;
extern volatile u16 rle_right_clip_probe;

#if defined(__WATCOMC__)
#pragma aux sprite_blit_rle_transparent_probe parm [di] modify exact []
#endif

void NEAR sprite_blit_rle_transparent_probe(
        volatile sprite_slot_probe *record)
{
    const volatile sprite_frame_probe FAR *frame;
    const volatile u8 FAR *source;
    const volatile u8 FAR *literal;
    volatile u8 FAR *destination;
    volatile u8 FAR *row_destination;
    volatile u8 NEAR *remap;
    i16 sprite_left;
    i16 sprite_top;
    i16 sprite_right;
    i16 sprite_bottom;
    i16 destination_x;
    i16 destination_y;
    i16 destination_step;
    i16 row_step;
    i8 control;
    u8 pixel;
    u16 draw_width;
    u16 draw_height;
    u16 clipped;
    u16 rows;
    u16 skip_rows;
    u16 decoded;
    u16 run_start;
    u16 run_end;
    u16 run_length;
    u16 visible_start;
    u16 visible_end;
    u16 copy_start;
    u16 copy_end;
    u16 columns;

    frame = record->frame;
    sprite_top = (i16)(record->draw_y + (u16)frame->y_offset);
    sprite_right = (i16)(record->draw_x + record->extent_width +
            (u16)frame->x_offset);
    sprite_bottom = (i16)(record->draw_y + record->extent_height +
            (u16)frame->y_offset);
    rle_stride_probe = frame->stride;
    rle_left_clip_probe = 0u;
    rle_right_clip_probe = 0u;
    source = (const volatile u8 FAR *)frame + 8u;
    draw_width = record->extent_width;
    draw_height = record->extent_height;
    destination_y = sprite_top;

    if (sprite_top < (i16)record->dirty_rect.top) {
        clipped = (u16)((i16)record->dirty_rect.top - sprite_top);
        draw_height = (u16)(draw_height - clipped);
        if ((sprite_flip_y_probe & 1u) == 0u) {
            skip_rows = clipped;
            do {
                decoded = rle_stride_probe;
                do {
                    control = (i8)*source++;
                    if (control < 0) {
                        run_length = (u16)(-(i16)control + 1);
                        ++source;
                    } else {
                        run_length = (u16)((u8)control + 1u);
                        source += run_length;
                    }
                    decoded = (u16)(decoded - run_length);
                } while (decoded != 0u);
                --skip_rows;
            } while (skip_rows != 0u);
        }
        destination_y = (i16)record->dirty_rect.top;
    }
    if (sprite_bottom > (i16)record->dirty_rect.bottom) {
        clipped = (u16)(sprite_bottom - (i16)record->dirty_rect.bottom);
        draw_height = (u16)(draw_height - clipped);
        if ((sprite_flip_y_probe & 1u) != 0u) {
            skip_rows = clipped;
            do {
                decoded = rle_stride_probe;
                do {
                    control = (i8)*source++;
                    if (control < 0) {
                        run_length = (u16)(-(i16)control + 1);
                        ++source;
                    } else {
                        run_length = (u16)((u8)control + 1u);
                        source += run_length;
                    }
                    decoded = (u16)(decoded - run_length);
                } while (decoded != 0u);
                --skip_rows;
            } while (skip_rows != 0u);
        }
    }

    sprite_left = (i16)(record->draw_x + (u16)
            *(const volatile i16 FAR *)(source - 4u));
    destination_x = sprite_left;
    if (sprite_left < (i16)record->dirty_rect.left) {
        clipped = (u16)((i16)record->dirty_rect.left - sprite_left);
        draw_width = (u16)(draw_width - clipped);
        rle_left_clip_probe = clipped;
        destination_x = (i16)record->dirty_rect.left;
    }
    if (sprite_right >= (i16)record->dirty_rect.right) {
        clipped = (u16)(sprite_right - (i16)record->dirty_rect.right);
        draw_width = (u16)(draw_width - clipped);
        rle_right_clip_probe = clipped;
    }

    switch ((record->flags >> 8) & 3u) {
    case 0u:
        remap = 0;
        break;
    case 1u:
        remap = remap_5f11_probe;
        break;
    default:
        remap = remap_6011_probe;
        break;
    }
    selected_remap_probe = remap;

    if ((sprite_flip_y_probe & 1u) != 0u) {
        destination_y = (i16)(destination_y + draw_height - 1u);
    }
    if (sprite_flip_y_probe == 0u) {
        row_step = 320;
    } else {
        row_step = -320;
    }
    if (sprite_flip_x_probe != 0u) {
        destination_x = (i16)(destination_x + draw_width - 1u);
        destination_step = -1;
        visible_start = rle_right_clip_probe;
    } else {
        destination_step = 1;
        visible_start = rle_left_clip_probe;
    }
    visible_end = (u16)(visible_start + draw_width);
    row_destination = display_buffer_probe +
            (u16)((u16)destination_y * 320u + (u16)destination_x);

    rows = draw_height;
    do {
        destination = row_destination;
        decoded = 0u;
        do {
            control = (i8)*source++;
            run_start = decoded;
            if (control < 0) {
                run_length = (u16)(-(i16)control + 1);
                pixel = *source++;
                literal = 0;
            } else {
                run_length = (u16)((u8)control + 1u);
                literal = source;
                source += run_length;
            }
            run_end = (u16)(run_start + run_length);

            copy_start = run_start;
            if (copy_start < visible_start) {
                copy_start = visible_start;
            }
            copy_end = run_end;
            if (copy_end > visible_end) {
                copy_end = visible_end;
            }
            if (copy_start < copy_end) {
                columns = (u16)(copy_end - copy_start);
                if (literal != 0) {
                    literal += (u16)(copy_start - run_start);
                }
                do {
                    if (literal != 0) {
                        pixel = *literal++;
                    }
                    if (pixel != 0u) {
                        if (remap == 0) {
                            *destination = pixel;
                        } else {
                            *destination = remap[*destination];
                        }
                    }
                    destination += destination_step;
                    --columns;
                } while (columns != 0u);
            }
            decoded = run_end;
        } while (decoded != rle_stride_probe);

        row_destination += row_step;
        --rows;
    } while (rows != 0u);
}
