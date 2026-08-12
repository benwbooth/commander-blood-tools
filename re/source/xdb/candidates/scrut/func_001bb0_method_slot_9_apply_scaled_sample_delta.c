#include "../include/xdb_alien.h"

xdb_i16 XDB_NEAR xdb_scrut_method_slot_9_apply_scaled_sample_delta(
        xdb_alien_method_context XDB_NEAR *context)
{
    xdb_u16 cursor = context->continuation.sample_state.cursor;
    xdb_i16 previous = context->continuation.sample_state.previous;
    xdb_i16 current;
    xdb_i16 delta;
    xdb_alien_object_record XDB_FAR *object;
    xdb_u16 count;

    current = *(volatile xdb_i16 XDB_NEAR *)(xdb_alien_motion_samples + cursor);
    current >>= 4;
    context->continuation.sample_state.cursor = (cursor + 4u) & 0x0ffcu;
    context->continuation.sample_state.previous = current;
    delta = (xdb_i16)((xdb_u16)current - (xdb_u16)previous);

    object = XDB_FAR_AT(
            xdb_alien_object_record,
            xdb_alien_object_segment,
            context->object_offset);
    count = context->object_count;
    do {
        object->position = (xdb_i16)(
                (xdb_u16)object->position + (xdb_u16)delta);
        ++object;
    } while (--count != 0u);

    return delta;
}
