/* Codegen probe for BLOODPRG 0x00420D. */
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
    u8 unused[20];
} sprite_slot_probe;

typedef union sprite_flags_probe {
    u16 word;
    struct {
        u8 low;
        u8 high;
    } bytes;
} sprite_flags_probe;

extern volatile sprite_slot_probe sprite_slot_table_probe[];

#if defined(__WATCOMC__)
#pragma aux sprite_slot_position_update_probe parm [ax] [bx] [cx]
#endif

void FAR sprite_slot_position_update_probe(u16 object_id, u16 draw_x, u16 draw_y)
{
    volatile sprite_slot_probe *record;
    sprite_flags_probe flags;

    record = &sprite_slot_table_probe[object_id];
    flags.word = record->flags;
    if ((flags.bytes.low & 0x81u) != 0u) {
        if (record->draw_x != draw_x) {
            flags.bytes.low = (u8)(flags.bytes.low | 0x02u);
            record->draw_x = draw_x;
        }
        if (record->draw_y != draw_y) {
            flags.bytes.low = (u8)(flags.bytes.low | 0x02u);
            record->draw_y = draw_y;
        }
    }
    record->flags = flags.word;
}
