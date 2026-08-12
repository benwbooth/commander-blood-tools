/* Codegen probe for BLOODPRG 0x004471. */
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

typedef struct sprite_slot_probe {
    u16 flags;
    u16 field_02;
    u16 data_offset;
    u16 data_segment;
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

typedef void NEAR sprite_blitter_probe(volatile sprite_slot_probe *record);

#if defined(__WATCOMC__)
#pragma aux sprite_blitter_probe parm [di] modify exact []
#endif

extern volatile sprite_slot_probe sprite_slot_table_probe[];
extern volatile dirty_rect_probe dirty_rect_list_probe[];
extern sprite_blitter_probe *sprite_blitter_table_probe[8];
extern sprite_blitter_probe *selected_sprite_blitter_probe;
extern volatile u8 sprite_flip_x_probe;
extern volatile u8 sprite_flip_y_probe;

#if defined(__WATCOMC__)
#pragma aux sprite_slot_dirty_range_render_probe parm [ax] [bx]
#endif

void FAR sprite_slot_dirty_range_render_probe(u16 first_id, u16 last_id)
{
    volatile sprite_slot_probe *record;
    volatile dirty_rect_probe *dirty_rect;
    u16 remaining;
    u16 flags;
    u16 slot_right;
    u16 slot_bottom;

    if ((i16)dirty_rect_list_probe[0].left < 0) {
        return;
    }

    remaining = (u16)(last_id - first_id + 1u);
    record = &sprite_slot_table_probe[(u16)(last_id + 1u)];
    do {
        --record;
        flags = record->flags;
        if ((flags & 1u) != 0u) {
            selected_sprite_blitter_probe =
                    sprite_blitter_table_probe[(flags >> 2) & 7u];
            sprite_flip_x_probe = (u8)((flags & 0x0020u) != 0u);
            sprite_flip_y_probe = (u8)((flags & 0x0040u) != 0u);

            slot_right = (u16)(record->draw_x + record->extent_width);
            slot_bottom = (u16)(record->draw_y + record->extent_height);
            dirty_rect = &dirty_rect_list_probe[0];
            do {
                record->dirty_rect = *dirty_rect;
                if ((i16)record->draw_x < (i16)dirty_rect->right &&
                        (i16)record->draw_y < (i16)dirty_rect->bottom &&
                        (i16)slot_right > (i16)dirty_rect->left &&
                        (i16)slot_bottom > (i16)dirty_rect->top) {
                    selected_sprite_blitter_probe(record);
                }
                ++dirty_rect;
            } while ((i16)dirty_rect->left >= 0);
        }

        record->flags = (u16)(record->flags & ~2u);
        --remaining;
    } while (remaining != 0u);
}
