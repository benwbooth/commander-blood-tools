#include "../include/xdb_alien.h"

void XDB_NEAR xdb_scrut_slot2_active_reset_setup(
        xdb_alien_biased_state XDB_NEAR *state,
        xdb_alien_method_context XDB_NEAR *context)
{
    xdb_scrut_slot2_active = 1;
    xdb_scrut_slot2_selection_value = 0;
    xdb_scrut_slot2_reset_or_camera(state, context);
}
