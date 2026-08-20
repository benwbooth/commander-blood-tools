#include "../include/xdb_alien.h"

static volatile xdb_i16 XDB_NEAR *state_word(
        xdb_alien_biased_state XDB_NEAR *state,
        xdb_u16 offset)
{
    return (volatile xdb_i16 XDB_NEAR *)
            ((volatile xdb_u8 XDB_NEAR *)state + offset);
}

void XDB_NEAR xdb_scrut_slot2_selection_reset_restart(
        xdb_alien_biased_state XDB_NEAR *state,
        xdb_alien_method_context XDB_NEAR *context)
{
    *state_word(state, 0x0332) = *state_word(state, 0x0346);
    *state_word(state, 0x033a) = *state_word(state, 0x034a);
    *state_word(state, 0x0390) = *state_word(state, 0x03a4);
    *state_word(state, 0x0398) = *state_word(state, 0x03a8);
    xdb_scrut_slot2_active = 0;
    state->field_058 = 0;
    state->ring_offset = 0;
    xdb_scrut_slot2_restart(state, context);
}
