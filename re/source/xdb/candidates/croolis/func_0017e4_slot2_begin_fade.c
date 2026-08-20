#include "../include/xdb_alien.h"

static volatile xdb_i16 XDB_NEAR *state_word(
        xdb_alien_biased_state XDB_NEAR *state,
        xdb_u16 offset)
{
    return (volatile xdb_i16 XDB_NEAR *)
            ((volatile xdb_u8 XDB_NEAR *)state + offset);
}

void XDB_NEAR xdb_croolis_slot2_begin_fade(
        xdb_alien_biased_state XDB_NEAR *state,
        xdb_alien_method_context XDB_NEAR *context)
{
    *state_word(state, 0x46) = (xdb_i16)(
            (xdb_u16)*state_word(state, 0x46) - 0x1eu);
    context->continuation.croolis_slot2.duration = 0x00b2;
    state->callback = xdb_croolis_slot2_fade_update;
    xdb_croolis_slot2_fade_update(state, context);
}
