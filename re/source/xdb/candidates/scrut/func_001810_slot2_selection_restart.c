#include "../include/xdb_alien.h"

static volatile xdb_i16 XDB_NEAR *state_word(
        xdb_alien_biased_state XDB_NEAR *state,
        xdb_u16 offset)
{
    return (volatile xdb_i16 XDB_NEAR *)
            ((volatile xdb_u8 XDB_NEAR *)state + offset);
}

void XDB_NEAR xdb_scrut_slot2_selection_restart(
        xdb_alien_biased_state XDB_NEAR *state,
        xdb_alien_method_context XDB_NEAR *context)
{
    state->callback = xdb_scrut_slot2_selection_begin;
    state->field_056 = *state_word(state, 0x36);
    xdb_scrut_slot2_selection_begin(state, context);
}
