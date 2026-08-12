#include "../include/xdb_manu3.h"

void XDB_NEAR xdb_manu3_tween_constructor(
        volatile xdb_u16 XDB_NEAR *active_slot_cursor)
{
    volatile xdb_manu3_tween_spec XDB_NEAR *spec =
            (volatile xdb_manu3_tween_spec XDB_NEAR *)
            xdb_manu3_tween_script_offset;
    xdb_u16 count;

    for (;;) {
        volatile xdb_manu3_tween_record XDB_NEAR *record;
        volatile xdb_i16 XDB_NEAR *target;
        xdb_u16 target_offset;
        xdb_i16 current;
        xdb_i16 delta;
        xdb_i32 step;
        xdb_i32 accumulator;

        count = spec->count;
        if (count == 0u || spec->phase != (xdb_u8)xdb_manu3_tween_phase) {
            break;
        }

        record = (volatile xdb_manu3_tween_record XDB_NEAR *)
                *active_slot_cursor++;
        target_offset = spec->target_offset;
        record->target_offset = target_offset;
        target = (volatile xdb_i16 XDB_NEAR *)target_offset;
        current = *target;
        delta = (xdb_i16)((xdb_u16)spec->end_value - (xdb_u16)current);
        step = ((xdb_i32)delta * 65536L) / count;
        accumulator = (xdb_i32)(
                (xdb_u32)((xdb_i32)current * 65536L) + (xdb_u32)step);

        record->step = step;
        record->counter = (xdb_i16)(count - 1u);
        record->accumulator.raw = (xdb_u32)accumulator;
        ++spec;
    }

    xdb_manu3_tween_script_offset = (xdb_u16)spec;
    xdb_manu3_active_end_offset = (xdb_u16)active_slot_cursor;

    if (count != 0u) {
        ++xdb_manu3_tween_phase;
    } else if (active_slot_cursor == xdb_manu3_active_slot_offsets) {
        xdb_u16 cursor_delta = (xdb_u16)(xdb_manu3_cursor_x - 0x00a0u);

        cursor_delta = (xdb_u16)(cursor_delta << 1);
        xdb_manu3_finished_yaw = (xdb_u16)(
                xdb_manu3_view_yaw - cursor_delta);
        xdb_manu3_finished_pitch = xdb_manu3_view_pitch;
        xdb_manu3_tween_phase = 0x0100u;
    } else {
        ++xdb_manu3_tween_phase;
    }
}
