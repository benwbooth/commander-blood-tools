/* Codegen probe for BLOODPRG 0x0042CD. */
typedef unsigned char u8;
typedef unsigned int u16;

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#define FAR far
#else
#define FAR
#endif

typedef struct sprite_slot_probe {
    u16 flags;
    u16 field_02;
    u16 data_offset;
    u16 data_segment;
    u16 draw_x;
    u16 draw_y;
    u16 extent_width;
    u16 extent_height;
    u8 unused[16];
} sprite_slot_probe;

typedef struct source_extent_probe {
    u16 width;
    u16 height;
} source_extent_probe;

typedef union sprite_flags_probe {
    u16 word;
    struct {
        u8 low;
        u8 high;
    } bytes;
} sprite_flags_probe;

extern volatile sprite_slot_probe sprite_slot_table_probe[];

#if defined(__WATCOMC__)
#pragma aux sprite_slot_extent_update_probe parm [ax] [cx] [dx] [es si]
#endif

void FAR sprite_slot_extent_update_probe(u16 object_id,
        u16 width,
        u16 height,
        const volatile source_extent_probe FAR *source_extent)
{
    volatile sprite_slot_probe *record;
    sprite_flags_probe flags;

    record = &sprite_slot_table_probe[object_id];
    flags.word = record->flags;
    if ((flags.bytes.low & 0x81u) != 0u) {
        if (width == source_extent->width && height == source_extent->height) {
            if ((flags.bytes.low & 0x10u) != 0u) {
                flags.bytes.low = (u8)((flags.bytes.low & 0xefu) | 0x02u);
            }
        } else if (width != record->extent_width ||
                height != record->extent_height) {
            flags.bytes.low = (u8)(flags.bytes.low | 0x12u);
            record->extent_width = width;
            record->extent_height = height;
        }
    }
    record->flags = flags.word;
}
