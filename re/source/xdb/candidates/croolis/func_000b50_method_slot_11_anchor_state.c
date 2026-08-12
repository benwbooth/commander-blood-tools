#include "../include/xdb_alien.h"

void XDB_NEAR xdb_croolis_method_slot_11_anchor_state(
    const xdb_alien_method_context *context)
{
    volatile xdb_alien_state *state;
    volatile xdb_alien_biased_state *biased;

    state = context->state;
    biased = (volatile xdb_alien_biased_state *)
        ((volatile xdb_u8 *)state + XDB_ALIEN_CURSOR_BIAS);
    biased->field_052 = (xdb_i16)(biased->field_052 - XDB_ALIEN_FIELD_DELTA);
    xdb_croolis_slot11_cursor = (volatile xdb_u8 *)biased;
}
