/* Codegen probe for BLOODPRG 0x0041D1. */
typedef unsigned char u8;
typedef unsigned int u16;

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#define FAR far
#else
#define FAR
#endif

typedef struct entity_record_probe {
    u16 flags;
    u8 unused[30];
} entity_record_probe;

typedef union entity_flags_probe {
    u16 word;
    struct {
        u8 low;
        u8 high;
    } bytes;
} entity_flags_probe;

extern volatile entity_record_probe entity_table_probe[];

#if defined(__WATCOMC__)
#pragma aux entity_flag_state_transition_probe parm [ax]
#endif

void FAR entity_flag_state_transition_probe(u16 object_id)
{
    volatile entity_record_probe *record;
    entity_flags_probe flags;

    record = &entity_table_probe[object_id];
    flags.word = record->flags;
    if ((flags.bytes.low & 0x80u) != 0u &&
            (flags.bytes.low & 0x01u) != 0u) {
        flags.bytes.low = (u8)((flags.bytes.low & 0xfeu) | 0x02u);
    }
    record->flags = flags.word;
}
