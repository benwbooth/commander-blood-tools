/* Codegen probe for the MANU3 phased tween constructor. */
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

typedef struct tween_spec {
    u8 count;
    u8 phase;
    u16 flags;
    u16 target_offset;
    i16 end_value;
} tween_spec;

typedef struct tween_record {
    u16 counter;
    u16 field_002;
    u16 target_offset;
    i32 accumulator;
    i32 step;
} tween_record;

extern volatile u16 cursor_x;
extern volatile u16 tween_phase;
extern volatile u16 tween_script_offset;
extern volatile u16 active_end_offset;
extern volatile u16 active_slot_offsets[];
extern volatile u16 finished_pitch;
extern volatile u16 finished_yaw;
extern volatile u16 view_pitch;
extern volatile u16 view_yaw;

void NEAR xdb_manu3_tween_constructor_probe(
        volatile u16 NEAR *active_slot_cursor);

#if defined(__WATCOMC__)
#pragma aux xdb_manu3_tween_constructor_probe \
        parm [bx] modify exact [ax bx cx dx si di bp]
#endif

void NEAR xdb_manu3_tween_constructor_probe(
        volatile u16 NEAR *active_slot_cursor)
{
    volatile tween_spec NEAR *spec =
            (volatile tween_spec NEAR *)tween_script_offset;
    u16 count;

    for (;;) {
        volatile tween_record NEAR *record;
        volatile i16 NEAR *target;
        u16 target_offset;
        i16 current;
        i16 delta;
        i32 step;
        i32 accumulator;

        count = spec->count;
        if (count == 0u || spec->phase != (u8)tween_phase) {
            break;
        }

        record = (volatile tween_record NEAR *)*active_slot_cursor++;
        target_offset = spec->target_offset;
        record->target_offset = target_offset;
        target = (volatile i16 NEAR *)target_offset;
        current = *target;
        delta = (i16)((u16)spec->end_value - (u16)current);
        step = ((i32)delta * 65536L) / count;
        accumulator = (i32)(
                (u32)((i32)current * 65536L) + (u32)step);

        record->step = step;
        record->counter = count - 1u;
        record->accumulator = accumulator;
        ++spec;
    }

    tween_script_offset = (u16)spec;
    active_end_offset = (u16)active_slot_cursor;

    if (count != 0u) {
        ++tween_phase;
    } else if (active_slot_cursor == active_slot_offsets) {
        u16 cursor_delta = (u16)(cursor_x - 0x00a0u);

        cursor_delta = (u16)(cursor_delta << 1);
        finished_yaw = (u16)(view_yaw - cursor_delta);
        finished_pitch = view_pitch;
        tween_phase = 0x0100u;
    } else {
        ++tween_phase;
    }
}
