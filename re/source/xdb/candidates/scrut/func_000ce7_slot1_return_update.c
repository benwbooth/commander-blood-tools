#include "../include/xdb_alien.h"

void XDB_NEAR xdb_scrut_slot1_return_update(
        xdb_alien_biased_state XDB_NEAR *state,
        xdb_alien_method_context XDB_NEAR *context)
{
    (void)context;
    state->field_054 = (xdb_i16)((xdb_u16)state->field_054 - 1u);
    if (state->field_054 != 0) {
        return;
    }
    state->callback = xdb_scrut_slot1_state_update;
}
