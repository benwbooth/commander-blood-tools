#include "../include/xdb_manu3.h"

void XDB_NEAR xdb_manu3_tween_step(void)
{
    volatile xdb_u16 XDB_NEAR *cursor;
    volatile xdb_u16 XDB_NEAR *end;

    if ((xdb_manu3_tween_phase & 0xff00u) != 0u) {
        return;
    }

    cursor = xdb_manu3_active_slot_offsets;
    end = (volatile xdb_u16 XDB_NEAR *)xdb_manu3_active_end_offset;
    while (cursor != end) {
        volatile xdb_manu3_tween_record XDB_NEAR *record =
                (volatile xdb_manu3_tween_record XDB_NEAR *)*cursor;
        volatile xdb_i16 XDB_NEAR *target =
                (volatile xdb_i16 XDB_NEAR *)record->target_offset;
        *target = record->accumulator.parts.whole;
        if (--record->counter < 0) {
            xdb_u16 replacement;

            --end;
            replacement = *end;
            *end = (xdb_u16)record;
            *cursor = replacement;
        } else {
            record->accumulator.raw += (xdb_u32)record->step;
            ++cursor;
        }
    }

    xdb_manu3_tween_constructor(end);
}
