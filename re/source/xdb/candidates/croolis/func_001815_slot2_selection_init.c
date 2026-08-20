#include "../include/xdb_alien.h"

void XDB_NEAR xdb_croolis_slot2_selection_init(
        xdb_alien_biased_state XDB_NEAR *state,
        xdb_alien_method_context XDB_NEAR *context)
{
    state->callback = xdb_croolis_slot2_selection_update;
    xdb_croolis_slot2_selected_state = NULL;
    xdb_croolis_slot2_active = 0;
    xdb_croolis_slot2_selection_update(state, context);
}
