#include "../include/xdb_alien.h"

static volatile xdb_i16 XDB_NEAR *state_word(
        xdb_alien_biased_state XDB_NEAR *state,
        xdb_u16 offset)
{
    return (volatile xdb_i16 XDB_NEAR *)
            ((volatile xdb_u8 XDB_NEAR *)state + offset);
}

void XDB_NEAR xdb_croolis_slot2_fade_update(
        xdb_alien_biased_state XDB_NEAR *state,
        xdb_alien_method_context XDB_NEAR *context)
{
    xdb_i16 duration = context->continuation.croolis_slot2.duration;
    xdb_i16 next_duration;

    *state_word(state, 0x01be) = duration;
    next_duration = (xdb_i16)(duration - 4);
    if (next_duration >= 0x0092) {
        context->continuation.croolis_slot2.duration = next_duration;
        xdb_croolis_slot2_motion_update(state, context);
        return;
    }

    context->continuation.croolis_slot2.duration = 0;
    xdb_croolis_slot2_active = 0;
    xdb_croolis_slot2_restart(state, context);
}
