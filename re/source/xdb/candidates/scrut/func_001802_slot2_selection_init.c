#include "../include/xdb_alien.h"

void XDB_NEAR xdb_scrut_slot2_selection_init(
        xdb_alien_biased_state XDB_NEAR *state,
        xdb_alien_method_context XDB_NEAR *context)
{
    xdb_scrut_slot2_selection_value = -1;
    xdb_scrut_slot2_active = 0;
    xdb_scrut_slot2_selection_restart(state, context);
}
