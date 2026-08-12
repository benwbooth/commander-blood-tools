/* Codegen probe for BLOODPRG 0x0043F7. */
typedef unsigned char u8;
typedef unsigned int u16;

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#define FAR far
#else
#define FAR
#endif

typedef union sprite_flags_probe {
    u16 word;
    struct {
        u8 low;
        u8 high;
    } bytes;
} sprite_flags_probe;

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
    u8 unused[8];
} sprite_slot_probe;

typedef struct dirty_rect_probe {
    u16 left;
    u16 right;
    u16 top;
    u16 bottom;
} dirty_rect_probe;

extern volatile sprite_slot_probe sprite_slot_table_probe[];
extern volatile dirty_rect_probe clip_bounds_probe;
extern volatile u16 clip_snapshot_flags_probe;
extern volatile dirty_rect_probe dirty_rect_list_probe[];

#if defined(__WATCOMC__)
#pragma aux sprite_slot_commit_dirty_range_probe parm [ax] [bx]
#endif

void FAR sprite_slot_commit_dirty_range_probe(u16 first_id, u16 last_id)
{
    volatile sprite_slot_probe *record;
    u16 remaining;
    sprite_flags_probe flags;

    if ((clip_snapshot_flags_probe & 1u) != 0u) {
        dirty_rect_list_probe[0] = clip_bounds_probe;
        dirty_rect_list_probe[1].left = 0xffffu;
        clip_snapshot_flags_probe = 0;
        return;
    }

    remaining = (u16)(last_id - first_id + 1u);
    record = &sprite_slot_table_probe[first_id];
    while (remaining != 0u) {
        flags.word = record->flags;
        if ((flags.bytes.low & 0x02u) != 0u &&
                (flags.bytes.low & 0x01u) != 0u) {
            record->committed_draw_x = record->draw_x;
            record->committed_draw_y = record->draw_y;
            record->committed_extent_width = record->extent_width;
            record->committed_extent_height = record->extent_height;
        }
        ++record;
        --remaining;
    }
}
