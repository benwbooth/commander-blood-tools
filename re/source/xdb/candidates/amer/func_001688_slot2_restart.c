#include "../include/xdb_alien.h"

void XDB_NEAR xdb_amer_slot2_restart(
        xdb_alien_biased_state XDB_NEAR *state,
        xdb_alien_method_context XDB_NEAR *context)
{
    state->callback = xdb_amer_slot2_update;
    state->field_058 = 0x14;
    xdb_amer_slot2_update(state, context);
}
