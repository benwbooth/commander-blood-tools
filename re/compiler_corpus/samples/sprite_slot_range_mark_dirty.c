/* Codegen probe for BLOODPRG 0x004240. */
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
    u8 unused[30];
} sprite_slot_probe;

extern volatile sprite_slot_probe sprite_slot_table_probe[];

#if defined(__WATCOMC__)
#pragma aux sprite_slot_range_mark_dirty_probe parm [ax] [bx]
#endif

void FAR sprite_slot_range_mark_dirty_probe(u16 first_id, u16 last_id)
{
    volatile sprite_slot_probe *record;
    u16 remaining;
    sprite_flags_probe flags;

    remaining = (u16)(last_id - first_id + 1u);
    record = &sprite_slot_table_probe[first_id];
    while (remaining != 0u) {
        flags.word = record->flags;
        if ((flags.bytes.low & 0x80u) != 0u) {
            flags.bytes.low = (u8)((flags.bytes.low & 0x7eu) | 0x02u);
            record->flags = flags.word;
        }
        ++record;
        --remaining;
    }
}
