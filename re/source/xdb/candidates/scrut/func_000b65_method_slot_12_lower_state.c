#include "../include/xdb_alien.h"

void XDB_NEAR xdb_scrut_method_slot_12_lower_state(
    const xdb_alien_method_context *context)
{
    volatile xdb_alien_state *state;
    volatile xdb_alien_biased_state *biased;

    state = context->state;
    biased = (volatile xdb_alien_biased_state *)
        ((volatile xdb_u8 *)state + XDB_ALIEN_CURSOR_BIAS);
    biased->field_052 = (xdb_i16)(biased->field_052 - XDB_ALIEN_FIELD_DELTA);
}
