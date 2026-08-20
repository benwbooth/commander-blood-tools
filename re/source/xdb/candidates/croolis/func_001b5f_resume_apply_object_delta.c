#include "../include/xdb_alien.h"

void XDB_NEAR xdb_croolis_resume_apply_object_delta(
        xdb_alien_method_context XDB_NEAR *context)
{
    xdb_i16 delta = (xdb_i8)context->continuation.resume_state.phase;
    volatile xdb_i16 XDB_FAR *object;
    xdb_u8 low;
    xdb_u8 high;

    object = XDB_FAR_AT(xdb_i16, xdb_alien_object_segment,
            (xdb_u16)(context->object_offset + 0x0002u));
    *object = (xdb_i16)((xdb_u16)*object + (xdb_u16)delta);
    object = XDB_FAR_AT(xdb_i16, xdb_alien_object_segment,
            (xdb_u16)(context->object_offset + 0x01f4u));
    *object = (xdb_i16)((xdb_u16)*object - (xdb_u16)delta);

    low = (xdb_u8)context->continuation.resume_state.phase;
    high = (xdb_u8)(context->continuation.resume_state.phase >> 8);
    high = (xdb_u8)(high + low);
    if (high == 0u) {
        low = 2u;
    }
    if ((xdb_i8)high >= 0x16) {
        low = 0xfeu;
    }
    context->continuation.resume_state.phase =
            (xdb_u16)(((xdb_u16)high << 8) | low);
}
