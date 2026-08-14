/* Codegen probe for BLOODPRG 0x0082C3. */
typedef unsigned int u16;
typedef signed int i16;

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#define FAR far
#define NEAR near
#else
#define FAR
#define NEAR
#endif

typedef struct rect_i16_probe {
    i16 x;
    i16 y;
    i16 width;
    i16 height;
} rect_i16_probe;

typedef struct entity_record_probe {
    u16 flags;
    u16 field_02;
    u16 frame_offset;
    u16 frame_segment;
    i16 draw_x;
    i16 draw_y;
    i16 extent_width;
    i16 extent_height;
    u16 unused[8];
} entity_record_probe;

extern volatile entity_record_probe entity_table_probe[];
extern int FAR region_record_hittest_probe(
        const volatile rect_i16_probe NEAR *rect);

#if defined(__WATCOMC__)
#pragma aux ui_region_31_poll_probe value [ax] modify exact [ax]
#endif

i16 FAR ui_region_31_poll_probe(void)
{
    i16 attempts_remaining;
    volatile entity_record_probe *record;

    attempts_remaining = 31;
    record = &entity_table_probe[31];

    do {
        if ((record->flags & 1u) != 0u &&
                region_record_hittest_probe(
                    (const volatile rect_i16_probe NEAR *)&record->draw_x)) {
            return attempts_remaining;
        }
        --attempts_remaining;
    } while (attempts_remaining >= 0);

    return -1;
}
