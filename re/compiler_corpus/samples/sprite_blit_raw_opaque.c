/* Codegen probe for BLOODPRG 0x004BA8. */
typedef unsigned char u8;
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

#if defined(__WATCOMC__)
#pragma aux sprite_blit_raw_opaque_probe parm [di] modify exact []
#endif

void NEAR sprite_blit_raw_opaque_probe(volatile sprite_slot_probe *record)
{
    const volatile sprite_frame_probe FAR *frame;
    const volatile u8 FAR *source;
    volatile u8 FAR *destination;
    volatile u8 FAR *row_destination;
    i16 sprite_left;
    i16 sprite_top;
    i16 sprite_right;
    i16 sprite_bottom;
    i16 destination_x;
    i16 destination_y;
    u16 draw_width;
    u16 draw_height;
    u16 frame_stride;
    u16 clipped;
    u16 source_row_skip;
    u16 rows;
    u16 columns;
    i16 destination_step;
    i16 row_step;

    frame = record->frame;
    sprite_top = (i16)(record->draw_y + (u16)frame->y_offset);
    sprite_right = (i16)(record->draw_x + record->extent_width +
            (u16)frame->x_offset);
    sprite_bottom = (i16)(record->draw_y + record->extent_height +
            (u16)frame->y_offset);
    frame_stride = frame->stride;
    source = (const volatile u8 FAR *)frame;
    draw_width = record->extent_width;
    draw_height = record->extent_height;
    destination_y = sprite_top;

    if (sprite_top < (i16)record->dirty_rect.top) {
        clipped = (u16)((i16)record->dirty_rect.top - sprite_top);
        draw_height = (u16)(draw_height - clipped);
        if ((sprite_flip_y_probe & 1u) == 0u) {
            source += (u16)(clipped * frame_stride);
        }
        destination_y = (i16)record->dirty_rect.top;
    }
    if (sprite_bottom >= (i16)record->dirty_rect.bottom) {
        clipped = (u16)(sprite_bottom - (i16)record->dirty_rect.bottom);
        draw_height = (u16)(draw_height - clipped);
        if ((sprite_flip_y_probe & 1u) != 0u) {
            source += (u16)(clipped * frame_stride);
        }
    }

    /* The original reloads +4 through the vertically adjusted source cursor. */
    sprite_left = (i16)(record->draw_x + (u16)
            ((const volatile sprite_frame_probe FAR *)source)->x_offset);
    destination_x = sprite_left;
    if (sprite_left < (i16)record->dirty_rect.left) {
        clipped = (u16)((i16)record->dirty_rect.left - sprite_left);
        draw_width = (u16)(draw_width - clipped);
        if ((sprite_flip_x_probe & 1u) == 0u) {
            source += clipped;
        }
        destination_x = (i16)record->dirty_rect.left;
    }
    if (sprite_right >= (i16)record->dirty_rect.right) {
        clipped = (u16)(sprite_right - (i16)record->dirty_rect.right);
        draw_width = (u16)(draw_width - clipped);
        if ((sprite_flip_x_probe & 1u) != 0u) {
            source += clipped;
        }
    }

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
    } else {
        destination_step = 1;
    }
    source += 8u;
    row_destination = display_buffer_probe +
            (u16)((u16)destination_y * 320u + (u16)destination_x);
    source_row_skip = (u16)(frame_stride - draw_width);

    rows = draw_height;
    do {
        destination = row_destination;
        columns = draw_width;
        do {
            *destination = *source++;
            destination += destination_step;
            --columns;
        } while (columns != 0u);

        source += source_row_skip;
        row_destination += row_step;
        --rows;
    } while (rows != 0u);
}
