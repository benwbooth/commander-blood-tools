/* Codegen probe for the MANU3 active-tween stepper. */
typedef unsigned char u8;
typedef unsigned int u16;
typedef signed int i16;
typedef unsigned long u32;
typedef signed long i32;

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#define NEAR near
#else
#define NEAR
#endif

typedef union q16_value {
    u32 raw;
    struct {
        u16 fraction;
        i16 whole;
    } parts;
} q16_value;

typedef struct tween_record {
    i16 counter;
    u16 field_002;
    u16 target_offset;
    q16_value accumulator;
    i32 step;
} tween_record;

extern volatile u16 tween_phase;
extern volatile u16 active_end_offset;
extern volatile u16 active_slot_offsets[];
extern void NEAR xdb_manu3_tween_constructor_probe(
        volatile u16 NEAR *active_slot_cursor);

void NEAR xdb_manu3_tween_step_probe(void);

#if defined(__WATCOMC__)
#pragma aux xdb_manu3_tween_constructor_probe \
        parm [bx] modify exact [ax bx cx dx si di bp]
#pragma aux xdb_manu3_tween_step_probe \
        modify exact [ax bx cx dx si di bp]
#endif

void NEAR xdb_manu3_tween_step_probe(void)
{
    volatile u16 NEAR *cursor;
    volatile u16 NEAR *end;

    if ((tween_phase & 0xff00u) != 0u) {
        return;
    }

    cursor = active_slot_offsets;
    end = (volatile u16 NEAR *)active_end_offset;
    while (cursor != end) {
        volatile tween_record NEAR *record =
                (volatile tween_record NEAR *)*cursor;
        volatile i16 NEAR *target =
                (volatile i16 NEAR *)record->target_offset;
        *target = record->accumulator.parts.whole;
        if (--record->counter < 0) {
            u16 replacement;

            --end;
            replacement = *end;
            *end = (u16)record;
            *cursor = replacement;
        } else {
            record->accumulator.raw += (u32)record->step;
            ++cursor;
        }
    }

    xdb_manu3_tween_constructor_probe(end);
}
