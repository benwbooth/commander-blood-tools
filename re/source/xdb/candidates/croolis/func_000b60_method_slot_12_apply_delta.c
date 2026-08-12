#include "../include/xdb_alien.h"

xdb_i16 XDB_NEAR xdb_croolis_method_slot_12_apply_delta(
    const xdb_alien_method_context XDB_NEAR *context)
{
    volatile xdb_alien_state XDB_NEAR *state;
    xdb_i16 delta;

    state = context->state;
    if ((delta = (xdb_i16)(xdb_alien_method_delta >> 1)) >= 0) {
        state->field_0b0 = (xdb_i16)(state->field_0b0 + delta);
    }
    return delta;
}
