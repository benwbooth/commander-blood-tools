/* Codegen probe for BLOODPRG 0x004F62. */
typedef unsigned char u8;
typedef unsigned int u16;
typedef signed int i16;
typedef unsigned long u32;

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

extern volatile u8 FAR *display_buffer_probe;

#if defined(__WATCOMC__)
#pragma aux sprite_blit_scaled_transparent_probe parm [di] modify exact []
#endif

void NEAR sprite_blit_scaled_transparent_probe(
        volatile sprite_slot_probe *record)
{
    const volatile sprite_frame_probe FAR *frame;
    const volatile u8 FAR *pixels;
    const volatile u8 FAR *source_row;
    volatile u8 FAR *destination;
    u32 x_step;
    u32 y_step;
    u32 x_start;
    u32 x_position;
    u32 y_position;
    u16 source_width;
    u16 source_height;
    i16 destination_x;
    i16 destination_y;
    i16 draw_width;
    i16 draw_height;
    u16 clipped;
    u16 rows;
    u16 columns;
    u8 pixel;

    frame = record->frame;
    source_width = frame->stride;
    if (record->extent_width == 0u) {
        return;
    }
    x_step = ((u32)source_width << 16) / record->extent_width;

    source_height = frame->height;
    if (record->extent_height == 0u) {
        return;
    }
    y_step = ((u32)source_height << 16) / record->extent_height;
    x_start = 0u;
    y_position = 0u;
    destination_x = (i16)record->draw_x;
    destination_y = (i16)record->draw_y;
    draw_width = (i16)record->extent_width;
    draw_height = (i16)record->extent_height;

    if (destination_y < (i16)record->dirty_rect.top) {
        clipped = (u16)((i16)record->dirty_rect.top - destination_y);
        draw_height = (i16)(draw_height - (i16)clipped);
        y_position = (u32)clipped * y_step;
        destination_y = (i16)record->dirty_rect.top;
    }
    if ((i16)(record->draw_y + record->extent_height) >=
            (i16)record->dirty_rect.bottom) {
        clipped = (u16)((i16)(record->draw_y +
                record->extent_height) - (i16)record->dirty_rect.bottom);
        draw_height = (i16)(draw_height - (i16)clipped);
    }

    if (destination_x < (i16)record->dirty_rect.left) {
        clipped = (u16)((i16)record->dirty_rect.left - destination_x);
        draw_width = (i16)(draw_width - (i16)clipped);
        x_start = (u32)clipped * x_step;
        destination_x = (i16)record->dirty_rect.left;
    }
    if ((i16)(record->draw_x + record->extent_width) >=
            (i16)record->dirty_rect.right) {
        clipped = (u16)((i16)(record->draw_x +
                record->extent_width) - (i16)record->dirty_rect.right);
        draw_width = (i16)(draw_width - (i16)clipped);
    }

    if (draw_width <= 0 || draw_height <= 0) {
        return;
    }

    pixels = (const volatile u8 FAR *)frame + 8u;
    destination = display_buffer_probe +
            (u16)((u16)destination_y * 320u + (u16)destination_x);
    rows = (u16)draw_height;
    do {
        source_row = pixels + (u16)((y_position >> 16) * source_width);
        x_position = x_start;
        columns = (u16)draw_width;
        do {
            pixel = source_row[(u16)(x_position >> 16)];
            if (pixel != 0u) {
                *destination = pixel;
            }
            ++destination;
            x_position += x_step;
            --columns;
        } while (columns != 0u);

        destination += (u16)(320u - (u16)draw_width);
        y_position += y_step;
        --rows;
    } while (rows != 0u);
}
