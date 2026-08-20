#include "../include/xdb_alien.h"

void XDB_NEAR xdb_amer_slot2_selection_wait(
        xdb_alien_biased_state XDB_NEAR *state,
        xdb_alien_method_context XDB_NEAR *context)
{
    state->field_058 = 0x28;
    state->callback = xdb_amer_slot2_selection_update;
    xdb_amer_slot2_selection_update(state, context);
}
