#include "../include/xdb_alien.h"

void XDB_NEAR xdb_amer_method_slot_12_apply_delta(
    const xdb_alien_method_context *context)
{
    volatile xdb_alien_state *state;
    xdb_i16 delta;

    state = context->state;
    delta = (xdb_i16)(xdb_alien_method_delta >> 1);
    if (delta >= 0) {
        state->field_0b0 = (xdb_i16)(state->field_0b0 + delta);
    }
}
